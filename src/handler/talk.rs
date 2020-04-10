use std::future::Future;

use actix::prelude::{Actor, Addr, Context, Handler, Message, ResponseFuture};
use async_std::sync::{RwLockReadGuard, RwLockWriteGuard};
use chrono::{NaiveDateTime, Utc};
use hashbrown::HashMap;
use redis::cmd;
use tokio_postgres::types::ToSql;

use crate::handler::{
    cache::{MyRedisPool, POOL_REDIS},
    db::{GetStatement, ParseRowStream, POOL},
};
use crate::model::{
    actors::WsChatSession,
    common::{SESSIONS, TALKS},
    errors::ResError,
    talk::{PrivateMessage, PublicMessage, Relation, SendMessage, SessionMessage, Talk},
};

// statements that are not constructed on pool start.
const INSERT_TALK: &str =
    "INSERT INTO talks (id, name, description, owner, admin, users) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *";
const REMOVE_TALK: &str = "DELETE FROM talks WHERE id=$1";
const INSERT_ADMIN: &str =
    "UPDATE talks SET admin=array_append(admin, $1) WHERE id=$2 AND owner=$3";
const REMOVE_ADMIN: &str =
    "UPDATE talks SET admin=array_remove(admin, $1) WHERE id=$2 AND owner=$3";
const REMOVE_USER: &str = "UPDATE talks SET users=array_remove(users, $1) WHERE id=$2";
const GET_PUB_MSG: &str =
    "SELECT * FROM public_messages1 WHERE talk_id = $1 AND time <= $2 ORDER BY time DESC LIMIT 999";
const GET_PRV_MSG: &str =
    "SELECT * FROM private_messages1 WHERE to_id = $1 AND time <= $2 ORDER BY time DESC LIMIT 999";
const GET_FRIENDS: &str = "SELECT friends FROM relations WHERE id = $1";
const INSERT_USER: &str = "UPDATE talks SET users=array_append(users, $1) WHERE id= $2";

// talk service actor handle communication to websocket sessions actors
pub struct TalkService;

impl Actor for TalkService {
    type Context = Context<Self>;
    fn started(&mut self, _: &mut Self::Context) {
        println!("talk service actor have started");
    }
}

pub type TalkServiceAddr = Addr<TalkService>;

// lock global sessions and read write session id and/or associate session addr(WebSocket session actor's address) and send string messages.
impl crate::model::common::GlobalSessions {
    async fn send_message(&self, sid: u32, msg: &str) {
        match self.get_session_hm(sid).await {
            Ok(addr) => addr.do_send(SessionMessage(msg.to_owned())),
            Err(e) => self.send_error(sid, &e).await,
        };
    }

    async fn send_error(&self, sid: u32, e: &ResError) {
        if let Ok(addr) = self.get_session_hm(sid).await {
            addr.do_send(SessionMessage(
                SendMessage::Error(e.to_string().as_str()).stringify(),
            ));
        }
    }

    fn get_session_hm(
        &self,
        sid: u32,
    ) -> impl Future<Output = Result<Addr<WsChatSession>, ResError>> + '_ {
        self.read_sessions(move |s| s.get(&sid).cloned().ok_or(ResError::NotFound))
    }

    fn insert_session_hm(
        &self,
        sid: u32,
        addr: Addr<WsChatSession>,
    ) -> impl Future<Output = Result<(), ResError>> + '_ {
        self.write_sessions(move |mut s| s.insert(sid, addr).map(|_| ()).ok_or(ResError::NotFound))
    }

    fn remove_session_hm(&self, sid: u32) -> impl Future<Output = Result<(), ResError>> + '_ {
        self.write_sessions(move |mut s| s.remove(&sid).map(|_| ()).ok_or(ResError::NotFound))
    }

    async fn read_sessions<F, T>(&self, f: F) -> Result<T, ResError>
    where
        F: FnOnce(RwLockReadGuard<HashMap<u32, Addr<WsChatSession>>>) -> Result<T, ResError>,
    {
        let r = self.0.read().await;
        f(r)
    }

    async fn write_sessions<F>(&self, f: F) -> Result<(), ResError>
    where
        F: FnOnce(RwLockWriteGuard<HashMap<u32, Addr<WsChatSession>>>) -> Result<(), ResError>,
    {
        let r = self.0.write().await;
        f(r)
    }
}

