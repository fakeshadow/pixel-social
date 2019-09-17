use std::{
    collections::VecDeque, convert::TryInto, fmt::Write, future::Future, pin::Pin, sync::Arc,
    time::Duration,
};

use chrono::Utc;
use futures::{channel::mpsc::UnboundedReceiver, lock::Mutex as FutMutex, FutureExt, TryFutureExt};
use psn_api_rs::{PSNRequest as PSNRequestLib, PSN};
use redis::aio::SharedConnection;
use tokio_postgres::{Client, Statement};

use crate::handler::{
    cache::{CacheService, CheckRedisMut, GetSharedConn},
    db::{AsCrateClient, CrateClientLike, DatabaseService},
};
use crate::model::runtime::SpawnIntervalHandler;
use crate::model::{
    common::{dur, dur_as_sec},
    errors::ResError,
    psn::{
        PSNTrophyArgumentRequest, PSNUserLib, TrophySetLib, TrophyTitlesLib, UserPSNProfile,
        UserTrophySet, UserTrophyTitle,
    },
    runtime::{ChannelAddress, ChannelCreate, SpawnQueueHandler},
};

const PSN_REQ_INTERVAL: Duration = dur(3000);

// how often user can sync their data to psn in seconds.
const PROFILE_TIME_GATE: i64 = dur_as_sec(900_000);
const TROPHY_TITLES_TIME_GATE: i64 = dur_as_sec(900_000);
const TROPHY_SET_TIME_GATE: i64 = dur_as_sec(900_000);

const PSN_TITLES_NY_TIME: &str = "SELECT * FROM psn_user_trophy_titles WHERE np_id=$1 ORDER BY last_update_date DESC OFFSET $2 LIMIT 20";
const PSN_SET_BY_NPID: &str =
    "SELECT * FROM psn_user_trophy_sets WHERE np_id=$1 and np_communication_id=$2";
const INSERT_TITLES: &str =
    "INSERT INTO psn_user_trophy_titles (np_id, np_communication_id, progress, earned_platinum, earned_gold, earned_silver, earned_bronze, last_update_date)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (np_id, np_communication_id) DO UPDATE SET
            progress = CASE WHEN psn_user_trophy_titles.progress < EXCLUDED.progress
                THEN EXCLUDED.progress
                ELSE psn_user_trophy_titles.progress
                END,
            earned_platinum = CASE WHEN psn_user_trophy_titles.earned_platinum < EXCLUDED.earned_platinum
                THEN EXCLUDED.earned_platinum
                ELSE psn_user_trophy_titles.earned_platinum
                END,
            earned_gold = CASE WHEN psn_user_trophy_titles.earned_gold < EXCLUDED.earned_gold
                THEN EXCLUDED.earned_gold
                ELSE psn_user_trophy_titles.earned_gold
                END,
            earned_silver = CASE WHEN psn_user_trophy_titles.earned_silver < EXCLUDED.earned_silver
                THEN EXCLUDED.earned_silver
                ELSE psn_user_trophy_titles.earned_silver
                END,
            earned_bronze = CASE WHEN psn_user_trophy_titles.earned_bronze < EXCLUDED.earned_bronze
                THEN EXCLUDED.earned_bronze
                ELSE psn_user_trophy_titles.earned_bronze
                END,
            last_update_date = CASE WHEN psn_user_trophy_titles.last_update_date < EXCLUDED.last_update_date
                THEN EXCLUDED.last_update_date
                ELSE psn_user_trophy_titles.last_update_date
                END,
            is_visible = CASE WHEN psn_user_trophy_titles.progress > EXCLUDED.progress
                THEN FALSE
                ELSE TRUE
                END";

pub struct PSNService {
    pub db_url: String,
    pub cache_url: String,
    pub psn: PSN,
    pub db: tokio_postgres::Client,
    pub insert_trophy_title: Statement,
    pub cache: SharedConnection,
    pub queue: Arc<FutMutex<VecDeque<PSNRequest>>>,
}

impl ChannelCreate for PSNService {
    type Message = (PSNRequest, bool);
}

impl GetSharedConn for PSNService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.clone()
    }
}

impl CheckRedisMut for PSNService {
    fn self_url(&self) -> &str {
        self.cache_url.as_str()
    }

    fn replace_redis_mut(&mut self, c: SharedConnection) {
        self.cache = c;
    }
}

struct SharedPSNService(Arc<FutMutex<PSNService>>);

impl From<Arc<FutMutex<PSNService>>> for SharedPSNService {
    fn from(p: Arc<FutMutex<PSNService>>) -> SharedPSNService {
        SharedPSNService(p)
    }
}

