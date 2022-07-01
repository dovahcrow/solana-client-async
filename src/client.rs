use crate::{
    background::BackgroundProcess,
    errors::{Result as MyResult, SolanaClientError},
    rpc_message::{RpcError, RpcNotification, RpcResponse},
    Responder,
};
use fehler::{throw, throws};
use futures::{
    task::{Context, Poll},
    Future,
};
use http::request::Request;
use log::trace;
use paste::paste;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::value::{RawValue, Value};
use serde_json::{from_str, json, to_string};
use solana_client::rpc_config::{
    RpcAccountInfoConfig, RpcBlockSubscribeConfig, RpcBlockSubscribeFilter,
    RpcProgramAccountsConfig, RpcSignatureSubscribeConfig, RpcTransactionLogsConfig,
    RpcTransactionLogsFilter,
};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_tungstenite::connect_async;
use tungstenite::handshake::client::generate_key;
use url::Url;

#[derive(Default, Debug, Clone)]
pub struct ClientBuilder {
    headers: HashMap<String, String>,
    url: Option<String>,
    ws_url: Option<String>,
    ping_every: Option<u64>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn header(&mut self, name: &str, value: &str) -> &mut Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn url(&mut self, url: &str) -> &mut Self {
        self.url = Some(url.into());
        self
    }

    pub fn ws_url(&mut self, ws_url: &str) -> &mut Self {
        self.ws_url = Some(ws_url.into());
        self
    }

    pub fn ping_every(&mut self, ping_every: u64) -> &mut Self {
        self.ping_every = Some(ping_every);
        self
    }

    #[throws(SolanaClientError)]
    pub async fn build(&mut self) -> Client {
        let ws_url = self.ws_url.as_ref().unwrap();
        let url = Url::parse(ws_url)?;
        let host = url.host_str().ok_or(SolanaClientError::NoHostName)?;

        let mut builder = Request::builder()
            .method("GET")
            .header("Host", host)
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", generate_key())
            .uri(ws_url);

        for (key, value) in &self.headers {
            builder = builder.header(key, value);
        }

        let (stream, _) = connect_async(builder.body(())?).await?;

        let (bp, sub_rx, req_tx) = BackgroundProcess::new(stream, self.ping_every.unwrap_or(5));
        bp.start();

        Client { req_tx, sub_rx }
    }
}

macro_rules! unsubscribe_method {
    ($meth:ident) => {
        paste! {
            #[throws(SolanaClientError)]
            pub async fn [<$meth _unsubscribe>](&mut self, subscription_id: u64) -> ResponseAwaiter<bool> {
                let awaiter = self.request(concat!(stringify!($meth), "Unsubscribe"), &[subscription_id]).await?;
                awaiter
            }
        }
    };
}

pub struct Client {
    req_tx: mpsc::Sender<(String, Box<RawValue>, Responder)>,
    sub_rx: broadcast::Receiver<MyResult<RpcNotification>>,
}

impl Client {
    #[throws(SolanaClientError)]
    pub async fn recv<T>(&mut self) -> (u64, T)
    where
        T: DeserializeOwned,
    {
        let (subid, notif) = self.recv_raw().await?;
        trace!("[Client] Recv payload: {}", notif);
        let payload = from_str(notif.get())?;
        (subid, payload)
    }

    #[throws(SolanaClientError)]
    pub async fn recv_raw(&mut self) -> (u64, Box<RawValue>) {
        let notif = self.sub_rx.recv().await??;
        (notif.params.subscription, notif.params.result)
    }

    #[throws(SolanaClientError)]
    pub async fn account_subscribe(
        &mut self,
        pubkey: &Pubkey,
        config: Option<RpcAccountInfoConfig>,
    ) -> ResponseAwaiter<u64> {
        let awaiter = self
            .request("accountSubscribe", &json! {[pubkey.to_string(), config]})
            .await?;
        awaiter
    }
    unsubscribe_method!(account);