// lock the global talks and read/write the inner HashMap<talk_id, Talk>;
impl crate::model::common::GlobalTalks {
    fn get_talk_hm(&self, talk_id: u32) -> impl Future<Output = Result<Talk, ResError>> + '_ {
        self.read_talks(move |t| t.get(&talk_id).cloned().ok_or(ResError::NotFound))
    }

    fn get_talks_hm(&self) -> impl Future<Output = Result<HashMap<u32, Talk>, ResError>> + '_ {
        self.read_talks(move |t| Ok(t.clone()))
    }

    fn insert_talk_hm(&self, talks: Vec<Talk>) -> impl Future<Output = Result<(), ResError>> + '_ {
        self.write_talks(move |mut t| {
            for talk in talks.into_iter() {
                t.insert(talk.id, talk)
                    .map(|_| ())
                    .ok_or(ResError::NotFound)?;
            }
            Ok(())
        })
    }

    fn remove_talk_hm(&self, tid: u32) -> impl Future<Output = Result<(), ResError>> + '_ {
        self.write_talks(move |mut t| t.remove(&tid).map(|_| ()).ok_or(ResError::NotFound))
    }

    async fn read_talks<F, T>(&self, f: F) -> Result<T, ResError>
    where
        F: FnOnce(RwLockReadGuard<HashMap<u32, Talk>>) -> Result<T, ResError>,
    {
        let r = self.0.read().await;
        f(r)
    }

    async fn write_talks<F>(&self, f: F) -> Result<(), ResError>
    where
        F: FnOnce(RwLockWriteGuard<HashMap<u32, Talk>>) -> Result<(), ResError>,
    {
        let r = self.0.write().await;
        f(r)
    }
}

// helper function to send message to multiple sessions.
async fn send_message_many(tid: u32, msg: &str) -> Result<(), ResError> {
    let t = TALKS.get_talk_hm(tid).await?;

    for u in t.users.iter() {
        SESSIONS.send_message(*u, msg).await;
    }
    Ok(())
}

impl MyRedisPool {
    // we set user's online status in redis cache when user connect with websocket.
    async fn set_online_status(
        &self,
        uid: u32,
        status: u32,
        set_last_online_time: bool,
    ) -> Result<(), ResError> {
        let mut conn = self.get().await?.get_conn().clone();

        let mut arg = Vec::with_capacity(2);
        arg.push(("online_status", status.to_string()));

        if set_last_online_time {
            arg.push(("last_online", Utc::now().naive_utc().to_string()))
        }

        cmd("HMSET")
            .arg(&format!("user:{}:set_perm", uid))
            .arg(arg)
            .query_async::<_, ()>(&mut conn)
            .await?;

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct AuthRequest {
    pub token: String,
    pub online_status: u32,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct DisconnectRequest {
    pub session_id: u32,
}

impl Handler<DisconnectRequest> for TalkService {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: DisconnectRequest, _: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            let sid = msg.session_id;

            if let Err(e) = SESSIONS.remove_session_hm(sid).await {
                SESSIONS.send_error(sid, &e).await
            };

            // we set user's online status in redis to 0 when user's websocket session disconnecting
            if let Err(e) = POOL_REDIS.set_online_status(sid, 0, true).await {
                SESSIONS.send_error(sid, &e).await
            };
        })
    }
}

// pass Some(talk_id) in json for public message, pass None for private message
#[derive(Deserialize, Message)]
#[rtype(result = "()")]
pub struct TextMessageRequest {
    pub text: String,
    pub talk_id: Option<u32>,
    pub user_id: Option<u32>,
    pub session_id: Option<u32>,
}