impl SpawnIntervalHandler for SharedPSNService {
    fn handle<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), ResError>> + Send + 'a>> {
        Box::pin(async move {
            let mut psn = self.0.lock().await;

            // pattern match PSNRequest and handle the PSN network along with postgres and redis requests.

            // check tokens and refresh access token. then pop the front entry from queue.
            let queue = psn.check_token().await?.queue.lock().await.pop_front();

            if let Some(r) = queue {
                match r {
                    PSNRequest::Profile { online_id } => {
                        psn.handle_profile_request(online_id).await
                    }
                    PSNRequest::TrophyTitles { online_id, .. } => {
                        let r = psn.handle_trophy_titles_request(online_id).await?;
                        // only check db connection when update user trophy titles.
                        psn.check_conn_db()
                            .await?
                            .update_user_trophy_titles(&r)
                            .await
                    }
                    PSNRequest::TrophySet {
                        online_id,
                        np_communication_id,
                    } => {
                        let r = psn
                            .handle_trophy_set_request(online_id, np_communication_id)
                            .await?;
                        psn.query_update_user_trophy_set(r).await
                    }
                    PSNRequest::Auth {
                        uuid,
                        two_step,
                        refresh_token,
                    } => psn.handle_auth_request(uuid, two_step, refresh_token).await,
                    PSNRequest::Activation {
                        user_id,
                        online_id,
                        code,
                    } => {
                        psn.handle_activation_request(user_id, online_id, code)
                            .await
                    }
                }
            } else {
                Ok(())
            }
        })
    }
}

impl PSNService {
    async fn check_conn_db(&mut self) -> Result<&mut Self, ResError> {
        if self.db.is_closed() {
            let (c, mut sts) = PSNService::connect(self.db_url.as_str()).await?;
            let insert_trophy_title = sts.pop().unwrap();
            self.db = c;
            self.insert_trophy_title = insert_trophy_title;
            Ok(self)
        } else {
            Ok(self)
        }
    }

    async fn connect(postgres_url: &str) -> Result<(Client, Vec<Statement>), ResError> {
        let (mut db, conn) = tokio_postgres::connect(postgres_url, tokio_postgres::NoTls).await?;

        let conn = conn.map(|_| ());
        tokio::spawn(conn);

        let insert_trophy_title = db.prepare(INSERT_TITLES).await?;

        Ok((db, vec![insert_trophy_title]))
    }

    pub(crate) async fn init(
        postgres_url: &str,
        redis_url: &str,
    ) -> Result<ChannelAddress<(PSNRequest, bool)>, ResError> {
        let db_url = postgres_url.to_owned();
        let cache_url = redis_url.to_owned();

        let cache = crate::handler::cache::connect_cache(redis_url)
            .await?
            .ok_or(ResError::RedisConnection)?;

        let (db, mut sts) = PSNService::connect(postgres_url).await?;
        let insert_trophy_title = sts.pop().unwrap();

        // use an unbounded channel to inject request to queue from other threads.
        let (addr, receiver) = PSNService::create_channel();

        // queue is passed to both PSNService and QueueInjector.
        let queue = Arc::new(FutMutex::new(VecDeque::new()));

        // run queue injector in a separate future.
        PSNQueue::new(queue.clone(), receiver).spawn_queue();

        // use double Arc<Mutex<_>> as the queue is passed to both PSNQueue and PSNService
        let psn = Arc::new(FutMutex::new(PSNService {
            db_url,
            cache_url,
            psn: PSN::new(),
            db,
            insert_trophy_title,
            cache,
            queue: queue.clone(),
        }));

        // run interval functions handle PSNService in a spawned future.
        SharedPSNService::from(psn).spawn_interval(PSN_REQ_INTERVAL);

        Ok(addr)
    }
}

struct PSNQueue {
    queue: Arc<FutMutex<VecDeque<PSNRequest>>>,
    receiver: UnboundedReceiver<(PSNRequest, bool)>,
    // stores all reqs' timestamp goes to PSN.
    // profile request use <online_id> as key,
    // trophy_list request use <online_id:::titles> as key,
    // trophy_set request use <online_id:::np_communication_id> as key
    // chrono::Utc::now().timestamp is score
    time_gate: hashbrown::HashMap<Vec<u8>, i64>,
}

impl SpawnQueueHandler<(PSNRequest, bool)> for PSNQueue {
    type Error = ResError;

    fn receiver(&mut self) -> &mut UnboundedReceiver<(PSNRequest, bool)> {
        &mut self.receiver
    }

