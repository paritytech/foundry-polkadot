use std::time::Duration;

use crate::utils::{unwrap_response, TestNode};
use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_primitives::ruint::aliases::U256;
use alloy_rpc_types::TransactionRequest;
use alloy_serde::WithOtherFields;
use anvil_core::eth::EthRequest;
use anvil_polkadot::{
    api_server::convert::from_h160_to_address,
    config::{AnvilNodeConfig, SubstrateNodeConfig},
};
use anvil_rpc::{
    error::{ErrorCode, RpcError},
    response::ResponseResult,
};
use assert_matches::assert_matches;
use polkadot_sdk::{
    pallet_revive::evm::Account,
    sp_core::{H256, U256 as SU256},
};

#[tokio::test(flavor = "multi_thread")]
async fn test_chain_id() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    assert_eq!(node.best_block_number().await, 0);

    let default_chain_id = 420_420_420u64;

    assert_eq!(
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthChainId(())).await.unwrap()).unwrap(),
        "0x190f1b44",
    );

    assert_eq!(
        unwrap_response::<u64>(node.eth_rpc(EthRequest::EthNetworkId(())).await.unwrap()).unwrap(),
        420_420_420u64,
    );

    unwrap_response::<()>(node.eth_rpc(EthRequest::SetChainId(10)).await.unwrap()).unwrap();

    assert_eq!(
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthChainId(())).await.unwrap()).unwrap(),
        "0xa",
    );

    assert_eq!(
        unwrap_response::<u64>(node.eth_rpc(EthRequest::EthNetworkId(())).await.unwrap()).unwrap(),
        10u64,
    );

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(1)), None)).await.unwrap())
        .unwrap();

    assert_eq!(
        unwrap_response::<String>(node.eth_rpc(EthRequest::EthChainId(())).await.unwrap()).unwrap(),
        "0xa",
    );

    let fr = from_h160_to_address(Account::from(subxt_signer::eth::dev::alith()).address());
    let to = from_h160_to_address(Account::from(subxt_signer::eth::dev::baltathar()).address());
    let mut tx = TransactionRequest::default().value(U256::from(100)).from(fr).to(to);

    // Set the old chain id, the transaction will be rejected.
    tx.chain_id = Some(default_chain_id);

    assert_matches!(
        node
            .eth_rpc(EthRequest::EthSendTransaction(Box::new(WithOtherFields::new(tx))))
            .await
            .unwrap(),
        ResponseResult::Error(RpcError {code, message, ..}) => {
            assert_eq!(code, ErrorCode::InternalError);
            message.contains("Invalid Transaction")
        }
    );

    let tx = TransactionRequest::default().value(U256::from(100)).from(fr).to(to);

    let tx_hash = unwrap_response::<H256>(
        node.eth_rpc(EthRequest::EthSendTransaction(Box::new(WithOtherFields::new(tx))))
            .await
            .unwrap(),
    )
    .unwrap();

    // TODO: check that the transaction is in block.
}

#[tokio::test(flavor = "multi_thread")]
async fn test_nonce() {
    let anvil_node_config = AnvilNodeConfig::test_config();
    let substrate_node_config = SubstrateNodeConfig::new(&anvil_node_config);
    let mut node = TestNode::new(anvil_node_config, substrate_node_config).await.unwrap();

    assert_eq!(node.best_block_number().await, 0);

    let address = from_h160_to_address(Account::from(subxt_signer::eth::dev::alith()).address());

    assert_eq!(
        unwrap_response::<SU256>(
            node.eth_rpc(EthRequest::EthGetTransactionCount(address.clone(), None)).await.unwrap()
        )
        .unwrap(),
        SU256::from(0)
    );

    unwrap_response::<()>(
        node.eth_rpc(EthRequest::SetNonce(address.clone(), U256::from(1))).await.unwrap(),
    )
    .unwrap();

    assert_eq!(
        unwrap_response::<SU256>(
            node.eth_rpc(EthRequest::EthGetTransactionCount(address.clone(), None)).await.unwrap()
        )
        .unwrap(),
        SU256::from(1)
    );

    unwrap_response::<()>(node.eth_rpc(EthRequest::Mine(Some(U256::from(1)), None)).await.unwrap())
        .unwrap();

    assert_eq!(
        unwrap_response::<SU256>(
            node.eth_rpc(EthRequest::EthGetTransactionCount(address.clone(), None)).await.unwrap()
        )
        .unwrap(),
        SU256::from(1)
    );

    let to = from_h160_to_address(Account::from(subxt_signer::eth::dev::baltathar()).address());
    let tx = TransactionRequest::default().value(U256::from(100)).from(address).to(to).nonce(1);

    let tx_hash = unwrap_response::<H256>(
        node.eth_rpc(EthRequest::EthSendTransaction(Box::new(WithOtherFields::new(tx))))
            .await
            .unwrap(),
    )
    .unwrap();

    // TODO: check that the transaction is in block.

    // Set nonce for a non-existant account. Should work.
    let address = from_h160_to_address(Account::from(subxt_signer::eth::dev::dorothy()).address());

    assert_eq!(
        unwrap_response::<SU256>(
            node.eth_rpc(EthRequest::EthGetTransactionCount(address.clone(), None)).await.unwrap()
        )
        .unwrap(),
        SU256::from(0)
    );

    unwrap_response::<()>(
        node.eth_rpc(EthRequest::SetNonce(address.clone(), U256::from(1))).await.unwrap(),
    )
    .unwrap();

    assert_eq!(
        unwrap_response::<SU256>(
            node.eth_rpc(EthRequest::EthGetTransactionCount(
                address.clone(),
                Some(BlockId::Number(BlockNumberOrTag::Number(1)))
            ))
            .await
            .unwrap()
        )
        .unwrap(),
        SU256::from(1)
    );
}
