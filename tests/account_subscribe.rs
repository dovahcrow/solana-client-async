use solana_account_decoder::UiAccount;
use solana_client::rpc_response::Response;
use solana_client_async::prelude::*;

#[tokio::test]
async fn account_subscribe() {
    let mut client = ClientBuilder::new()
        .ws_url("wss://api.mainnet-beta.solana.com")
        .build()
        .await
        .unwrap();

    client
        .account_subscribe(
            &"5KKsLVU6TcbVDK4BS6K1DGDxnh4Q9xjYJ8XaDCG5t8ht"
                .parse()
                .unwrap(), // serum eventq for SOLUSD
            None,
        )
        .await
        .unwrap()
        .await
        .unwrap();

    client.recv::<Response<UiAccount>>().await.unwrap();
}
