use solana_client::rpc_response::SlotInfo;
use solana_client_async::prelude::*;

#[tokio::test]
async fn slot_subscribe() {
    let mut client = ClientBuilder::new()
        .ws_url("wss://api.mainnet-beta.solana.com")
        .build()
        .await
        .unwrap();

    let subscription_id = client.slot_subscribe().await.unwrap().await.unwrap();

    client.recv::<SlotInfo>().await.unwrap();

    assert!(client
        .slot_unsubscribe(subscription_id)
        .await
        .unwrap()
        .await
        .unwrap());
}
