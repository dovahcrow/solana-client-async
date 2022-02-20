use crate::errors::SolanaClientError;
use crate::{
    background::BackgroundProcess,
    rpc_message::{RpcNotification, RpcResponse},
};
use fehler::{throw, throws};
use futures::{
    task::{Context, Poll},
    Future,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::value::RawValue;
use serde_json::{from_str, to_string};
use std::pin::Pin;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_tungstenite::connect_async;

#[derive(Default, Debug, Clone)]
pub struct ClientBuilder {
    url: Option<String>,
    ws_url: Option<String>,
    ping_every: Option<u64>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Default::default()
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
        let (stream, _) = connect_async(self.ws_url.as_ref().unwrap()).await?;

        let (bp, sub_rx, req_tx) = BackgroundProcess::new(stream, self.ping_every.unwrap_or(5));
        bp.start();

        Client { req_tx, sub_rx }
    }
}

pub struct Client {
    req_tx: mpsc::Sender<(String, Box<RawValue>, oneshot::Sender<RpcResponse>)>,
    sub_rx: broadcast::Receiver<RpcNotification>,
}

impl Client {
    #[throws(SolanaClientError)]
    pub async fn recv<T>(&mut self) -> T
    where
        T: DeserializeOwned,
    {
        let notif = self.sub_rx.recv().await?;
        let payload = from_str(notif.params.result.get())?;
        payload
    }

    #[throws(SolanaClientError)]
    pub async fn recv_raw(&mut self) -> Box<RawValue> {
        let notif = self.sub_rx.recv().await?;
        let payload = notif.params.result;
        payload
    }

    #[throws(SolanaClientError)]
    pub async fn request<T>(&mut self, method: &str, params: &T) -> ResponseAwaiter
    where
        T: Serialize,
    {
        let params = to_string(params)?;
        let params = RawValue::from_string(params)?;

        let (tx, rx) = oneshot::channel();

        if let Err(_) = self.req_tx.send((method.into(), params, tx)).await {
            throw!(SolanaClientError::BackgroundProcessExited);
        }

        ResponseAwaiter { rx }
    }
}

pub struct ResponseAwaiter {
    rx: oneshot::Receiver<RpcResponse>,
}

impl Future for ResponseAwaiter {
    type Output = Result<RpcResponse, SolanaClientError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let rx = Pin::new(&mut this.rx);

        rx.poll(cx)
            .map(|r| r.map_err(|_| SolanaClientError::ResponderClosed))
    }
}
