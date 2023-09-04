use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web_actors::ws;
use tracing::info;

use crate::server::{
    ChatServer, ChattingWithUpdate, ClientMessage, CommunicateUser, CommunicationType, Connect,
    Disconnect, Message,
};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct WsChatSession {
    pub id: usize,
    pub user_id: usize,
    pub hb: Instant,
    pub name: Option<String>,
    pub addr: Addr<ChatServer>,
}

impl WsChatSession {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                info!("Websocket Client heartbeat failed, disconnecting!");
                act.addr.do_send(Disconnect { id: act.id });
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Actor for WsChatSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
        let addr = ctx.address();
        self.addr
            .send(Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => {
                        act.id = res;
                        act.name = Some("Main WebSocket".to_string());
                    }
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.addr.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

impl Handler<Message> for WsChatSession {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsChatSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(text) => {
                let m = text.trim();
                if m.starts_with('/') {
                    let v: Vec<&str> = m.splitn(2, ' ').collect();
                    match v[0] {
                        "/create-new-user" => self.addr.do_send(CommunicateUser {
                            ws_id: self.id,
                            user_data: v[1].to_string(),
                            comm_type: CommunicationType::CreateNewUser,
                        }),
                        "/update-chatting-with" => self.addr.do_send(ChattingWithUpdate {
                            chatting_from: self.id,
                            chatting_with: v[1].parse().unwrap(),
                        }),
                        "/get-user-data" => self.addr.do_send(CommunicateUser {
                            ws_id: self.id,
                            user_data: v[1].to_string(),
                            comm_type: CommunicationType::SendUserData,
                        }),
                        "/update-ids" => self.addr.do_send(CommunicateUser {
                            ws_id: self.id,
                            user_data: v[1].to_string(),
                            comm_type: CommunicationType::UpdateUserIDs,
                        }),
                        _ => ctx.text(format!("!!! unknown command: {m:?}")),
                    }
                } else {
                    self.addr.do_send(ClientMessage {
                        id: self.id,
                        msg: m.to_string(),
                    })
                }
            }
            ws::Message::Binary(_) => println!("Unexpected binary"),
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}
