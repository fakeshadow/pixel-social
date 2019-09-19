use std::cell::{RefCell, RefMut};
use std::future::Future;
use std::rc::Rc;

use actix::prelude::{Actor, Addr, Context, Handler, Message};
use actix::{ActorFuture, ResponseActFuture, WrapFuture};
use actix_async_await::ResponseStdFuture;
use async_std::sync::{RwLockReadGuard, RwLockWriteGuard};
use chrono::{NaiveDateTime, Utc};
use futures::{future::join_all, FutureExt, TryFutureExt};
use hashbrown::HashMap;
use redis::{aio::SharedConnection, cmd};
use tokio_postgres::{Client, Statement};

use crate::handler::cache::CheckRedisConn;
use crate::handler::{
    cache::{FromCache, GetSharedConn, UsersFromCache},
    db::{AsCrateClient, CrateClientLike},
};
use crate::model::{
    actors::WsChatSession,
    common::{GlobalSessions, GlobalTalks},
    errors::ResError,
    talk::{PrivateMessage, PublicMessage, Relation, SendMessage, SessionMessage, Talk},
};

// statements that are not constructed on actor start.
const INSERT_TALK: &str =
    "INSERT INTO talks (id, name, description, owner, admin, users) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *";
const REMOVE_TALK: &str = "DELETE FROM talks WHERE id=$1";
const INSERT_ADMIN: &str =
    "UPDATE talks SET admin=array_append(admin, $1) WHERE id=$2 AND owner=$3";
const REMOVE_ADMIN: &str =
    "UPDATE talks SET admin=array_remove(admin, $1) WHERE id=$2 AND owner=$3";
const REMOVE_USER: &str = "UPDATE talks SET users=array_remove(users, $1) WHERE id=$2";

// Frequent used statements that are constructed on actor start.
const INSERT_PUB_MSG: &str =
    "INSERT INTO public_messages1 (talk_id, text, time) VALUES ($1, $2, $3)";
const INSERT_PRV_MSG: &str =
    "INSERT INTO private_messages1 (from_id, to_id, text, time) VALUES ($1, $2, $3, $4)";
const GET_PUB_MSG: &str =
    "SELECT * FROM public_messages1 WHERE talk_id = $1 AND time <= $2 ORDER BY time DESC LIMIT 999";
const GET_PRV_MSG: &str =
    "SELECT * FROM private_messages1 WHERE to_id = $1 AND time <= $2 ORDER BY time DESC LIMIT 999";
const GET_FRIENDS: &str = "SELECT friends FROM relations WHERE id = $1";
const INSERT_USER: &str = "UPDATE talks SET users=array_append(users, $1) WHERE id= $2";

// actor handles communication between websocket sessions actors
// with a database connection(each actor) for messages and talks query. a redis connection(each actor) for users' cache info query.
pub struct TalkService {
    pub db_url: String,
    pub cache_url: String,
    pub talks: GlobalTalks,
    pub sessions: GlobalSessions,
    pub db: Rc<RefCell<tokio_postgres::Client>>,
    pub cache: Rc<RefCell<SharedConnection>>,
    pub insert_pub_msg: Rc<RefCell<Statement>>,
    pub insert_prv_msg: Rc<RefCell<Statement>>,
    pub get_pub_msg: Rc<RefCell<Statement>>,
    pub get_prv_msg: Rc<RefCell<Statement>>,
    pub get_relations: Rc<RefCell<Statement>>,
    pub join_talk: Rc<RefCell<Statement>>,
}

impl Actor for TalkService {
    type Context = Context<Self>;
}

pub type TALK = Addr<TalkService>;