    fn handle_message<'a>(
        &'a mut self,
        msg: (PSNRequest, bool),
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>> {
        let (req, is_front) = msg;
        Box::pin(async move {
            // push new PSNRequest to VecDeque according to the hash map of time_gate(to throw away spam requests by using time gate)
            if self.should_add_queue(&req) {
                self.update_time_stamp(&req);
                self.add_to_queue(req, is_front).await;
            }
            Ok(())
        })
    }
}

impl PSNQueue {
    fn new(
        queue: Arc<FutMutex<VecDeque<PSNRequest>>>,
        receiver: UnboundedReceiver<(PSNRequest, bool)>,
    ) -> Self {
        PSNQueue {
            queue,
            receiver,
            time_gate: hashbrown::HashMap::new(),
        }
    }

    async fn add_to_queue(&mut self, req: PSNRequest, is_front: bool) {
        let mut queue = self.queue.lock().await;
        if is_front {
            queue.push_front(req);
        } else {
            queue.push_back(req);
        }
    }

    fn should_add_queue(&self, req: &PSNRequest) -> bool {
        let time_gate = match req {
            PSNRequest::Profile { .. } => PROFILE_TIME_GATE,
            PSNRequest::TrophyTitles { .. } => TROPHY_TITLES_TIME_GATE,
            PSNRequest::TrophySet { .. } => TROPHY_SET_TIME_GATE,
            _ => return true,
        };

        !self.is_in_time_gate(req.generate_entry_key().as_slice(), time_gate)
    }

    fn is_in_time_gate(&self, entry: &[u8], time_gate: i64) -> bool {
        if let Some(timestamp) = self.time_gate.get(entry) {
            if (Utc::now().timestamp() - *timestamp) < time_gate {
                return true;
            }
        }
        false
    }

    fn update_time_stamp(&mut self, req: &PSNRequest) {
        let key = req.generate_entry_key();
        let time = Utc::now().timestamp();
        self.time_gate.insert(key, time);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "query_type")]
pub enum PSNRequest {
    Profile {
        online_id: String,
    },
    TrophyTitles {
        online_id: String,
        page: String,
    },
    TrophySet {
        online_id: String,
        np_communication_id: String,
    },
    Auth {
        uuid: Option<String>,
        two_step: Option<String>,
        refresh_token: Option<String>,
    },
    Activation {
        user_id: Option<u32>,
        online_id: String,
        code: String,
    },
}

impl PSNRequest {
    pub(crate) fn check_privilege(self, privilege: u32) -> Result<Self, ResError> {
        if privilege < 9 {
            Err(ResError::Unauthorized)
        } else {
            Ok(self)
        }
    }

    pub(crate) fn attach_user_id(self, uid: u32) -> Self {
        if let PSNRequest::Activation {
            online_id, code, ..
        } = self
        {
            PSNRequest::Activation {
                user_id: Some(uid),
                online_id,
                code,
            }
        } else {
            self
        }
    }

    fn generate_entry_key(&self) -> Vec<u8> {
        let mut entry = Vec::new();
        match self {
            PSNRequest::Profile { online_id } => {
                entry.extend_from_slice(online_id.as_bytes());
                entry
            }
            PSNRequest::TrophyTitles { online_id, .. } => {
                entry.extend_from_slice(online_id.as_bytes());
                entry.extend_from_slice(b":::titles");
                entry
            }
            PSNRequest::TrophySet {
                online_id,
                np_communication_id,
            } => {
                entry.extend_from_slice(online_id.as_bytes());
                entry.extend_from_slice(b":::");
                entry.extend_from_slice(np_communication_id.as_bytes());
                entry
            }
            _ => vec![],
        }
    }
}

