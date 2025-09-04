use crate::utils::TestNode;
use anvil_core::eth::EthRequest;
use anvil_polkadot::{cmd::NodeArgs, opts::Anvil};
use anvil_rpc::{
    error::{ErrorCode, RpcError},
    response::ResponseResult,
};
use tokio::time::{sleep, Duration};

#[tokio::test(flavor = "multi_thread")]
async fn demo_test() {
    let mut anvil_args = Anvil {
        global: foundry_cli::opts::GlobalArgs::default(),
        node: NodeArgs::default(),
        cmd: None,
    };

    anvil_args.node.no_mining = true;
    anvil_args.node.mixed_mining = false;
    anvil_args.node.port = 0; // auto-assign
    let mut node = TestNode::new_custom(anvil_args, |anvil_config, substrate_config| {
        *anvil_config = anvil_config.clone().set_silent(true);

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("db");
        substrate_config.set_base_path(Some(db_path));
        // Keep temp dir alive for the test
        std::mem::forget(temp_dir);
    })
    .await
    .unwrap();

    let _chain_name = node.system_chain().await.unwrap();

    let mine_req = EthRequest::Mine(None, None);

    let response = node.call_eth(mine_req).await.unwrap();
    assert!(matches!(
        response,
        ResponseResult::Error(RpcError { code: ErrorCode::InternalError, .. })
    ));

    while node.get_best_block_number().await.unwrap() < 3 {
        sleep(Duration::from_secs(8)).await;
    }
    let hash3 = node.block_hash_by_number(3).await.unwrap();
    let hash2 = node.block_hash_by_number(2).await.unwrap();
    let timestamp2 = node.get_decoded_timestamp(Some(hash2)).await;
    let timestamp3 = node.get_decoded_timestamp(Some(hash3)).await;
    let timestamp_diff = timestamp3.saturating_sub(timestamp2);
    let expected_block_time = 6000;
    let tolerance = 2000;
    assert!(
        timestamp_diff >= expected_block_time - tolerance,
        "❌ Block time too fast! Got {timestamp_diff}ms, expected ≥4000ms",
    );

    assert!(
        timestamp_diff <= expected_block_time + tolerance,
        "❌ Block time too slow! Got {timestamp_diff}ms, expected ≤8000ms",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn demo_test1() {
    let mut node = TestNode::new_default().await.unwrap();

    let _chain_name = node.system_chain().await.unwrap();

    let mine_req = EthRequest::Mine(None, None);

    let response = node.call_eth(mine_req).await.unwrap();
    assert!(matches!(
        response,
        ResponseResult::Error(RpcError { code: ErrorCode::InternalError, .. })
    ));

    while node.get_best_block_number().await.unwrap() < 3 {
        sleep(Duration::from_secs(8)).await;
    }
    let hash3 = node.block_hash_by_number(3).await.unwrap();
    let hash2 = node.block_hash_by_number(2).await.unwrap();
    let timestamp2 = node.get_decoded_timestamp(Some(hash2)).await;
    let timestamp3 = node.get_decoded_timestamp(Some(hash3)).await;
    let timestamp_diff = timestamp3.saturating_sub(timestamp2);
    let expected_block_time = 6000;
    let tolerance = 2000;
    assert!(
        timestamp_diff >= expected_block_time - tolerance,
        "❌ Block time too fast! Got {timestamp_diff}ms, expected ≥4000ms",
    );

    assert!(
        timestamp_diff <= expected_block_time + tolerance,
        "❌ Block time too slow! Got {timestamp_diff}ms, expected ≤8000ms",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn demo_test2() {
    let mut node = TestNode::new_default().await.unwrap();

    let mine_req = EthRequest::Mine(None, None);

    let response = node.call_eth(mine_req).await.unwrap();
    assert!(matches!(
        response,
        ResponseResult::Error(RpcError { code: ErrorCode::InternalError, .. })
    ));

    while node.get_best_block_number().await.unwrap() < 3 {
        sleep(Duration::from_secs(8)).await;
    }
    let hash3 = node.block_hash_by_number(3).await.unwrap();
    let hash2 = node.block_hash_by_number(2).await.unwrap();
    let timestamp2 = node.get_decoded_timestamp(Some(hash2)).await;
    let timestamp3 = node.get_decoded_timestamp(Some(hash3)).await;
    let timestamp_diff = timestamp3.saturating_sub(timestamp2);
    let expected_block_time = 6000;
    let tolerance = 2000;
    assert!(
        timestamp_diff >= expected_block_time - tolerance,
        "❌ Block time too fast! Got {timestamp_diff}ms, expected ≥4000ms",
    );

    assert!(
        timestamp_diff <= expected_block_time + tolerance,
        "❌ Block time too slow! Got {timestamp_diff}ms, expected ≤8000ms",
    );
}