impl Handler<TextMessageRequest> for TalkService {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: TextMessageRequest, _: &mut Context<Self>) -> Self::Result {
        // ToDo: batch insert messages to database.
        Box::pin(async move {
            let sid = msg.session_id.unwrap();

            // the double layer async/await is to handle ResError more easily. We stringify the error and send them to websocket session actor.
            let r = async {
                let now = Utc::now().naive_utc();

                let pool = POOL.get().await?;
                let (cli, sts) = &*pool;

                if let Some(tid) = msg.talk_id {
                    let st = sts.get_statement("insert_pub_msg")?;
                    cli.execute(st, &[&tid, &msg.text, &now]).await?;

                    drop(pool);

                    let s = SendMessage::PublicMessage(&[PublicMessage {
                        text: msg.text,
                        time: now,
                        talk_id: msg.talk_id.unwrap(),
                    }])
                    .stringify();

                    send_message_many(tid, s.as_str()).await
                } else {
                    let uid = msg.user_id.ok_or(ResError::BadRequest)?;

                    let st = sts.get_statement("insert_prv_msg")?;
                    cli.execute(st, &[&msg.session_id.unwrap(), &uid, &msg.text, &now])
                        .await?;

                    drop(pool);

                    let s = SendMessage::PrivateMessage(&[PrivateMessage {
                        user_id: msg.user_id.unwrap(),
                        text: msg.text,
                        time: now,
                    }])
                    .stringify();

                    SESSIONS.send_message(sid, s.as_str()).await;
                    Ok(())
                }
            }
            .await;

            if let Err(e) = r {
                SESSIONS.send_error(sid, &e).await;
            };
        })
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ConnectRequest {
    pub session_id: u32,
    pub online_status: u32,
    pub addr: Addr<WsChatSession>,
}

impl Handler<ConnectRequest> for TalkService {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: ConnectRequest, _: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            let sid = msg.session_id;

            let status = msg.online_status;
            let addr = msg.addr;

            if let Err(e) = POOL_REDIS.set_online_status(sid, status, true).await {
                SESSIONS.send_error(sid, &e).await;
            };

            if let Err(e) = SESSIONS.insert_session_hm(sid, addr.clone()).await {
                SESSIONS.send_error(sid, &e).await;
            };

            addr.do_send(SessionMessage(
                SendMessage::Success("Connection Success").stringify(),
            ));
        })
    }
}

#[derive(Deserialize, Message, Clone)]
#[rtype(result = "()")]
pub struct CreateTalkRequest {
    pub session_id: Option<u32>,
    pub name: String,
    pub description: String,
    pub owner: u32,
}

impl Handler<CreateTalkRequest> for TalkService {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: CreateTalkRequest, _: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                let admins = vec![msg.owner];

                let pool = POOL.get().await?;
                let (cli, _) = &*pool;

                let st = cli.prepare("SELECT Max(id) FROM talks").await?;
                let params: [&(dyn ToSql + Sync); 0] = [];
                let last_tid = cli
                    .query_raw(&st, params.iter().map(|s| *s as &dyn ToSql))
                    .await?
                    .parse_row::<Talk>()
                    .await?
                    .first()
                    .map(|t| t.id)
                    .ok_or(ResError::DataBaseReadError)?;

                let st = cli.prepare(INSERT_TALK).await?;
                let params: [&(dyn ToSql + Sync); 6] = [
                    &(last_tid + 1),
                    &msg.name,
                    &msg.description,
                    &msg.owner,
                    &admins,
                    &admins,
                ];

                let t = cli
                    .query_raw(&st, params.iter().map(|s| *s as _))
                    .await?
                    .parse_row()
                    .await?;

                drop(pool);

                let s = SendMessage::Talks(&t).stringify();
                TALKS.insert_talk_hm(t).await?;
                SESSIONS.send_message(msg.owner, s.as_str()).await;
                Ok(())
            }
            .await;

            if let Err(e) = r {
                SESSIONS.send_error(sid, &e).await;
            };
        })
    }
}

#[derive(Deserialize, Message)]
#[rtype(result = "()")]
pub struct JoinTalkRequest {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

impl Handler<JoinTalkRequest> for TalkService {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: JoinTalkRequest, _: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            let sid = msg.session_id.unwrap();
            let r = async {
                let tid = msg.talk_id;

                let t = TALKS.get_talk_hm(tid).await?;
                if t.users.contains(&sid) {
                    return Err(ResError::BadRequest);
                }

                let pool = POOL.get().await?;
                let (cli, _) = &*pool;

                let st = cli.prepare(INSERT_USER).await?;
                let params: [&(dyn ToSql + Sync); 2] = [&sid, &tid];
                let t = cli
                    .query_raw(&st, params.iter().map(|s| *s as _))
                    .await?
                    .parse_row()
                    .await?;

                drop(pool);

                let s = SendMessage::Talks(&t).stringify();
                TALKS.insert_talk_hm(t).await?;
                SESSIONS.send_message(sid, s.as_str()).await;

                Ok(())
            }
            .await;

