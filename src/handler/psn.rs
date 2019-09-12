use std::{
    cell::{RefCell, RefMut},
    collections::VecDeque,
    convert::TryInto,
    fmt::Write,
    future::Future,
    sync::Arc,
    time::Duration,
};

use chrono::Utc;
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    compat::Future01CompatExt,
    lock::Mutex,
    FutureExt, SinkExt, StreamExt, TryFutureExt,
};
use futures01::Future as Future01;
use psn_api_rs::{PSNRequest as PSNRequestLib, PSN};
use redis::aio::SharedConnection;
use tokio::timer::Interval;
use tokio_postgres::Statement;

use crate::handler::{
    cache::{CacheService, CheckCacheConn, GetSharedConn},
    db::{DatabaseService, GetDbClient, Query, SimpleQuery},
};
use crate::model::{
    errors::ResError,
    psn::{
        PSNTrophyArgumentRequest, PSNUserLib, TrophySetLib, TrophyTitlesLib, UserPSNProfile,
        UserTrophySet, UserTrophyTitle,
    },
};

const PSN_TIME_GAP: Duration = Duration::from_millis(3000);
const PSN_REQUEST_TIME_OUT: Duration = Duration::from_millis(6000);

// how often user can sync their data to psn in seconds.
const PROFILE_TIME_GATE: i64 = Duration::from_secs(900).as_secs() as i64;
const TROPHY_TITLES_TIME_GATE: i64 = Duration::from_secs(900).as_secs() as i64;
const TROPHY_SET_TIME_GATE: i64 = Duration::from_secs(900).as_secs() as i64;

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
    pub db: RefCell<tokio_postgres::Client>,
    pub insert_trophy_title: RefCell<Statement>,
    pub cache: RefCell<SharedConnection>,
    pub queue: Arc<Mutex<VecDeque<PSNRequest>>>,
}

impl PSNService {
    pub(crate) async fn init(
        postgres_url: &str,
        redis_url: &str,
    ) -> Result<PSNServiceAddr, ResError> {
        let db_url = postgres_url.to_owned();
        let cache_url = redis_url.to_owned();

        let cache = crate::handler::cache::connect_cache(redis_url)
            .await?
            .ok_or(ResError::RedisConnection)?;
        let (mut db, conn) = tokio_postgres::connect(postgres_url, tokio_postgres::NoTls).await?;

        //ToDo: remove compat layer when actix convert to use std::future;
        let conn = conn.map(|_| ());
        actix::spawn(conn.unit_error().boxed().compat());

        let p1 = db.prepare(INSERT_TITLES).await?;

        // use an unbounded channel to inject request to queue from other threads.
        let (addr, receiver) = futures::channel::mpsc::unbounded::<(PSNRequest, bool)>();
        
        // queue is passed to both PSNService and QueueInjector.
        let queue = Arc::new(Mutex::new(VecDeque::new()));

        // run queue injector in a separate future.
        QueueInjector::new(queue.clone(), receiver).handle_inject();

        let mut psn = PSNService {
            db_url,
            cache_url,
            psn: PSN::new(),
            db: RefCell::new(db),
            insert_trophy_title: RefCell::new(p1),
            cache: RefCell::new(cache),
            queue: queue.clone(),
        };

        // run interval futures which handle PSNService in a separate future.
        // ToDo: currently tokio 0.2 interval futures can't gracefully shut down when running tokio runtime along with actix runtime. So a compat layer is needed.
        actix::spawn(
            async move {
                let mut interval = Interval::new_interval(PSN_TIME_GAP);
                use tokio::future::FutureExt as TokioFutureExt;
                loop {
                    interval.next().await;
                    // set a timeout for the looped future
                    // ToDo: handle errors.
                    if let Ok(result) = psn.handle_queue().timeout(PSN_REQUEST_TIME_OUT).await {
                        if let Err(e) = result {
                            println!("{:?}", e.to_string());
                        }
                    };
                }
            }
                .boxed_local()
                .compat(),
        );

        // wrap the channel sender in a mutex as it has to be passed to other threads.
        Ok(PSNServiceAddr {
            inner: Mutex::new(addr),
        })
    }

