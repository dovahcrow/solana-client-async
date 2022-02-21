use crate::errors::SolanaClientError;
use crate::{
    background::BackgroundProcess,
    rpc_message::{RpcError, RpcNotification, RpcResponse},
};
use fehler::{throw, throws};
use futures::{
    task::{Context, Poll},
    Future,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::value::{RawValue, Value};
use serde_json::{from_str, to_string};
use std::marker::PhantomData;
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
    req_tx: mpsc::Sender<(
        String,
        Box<RawValue>,
        oneshot::Sender<Result<RpcResponse, RpcError>>,
    )>,
    sub_rx: broadcast::Receiver<RpcNotification>,
}

impl Client {
    #[throws(SolanaClientError)]
    pub async fn recv<T>(&mut self) -> T
    where
        T: DeserializeOwned,
    {
        let notif = self.recv_raw().await?;
        let payload = from_str(notif.get())?;
        payload
    }

    #[throws(SolanaClientError)]
    pub async fn recv_raw(&mut self) -> Box<RawValue> {
        let notif = self.sub_rx.recv().await?;
        let payload = notif.params.result;
        payload
    }

    #[throws(SolanaClientError)]
    pub async fn slot_subscribe(&mut self) -> ResponseAwaiter<usize> {
        let awaiter = self.request("slotSubscribe", &Value::Null).await?;
        awaiter
    }

    #[throws(SolanaClientError)]
    pub async fn request<T, R>(&mut self, method: &str, params: &T) -> ResponseAwaiter<R>
    where
        T: Serialize,
    {
        let params = to_string(params)?;
        let params = RawValue::from_string(params)?;

        let (tx, rx) = oneshot::channel();

        if let Err(_) = self.req_tx.send((method.into(), params, tx)).await {
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