    #[throws(SolanaClientError)]
    pub async fn block_subscribe(
        &mut self,
        filter: RpcBlockSubscribeFilter,
        config: Option<RpcBlockSubscribeConfig>,
    ) -> ResponseAwaiter<u64> {
        let awaiter = self
            .request("blockSubscribe", &json! {[filter, config]})
            .await?;
        awaiter
    }
    unsubscribe_method!(block);

    #[throws(SolanaClientError)]
    pub async fn logs_subscribe(
        &mut self,
        filter: RpcTransactionLogsFilter,
        config: RpcTransactionLogsConfig,
    ) -> ResponseAwaiter<u64> {
        let awaiter = self
            .request("logsSubscribe", &json! {[filter, config]})
            .await?;
        awaiter
    }
    unsubscribe_method!(logs);

    #[throws(SolanaClientError)]
    pub async fn program_subscribe(
        &mut self,
        pubkey: &Pubkey,
        config: Option<RpcProgramAccountsConfig>,
    ) -> ResponseAwaiter<u64> {
        let awaiter = self
            .request("programSubscribe", &json! {[pubkey.to_string(), config]})
            .await?;
        awaiter
    }
    unsubscribe_method!(program);

    #[throws(SolanaClientError)]
    pub async fn vote_subscribe(&mut self) -> ResponseAwaiter<u64> {
        let awaiter = self.request("voteSubscribe", &Value::Null).await?;
        awaiter
    }
    unsubscribe_method!(vote);

    #[throws(SolanaClientError)]
    pub async fn root_subscribe(&mut self) -> ResponseAwaiter<u64> {
        let awaiter = self.request("rootSubscribe", &Value::Null).await?;
        awaiter
    }
    unsubscribe_method!(root);

    #[throws(SolanaClientError)]
    pub async fn slot_subscribe(&mut self) -> ResponseAwaiter<u64> {
        let awaiter = self.request("slotSubscribe", &Value::Null).await?;
        awaiter
    }
    unsubscribe_method!(slot);

    #[throws(SolanaClientError)]
    pub async fn signature_subscribe(
        &mut self,
        signature: &Signature,
        config: Option<RpcSignatureSubscribeConfig>,
    ) -> ResponseAwaiter<u64> {
        let awaiter = self
            .request(
                "signatureSubscribe",
                &json! {[signature.to_string(), config]},
            )
            .await?;
        awaiter
    }
    unsubscribe_method!(signature);

    #[throws(SolanaClientError)]
    pub async fn request<T, R>(&mut self, method: &str, params: &T) -> ResponseAwaiter<R>
    where
        T: Serialize,
    {
        let params = to_string(params)?;
        let params = RawValue::from_string(params)?;

        let (tx, rx) = oneshot::channel();

        if self.req_tx.send((method.into(), params, tx)).await.is_err() {
            throw!(SolanaClientError::BackgroundProcessExited);
        }

        ResponseAwaiter {
            rx,
            _phantom: PhantomData,
        }
    }
}

pub struct ResponseAwaiter<T> {
    rx: oneshot::Receiver<Result<RpcResponse, RpcError>>,
    _phantom: PhantomData<T>,
}

impl<T> Future for ResponseAwaiter<T>
where
    T: DeserializeOwned,
{
    type Output = Result<T, SolanaClientError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() }; // todo why is PhantomData<T> asked for T to be unpin?
        let rx = Pin::new(&mut this.rx);

        match rx.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(_)) => Poll::Ready(Err(SolanaClientError::ResponderClosed)),
            Poll::Ready(Ok(Ok(r))) => match from_str(r.result.get()) {
                Ok(r) => Poll::Ready(Ok(r)),
                Err(e) => Poll::Ready(Err(SolanaClientError::Json(e))),
            },
            Poll::Ready(Ok(Err(e))) => Poll::Ready(Err(SolanaClientError::RpcError {
                code: e.error.code,
                message: e.error.message,
            })),
        }
    }
}
