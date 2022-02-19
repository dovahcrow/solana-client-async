use crate::errors::SolanaClientError;
use crate::rpc_message::{RpcNotification, RpcRequest, RpcResponse};
use crate::WsStream;
use fehler::throws;
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
    pendings: HashMap<u64, oneshot::Sender<RpcResponse>>,
    ws: WsStream,
    sub_tx: broadcast::Sender<RpcNotification>,
    request_rx: mpsc::Receiver<(String, Box<RawValue>, oneshot::Sender<RpcResponse>)>,
    ping_timer: Interval,
    reqid: u64,
}

impl BackgroundProcess {
    pub fn new(
        stream: WsStream,
        ping_every: u64,
    ) -> (
        Self,
        broadcast::Receiver<RpcNotification>,
        mpsc::Sender<(String, Box<RawValue>, oneshot::Sender<RpcResponse>)>,
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
                msg = self.ws.next() => { self.process_ws(msg).await? }
                req = self.request_rx.recv() => { self.process_req(req).await? }
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
    pub async fn process_ws(&mut self, msg: Option<Result<Message, tungstenite::Error>>) {
        trace!("[Background] Received ws message {:?}", msg);
        if msg.is_none() {}

        let msg = msg.unwrap()?;

        let msg = match msg {
            Message::Text(msg) => msg,
            Message::Ping(_) => {
                self.pong().await?;
                return;
            }
            Message::Pong(_) => return,
            _ => {
                unreachable!()
            }
        };

        let msg: Either<RpcNotification, RpcResponse> = from_str(&msg)?;
        match msg {
            Either::Left(notif) => {
                if let Err(_) = self.sub_tx.send(notif) {
                    warn!("Subscription tx droppped");
                }
            }
            Either::Right(resp) => {
                let id = resp.id;
                if let Some(responder) = self.pendings.remove(&id) {
                    if let Err(_) = responder.send(resp) {
                        warn!("Responder for req: {} droppped", id);
                    }
                } else {
                    warn!("Responder for req: {} not found", id);
                }
            }
        }
    }

    #[throws(SolanaClientError)]
    pub async fn process_req(
        &mut self,
        rr: Option<(String, Box<RawValue>, oneshot::Sender<RpcResponse>)>,
    ) {
        trace!("[Background] Received request {:?}", rr);

        let (method, params, responder) = rr.unwrap();
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