            if let Err(e) = r {
                SESSIONS.send_error(sid, &e).await;
            };
        })
    }
}

#[derive(Message, Deserialize)]
#[rtype(result = "()")]
pub struct TalkByIdRequest {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

impl Handler<TalkByIdRequest> for TalkService {
    type Result = ResponseFuture<()>;
    fn handle(&mut self, msg: TalkByIdRequest, _: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                let talks = TALKS.get_talks_hm().await?;

                // we return all talks if the query talk_id is 0
                let t = match msg.talk_id {
                    0 => talks.into_iter().map(|(_, t)| t).collect(),
                    _ => talks
                        .get(&msg.talk_id)
                        .map(|t| vec![t.clone()])
                        .unwrap_or_else(|| vec![]),
                };

                let s = SendMessage::Talks(&t).stringify();
                SESSIONS.send_message(sid, s.as_str()).await;

                Ok(())
            }
            .await;

            if let Err(e) = r {
                SESSIONS.send_error(sid, &e).await;
            }
        })
    }
}

#[derive(Message, Deserialize)]
#[rtype(result = "()")]
pub struct UsersByIdRequest {
    pub session_id: Option<u32>,
    user_id: Vec<u32>,
}

impl Handler<UsersByIdRequest> for TalkService {
    type Result = ResponseFuture<()>;
    fn handle(&mut self, msg: UsersByIdRequest, _: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            let sid = msg.session_id.unwrap();

            match POOL_REDIS.get_users(msg.user_id).await {
                Ok(u) => {
                    let s = SendMessage::Users(&u).stringify();
                    SESSIONS.send_message(sid, s.as_str()).await;
                }
                Err(e) => {
                    SESSIONS.send_error(sid, &e).await;
                }
            }
        })
    }
}

#[derive(Message, Deserialize)]
#[rtype(result = "()")]
pub struct UserRelationRequest {
    pub session_id: Option<u32>,
}

impl Handler<UserRelationRequest> for TalkService {
    type Result = ResponseFuture<()>;
    fn handle(&mut self, msg: UserRelationRequest, _: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                let pool = POOL.get().await?;
                let (cli, _) = &*pool;

                let st = cli.prepare(GET_FRIENDS).await?;
                let params: [&(dyn ToSql + Sync); 1] = [&sid];
                let r = cli
                    .query_raw(&st, params.iter().map(|s| *s as _))
                    .await?
                    .parse_row::<Relation>()
                    .await?
                    .pop()
                    .ok_or(ResError::DataBaseReadError)?;

                drop(pool);

                let s = SendMessage::Friends(&r.friends).stringify();
                SESSIONS.send_message(sid, s.as_str()).await;

                Ok(())
            };

            if let Err(e) = r.await {
                SESSIONS.send_error(sid, &e).await;
            }
        })
    }
}

// pass talk id for talk public messages. pass none for private history message.
#[derive(Deserialize, Message)]
#[rtype(result = "()")]
pub struct GetHistory {
    pub time: String,
    pub talk_id: Option<u32>,
    pub session_id: Option<u32>,
}

impl Handler<GetHistory> for TalkService {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: GetHistory, _: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            let sid = msg.session_id.unwrap();

            let f = async {
                let time = NaiveDateTime::parse_from_str(&msg.time, "%Y-%m-%d %H:%M:%S%.f")?;

                let pool = POOL.get().await?;
                let (cli, _) = &*pool;

                let s = match msg.talk_id {
                    Some(tid) => {
                        let st = cli.prepare(GET_PUB_MSG).await?;
                        let params: [&(dyn ToSql + Sync); 2] = [&tid, &time];
                        let msg = cli
                            .query_raw(&st, params.iter().map(|s| *s as _))
                            .await?
                            .parse_row()
                            .await?;

                        drop(pool);

                        SendMessage::PublicMessage(&msg).stringify()
                    }
                    None => {
                        let st = cli.prepare(GET_PRV_MSG).await?;
                        let params: [&(dyn ToSql + Sync); 2] = [&sid, &time];
                        let msg = cli
                            .query_raw(&st, params.iter().map(|s| *s as _))
                            .await?
                            .parse_row()
                            .await?;

                        drop(pool);

                        SendMessage::PrivateMessage(&msg).stringify()
                    }
                };

                SESSIONS.send_message(sid, s.as_str()).await;
                Ok(())
            };