impl TalkService {
    pub(crate) async fn init(
        postgres_url: &str,
        redis_url: &str,
        talks: GlobalTalks,
        sessions: GlobalSessions,
    ) -> Result<TALK, ResError> {
        let cache = crate::handler::cache::connect_cache(redis_url)
            .await?
            .ok_or(ResError::RedisConnection)?;
        let (db, mut sts) = TalkService::connect_postgres(postgres_url).await?;

        let db_url = postgres_url.to_owned();
        let cache_url = redis_url.to_owned();

        Ok(TalkService::create(move |_| {
            let insert_pub_msg = sts.pop().unwrap();
            let insert_prv_msg = sts.pop().unwrap();
            let get_pub_msg = sts.pop().unwrap();
            let get_prv_msg = sts.pop().unwrap();
            let get_relations = sts.pop().unwrap();
            let join_talk = sts.pop().unwrap();

            TalkService {
                db_url,
                cache_url,
                talks,
                sessions,
                db: Rc::new(RefCell::new(db)),
                cache: Rc::new(RefCell::new(cache)),
                insert_pub_msg: Rc::new(RefCell::new(insert_pub_msg)),
                insert_prv_msg: Rc::new(RefCell::new(insert_prv_msg)),
                get_pub_msg: Rc::new(RefCell::new(get_pub_msg)),
                get_prv_msg: Rc::new(RefCell::new(get_prv_msg)),
                get_relations: Rc::new(RefCell::new(get_relations)),
                join_talk: Rc::new(RefCell::new(join_talk)),
            }
        }))
    }

    async fn connect_postgres(postgres_url: &str) -> Result<(Client, Vec<Statement>), ResError> {
        let (mut db, conn) = tokio_postgres::connect(postgres_url, tokio_postgres::NoTls).await?;
        tokio::spawn(conn.map(|_| ()));

        let p1 = db.prepare(INSERT_PUB_MSG);
        let p2 = db.prepare(INSERT_PRV_MSG);
        let p3 = db.prepare(GET_PUB_MSG);
        let p4 = db.prepare(GET_PRV_MSG);
        let p5 = db.prepare(GET_FRIENDS);
        let p6 = db.prepare(INSERT_USER);

        let v: Vec<Result<Statement, tokio_postgres::Error>> =
            join_all(vec![p6, p5, p4, p3, p2, p1]).await;
        let mut sts = Vec::new();
        for v in v.into_iter() {
            sts.push(v?);
        }

        Ok((db, sts))
    }

    fn rw_sessions(&mut self) -> ReadWriteSessions {
        ReadWriteSessions::from(self.sessions.clone())
    }

    fn rw_talks(&mut self) -> ReadWriteTalks {
        ReadWriteTalks::from(self.talks.clone())
    }

    fn rw_db(&mut self) -> ReadWriteDb {
        ReadWriteDb::from(self.db.clone())
    }

    fn rw_cache(&mut self) -> ReadWriteCache {
        ReadWriteCache::from((self.cache.clone(), None))
    }

    fn rw_cache_with_url(&mut self) -> ReadWriteCache {
        ReadWriteCache::from((self.cache.clone(), Some(self.cache_url.clone())))
    }
}

#[derive(Deserialize)]
pub struct AuthRequest {
    pub token: String,
    pub online_status: u32,
}

pub struct ReadWriteSessions(GlobalSessions);

impl From<GlobalSessions> for ReadWriteSessions {
    fn from(s: GlobalSessions) -> ReadWriteSessions {
        ReadWriteSessions(s)
    }
}

impl ReadWriteSessions {
    async fn send_message(&self, sid: u32, msg: &str) {
        match self.get_session_hm(sid).await {
            Ok(addr) => addr.do_send(SessionMessage(msg.to_owned())),
            Err(e) => self.send_error(sid, &e).await,
        };
    }

