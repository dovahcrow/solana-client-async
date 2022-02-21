use solana_client::rpc_response::{Response, RpcKeyedAccount};
use solana_client_async::prelude::*;

#[tokio::test]
async fn program_subscribe() {
    let mut client = ClientBuilder::new()
        .ws_url("wss://api.mainnet-beta.solana.com")
        .build()
        .await
        .unwrap();

    client
        .program_subscribe(
            &"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
                .parse()
                .unwrap(), // serum eventq for SOLUSD
            None,
        )
        .await
        .unwrap()
        .await
        .unwrap();

    client.recv::<Response<RpcKeyedAccount>>().await.unwrap();
}