    // pattern match PSNRequest and handle the PSN network along with postgres and redis requests.
    async fn handle_queue(&mut self) -> Result<(), ResError> {
        let queue = self.check_token().await?.queue.lock().await.pop_front();

        if let Some(r) = queue {
            match r {
                PSNRequest::Profile { online_id } => self.handle_profile_request(online_id).await,
                PSNRequest::TrophyTitles { online_id, .. } => {
                    let r = self.handle_trophy_titles_request(online_id).await?;
                    self.update_user_trophy_titles(&r).await
                }
                PSNRequest::TrophySet {
                    online_id,
                    np_communication_id,
                } => {
                    let r = self
                        .handle_trophy_set_request(online_id, np_communication_id)
                        .await?;
                    self.query_update_user_trophy_set(r).await
                }
                PSNRequest::Auth {
                    uuid,
                    two_step,
                    refresh_token,
                } => {
                    self.handle_auth_request(uuid, two_step, refresh_token)
                        .await
                }
                PSNRequest::Activation {
                    user_id,
                    online_id,
                    code,
                } => {
                    self.handle_activation_request(user_id, online_id, code)
                        .await
                }
            }
        } else {
            Ok(())
        }
    }
}

/// pass channel sender to app.data so
pub struct PSNServiceAddr {
    inner: Mutex<UnboundedSender<(PSNRequest, bool)>>,
}

impl PSNServiceAddr {
    pub async fn do_send(&self, req: (PSNRequest, bool)) {
        let mut sender = self.inner.lock().await;
        let _ = sender.send(req).await;
    }
}

// QueueInjector take in a channel receiver and iter through the message received.
// push new PSNRequest to VecDeque according to the hash map of time_gate(to throw away spam requests by using time gate)
struct QueueInjector {
    queue: Arc<Mutex<VecDeque<PSNRequest>>>,
    receiver: UnboundedReceiver<(PSNRequest, bool)>,
    // stores all reqs' timestamp goes to PSN.
    // profile request use <online_id> as key,
    // trophy_list request use <online_id:::titles> as key,
    // trophy_set request use <online_id:::np_communication_id> as key
    // chrono::Utc::now().timestamp is score
    time_gate: hashbrown::HashMap<Vec<u8>, i64>,
}

impl QueueInjector {
    fn new(
        queue: Arc<Mutex<VecDeque<PSNRequest>>>,
        receiver: UnboundedReceiver<(PSNRequest, bool)>,
    ) -> Self {
        QueueInjector {
            queue,
            receiver,
            time_gate: hashbrown::HashMap::new(),
        }
    }

    fn handle_inject(mut self) {
        tokio::spawn(
            async move {
                loop {
                    let (req, is_front) = self
                        .receiver
                        .next()
                        .await
                        .ok_or(ResError::InternalServerError)?;
                    if self.should_add_queue(&req) {
                        self.update_time_stamp(&req);
                        self.add_to_queue(req, is_front).await;
                    }
                }
                // ToDo: add error handler
            }
                .map(|_r: Result<(), ResError>| ()),
        );
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

#[derive(Serialize, Deserialize, Clone)]
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

impl CheckCacheConn for PSNService {
    fn self_url(&self) -> String {
        self.cache_url.to_owned()
    }

    fn replace_cache(&self, c: SharedConnection) {
        self.cache.replace(c);
    }
}

impl GetSharedConn for PSNService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.borrow().clone()
    }
}

impl GetDbClient for PSNService {
    fn get_client(&self) -> RefMut<tokio_postgres::Client> {
        self.db.borrow_mut()
    }
}

impl Query for PSNService {}

