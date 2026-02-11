use jsonrpsee::{core::ClientError, http_client::HttpClient};
use polkadot_sdk::{
    sc_chain_spec,
    sp_api::__private::HeaderT,
    sp_core::H256,
    sp_rpc::{list::ListOrValue, number::NumberOrHex},
    sp_runtime::{generic::SignedBlock, traits::Block as BlockT},
    sp_state_machine,
    sp_storage::{ChildInfo, StorageData, StorageKey},
    substrate_rpc_client,
};
use serde::de::DeserializeOwned;
use std::{
    marker::PhantomData,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::runtime::Handle;

type BlockNumber = u64;

pub trait RPCClient<Block: BlockT + DeserializeOwned>: Send + Sync + std::fmt::Debug {
    fn system_chain(&self) -> Result<String, jsonrpsee::core::ClientError>;

    fn system_properties(&self) -> Result<sc_chain_spec::Properties, jsonrpsee::core::ClientError>;

    fn block(
        &self,
        hash: Option<Block::Hash>,
    ) -> Result<Option<SignedBlock<Block>>, jsonrpsee::core::ClientError>;

    fn block_hash(
        &self,
        block_number: Option<<Block::Header as HeaderT>::Number>,
    ) -> Result<Option<Block::Hash>, jsonrpsee::core::ClientError>;

    fn header(
        &self,
        hash: Option<Block::Hash>,
    ) -> Result<Option<Block::Header>, jsonrpsee::core::ClientError>;

    fn storage(
        &self,
        key: StorageKey,
        at: Option<Block::Hash>,
    ) -> Result<Option<StorageData>, jsonrpsee::core::ClientError>;

    fn storage_hash(
        &self,
        key: StorageKey,
        at: Option<Block::Hash>,
    ) -> Result<Option<Block::Hash>, jsonrpsee::core::ClientError>;

    fn storage_keys_paged(
        &self,
        key: Option<StorageKey>,
        count: u32,
        start_key: Option<StorageKey>,
        at: Option<Block::Hash>,
    ) -> Result<Vec<sp_state_machine::StorageKey>, ClientError>;

    fn child_storage(
        &self,
        child_info: &ChildInfo,
        key: StorageKey,
        at: Option<Block::Hash>,
    ) -> Result<Option<StorageData>, ClientError>;

    fn child_storage_hash(
        &self,
        child_info: &ChildInfo,
        key: StorageKey,
        at: Option<Block::Hash>,
    ) -> Result<Option<Block::Hash>, ClientError>;

    fn child_storage_keys_paged(
        &self,
        child_info: &ChildInfo,
        key: Option<StorageKey>,
        count: u32,
        start_key: Option<StorageKey>,
        at: Option<Block::Hash>,
    ) -> Result<Vec<sp_state_machine::StorageKey>, ClientError>;
}

#[derive(Debug, Clone)]
pub struct Rpc<Block: BlockT + DeserializeOwned> {
    http_client: HttpClient,
    delay_between_requests_ms: u32,
    counter: Arc<AtomicU64>,
    block_type: PhantomData<Block>,
}

impl<Block: BlockT + DeserializeOwned> Rpc<Block> {
    pub fn new(http_client: HttpClient, delay_between_requests_ms: u32) -> Self {
        Self {
            http_client,
            delay_between_requests_ms,
            counter: Default::default(),
            block_type: PhantomData,
        }
    }

    fn block_on<F, T, E>(&self, future: F) -> Result<T, E>
    where
        F: std::future::Future<Output = Result<T, E>> + Send,
        T: Send,
        E: Send,
    {
        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        let start = std::time::Instant::now();
        let delay_between_requests = Duration::from_millis(self.delay_between_requests_ms.into());

        tokio::task::block_in_place(move || {
            Handle::current().block_on(async move {
                let start_req = std::time::Instant::now();
                tracing::debug!(
                    target: super::LAZY_LOADING_LOG_TARGET,
                    "Sending request: {}",
                    id
                );

                // Explicit request delay, to avoid getting 429 errors
                let _ = tokio::time::sleep(delay_between_requests).await;

                // Execute the request
                let result = future.await;

                tracing::debug!(
                    target: super::LAZY_LOADING_LOG_TARGET,
                    "Completed request (id: {}, successful: {}, elapsed_time: {:?}, query_time: {:?})",
                    id,
                    result.is_ok(),
                    start.elapsed(),
                    start_req.elapsed()
                );

                result
            })
        })
    }
}

impl<Block: BlockT + DeserializeOwned> RPCClient<Block> for Rpc<Block> {
    fn system_chain(&self) -> Result<String, jsonrpsee::core::ClientError> {
        let client = self.http_client.clone();
        self.block_on(async move {
            substrate_rpc_client::SystemApi::<H256, BlockNumber>::system_chain(&client).await
        })
    }

    fn system_properties(&self) -> Result<sc_chain_spec::Properties, jsonrpsee::core::ClientError> {
        let client = self.http_client.clone();
        self.block_on(async move {
            substrate_rpc_client::SystemApi::<H256, BlockNumber>::system_properties(&client).await
        })
    }

    fn block(
        &self,
        hash: Option<Block::Hash>,
    ) -> Result<Option<SignedBlock<Block>>, jsonrpsee::core::ClientError> {
        let client = self.http_client.clone();
        self.block_on(async move {
            substrate_rpc_client::ChainApi::<
                BlockNumber,
                Block::Hash,
                Block::Header,
                SignedBlock<Block>,
            >::block(&client, hash)
            .await
        })
    }

    fn block_hash(
        &self,
        block_number: Option<<Block::Header as HeaderT>::Number>,
    ) -> Result<Option<Block::Hash>, jsonrpsee::core::ClientError> {
        let client = self.http_client.clone();
        self.block_on(async move {
            substrate_rpc_client::ChainApi::<
                <Block::Header as HeaderT>::Number,
                Block::Hash,
                Block::Header,
                SignedBlock<Block>,
            >::block_hash(
                &client,
                block_number.map(|n| ListOrValue::Value(NumberOrHex::Hex(n.into()))),
            )
            .await
        })
        .map(|ok| match ok {
            ListOrValue::List(v) => v.first().and_then(|some| *some),
            ListOrValue::Value(v) => v,
        })
    }

    fn header(
        &self,
        hash: Option<Block::Hash>,
    ) -> Result<Option<Block::Header>, jsonrpsee::core::ClientError> {
        let client = self.http_client.clone();
        self.block_on(async move {
            substrate_rpc_client::ChainApi::<
                BlockNumber,
                Block::Hash,
                Block::Header,
                SignedBlock<Block>,
            >::header(&client, hash)
            .await
        })
    }

    fn storage(
        &self,
        key: StorageKey,
        at: Option<Block::Hash>,
    ) -> Result<Option<StorageData>, jsonrpsee::core::ClientError> {
        let client = self.http_client.clone();
        self.block_on(async move {
            substrate_rpc_client::StateApi::<Block::Hash>::storage(&client, key, at).await
        })
    }

    fn storage_hash(
        &self,
        key: StorageKey,
        at: Option<Block::Hash>,
    ) -> Result<Option<Block::Hash>, jsonrpsee::core::ClientError> {
        let client = self.http_client.clone();
        self.block_on(async move {
            substrate_rpc_client::StateApi::<Block::Hash>::storage_hash(&client, key, at).await
        })
    }

    fn storage_keys_paged(
        &self,
        key: Option<StorageKey>,
        count: u32,
        start_key: Option<StorageKey>,
        at: Option<Block::Hash>,
    ) -> Result<Vec<sp_state_machine::StorageKey>, ClientError> {
        let client = self.http_client.clone();
        let result = self.block_on(async move {
            substrate_rpc_client::StateApi::<Block::Hash>::storage_keys_paged(
                &client, key, count, start_key, at,
            )
            .await
        });

        match result {
            Ok(result) => Ok(result.iter().map(|item| item.0.clone()).collect()),
            Err(err) => Err(err),
        }
    }

    fn child_storage(
        &self,
        child_info: &ChildInfo,
        key: StorageKey,
        at: Option<Block::Hash>,
    ) -> Result<Option<StorageData>, ClientError> {
        let client = self.http_client.clone();
        let child_storage_key = child_info.prefixed_storage_key();
        self.block_on(async move {
            substrate_rpc_client::ChildStateApi::<Block::Hash>::storage(
                &client,
                child_storage_key,
                key,
                at,
            )
            .await
        })
    }

    fn child_storage_hash(
        &self,
        child_info: &ChildInfo,
        key: StorageKey,
        at: Option<Block::Hash>,
    ) -> Result<Option<Block::Hash>, ClientError> {
        let client = self.http_client.clone();
        let child_storage_key = child_info.prefixed_storage_key();
        self.block_on(async move {
            substrate_rpc_client::ChildStateApi::<Block::Hash>::storage_hash(
                &client,
                child_storage_key,
                key,
                at,
            )
            .await
        })
    }

    fn child_storage_keys_paged(
        &self,
        child_info: &ChildInfo,
        key: Option<StorageKey>,
        count: u32,
        start_key: Option<StorageKey>,
        at: Option<Block::Hash>,
    ) -> Result<Vec<sp_state_machine::StorageKey>, ClientError> {
        let client = self.http_client.clone();
        let child_storage_key = child_info.prefixed_storage_key();
        let result = self.block_on(async move {
            substrate_rpc_client::ChildStateApi::<Block::Hash>::storage_keys_paged(
                &client,
                child_storage_key,
                key,
                count,
                start_key,
                at,
            )
            .await
        });

        match result {
            Ok(result) => Ok(result.iter().map(|item| item.0.clone()).collect()),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonrpsee::{
        RpcModule,
        http_client::HttpClientBuilder,
        server::{ServerBuilder, ServerHandle},
        types::error::ErrorObjectOwned,
    };
    use polkadot_sdk::sp_runtime::{
        OpaqueExtrinsic,
        generic::{Block as GenericBlock, Header as GenericHeader},
        traits::BlakeTwo256,
    };
    use serde_json::json;
    use std::{net::SocketAddr, time::Instant};

    // ---------------------------
    // Unit tests for `block_on`.
    // ---------------------------

    fn rpc_for_tests(delay_ms: u32) -> Rpc<BlockType> {
        let client =
            HttpClientBuilder::default().build("http://127.0.0.1:8080").expect("build http client");
        Rpc::new(client, delay_ms)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn block_on_success() {
        let rpc = rpc_for_tests(0);

        let res: Result<i32, &'static str> = rpc.block_on(async { Ok(42) });

        assert_eq!(res.unwrap(), 42, "should propagate Ok value");
    }

    /// Verifies that `block_on` propagates errors.
    #[tokio::test(flavor = "multi_thread")]
    async fn block_on_propagates_errors() {
        let rpc = rpc_for_tests(0);

        let err = rpc.block_on(async { Err::<(), &str>("failed") }).unwrap_err();

        assert_eq!(err, "failed");
    }

    /// Verifies that the initial fixed delay (to avoid 429s) is honored before the first attempt.
    #[tokio::test(flavor = "multi_thread")]
    async fn block_on_respects_initial_delay() {
        let delay_ms = 60u32;
        let rpc = rpc_for_tests(delay_ms);

        let start = Instant::now();
        let _ = rpc.block_on(async { Ok::<(), ()>(()) });
        let elapsed = start.elapsed();

        assert!(
            elapsed >= Duration::from_millis(delay_ms as u64),
            "elapsed {elapsed:?} should be >= configured initial delay {delay_ms:?}ms"
        );
    }

    // ---------------------------------------------------------
    // Integration-style tests with an embedded JSON-RPC server.
    // ---------------------------------------------------------

    async fn start_mock_server_ok() -> (SocketAddr, ServerHandle) {
        let mut module = RpcModule::new(());

        module
            .register_method("system_chain", |_, _, _| {
                Ok::<String, ErrorObjectOwned>("Local Testnet".to_string())
            })
            .expect("register system_chain");

        module
            .register_method("system_properties", |_, _, _| {
                Ok::<serde_json::Value, ErrorObjectOwned>(json!({
                    "tokenSymbol": "UNIT",
                    "tokenDecimals": 12,
                }))
            })
            .expect("register system_properties");

        module
            .register_method("chain_getBlockHash", |_, _, _| {
                Ok::<Option<String>, ErrorObjectOwned>(Some(
                    "0x0000000000000000000000000000000000000000000000000000000000000001".into(),
                ))
            })
            .expect("register chain_getBlockHash");

        module
            .register_method("state_getStorage", |_, _, _| {
                Ok::<Option<String>, ErrorObjectOwned>(Some("0xdeadbeef".into()))
            })
            .expect("register state_getStorage");

        module
            .register_method("state_getStorageHash", |_, _, _| {
                Ok::<Option<String>, ErrorObjectOwned>(Some(
                    "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                ))
            })
            .expect("register state_getStorageHash");

        module
            .register_method("state_getKeysPaged", |_, _, _| {
                Ok::<Vec<String>, ErrorObjectOwned>(vec!["0x1122".into(), "0xaabbcc".into()])
            })
            .expect("register state_getKeysPaged");

        let server = ServerBuilder::default().build("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        let handle = server.start(module);
        (addr, handle)
    }

    // Return RPC plus the ServerHandle so tests keep the server alive.
    async fn rpc_against_mock() -> (Rpc<BlockType>, ServerHandle) {
        let (addr, handle) = start_mock_server_ok().await;
        let url = format!("http://{addr}");
        let client = HttpClientBuilder::default().build(&url).unwrap();
        (Rpc::<BlockType>::new(client, 0), handle)
    }

    type BlockType = GenericBlock<GenericHeader<u32, BlakeTwo256>, OpaqueExtrinsic>;

    #[tokio::test(flavor = "multi_thread")]
    async fn integration_system_endpoints_ok() {
        // Keep `_server` in scope so the server isn't dropped early.
        let (rpc, _server) = rpc_against_mock().await;

        let chain = <super::Rpc<BlockType> as super::RPCClient<BlockType>>::system_chain(&rpc)
            .expect("system_chain ok");
        assert_eq!(chain, "Local Testnet");

        let props = <super::Rpc<BlockType> as super::RPCClient<BlockType>>::system_properties(&rpc)
            .expect("system_properties ok");
        assert_eq!(props.get("tokenSymbol").and_then(|v| v.as_str()), Some("UNIT"));
        assert_eq!(props.get("tokenDecimals").and_then(|v| v.as_u64()), Some(12));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn integration_chain_and_state_endpoints_ok() {
        let (rpc, _server) = rpc_against_mock().await;

        let hash = <super::Rpc<BlockType> as super::RPCClient<BlockType>>::block_hash(&rpc, None)
            .expect("ok")
            .expect("some hash");
        assert_eq!(hash, polkadot_sdk::sp_core::H256::from_low_u64_be(1));

        let val = <super::Rpc<BlockType> as super::RPCClient<BlockType>>::storage(
            &rpc,
            StorageKey(vec![0x00, 0xde, 0xad]),
            None,
        )
        .expect("ok")
        .expect("some data");
        assert_eq!(val.0, vec![0xde, 0xad, 0xbe, 0xef]);

        let h = <super::Rpc<BlockType> as super::RPCClient<BlockType>>::storage_hash(
            &rpc,
            StorageKey(vec![0x00, 0xde, 0xad]),
            None,
        )
        .expect("ok")
        .expect("some hash");
        assert_eq!(h, polkadot_sdk::sp_core::H256::from_slice(&[0xaa; 32]));

        let keys: Vec<sp_state_machine::StorageKey> =
            <super::Rpc<BlockType> as super::RPCClient<BlockType>>::storage_keys_paged(
                &rpc, None, 10, None, None,
            )
            .expect("ok");
        let expected: Vec<sp_state_machine::StorageKey> =
            vec![vec![0x11, 0x22], vec![0xaa, 0xbb, 0xcc]];
        assert_eq!(keys, expected);
    }
}