    async fn send_error(&self, sid: u32, e: &ResError) {
        if let Ok(addr) = self.get_session_hm(sid).await {
            addr.do_send(SessionMessage(
                SendMessage::Error(e.stringify()).stringify(),
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

pub struct ReadWriteTalks(GlobalTalks);

impl From<GlobalTalks> for ReadWriteTalks {
    fn from(t: GlobalTalks) -> ReadWriteTalks {
        ReadWriteTalks(t)
    }
}

impl ReadWriteTalks {
    fn get_talk_hm(&self, talk_id: u32) -> impl Future<Output = Result<Talk, ResError>> + '_ {
        self.read_talks(move |t| t.get(&talk_id).cloned().ok_or(ResError::NotFound))
    }

    fn get_talks_hm(&self) -> impl Future<Output = Result<HashMap<u32, Talk>, ResError>> + '_ {
        self.read_talks(move |t| Ok(t.clone()))
    }

    fn insert_talk_hm(&self, talk: Talk) -> impl Future<Output = Result<(), ResError>> + '_ {
        self.write_talks(move |mut t| {
            t.insert(talk.id, talk)
                .map(|_| ())
                .ok_or(ResError::NotFound)
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
async fn send_message_many(
    talks: &ReadWriteTalks,
    sessions: &ReadWriteSessions,
    tid: u32,
    msg: &str,
) -> Result<(), ResError> {
    let t = talks.get_talk_hm(tid).await?;

    for u in t.users.iter() {
        sessions.send_message(*u, msg).await;
    }
    Ok(())
}

pub struct ReadWriteDb(Rc<RefCell<Client>>);

impl From<Rc<RefCell<Client>>> for ReadWriteDb {
    fn from(rc: Rc<RefCell<Client>>) -> ReadWriteDb {
        ReadWriteDb(rc)
    }
}

impl<'a> CrateClientLike<'a, RefMut<'a, Client>> for ReadWriteDb {
    fn cli_like(&'a self) -> RefMut<'a, Client> {
        (&*self.0).borrow_mut()
    }
}

pub struct ReadWriteCache(Rc<RefCell<SharedConnection>>, Option<String>);

impl From<(Rc<RefCell<SharedConnection>>, Option<String>)> for ReadWriteCache {
    fn from((rc, url): (Rc<RefCell<SharedConnection>>, Option<String>)) -> ReadWriteCache {
        ReadWriteCache(rc, url)
    }
}

impl GetSharedConn for ReadWriteCache {
    fn get_conn(&self) -> SharedConnection {
        (&*self.0).borrow().clone()
    }
}

impl CheckRedisConn for ReadWriteCache {
    fn self_url(&self) -> &str {
        self.1.as_ref().map(String::as_str).unwrap()
    }

    fn replace_redis(&self, c: SharedConnection) {
        self.0.replace(c);
    }
}

impl FromCache for ReadWriteCache {}

impl UsersFromCache for ReadWriteCache {}

impl ReadWriteCache {
    fn set_online_status(
        &self,
        uid: u32,
        status: u32,
        set_last_online_time: bool,
    ) -> impl Future<Output = Result<(), ResError>> {
        let conn = self.get_conn();

        let mut arg = Vec::with_capacity(2);
        arg.push(("online_status", status.to_string()));

        if set_last_online_time {
            arg.push(("last_online", Utc::now().naive_utc().to_string()))
        }

        cmd("HMSET")
            .arg(&format!("user:{}:set_perm", uid))
            .arg(arg)
            .query_async(conn)
            .err_into()
            .map_ok(|(_, ())| ())
    }
}

pub struct CheckPostgresMessage;

impl Message for CheckPostgresMessage {
    type Result = Result<(), ResError>;
}

// actor future have to be used as we want to access actor's context to replace the Rc<RefCell<_>>s
impl Handler<CheckPostgresMessage> for TalkService {
    type Result = ResponseActFuture<Self, (), ResError>;

    fn handle(&mut self, _msg: CheckPostgresMessage, _: &mut Self::Context) -> Self::Result {
        let url = self.db_url.to_owned();
        if self.db.borrow().is_closed() {
            Box::new(
                Box::pin(async move { TalkService::connect_postgres(&url).await })
                    .compat()
                    .into_actor(self)
                    .and_then(|(c, mut sts), act, _| {
                        let insert_pub_msg = sts.pop().unwrap();
                        let insert_prv_msg = sts.pop().unwrap();
                        let get_pub_msg = sts.pop().unwrap();
                        let get_prv_msg = sts.pop().unwrap();
                        let get_relations = sts.pop().unwrap();
                        let join_talk = sts.pop().unwrap();

                        act.db.replace(c);
                        act.insert_pub_msg.replace(insert_pub_msg);
                        act.insert_prv_msg.replace(insert_prv_msg);
                        act.get_pub_msg.replace(get_pub_msg);
                        act.get_prv_msg.replace(get_prv_msg);
                        act.get_relations.replace(get_relations);
                        act.join_talk.replace(join_talk);
                        actix::fut::ok(())
                    }),
            )
        } else {
            Box::new(actix::fut::ok(()))
        }
    }
}

pub struct CheckRedisMessage;

impl Message for CheckRedisMessage {
    type Result = Result<(), ResError>;
}

impl Handler<CheckRedisMessage> for TalkService {
    type Result = ResponseStdFuture<Result<(), ResError>>;

    fn handle(&mut self, _msg: CheckRedisMessage, _: &mut Self::Context) -> Self::Result {
        let cache = self.rw_cache_with_url();

        let f = async move {
            let opt = cache.check_redis().await?;
            cache.if_replace_redis(opt);
            Ok(())
        };

        ResponseStdFuture::from(f)
    }
}

#[derive(Message)]
pub struct DisconnectRequest {
    pub session_id: u32,
}

impl Handler<DisconnectRequest> for TalkService {
    type Result = ResponseStdFuture<()>;

    fn handle(&mut self, msg: DisconnectRequest, _: &mut Context<Self>) -> Self::Result {

        let cache = self.rw_cache();
        let sessions = self.rw_sessions();

        let f = async move {
            let sid = msg.session_id;

            let r: Result<(), ResError> = async {
                sessions.remove_session_hm(sid).await?;
                cache.set_online_status(sid, 0, true).await?;
                Ok(())
            }
                .await;

            if let Err(e) = r {
                sessions.send_error(sid, &e).await
            }
        };

        ResponseStdFuture::from(f)
    }
}

// pass Some(talk_id) in json for public message, pass None for private message
#[derive(Deserialize, Message)]
pub struct TextMessageRequest {
    pub text: String,
    pub talk_id: Option<u32>,
    pub user_id: Option<u32>,
    pub session_id: Option<u32>,
}

impl Handler<TextMessageRequest> for TalkService {
    type Result = ResponseStdFuture<()>;

    fn handle(&mut self, msg: TextMessageRequest, _: &mut Context<Self>) -> Self::Result {
        // ToDo: batch insert messages to database.

        let db = self.rw_db();
        let sessions = self.rw_sessions();
        let talks = self.rw_talks();

        let st_public_msg = self.insert_pub_msg.clone();
        let st_private_msg = self.insert_prv_msg.clone();

        let f = async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                let now = Utc::now().naive_utc();

                if let Some(tid) = msg.talk_id {
                    db.cli_like()
                        .as_cli()
                        .query_one::<PublicMessage>(
                            &(*st_public_msg).borrow(),
                            &[&tid, &msg.text, &now],
                        )
                        .await?;

                    let s = SendMessage::PublicMessage(&[PublicMessage {
                        text: msg.text,
                        time: now,
                        talk_id: msg.talk_id.unwrap(),
                    }])
                    .stringify();

                    send_message_many(&talks, &sessions, tid, s.as_str()).await
                } else {
                    let uid = msg.user_id.ok_or(ResError::BadRequest)?;
                    db.cli_like()
                        .as_cli()
                        .query_one::<PrivateMessage>(
                            &(*st_private_msg).borrow(),
                            &[&msg.session_id.unwrap(), &uid, &msg.text, &now],
                        )
                        .await?;

                    let s = SendMessage::PrivateMessage(&[PrivateMessage {
                        user_id: msg.user_id.unwrap(),
                        text: msg.text,
                        time: now,
                    }])
                    .stringify();

                    sessions.send_message(sid, s.as_str()).await;
                    Ok(())
                }
            }
                .await;

            if let Err(e) = r {
                sessions.send_error(sid, &e).await;
            }
        };

        ResponseStdFuture::from(f)
    }
}

#[derive(Message)]
pub struct ConnectRequest {
    pub session_id: u32,
    pub online_status: u32,
    pub addr: Addr<WsChatSession>,
}

impl Handler<ConnectRequest> for TalkService {
    type Result = ResponseStdFuture<()>;

    fn handle(&mut self, msg: ConnectRequest, _: &mut Context<Self>) -> Self::Result {
        let cache = self.rw_cache();
        let sessions = self.rw_sessions();

        let f = async move {
            let sid = msg.session_id;

            let r = async {
                let status = msg.online_status;
                let addr = msg.addr;

                cache.set_online_status(sid, status, true).await?;

                sessions.insert_session_hm(sid, addr.clone()).await?;

                addr.do_send(SessionMessage(
                    SendMessage::Success("Connection Success").stringify(),
                ));
                Ok(())
            }
                .await;

            if let Err(e) = r {
                sessions.send_error(sid, &e).await;
            }
        };

        ResponseStdFuture::from(f)
    }
}

#[derive(Deserialize, Message, Clone)]
pub struct CreateTalkRequest {
    pub session_id: Option<u32>,
    pub name: String,
    pub description: String,
    pub owner: u32,
}

impl Handler<CreateTalkRequest> for TalkService {
    type Result = ResponseStdFuture<()>;

    fn handle(&mut self, msg: CreateTalkRequest, _: &mut Context<Self>) -> Self::Result {
        let db = self.rw_db();
        let talks = self.rw_talks();
        let sessions = self.rw_sessions();

        let f = async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                let admins = vec![msg.owner];

                let st = db.cli_like().prepare("SELECT Max(id) FROM talks").await?;
                let t: Talk = db.cli_like().as_cli().query_one::<Talk>(&st, &[]).await?;
                let last_tid = t.id;

                let st = db.cli_like().prepare(INSERT_TALK).await?;
                let t = db
                    .cli_like()
                    .as_cli()
                    .query_one(
                        &st,
                        &[
                            &(last_tid + 1),
                            &msg.name,
                            &msg.description,
                            &msg.owner,
                            &admins,
                            &admins,
                        ],
                    )
                    .await?;

                let s = SendMessage::Talks(vec![&t]).stringify();
                talks.insert_talk_hm(t).await?;
                sessions.send_message(msg.owner, s.as_str()).await;
                Ok(())
            }
                .await;

            if let Err(e) = r {
                sessions.send_error(sid, &e).await;
            }
        };

        ResponseStdFuture::from(f)
    }
}

#[derive(Deserialize, Message)]
pub struct JoinTalkRequest {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

impl Handler<JoinTalkRequest> for TalkService {
    type Result = ResponseStdFuture<()>;

    fn handle(&mut self, msg: JoinTalkRequest, _: &mut Context<Self>) -> Self::Result {
        let db = self.rw_db();
        let talks = self.rw_talks();
        let sessions = self.rw_sessions();

        let st_join_talk = self.join_talk.clone();

        let f = async move {
            let sid = msg.session_id.unwrap();
            let r = async {
                let tid = msg.talk_id;

                let t = talks.get_talk_hm(tid).await?;
                if t.users.contains(&sid) {
                    return Err(ResError::BadRequest);
                }

                let t = db
                    .cli_like()
                    .as_cli()
                    .query_one::<Talk>(&(*st_join_talk).borrow(), &[&sid, &tid])
                    .await?;
                let s = SendMessage::Talks(vec![&t]).stringify();

                talks.insert_talk_hm(t).await?;
                sessions.send_message(sid, s.as_str()).await;

                Ok(())
            }
                .await;

            if let Err(e) = r {
                sessions.send_error(sid, &e).await;
            }
        };

        ResponseStdFuture::from(f)
    }
}

#[derive(Message, Deserialize)]
pub struct TalkByIdRequest {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

impl Handler<TalkByIdRequest> for TalkService {
    type Result = ResponseStdFuture<()>;
    fn handle(&mut self, msg: TalkByIdRequest, _: &mut Context<Self>) -> Self::Result {
        let talks = self.rw_talks();
        let sessions = self.rw_sessions();

        let f = async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                let talks = talks.get_talks_hm().await?;

                // we return all talks if the query talk_id is 0
                let t = match msg.talk_id {
                    0 => talks.iter().map(|(_, t)| t).collect(),
                    _ => talks
                        .get(&msg.talk_id)
                        .map(|t| vec![t])
                        .unwrap_or_else(|| vec![]),
                };

                let s = SendMessage::Talks(t).stringify();
                sessions.send_message(sid, s.as_str()).await;

                Ok(())
            }
                .await;

            if let Err(e) = r {
                sessions.send_error(sid, &e).await;
            }
        };

        ResponseStdFuture::from(f)
    }
}

#[derive(Message, Deserialize)]
pub struct UsersByIdRequest {
    pub session_id: Option<u32>,
    user_id: Vec<u32>,
}

impl Handler<UsersByIdRequest> for TalkService {
    type Result = ResponseStdFuture<()>;
    fn handle(&mut self, msg: UsersByIdRequest, _: &mut Context<Self>) -> Self::Result {
        let cache = self.rw_cache();
        let sessions = self.rw_sessions();

        let f = async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                // ToDo: remove compat layer
                let u = cache.users_from_cache(msg.user_id).await?;
                let s = SendMessage::Users(&u).stringify();

                sessions.send_message(sid, s.as_str()).await;
                Ok(())
            }
                .await;
            if let Err(e) = r {
                sessions.send_error(sid, &e).await;
            }
        };

        ResponseStdFuture::from(f)
    }
}

