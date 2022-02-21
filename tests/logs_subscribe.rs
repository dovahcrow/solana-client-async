use solana_client::rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_client::rpc_response::{Response, RpcLogsResponse};
use solana_client_async::prelude::*;

#[tokio::test]
async fn logs_subscribe() {
    let mut client = ClientBuilder::new()
        .ws_url("wss://api.mainnet-beta.solana.com")
        .build()
        .await
        .unwrap();

    client
        .logs_subscribe(
            RpcTransactionLogsFilter::All,
            RpcTransactionLogsConfig { commitment: None },
        )
        .await
        .unwrap()
        .await
        .unwrap(); // Double await because the first await is for `Send` and the second one for `Receive`. It is fine to drop the second one.

    client.recv::<Response<RpcLogsResponse>>().await.unwrap();
}