impl SimpleQuery for PSNService {}

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
            // ToDo: handle potential attacker here as the np_id of user doesn't match.
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
            // ToDo: handle potential attacker here as the np_id of user doesn't match.
            Err(ResError::Unauthorized)
        }
    }

    async fn handle_profile_request(&mut self, online_id: String) -> Result<(), ResError> {
        let u: PSNUserLib = self.psn.add_online_id(online_id).get_profile().await?;

        self.update_profile_cache(u.into());

        Ok(())
    }

    fn update_profile_cache(&self, p: UserPSNProfile) {
        actix::spawn(
            crate::handler::cache::build_hmsets_01(
                self.get_conn(),
                &[p],
                crate::handler::cache::USER_PSN_U8,
                false,
            )
            .map_err(|_| ()),
        );
    }

    // a costly update for updating existing trophy set.
    // The purpose is to flag people who have a changed trophy timestamp on the trophy already earned
    // by comparing the The first_earned_date with the earned_date
    async fn query_update_user_trophy_set(&self, mut t: UserTrophySet) -> Result<(), ResError> {
        let st = self
            .get_client()
            .prepare("SELECT * FROM psn_user_trophy_sets WHERE np_id=$1 and np_communication_id=$2")
            .await?;

        let r: Result<UserTrophySet, ResError> = self
            .query_one(&st, &[&t.np_id.as_str(), &t.np_communication_id.as_str()])
            .await;

        //        let query = format!(
        //            "SELECT * FROM psn_user_trophy_sets WHERE np_id='{}' and np_communication_id='{}';",
        //            t.np_id.as_str(),
        //            t.np_communication_id.as_str()
        //        );
        //        let r: Result<UserTrophySet, ResError> = self.simple_query_one(query.as_str()).await;

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

    async fn update_user_trophy_set(&self, t: &UserTrophySet) -> Result<(), ResError> {
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

        self.simple_query_row(query.as_str()).map_ok(|_| ()).await
    }

    fn update_user_trophy_titles(
        &mut self,
        t: &[UserTrophyTitle],
    ) -> impl Future<Output = Result<(), ResError>> {
        let mut v = Vec::with_capacity(t.len());

        for t in t.iter() {
            let f = self.get_client().execute(
                &self.insert_trophy_title.borrow(),
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

        futures::future::join_all(v).map(|r| {
            for r in r {
                let _ = r?;
            }
            Ok(())
        })
    }

    async fn check_token(&mut self) -> Result<&mut Self, ResError> {
        if self.psn.should_refresh() {
            let p: PSN = PSN::new()
                .add_refresh_token(
                    self.psn
                        .get_refresh_token()
                        .map(String::from)
                        .unwrap_or_else(|| "".to_owned()),
                )
                .auth()
                .await?;
            self.psn = p;
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
        let query = format!(
            "SELECT * FROM psn_user_trophy_titles WHERE np_id=$1 ORDER BY last_update_date DESC OFFSET {} LIMIT 20",
            (page - 1) * 20
        );

        let st = self.get_client().prepare(query.as_str()).await?;

        self.query_multi(&st, &[&np_id], Vec::with_capacity(20))
            .await
    }

    pub(crate) async fn get_trophy_set(
        &self,
        np_id: &str,
        np_communication_id: &str,
    ) -> Result<UserTrophySet, ResError> {
        let st = self
            .get_client()
            .prepare("SELECT * FROM psn_user_trophy_sets WHERE np_id=$1 and np_communication_id=$2")
            .await?;

        self.query_one(&st, &[&np_id, &np_communication_id]).await
        //        let query = format!(
        //            "SELECT * FROM psn_user_trophy_sets WHERE np_id='{}' and np_communication_id='{}'",
        //            np_id, np_communication_id
        //        );
        //        self.simple_query_one::<UserTrophySet>(query.as_str())
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
    ) -> impl Future<Output = Result<UserPSNProfile, ResError>> {
        use crate::handler::cache::FromCacheSingle;
        self.from_cache_single_01(online_id, "user_psn").compat()
    }
}