#[derive(Message, Deserialize)]
pub struct UserRelationRequest {
    pub session_id: Option<u32>,
}

impl Handler<UserRelationRequest> for TalkService {
    type Result = ResponseStdFuture<()>;
    fn handle(&mut self, msg: UserRelationRequest, _: &mut Context<Self>) -> Self::Result {
        let db = self.rw_db();
        let sessions = self.rw_sessions();
        let st_relation = self.get_relations.clone();

        let f = async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                let r: Relation = db
                    .cli_like()
                    .as_cli()
                    .query_one(&(*st_relation).borrow(), &[&sid])
                    .await?;
                let s = SendMessage::Friends(&r.friends).stringify();

                sessions.send_message(sid, s.as_str()).await;

                Ok(())
            }
                .await;

            if let Err(e) = r {
                sessions.send_error(sid, &e).await;
            }
        };

        ResponseStdFuture::from(f)
    }
}

// pass talk id for talk public messages. pass none for private history message.
#[derive(Deserialize, Message)]
pub struct GetHistory {
    pub time: String,
    pub talk_id: Option<u32>,
    pub session_id: Option<u32>,
}

impl Handler<GetHistory> for TalkService {
    type Result = ResponseStdFuture<()>;

    fn handle(&mut self, msg: GetHistory, _: &mut Context<Self>) -> Self::Result {
        let db = self.rw_db();
        let sessions = self.rw_sessions();

        let st_public_msg = self.get_pub_msg.clone();
        let st_private_msg = self.get_prv_msg.clone();

        let f = async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                let time = NaiveDateTime::parse_from_str(&msg.time, "%Y-%m-%d %H:%M:%S%.f")?;

                let s = match msg.talk_id {
                    Some(tid) => {
                        let msg = db
                            .cli_like()
                            .as_cli()
                            .query_multi::<PublicMessage>(
                                &(*st_public_msg.borrow()),
                                &[&tid, &time],
                                Vec::with_capacity(20),
                            )
                            .await?;
                        SendMessage::PublicMessage(&msg).stringify()
                    }
                    None => {
                        let msg = db
                            .cli_like()
                            .as_cli()
                            .query_multi::<PrivateMessage>(
                                &(*st_private_msg.borrow()),
                                &[&sid, &time],
                                Vec::with_capacity(20),
                            )
                            .await?;
                        SendMessage::PrivateMessage(&msg).stringify()
                    }
                };

                sessions.send_message(sid, s.as_str()).await;
                Ok(())
            }
                .await;

