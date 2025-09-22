use std::time::Duration;

use crate::{
    api_server::{
        convert::{
            from_alloy_u256_to_sp_u256, from_sp_u256_to_alloy_u256, to_block_number_or_tag_or_hash,
            try_convert_transaction_request,
        },
        error::{Error, Result},
        ApiServer,
    },
    macros::node_info,
};
use alloy_primitives::{B256, U256, U64};
use alloy_rpc_types::TransactionRequest;
use alloy_serde::WithOtherFields;
use parity_scale_codec::{Decode, DecodeLimit, Encode};
use polkadot_sdk::{
    frame_support::MAX_EXTRINSIC_DEPTH,
    pallet_revive,
    pallet_revive::evm::{
        Block, BlockNumberOrTagOrHash, BlockTag, Bytes, GenericTransaction, ReceiptInfo,
        TransactionSigned,
    },
    pallet_revive_eth_rpc::{subxt_client, EthRpcError},
    sc_service::TransactionPool,
    sp_core::{self, keccak_256, H256, U256 as SU256},
    sp_runtime,
};
use substrate_runtime::{RuntimeCall, UncheckedExtrinsic};
use subxt::utils::H160;

impl ApiServer {
    pub(crate) fn eth_chain_id(&self) -> Result<U64> {
        node_info!("eth_chainId");
        Ok(U256::from(self.eth_rpc_client.chain_id()).to::<U64>())
    }

    pub(crate) fn network_id(&self) -> Result<u64> {
        node_info!("eth_networkId");
        Ok(self.eth_rpc_client.chain_id())
    }

    pub(crate) fn net_listening(&self) -> Result<bool> {
        node_info!("net_listening");
        Ok(true)
    }

    pub(crate) fn syncing(&self) -> Result<bool> {
        node_info!("eth_syncing");
        Ok(false)
    }

    pub(crate) async fn transaction_receipt(&self, tx_hash: B256) -> Result<Option<ReceiptInfo>> {
        node_info!("eth_getTransactionReceipt");
        // TODO: do we really need to return Ok(None) if the transaction is still in the pool?
        Ok(self.eth_rpc_client.receipt(&(tx_hash.0.into())).await)
    }

    pub(crate) async fn get_balance(
        &self,
        address: H160,
        block: BlockNumberOrTagOrHash,
    ) -> Result<U256> {
        node_info!("eth_getBalance");
        let hash =
            self.eth_rpc_client.block_hash_for_tag(block).await.map_err(Error::ClientError)?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        let balance = runtime_api.balance(address).await.map_err(Error::ClientError)?;
        Ok(from_sp_u256_to_alloy_u256(balance))
    }

    pub(crate) async fn get_storage_at(
        &self,
        address: H160,
        storage_slot: sp_core::U256,
        block: BlockNumberOrTagOrHash,
    ) -> Result<Bytes> {
        let hash =
            self.eth_rpc_client.block_hash_for_tag(block).await.map_err(Error::ClientError)?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        let bytes = runtime_api
            .get_storage(address, storage_slot.to_big_endian())
            .await
            .map_err(Error::ClientError)?;
        Ok(bytes.unwrap_or_default().into())
    }

    pub(crate) async fn get_code(
        &self,
        address: H160,
        block: BlockNumberOrTagOrHash,
    ) -> Result<Bytes> {
        let hash =
            self.eth_rpc_client.block_hash_for_tag(block).await.map_err(Error::ClientError)?;
        let code = self
            .eth_rpc_client
            .runtime_api(hash)
            .code(address)
            .await
            .map_err(Error::ClientError)?;
        Ok(code.into())
    }

    pub(crate) async fn get_block_by_hash(
        &self,
        block_hash: H256,
        hydrated_transactions: bool,
    ) -> Result<Option<Block>> {
        let Some(block) =
            self.eth_rpc_client.block_by_hash(&block_hash).await.map_err(Error::ClientError)?
        else {
            return Ok(None);
        };
        let block = self.eth_rpc_client.evm_block(block, hydrated_transactions).await;
        Ok(Some(block))
    }