impl PSNService {
    // handle_xxx_request are mostly the network request to PSN.
    // update_xxx are mostly postgres and redis write operation.
    fn handle_auth_request<'a>(
        &'a mut self,
        uuid: Option<String>,
        two_step: Option<String>,
        refresh_token: Option<String>,
    ) -> impl Future<Output = Result<(), ResError>> + 'a {
        let mut psn = PSN::new();

        if let Some(uuid) = uuid {
            if let Some(two_step) = two_step {
                psn = psn.add_uuid(uuid).add_two_step(two_step);
            }
        };

        if let Some(refresh_token) = refresh_token {
            psn = psn.add_refresh_token(refresh_token);
        }

        psn.auth().map_err(ResError::from).map_ok(move |p| {
            println!("{:#?}", p);
            self.psn = p;
        })
    }

    async fn handle_activation_request(
        &mut self,
        user_id: Option<u32>,
        online_id: String,
        code: String,
    ) -> Result<(), ResError> {
        let u: PSNUserLib = self.psn.add_online_id(online_id).get_profile().await?;

        if u.about_me == code {
            let mut u = UserPSNProfile::from(u);
            u.id = user_id;
            self.update_profile_cache(u);
            Ok(())
        } else {
            // ToDo: add more error detail and send it through message to user.
            Err(ResError::Unauthorized)
        }
    }

    async fn handle_trophy_titles_request(
        &mut self,
        online_id: String,
    ) -> Result<Vec<UserTrophyTitle>, ResError> {
        // get profile before and after getting titles and check if the user's np_id remains unchanged.
        let u: PSNUserLib = self.psn.add_online_id(online_id).get_profile().await?;
        let titles_first: TrophyTitlesLib = self.psn.get_titles(0).await?;

        let total = titles_first.total_results;
        let page = total / 100;
        let mut f = Vec::with_capacity(page as usize);
        for i in 0..page {
            f.push(
                self.psn
                    .get_titles::<TrophyTitlesLib>((i + 1) * 100)
                    .map_err(ResError::from),
            )
        }

        let titles: Vec<Result<TrophyTitlesLib, ResError>> = futures::future::join_all(f).await;

        let mut v: Vec<UserTrophyTitle> = Vec::new();

        for title in titles_first.trophy_titles.into_iter() {
            if let Ok(title) = title.try_into() {
                v.push(title)
            }
        }

        for titles in titles.into_iter() {
            if let Ok(titles) = titles {
                for title in titles.trophy_titles.into_iter() {
                    if let Ok(title) = title.try_into() {
                        v.push(title)
                    }
                }
            }
        }

        let uu: PSNUserLib = self.psn.get_profile().await?;

        if u.np_id.as_str() == uu.np_id.as_str() && u.online_id.as_str() == uu.online_id.as_str() {
            let np_id = uu.np_id;

            let v = v
                .into_iter()
                .map(|mut t| {
                    t.np_id = np_id.clone();
                    t
                })
                .collect();

            Ok(v)
        } else {
            Err(ResError::Unauthorized)
        }
    }

    async fn handle_trophy_set_request(
        &mut self,
        online_id: String,
        np_communication_id: String,
    ) -> Result<UserTrophySet, ResError> {
        let u: PSNUserLib = self.psn.add_online_id(online_id).get_profile().await?;

        let set: TrophySetLib = self
            .psn
            .add_np_communication_id(np_communication_id.clone())
            .get_trophy_set()
            .await?;

        let uu: PSNUserLib = self.psn.get_profile().await?;

        if u.np_id == uu.np_id && u.online_id == uu.online_id {
            Ok(UserTrophySet {
                np_id: uu.np_id,
                np_communication_id,
                is_visible: true,
                trophies: set.trophies.iter().map(|t| t.into()).collect(),
            })
        } else {
            Err(ResError::Unauthorized)
        }
    }

    async fn handle_profile_request(&mut self, online_id: String) -> Result<(), ResError> {
        let u: PSNUserLib = self.psn.add_online_id(online_id).get_profile().await?;

        self.update_profile_cache(u.into());

        Ok(())
    }

    fn update_profile_cache(&self, p: UserPSNProfile) {
        let conn = self.get_conn();
        tokio::spawn(
            crate::handler::cache::build_hmsets(
                conn,
                &[p],
                crate::handler::cache::USER_PSN_U8,
                false,
            )
            .map(|_| ()),
        );
    }

    // a costly update for updating existing trophy set.
    // The purpose is to flag people who have a changed trophy timestamp on the trophy already earned
    // by comparing the The first_earned_date with the earned_date
    async fn query_update_user_trophy_set(&mut self, mut t: UserTrophySet) -> Result<(), ResError> {
        let st = self.db.prepare(PSN_SET_BY_NPID).await?;

        let r = (&mut self.db)
            .as_cli()
            .query_one::<UserTrophySet>(&st, &[&t.np_id.as_str(), &t.np_communication_id.as_str()])
            .await;

        match r {
            Ok(t_old) => {
                // count earned_date from existing user trophy set.
                // if the count is reduced then we mark this trophy set not visible.
                let mut earned_count = 0;
                let mut earned_count_old = 0;
                // ToDo: handle case when user hide this trophy set.
                for t in t.trophies.iter_mut() {
                    if t.earned_date.is_some() {
                        earned_count += 1;
                    }

                    // iter existing trophy set and keep the first_earned_date if it's Some().

                    for t_old in t_old.trophies.iter() {
                        if t.trophy_id == t_old.trophy_id {
                            if t_old.first_earned_date.is_some() {
                                earned_count_old += 1;
                                t.first_earned_date = t_old.first_earned_date;
                                if t.earned_date.is_none() {
                                    t.earned_date = t_old.earned_date;
                                }
                            }
                            break;
                        }
                    }
                }

                if earned_count < earned_count_old {
                    t.is_visible = false;
                }
            }
            // if we get rows from db successfully but failed to parse it to data
            // then it's better to look into the data before overwriting it with the following upsert
            // as we don't want to lose any first_earned_date.
            Err(e) => match e {
                ResError::DataBaseReadError => return Err(e),
                ResError::ParseError => return Err(e),
                _ => {}
            },
        };
        self.update_user_trophy_set(&t).await
    }

    async fn update_user_trophy_set(&mut self, t: &UserTrophySet) -> Result<(), ResError> {
        let mut query = String::new();

        let _ = write!(
            &mut query,
            "INSERT INTO psn_user_trophy_sets
                        (np_id, np_communication_id, trophy_set)
                        VALUES ('{}', '{}', '{{",
            t.np_id.as_str(),
            t.np_communication_id.as_str()
        );

        for t in t.trophies.iter() {
            let _ = write!(&mut query, "\"({},", t.trophy_id);
            let _ = match t.earned_date {
                Some(date) => write!(&mut query, "{},", date),
                None => write!(&mut query, ","),
            };
            let _ = match t.first_earned_date {
                Some(date) => write!(&mut query, "{})\",", date),
                None => write!(&mut query, ")\","),
            };
        }

        if !query.ends_with(',') {
            return Err(ResError::BadRequest);
        }
        query.remove(query.len() - 1);
        query.push_str(
            "}')
                ON CONFLICT (np_id, np_communication_id)
                    DO UPDATE SET
                        trophy_set = EXCLUDED.trophy_set,
                        is_visible = EXCLUDED.is_visible
                            RETURNING NULL;",
        );

        (&mut self.db)
            .as_cli()
            .simple_query_row(query.as_str())
            .map_ok(|_| ())
            .await
    }

    fn update_user_trophy_titles(
        &mut self,
        t: &[UserTrophyTitle],
    ) -> impl Future<Output = Result<(), ResError>> + Send {
        let mut v = Vec::with_capacity(t.len());
        let st = &self.insert_trophy_title;
        let c = &mut self.db;

        for t in t.iter() {
            let f = c.execute(
                &st,
                &[
                    &t.np_id,
                    &t.np_communication_id,
                    &t.progress,
                    &t.earned_platinum,
                    &t.earned_gold,
                    &t.earned_silver,
                    &t.earned_bronze,
                    &t.last_update_date,
                ],
            );

            v.push(f);
        }

        futures::future::join_all(v).map(|_v| Ok(()))
    }

    async fn check_token(&mut self) -> Result<&mut Self, ResError> {
        if self.psn.should_refresh() {
            self.psn.gen_access_from_refresh().await?;
            Ok(self)
        } else {
            Ok(self)
        }
    }
}