            if let Err(e) = r {
                sessions.send_error(sid, &e).await;
            }
        };

        ResponseStdFuture::from(f)
    }
}

#[derive(Deserialize, Message)]
pub struct RemoveUserRequest {
    pub session_id: Option<u32>,
    user_id: u32,
    talk_id: u32,
}

impl Handler<RemoveUserRequest> for TalkService {
    type Result = ResponseStdFuture<()>;

    fn handle(&mut self, msg: RemoveUserRequest, _: &mut Context<Self>) -> Self::Result {
        let db = self.rw_db();
        let talks = self.rw_talks();
        let sessions = self.rw_sessions();

        let f = async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                let tid = msg.talk_id;
                let uid = msg.user_id;

                let talk = talks.get_talk_hm(tid).await?;

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

                let st = db.cli_like().prepare(REMOVE_USER).await?;
                let talk = db
                    .cli_like()
                    .as_cli()
                    .query_one::<Talk>(&st, &[&uid, &tid])
                    .await?;

                let s = SendMessage::Talks(vec![&talk]).stringify();
                talks.insert_talk_hm(talk).await?;
                sessions.send_message(sid, s.as_str()).await;

                Ok(())
            }
                .await;

            if let Err(e) = r {
                sessions.send_error(sid, &e).await;
            }
        };

        ResponseStdFuture::from(f)
    }
}