    pub(crate) async fn send_raw_transaction(&self, transaction: Bytes) -> Result<H256> {
        let hash = H256(keccak_256(&transaction.0));
        let call = subxt_client::tx().revive().eth_transact(transaction.0);
        self.eth_rpc_client
            .submit(call)
            .await
            .map_err(|err| {
                node_info!("submit call failed: {err:?}");
                err
            })
            .map_err(Error::ClientError)?;

        node_info!("send_raw_transaction hash: {hash:?}");
        Ok(hash)
    }

    pub(crate) async fn send_transaction(
        &self,
        mut transaction_req: WithOtherFields<TransactionRequest>,
    ) -> Result<H256> {
        let mut transaction = try_convert_transaction_request(transaction_req.clone().into_inner());
        node_info!("{transaction:#?}");

        let Some(from) = transaction.from else {
            node_info!("Transaction must have a sender");
            return Err(Error::EthRpcError(EthRpcError::InvalidTransaction));
        };

        let account = self
            .wallet
            .accounts
            .iter()
            .find(|account| account.address() == from)
            .ok_or(Error::EthRpcError(EthRpcError::AccountNotFound(from)))?;

        if transaction.gas.is_none() {
            transaction.gas = Some(self.estimate_gas(transaction_req.clone(), None).await?);
        }

        if transaction.gas_price.is_none() {
            transaction.gas_price = Some(self.gas_price().await?);
        }

        if transaction.nonce.is_none() {
            transaction.nonce =
                Some(self.get_transaction_count(from, BlockTag::Latest.into()).await?);
        }

        if transaction.chain_id.is_none() {
            transaction.chain_id =
                Some(from_alloy_u256_to_sp_u256(U256::from(self.eth_chain_id()?)));
        }

        let tx = transaction
            .try_into_unsigned()
            .map_err(|_| Error::EthRpcError(EthRpcError::InvalidTransaction))?;
        let payload = account.sign_transaction(tx).signed_payload();

        let hash = self.send_raw_transaction(Bytes(payload)).await?;

        tokio::time::sleep(Duration::from_secs(1)).await;
        for tx in self.tx_pool.ready() {
            // let data = <Vec<u8> as Decode>::decode(&mut &(tx.data.encode()[..])).unwrap();

            let ext = UncheckedExtrinsic::decode_all_with_depth_limit(
                MAX_EXTRINSIC_DEPTH,
                &mut &(tx.data.encode()[..]),
            )
            .unwrap()
            .0;

            if let sp_runtime::generic::UncheckedExtrinsic {
                function: RuntimeCall::Revive(pallet_revive::Call::eth_transact { payload }),
                ..
            } = ext
            {
                let tx = TransactionSigned::decode(&payload.to_vec()).unwrap();

                panic!("{:#?}", tx);
            }
        }

        Ok(hash)
    }

    pub(crate) async fn estimate_gas(
        &self,
        request: WithOtherFields<TransactionRequest>,
        block: Option<alloy_rpc_types::BlockId>,
    ) -> Result<SU256> {
        node_info!("eth_estimateGas");

        let hash = self
            .eth_rpc_client
            .block_hash_for_tag(to_block_number_or_tag_or_hash(block))
            .await
            .map_err(Error::ClientError)?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        let dry_run = runtime_api
            .dry_run(try_convert_transaction_request(request.into_inner()))
            .await
            .map_err(Error::ClientError)?;
        Ok(dry_run.eth_gas)
    }

    async fn gas_price(&self) -> Result<SU256> {
        let hash = self
            .eth_rpc_client
            .block_hash_for_tag(BlockTag::Latest.into())
            .await
            .map_err(Error::ClientError)?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        Ok(runtime_api.gas_price().await.map_err(Error::ClientError)?)
    }

    async fn get_transaction_count(
        &self,
        address: H160,
        block: BlockNumberOrTagOrHash,
    ) -> Result<SU256> {
        let hash =
            self.eth_rpc_client.block_hash_for_tag(block).await.map_err(Error::ClientError)?;
        let runtime_api = self.eth_rpc_client.runtime_api(hash);
        let nonce = runtime_api.nonce(address).await.map_err(Error::ClientError)?;
        Ok(nonce)
    }
}