            if let Err(e) = f.await {
                SESSIONS.send_error(sid, &e).await;
            }
        })
    }
}

#[derive(Deserialize, Message)]
#[rtype(result = "()")]
pub struct RemoveUserRequest {
    pub session_id: Option<u32>,
    user_id: u32,
    talk_id: u32,
}

impl Handler<RemoveUserRequest> for TalkService {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: RemoveUserRequest, _: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                let tid = msg.talk_id;
                let uid = msg.user_id;

                let talk = TALKS.get_talk_hm(tid).await?;

                if !talk.users.contains(&uid) {
                    return Err(ResError::BadRequest);
                }

                let other_is_admin = talk.admin.contains(&uid);
                let other_is_owner = talk.owner == uid;
                let self_is_admin = talk.admin.contains(&sid);
                let self_is_owner = talk.owner == sid;

                if other_is_admin || other_is_owner || !self_is_admin || !self_is_owner {
                    return Err(ResError::Unauthorized);
                };

                let pool = POOL.get().await?;
                let (cli, _) = &*pool;

                let st = cli.prepare(REMOVE_USER).await?;
                let params: [&(dyn ToSql + Sync); 2] = [&uid, &tid];
                let t = cli
                    .query_raw(&st, params.iter().map(|s| *s as _))
                    .await?
                    .parse_row()
                    .await?;

                drop(pool);

                let s = SendMessage::Talks(&t).stringify();
                TALKS.insert_talk_hm(t).await?;
                SESSIONS.send_message(sid, s.as_str()).await;

                Ok(())
            }
            .await;

            if let Err(e) = r {
                SESSIONS.send_error(sid, &e).await;
            };
        })
    }
}

#[derive(Deserialize, Message)]
#[rtype(result = "()")]
pub struct Admin {
    pub add: Option<u32>,
    pub remove: Option<u32>,
    pub talk_id: u32,
    pub session_id: Option<u32>,
}

impl Handler<Admin> for TalkService {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: Admin, _: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                let tid = msg.talk_id;

                let (query, uid) = if let Some(uid) = msg.add {
                    (INSERT_ADMIN, uid)
                } else {
                    let uid = msg.remove.ok_or(ResError::BadRequest)?;
                    (REMOVE_ADMIN, uid)
                };

                let pool = POOL.get().await?;
                let (cli, _) = &*pool;

                let params: [&(dyn ToSql + Sync); 3] = [&uid, &tid, &sid];

                let st = cli.prepare(query).await?;
                let t = cli
                    .query_raw(&st, params.iter().map(|s| *s as _))
                    .await?
                    .parse_row::<Talk>()
                    .await?;

                drop(pool);

                let s = SendMessage::Talks(&t).stringify();
                TALKS.insert_talk_hm(t).await?;
                SESSIONS.send_message(sid, s.as_str()).await;

                Ok(())
            }
            .await;

            if let Err(e) = r {
                SESSIONS.send_error(sid, &e).await;
            };
        })
    }
}

#[derive(Deserialize, Message)]
#[rtype(result = "()")]
pub struct DeleteTalkRequest {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

impl Handler<DeleteTalkRequest> for TalkService {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: DeleteTalkRequest, _: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                let tid = msg.talk_id;

                let pool = POOL.get().await?;
                let (cli, _) = &*pool;

                let st = cli.prepare(REMOVE_TALK).await?;
                cli.execute(&st, &[&tid]).await?;

                drop(pool);

                TALKS.remove_talk_hm(tid).await?;
                let s = SendMessage::Success("Delete Talk Success").stringify();
                SESSIONS.send_message(sid, s.as_str()).await;

                Ok(())
            }
            .await;

            if let Err(e) = r {
                SESSIONS.send_error(sid, &e).await;
            };
        })
    }
}