#[derive(Deserialize, Message)]
pub struct Admin {
    pub add: Option<u32>,
    pub remove: Option<u32>,
    pub talk_id: u32,
    pub session_id: Option<u32>,
}

impl Handler<Admin> for TalkService {
    type Result = ResponseStdFuture<()>;

    fn handle(&mut self, msg: Admin, _: &mut Context<Self>) -> Self::Result {
        let db = self.rw_db();
        let talks = self.rw_talks();
        let sessions = self.rw_sessions();

        let f = async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                let tid = msg.talk_id;

                let (query, uid) = if let Some(uid) = msg.add {
                    (INSERT_ADMIN, uid)
                } else {
                    let uid = msg.remove.ok_or(ResError::BadRequest)?;
                    (REMOVE_ADMIN, uid)
                };

                let st = db.cli_like().prepare(query).await?;
                let t = db
                    .cli_like()
                    .as_cli()
                    .query_one::<Talk>(&st, &[&uid, &tid, &sid])
                    .await?;

                let s = SendMessage::Talks(vec![&t]).stringify();
                talks.insert_talk_hm(t).await?;
                sessions.send_message(sid, s.as_str()).await;

                Ok(())
            }
                .await;

            if let Err(e) = r {
                sessions.send_error(sid, &e).await;
            }
        };

        ResponseStdFuture::from(f)
    }
}

#[derive(Deserialize, Message)]
pub struct DeleteTalkRequest {
    pub session_id: Option<u32>,
    pub talk_id: u32,
}

impl Handler<DeleteTalkRequest> for TalkService {
    type Result = ResponseStdFuture<()>;

    fn handle(&mut self, msg: DeleteTalkRequest, _: &mut Context<Self>) -> Self::Result {
        let talks = self.rw_talks();
        let sessions = self.rw_sessions();
        let db = self.rw_db();

        let f = async move {
            let sid = msg.session_id.unwrap();

            let r = async {
                let tid = msg.talk_id;

                let st = db.cli_like().prepare(REMOVE_TALK).await?;
                let _r = db.cli_like().execute(&st, &[&tid]).await?;

                talks.remove_talk_hm(tid).await?;

                let s = SendMessage::Success("Delete Talk Success").stringify();
                sessions.send_message(sid, s.as_str()).await;

                Ok(())
            }
                .await;

            if let Err(e) = r {
                sessions.send_error(sid, &e).await;
            }
        };

        ResponseStdFuture::from(f)
    }
}