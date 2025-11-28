//! Contains the code to launch an Ethereum RPC server.

use anvil_server::{ServerConfig, ipc::IpcEndpoint};
use axum::Router;
use futures::StreamExt;
use handler::{HttpEthRpcHandler, PubSubEthRpcHandler};
use polkadot_sdk::sc_service::SpawnTaskHandle;
use std::{io, pin::pin};
use tokio::net::TcpListener;

use crate::api_server::ApiHandle;

mod handler;

/// Configures a server that handles JSON-RPC calls via HTTP and WS.
pub async fn serve_on(
    tcp_listener: TcpListener,
    config: ServerConfig,
    api_handle: ApiHandle,
) -> io::Result<()> {
    axum::serve(tcp_listener, router(api_handle, config).into_make_service()).await
}

/// Launches an ipc server at the given path in a new task.
pub fn try_spawn_ipc(
    spawn_handle: &SpawnTaskHandle,
    path: String,
    api_handle: ApiHandle,
) -> io::Result<()> {
    let handler = PubSubEthRpcHandler::new(api_handle);
    let ipc = IpcEndpoint::new(handler, path);
    let incoming = ipc.incoming()?;

    let inner_spawn_handle = spawn_handle.clone();

    spawn_handle.spawn("ipc", "anvil", async move {
        let mut incoming = pin!(incoming);
        while let Some(stream) = incoming.next().await {
            trace!(target: "ipc", "new ipc connection");
            inner_spawn_handle.spawn("ipc-connection", "anvil", stream);
        }
    });

    Ok(())
}

/// Configures an [`axum::Router`] that handles JSON-RPC calls via HTTP and WS.
fn router(api_handle: ApiHandle, config: ServerConfig) -> Router {
    let http = HttpEthRpcHandler::new(api_handle.clone());
    let ws = PubSubEthRpcHandler::new(api_handle);
    anvil_server::http_ws_router(config, http, ws)
}
