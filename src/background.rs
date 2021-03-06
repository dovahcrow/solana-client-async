use crate::errors::{Result as MyResult, SolanaClientError};
use crate::rpc_message::{RpcError, RpcNotification, RpcRequest, RpcResponse};
use crate::{Responder, WsStream};
use fehler::{throw, throws};
use futures::{SinkExt, StreamExt};
use log::{debug, error, trace, warn};
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::{from_str, to_string};
use std::{collections::HashMap, time::Duration};
use tokio::{
    select, spawn,
    sync::{broadcast, mpsc, oneshot},
    time::{interval, Interval},
};
use tungstenite::Message;

pub struct BackgroundProcess {
    pendings: HashMap<u64, oneshot::Sender<Result<RpcResponse, RpcError>>>,
    ws: WsStream,
    sub_tx: broadcast::Sender<MyResult<RpcNotification>>,
    request_rx: mpsc::Receiver<(String, Box<RawValue>, Responder)>,
    ping_timer: Interval,
    reqid: u64,
}

impl BackgroundProcess {
    #[allow(clippy::type_complexity)]
    pub fn new(
        stream: WsStream,
        ping_every: u64,
    ) -> (
        Self,
        broadcast::Receiver<MyResult<RpcNotification>>,
        mpsc::Sender<(String, Box<RawValue>, Responder)>,
    ) {
        let (request_tx, request_rx) = mpsc::channel(1024);
        let (sub_tx, sub_rx) = broadcast::channel(1024);
        let ping_timer = interval(Duration::from_secs(ping_every));

        (
            Self {
                pendings: HashMap::new(),
                ws: stream,
                sub_tx,
                request_rx,
                ping_timer,
                reqid: 0,
            },
            sub_rx,
            request_tx,
        )
    }

    pub fn start(self) {
        spawn(async {
            match self.start_impl().await {
                Ok(_) => {
                    unreachable!()
                }
                Err(e) => {
                    error!("[Background] Exited due to error: {}", e)
                }
            }
        });
    }

    pub async fn start_impl(mut self) -> Result<(), SolanaClientError> {
        loop {
            select! {
                _ = self.ping_timer.tick() => { self.ping().await? }
                msg = self.ws.next() => {
                    if msg.is_none() {
                        let _ = self.sub_tx.send(Err(SolanaClientError::WsClosed(None)));
                        throw!(SolanaClientError::WsClosed(None));
                    }
                    if let Err(e) = self.process_ws(msg.unwrap()?).await {
                        let _ = self.sub_tx.send(Err(e.clone()));
                        throw!(e);
                    }
                }
                req = self.request_rx.recv() => {
                    if req.is_none() {
                        warn!("Request rx exited");
                        break;
                    }
                    if let Err(e) = self.process_req(req.unwrap()).await {
                        let _ = self.sub_tx.send(Err(e.clone()));
                        throw!(e);
                    }
                }
            }
        }

        loop {
            select! {
                _ = self.ping_timer.tick() => { self.ping().await? }
                msg = self.ws.next() => {
                    if msg.is_none() {
                        let _ = self.sub_tx.send(Err(SolanaClientError::WsClosed(None)));
                        throw!(SolanaClientError::WsClosed(None));
                    }
                    if let Err(e) = self.process_ws(msg.unwrap()?).await {
                        let _ = self.sub_tx.send(Err(e.clone()));
                        throw!(e);
                    }
                }
            }
        }
    }

    #[throws(SolanaClientError)]
    pub async fn ping(&mut self) {
        debug!("[Background] Ping");
        self.ws.send(Message::Ping(vec![])).await?
    }

    #[throws(SolanaClientError)]
    pub async fn pong(&mut self) {
        debug!("[Background] Pong");
        self.ws.send(Message::Pong(vec![])).await?
    }

    #[throws(SolanaClientError)]
    pub async fn process_ws(&mut self, msg: Message) {
        trace!("[Background] Received ws message {:?}", msg);

        let msg = match msg {
            Message::Text(msg) => msg,
            Message::Ping(_) => {
                self.pong().await?;
                return;
            }
            Message::Pong(_) => return,
            Message::Close(reason) => {
                throw!(SolanaClientError::WsClosed(reason.map(|r| r.to_string())));
            }
            _ => {
                unreachable!()
            }
        };

        let mut errors = vec![];

        match from_str::<RpcNotification>(&msg) {
            Ok(notif) => {
                if self.sub_tx.send(Ok(notif)).is_err() {
                    throw!(SolanaClientError::SubscriptionDropped)
                }
                return;
            }
            Err(e) => errors.push(e),
        }

        match from_str::<RpcResponse>(&msg) {
            Ok(resp) => {
                let id = resp.id;
                if let Some(responder) = self.pendings.remove(&id) {
                    if responder.send(Ok(resp)).is_err() {
                        warn!("Responder for req: {} droppped", id);
                    }
                } else {
                    warn!("Responder for req: {} not found", id);
                }
                return;
            }
            Err(e) => errors.push(e),
        }

        match from_str::<RpcError>(&msg) {
            Ok(error) => {
                let id = error.id;
                if let Some(responder) = self.pendings.remove(&id) {
                    if responder.send(Err(error)).is_err() {
                        warn!("Responder for req: {} droppped", id);
                    }
                } else {
                    warn!("Responder for req: {} not found", id);
                }
                return;
            }
            Err(e) => {
                errors.push(e);
                warn!(
                    "Cannot deserialize ws message {}, errors: {:?}",
                    msg, errors
                );
            }
        }
    }

    #[throws(SolanaClientError)]
    pub async fn process_req(&mut self, rr: (String, Box<RawValue>, Responder)) {
        trace!("[Background] Received request {:?}", rr);

        let (method, params, responder) = rr;
        let id = self.id();
        let req = RpcRequest::new(id, &method, params);
        let exist = self.pendings.insert(id, responder);
        if exist.is_some() {
            error!("ReqId {} exists", id);
        }

        self.ws.send(Message::Text(to_string(&req)?)).await?
    }

    pub fn id(&mut self) -> u64 {
        self.reqid += 1;
        self.reqid
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Either<T, U> {
    Left(T),
    Right(U),
}
