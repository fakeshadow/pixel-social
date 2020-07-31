use actix::Addr;
use actix_send::prelude::*;
use chrono::{NaiveDateTime, Utc};
use hashbrown::HashMap;
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};
use redis::cmd;
use tokio_postgres::types::ToSql;

use crate::handler::{
    cache::MyRedisPool,
    db::{GetStatement, MyPostgresPool, ParseRowStream},
};
use crate::model::{
    actors::WsChatSession,
    common::{GlobalSessions, GlobalTalks},
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

// talk service actor handle communication to web socket sessions actors
#[actor]
pub struct TalkService {
    talks: GlobalTalks,
    sessions: GlobalSessions,
    db_pool: MyPostgresPool,
    cache_pool: MyRedisPool,
}

pub type TalkServiceAddr = Address<TalkService>;

pub(crate) async fn init_talk_service(
    db_pool: MyPostgresPool,
    cache_pool: MyRedisPool,
    talks: GlobalTalks,
    sessions: GlobalSessions,
) -> Result<TalkServiceAddr, ()> {
    let builder = TalkService::builder(move || {
        let db_pool = db_pool.clone();
        let cache_pool = cache_pool.clone();
        let talks = talks.clone();
        let sessions = sessions.clone();

        async {
            TalkService {
                talks,
                sessions,
                db_pool,
                cache_pool,
            }
        }
    });

    let addr = builder.start().await;

    Ok(addr)
}

#[derive(Deserialize)]
pub struct AuthRequest {
    pub token: String,
    pub online_status: u32,
}

pub struct DisconnectRequest {
    pub session_id: u32,
}

// pass Some(talk_id) in json for public message, pass None for private message
#[derive(Deserialize)]
pub struct TextMessageRequest {
    pub text: String,
    pub talk_id: Option<u32>,
    pub user_id: Option<u32>,
    pub session_id: Option<u32>,
}

pub struct ConnectRequest {
    pub session_id: u32,
    pub online_status: u32,
    pub addr: Addr<WsChatSession>,
}

#[derive(Deserialize, Clone)]
pub struct CreateTalkRequest {
    pub session_id: Option<u32>,
    pub name: String,
    pub description: String,
    pub owner: u32,
}

#[derive(Deserialize)]
pub struct JoinTalkRequest {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

#[derive(Deserialize)]
pub struct TalkByIdRequest {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

#[derive(Deserialize)]
pub struct UsersByIdRequest {
    pub session_id: Option<u32>,
    user_id: Vec<u32>,
}

#[derive(Deserialize)]
pub struct UserRelationRequest {
    pub session_id: Option<u32>,
}

// pass talk id for talk public messages. pass none for private history message.
#[derive(Deserialize)]
pub struct GetHistory {
    pub time: String,
    pub talk_id: Option<u32>,
    pub session_id: Option<u32>,
}

#[derive(Deserialize)]
pub struct RemoveUserRequest {
    pub session_id: Option<u32>,
    user_id: u32,
    talk_id: u32,
}

#[derive(Deserialize)]
pub struct Admin {
    pub add: Option<u32>,
    pub remove: Option<u32>,
    pub talk_id: u32,
    pub session_id: Option<u32>,
}

#[derive(Deserialize)]
pub struct DeleteTalkRequest {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

#[handler_v2]
impl TalkService {
    #[on_start]
    async fn on_start(&mut self) {
        println!("talk service actor have started");
    }

    async fn handle_disconnect(&mut self, msg: DisconnectRequest) {
        let sid = msg.session_id;

        if let Err(e) = self.sessions.remove_session_hm(sid) {
            self.sessions.send_error(sid, &e);
        };

        // we set user's online status in redis to 0 when user's websocket session disconnecting
        if let Err(e) = self.cache_pool.set_online_status(sid, 0, true).await {
            self.sessions.send_error(sid, &e);
        };
    }

    async fn handle_txt(&mut self, msg: TextMessageRequest) {
        // ToDo: batch insert messages to database.
        let sid = msg.session_id.unwrap();

        // the double layer async/await is to handle ResError more easily. We stringify the error and send them to websocket session actor.
        let r = async {
            let now = Utc::now().naive_utc();

            let pool = self.db_pool.get().await?;
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

                self.send_message_many(tid, s.as_str())
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

                self.sessions.send_message(sid, s.as_str());

                Ok(())
            }
        }
        .await;

        if let Err(e) = r {
            self.sessions.send_error(sid, &e);
        };
    }

    async fn handle_connect(&mut self, msg: ConnectRequest) {
        let sid = msg.session_id;

        let status = msg.online_status;
        let addr = msg.addr;

        if let Err(e) = self.cache_pool.set_online_status(sid, status, true).await {
            self.sessions.send_error(sid, &e);
        };

        if let Err(e) = self.sessions.insert_session_hm(sid, addr.clone()) {
            self.sessions.send_error(sid, &e);
        };

        addr.do_send(SessionMessage(
            SendMessage::Success("Connection Success").stringify(),
        ));
    }

    async fn handle_create(&mut self, msg: CreateTalkRequest) {
        let sid = msg.session_id.unwrap();

        let r = async {
            let admins = vec![msg.owner];

            let pool = self.db_pool.get().await?;
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
                .ok_or(ResError::PostgresError)?;

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
            self.talks.insert_talk_hm(t)?;
            self.sessions.send_message(msg.owner, s.as_str());
            Ok(())
        }
        .await;

        if let Err(e) = r {
            self.sessions.send_error(sid, &e);
        };
    }

    async fn handle_join(&mut self, msg: JoinTalkRequest) {
        let sid = msg.session_id.unwrap();
        let r = async {
            let tid = msg.talk_id;

            let t = self.talks.get_talk_hm(tid)?;
            if t.users.contains(&sid) {
                return Err(ResError::BadRequest);
            }

            let pool = self.db_pool.get().await?;
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
            self.talks.insert_talk_hm(t)?;
            self.sessions.send_message(sid, s.as_str());

            Ok(())
        }
        .await;

        if let Err(e) = r {
            self.sessions.send_error(sid, &e);
        };
    }

    async fn handle_talk_by_id(&mut self, msg: TalkByIdRequest) {
        let sid = msg.session_id.unwrap();

        let r = async {
            let talks = self.talks.get_talks_hm()?;

            // we return all talks if the query talk_id is 0
            let t = match msg.talk_id {
                0 => talks.into_iter().map(|(_, t)| t).collect(),
                _ => talks
                    .get(&msg.talk_id)
                    .map(|t| vec![t.clone()])
                    .unwrap_or_else(|| vec![]),
            };

            let s = SendMessage::Talks(&t).stringify();
            self.sessions.send_message(sid, s.as_str());

            Ok(())
        }
        .await;

        if let Err(e) = r {
            self.sessions.send_error(sid, &e);
        }
    }

    async fn handle_user_by_id(&mut self, msg: UsersByIdRequest) {
        let sid = msg.session_id.unwrap();

        match self.cache_pool.get_users(msg.user_id).await {
            Ok(u) => {
                let s = SendMessage::Users(&u).stringify();
                self.sessions.send_message(sid, s.as_str());
            }
            Err(e) => {
                self.sessions.send_error(sid, &e);
            }
        }
    }

    async fn handle_relation(&mut self, msg: UserRelationRequest) {
        let sid = msg.session_id.unwrap();

        let r = async {
            let pool = self.db_pool.get().await?;
            let (cli, _) = &*pool;

            let st = cli.prepare(GET_FRIENDS).await?;
            let params: [&(dyn ToSql + Sync); 1] = [&sid];
            let r = cli
                .query_raw(&st, params.iter().map(|s| *s as _))
                .await?
                .parse_row::<Relation>()
                .await?
                .pop()
                .ok_or(ResError::PostgresError)?;

            drop(pool);

            let s = SendMessage::Friends(&r.friends).stringify();
            self.sessions.send_message(sid, s.as_str());

            Ok(())
        };

        if let Err(e) = r.await {
            self.sessions.send_error(sid, &e);
        }
    }

    async fn handle_history(&mut self, msg: GetHistory) {
        let sid = msg.session_id.unwrap();

        let f = async {
            let time = NaiveDateTime::parse_from_str(&msg.time, "%Y-%m-%d %H:%M:%S%.f")?;

            let pool = self.db_pool.get().await?;
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

            self.sessions.send_message(sid, s.as_str());
            Ok(())
        };

        if let Err(e) = f.await {
            self.sessions.send_error(sid, &e);
        }
    }

    async fn handle_remove(&mut self, msg: RemoveUserRequest) {
        let sid = msg.session_id.unwrap();

        let r = async {
            let tid = msg.talk_id;
            let uid = msg.user_id;

            let talk = self.talks.get_talk_hm(tid)?;

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

            let pool = self.db_pool.get().await?;
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
            self.talks.insert_talk_hm(t)?;
            self.sessions.send_message(sid, s.as_str());

            Ok(())
        }
        .await;

        if let Err(e) = r {
            self.sessions.send_error(sid, &e);
        };
    }

    async fn handle_admin(&mut self, msg: Admin) {
        let sid = msg.session_id.unwrap();

        let r = async {
            let tid = msg.talk_id;

            let (query, uid) = if let Some(uid) = msg.add {
                (INSERT_ADMIN, uid)
            } else {
                let uid = msg.remove.ok_or(ResError::BadRequest)?;
                (REMOVE_ADMIN, uid)
            };

            let pool = self.db_pool.get().await?;
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
            self.talks.insert_talk_hm(t)?;
            self.sessions.send_message(sid, s.as_str());

            Ok(())
        }
        .await;

        if let Err(e) = r {
            self.sessions.send_error(sid, &e);
        };
    }

    async fn handle_delete(&mut self, msg: DeleteTalkRequest) {
        let sid = msg.session_id.unwrap();

        let tid = msg.talk_id;

        if let Err(e) = self._handle_delete(sid, tid).await {
            self.sessions.send_error(sid, &e);
        };
    }
}

impl TalkService {
    async fn _handle_delete(&mut self, sid: u32, tid: u32) -> Result<(), ResError> {
        let pool = self.db_pool.get().await?;
        let (cli, _) = &*pool;

        let st = cli.prepare(REMOVE_TALK).await?;
        cli.execute(&st, &[&tid]).await?;

        drop(pool);

        self.talks.remove_talk_hm(tid)?;
        let s = SendMessage::Success("Delete Talk Success").stringify();
        self.sessions.send_message(sid, s.as_str());

        Ok(())
    }

    // helper function to send message to multiple sessions.
    fn send_message_many(&self, tid: u32, msg: &str) -> Result<(), ResError> {
        let t = self.talks.get_talk_hm(tid)?;

        for u in t.users.iter() {
            self.sessions.send_message(*u, msg);
        }
        Ok(())
    }
}

// lock global sessions and read write session id and/or associate session addr(WebSocket session actor's address) and send string messages.
impl GlobalSessions {
    fn send_message(&self, sid: u32, msg: &str) {
        match self.get_session_hm(sid) {
            Ok(addr) => addr.do_send(SessionMessage(msg.to_owned())),
            Err(e) => self.send_error(sid, &e),
        };
    }

    fn send_error(&self, sid: u32, e: &ResError) {
        if let Ok(addr) = self.get_session_hm(sid) {
            addr.do_send(SessionMessage(
                SendMessage::Error(e.to_string().as_str()).stringify(),
            ));
        }
    }

    fn get_session_hm(&self, sid: u32) -> Result<Addr<WsChatSession>, ResError> {
        self.read_sessions(move |s| s.get(&sid).cloned().ok_or(ResError::NotFound))
    }

    fn insert_session_hm(&self, sid: u32, addr: Addr<WsChatSession>) -> Result<(), ResError> {
        self.write_sessions(move |mut s| s.insert(sid, addr).map(|_| ()).ok_or(ResError::NotFound))
    }

    fn remove_session_hm(&self, sid: u32) -> Result<(), ResError> {
        self.write_sessions(move |mut s| s.remove(&sid).map(|_| ()).ok_or(ResError::NotFound))
    }

    fn read_sessions<F, T>(&self, f: F) -> Result<T, ResError>
    where
        F: FnOnce(RwLockReadGuard<HashMap<u32, Addr<WsChatSession>>>) -> Result<T, ResError>,
    {
        let r = self.0.read();
        f(r)
    }

    fn write_sessions<F>(&self, f: F) -> Result<(), ResError>
    where
        F: FnOnce(RwLockWriteGuard<HashMap<u32, Addr<WsChatSession>>>) -> Result<(), ResError>,
    {
        let r = self.0.write();
        f(r)
    }
}

// lock the global talks and read/write the inner HashMap<talk_id, Talk>;
impl GlobalTalks {
    fn get_talk_hm(&self, talk_id: u32) -> Result<Talk, ResError> {
        self.read_talks(move |t| t.get(&talk_id).cloned().ok_or(ResError::NotFound))
    }

    fn get_talks_hm(&self) -> Result<HashMap<u32, Talk>, ResError> {
        self.read_talks(move |t| Ok(t.clone()))
    }

    fn insert_talk_hm(&self, talks: Vec<Talk>) -> Result<(), ResError> {
        self.write_talks(move |mut t| {
            for talk in talks.into_iter() {
                t.insert(talk.id, talk)
                    .map(|_| ())
                    .ok_or(ResError::NotFound)?;
            }
            Ok(())
        })
    }

    fn remove_talk_hm(&self, tid: u32) -> Result<(), ResError> {
        self.write_talks(move |mut t| t.remove(&tid).map(|_| ()).ok_or(ResError::NotFound))
    }

    fn read_talks<F, T>(&self, f: F) -> Result<T, ResError>
    where
        F: FnOnce(RwLockReadGuard<HashMap<u32, Talk>>) -> Result<T, ResError>,
    {
        let r = self.0.read();
        f(r)
    }

    fn write_talks<F>(&self, f: F) -> Result<(), ResError>
    where
        F: FnOnce(RwLockWriteGuard<HashMap<u32, Talk>>) -> Result<(), ResError>,
    {
        let r = self.0.write();
        f(r)
    }
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