// impl methods used in psn router.
impl DatabaseService {
    pub(crate) async fn get_trophy_titles(
        &self,
        np_id: &str,
        page: u32,
    ) -> Result<Vec<UserTrophyTitle>, ResError> {
        let st = self.cli_like().prepare(PSN_TITLES_NY_TIME).await?;
        let offset = (page - 1) * 20;

        self.cli_like()
            .as_cli()
            .query_multi(&st, &[&np_id, &offset], Vec::with_capacity(20))
            .await
    }

    pub(crate) async fn get_trophy_set(
        &self,
        np_id: &str,
        np_communication_id: &str,
    ) -> Result<UserTrophySet, ResError> {
        let st = self.cli_like().prepare(PSN_SET_BY_NPID).await?;

        self.cli_like()
            .as_cli()
            .query_one(&st, &[&np_id, &np_communication_id])
            .await
    }

    //    pub fn update_trophy_set_argument(
    //        &self,
    //        req: PSNTrophyArgumentRequest,
    //    ) -> impl Future<Output=Result<(), ResError>> {
    //        let mut query = String::from("INSERT INTO ");
    //
    //
    //
    //    }
}

impl CacheService {
    pub(crate) fn get_psn_profile(
        &self,
        online_id: &str,
    ) -> impl Future<Output = Result<UserPSNProfile, ResError>> + Send {
        use crate::handler::cache::FromCacheSingle;
        self.from_cache_single(online_id, "user_psn")
    }
}
