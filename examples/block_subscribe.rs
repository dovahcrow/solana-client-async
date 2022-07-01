use solana_client::rpc_config::RpcBlockSubscribeFilter;
use solana_client::rpc_response::{Response, RpcBlockUpdate};
use solana_client_async::prelude::*;

#[tokio::main]
async fn main() {
    let mut client = ClientBuilder::new()
        .ws_url("wss://api.mainnet-beta.solana.com")
        .build()
        .await
        .unwrap();

    client
        .block_subscribe(RpcBlockSubscribeFilter::All, None)
        .await
        .unwrap()
        .await
        .unwrap();

    loop {
        let block = client.recv::<Response<RpcBlockUpdate>>().await.unwrap();
        println!("block {:?}", block);
    }
}
