use std::cell::{RefCell, RefMut};
use std::collections::VecDeque;
use std::convert::TryInto;
use std::fmt::Write;
use std::future::Future;
use std::time::Duration;

use actix::{
    fut::Either as ActorEither, Actor, ActorFuture, Addr, AsyncContext, Context, Handler, Message,
    WrapFuture,
};
use chrono::Utc;
use futures::{compat::Future01CompatExt, FutureExt, TryFutureExt};
use futures01::{
    future::{err as ft_err, Either},
    Future as Future01,
};
use psn_api_rs::{PSNRequest as PSNRequestLib, PSN};
use redis::aio::SharedConnection;
use tokio_postgres::Statement;

use crate::handler::{
    cache::{CacheService, CheckCacheConn, GetSharedConn},
    db::{DatabaseService, GetDbClient, Query, SimpleQuery},
};
use crate::model::psn::PSNTrophyArgumentRequest;
use crate::model::{
    errors::ResError,
    psn::{
        PSNUserLib, TrophySetLib, TrophyTitlesLib, UserPSNProfile, UserTrophySet, UserTrophyTitle,
    },
};

const PSN_TIME_GAP: Duration = Duration::from_millis(3000);

// how often user can sync their data to psn in seconds.
const PROFILE_TIME_GATE: i64 = Duration::from_secs(900).as_secs() as i64;
const TROPHY_TITLES_TIME_GATE: i64 = Duration::from_secs(900).as_secs() as i64;
const TROPHY_SET_TIME_GATE: i64 = Duration::from_secs(900).as_secs() as i64;

const INSERT_TITLES: &str =
    "INSERT INTO psn_user_trophy_titles
(np_id, np_communication_id, progress, earned_platinum, earned_gold, earned_silver, earned_bronze, last_update_date)
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

pub type PSNServiceAddr = Addr<PSNService>;

pub struct PSNService {
    pub db_url: String,
    pub cache_url: String,
    pub psn: PSN,
    pub db: RefCell<tokio_postgres::Client>,
    pub insert_trophy_title: RefCell<Statement>,
    pub cache: RefCell<SharedConnection>,
    pub queue: VecDeque<PSNRequest>,
    // stores all reqs' timestamp goes to PSN.
    // profile request use <online_id> as key,
    // trophy_list request use <online_id:::titles> as key,
    // trophy_set request use <online_id:::np_communication_id> as key
    // chrono::Utc::now().timestamp is score
    pub req_time_stamp: hashbrown::HashMap<Vec<u8>, i64>,
}

impl Actor for PSNService {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.start_interval(ctx);
    }
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

        Ok(PSNService::create(move |_| PSNService {
            db_url,
            cache_url,
            psn: PSN::new(),
            db: RefCell::new(db),
            insert_trophy_title: RefCell::new(p1),
            cache: RefCell::new(cache),
            queue: VecDeque::new(),
            req_time_stamp: hashbrown::HashMap::new(),
        }))
    }

    fn add_to_queue(&mut self, req: PSNRequest, is_front: bool) {
        if is_front {
            self.queue.push_front(req);
        } else {
            self.queue.push_back(req);
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
        if let Some(timestamp) = self.req_time_stamp.get(entry) {
            if (Utc::now().timestamp() - *timestamp) < time_gate {
                return true;
            }
        }
        false
    }

    fn update_time_stamp(&mut self, req: PSNRequest) {
        let key = req.generate_entry_key();
        let time = Utc::now().timestamp();
        self.req_time_stamp.insert(key, time);
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

pub struct AddPSNRequest(pub PSNRequest, pub bool);

impl Message for AddPSNRequest {
    type Result = ();
}

impl Handler<AddPSNRequest> for PSNService {
    type Result = ();

    fn handle(&mut self, AddPSNRequest(req, is_front): AddPSNRequest, _: &mut Context<Self>) {
        if self.should_add_queue(&req) {
            self.add_to_queue(req, is_front);
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
    pub fn start_interval(&self, ctx: &mut Context<Self>) {
        self.process_psn_request(ctx);
    }

    fn process_psn_request(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(PSN_TIME_GAP, move |act, ctx| {
            ctx.spawn(
                act.check_token()
                    .and_then(move |_, act: &mut PSNService, _| {
                        if let Some(r) = act.queue.pop_front() {
                            let req = r.clone();
                            ActorEither::A(match r {
                                PSNRequest::Profile { online_id } => {
                                    ActorEither::A(ActorEither::A(ActorEither::A(
                                        act.handle_profile_request(online_id)
                                            .map(move |_, _, _| req),
                                    )))
                                }
                                PSNRequest::TrophyTitles { online_id, .. } => {
                                    ActorEither::A(ActorEither::A(ActorEither::B(
                                        act.handle_trophy_titles_request(online_id).and_then(
                                            move |r: Vec<UserTrophyTitle>,
                                                  act: &mut PSNService,
                                                  _| {
                                                act.update_user_trophy_titles(&r)
                                                    .into_actor(act)
                                                    .map(move |_, _, _| req)
                                            },
                                        ),
                                    )))
                                }
                                PSNRequest::TrophySet {
                                    online_id,
                                    np_communication_id,
                                } => ActorEither::A(ActorEither::B(ActorEither::A(
                                    act.handle_trophy_set_request(online_id, np_communication_id)
                                        .and_then(
                                            move |r: UserTrophySet, act: &mut PSNService, _| {
                                                act.query_update_user_trophy_set(r)
                                                    .map(move |_, _, _| req)
                                            },
                                        ),
                                ))),
                                PSNRequest::Auth {
                                    uuid,
                                    two_step,
                                    refresh_token,
                                } => ActorEither::A(ActorEither::B(ActorEither::B(
                                    act.handle_auth_request(uuid, two_step, refresh_token)
                                        .map(|_, _, _| req),
                                ))),
                                PSNRequest::Activation {
                                    user_id,
                                    online_id,
                                    code,
                                } => {
                                    ActorEither::B(act.get_profile_crate(Some(online_id)).and_then(
                                        move |u: PSNUserLib, act, _| {
                                            if u.about_me == code {
                                                let mut u = UserPSNProfile::from(u);
                                                u.id = user_id;
                                                act.update_profile_cache(u);
                                                actix::fut::ok(req)
                                            } else {
                                                // ToDo: add more error detail and send it through message to user.
                                                actix::fut::err(ResError::Unauthorized)
                                            }
                                        },
                                    ))
                                }
                            })
                        } else {
                            ActorEither::B(actix::fut::err(ResError::NoContent))
                        }
                    })
                    .map(|r: PSNRequest, act, _| act.update_time_stamp(r))
                    .map_err(|e: ResError, _, _| match e {
                        ResError::NoContent => (),
                        _ => println!("{:?}", e.to_string()),
                    }),
            );
        });
    }

    fn get_profile_crate(
        &mut self,
        online_id: Option<String>,
    ) -> impl ActorFuture<Item = PSNUserLib, Actor = Self, Error = ResError> {
        let psn = match online_id {
            Some(online_id) => self.psn.add_online_id(online_id),
            None => &mut self.psn,
        };

        psn.get_profile().from_err().into_actor(self)
    }

    fn handle_auth_request(
        &mut self,
        uuid: Option<String>,
        two_step: Option<String>,
        refresh_token: Option<String>,
    ) -> impl ActorFuture<Item = (), Actor = Self, Error = ResError> {
        let mut psn = PSN::new();

        if let Some(uuid) = uuid {
            if let Some(two_step) = two_step {
                psn = psn.add_uuid(uuid).add_two_step(two_step);
            }
        };

        if let Some(refresh_token) = refresh_token {
            psn = psn.add_refresh_token(refresh_token);
        }

        psn.auth().from_err().into_actor(self).map(|p, act, _| {
            println!("{:#?}", p);
            act.psn = p;
        })
    }

    fn handle_trophy_titles_request(
        &mut self,
        online_id: String,
    ) -> impl ActorFuture<Item = Vec<UserTrophyTitle>, Actor = Self, Error = ResError> {
        // get profile before and after getting titles and check if the user's np_id remains unchanged.
        self.get_profile_crate(Some(online_id)).and_then(
            |u: PSNUserLib, act: &mut PSNService, _| {
                act.psn.get_titles(0).from_err().into_actor(act).and_then(
                    |titles_first: TrophyTitlesLib, act, _| {
                        let total = titles_first.total_results;
                        let page = total / 100;
                        let mut f = Vec::with_capacity(page as usize);
                        for i in 0..page {
                            f.push(
                                act.psn
                                    .get_titles::<TrophyTitlesLib>((i + 1) * 100)
                                    .then(|r| match r {
                                        Ok(r) => Ok(Some(r)),
                                        Err(e) => Err(e),
                                    }),
                            )
                        }
                        futures01::future::join_all(f)
                            .from_err()
                            .into_actor(act)
                            .and_then(move |titles: Vec<Option<TrophyTitlesLib>>, act, _| {
                                let mut v: Vec<UserTrophyTitle> = Vec::new();

                                for title in titles_first.trophy_titles.into_iter() {
                                    if let Ok(title) = title.try_into() {
                                        v.push(title)
                                    }
                                }

                                for titles in titles.into_iter() {
                                    if let Some(titles) = titles {
                                        for title in titles.trophy_titles.into_iter() {
                                            if let Ok(title) = title.try_into() {
                                                v.push(title)
                                            }
                                        }
                                    }
                                }
                                act.get_profile_crate(None)
                                    .and_then(move |uu: PSNUserLib, _, _| {
                                        if u.np_id.as_str() == uu.np_id.as_str()
                                            && u.online_id.as_str() == uu.online_id.as_str()
                                        {
                                            let np_id = uu.np_id;

                                            let v = v
                                                .into_iter()
                                                .map(|mut t| {
                                                    t.np_id = np_id.clone();
                                                    t
                                                })
                                                .collect();

                                            actix::fut::ok(v)
                                        } else {
                                            // ToDo: handle potential attacker here as the np_id of user doesn't match.
                                            actix::fut::err(ResError::Unauthorized)
                                        }
                                    })
                            })
                    },
                )
            },
        )
    }

    fn handle_trophy_set_request(
        &mut self,
        online_id: String,
        np_communication_id: String,
    ) -> impl ActorFuture<Item = UserTrophySet, Actor = Self, Error = ResError> {
        self.get_profile_crate(Some(online_id)).and_then(
            |u: PSNUserLib, act: &mut PSNService, _| {
                act.psn
                    .add_np_communication_id(np_communication_id.clone())
                    .get_trophy_set::<TrophySetLib>()
                    .from_err()
                    .into_actor(act)
                    .and_then(|set, act, _| {
                        act.get_profile_crate(None)
                            .and_then(move |uu: PSNUserLib, _, _| {
                                if u.np_id == uu.np_id && u.online_id == uu.online_id {
                                    actix::fut::ok(UserTrophySet {
                                        np_id: uu.np_id,
                                        np_communication_id,
                                        is_visible: true,
                                        trophies: set.trophies.iter().map(|t| t.into()).collect(),
                                    })
                                } else {
                                    // ToDo: handle potential attacker here as the np_id of user doesn't match.
                                    actix::fut::err(ResError::Unauthorized)
                                }
                            })
                    })
            },
        )
    }

    fn handle_profile_request(
        &mut self,
        online_id: String,
    ) -> impl ActorFuture<Item = (), Actor = Self, Error = ResError> {
        self.get_profile_crate(Some(online_id)).and_then(
            |u: PSNUserLib, act: &mut PSNService, _| {
                act.update_profile_cache(u.into());
                actix::fut::ok(())
            },
        )
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
    fn query_update_user_trophy_set(
        &self,
        mut t: UserTrophySet,
    ) -> impl ActorFuture<Item = (), Actor = Self, Error = ResError> {
        let query = format!(
            "SELECT * FROM psn_user_trophy_sets WHERE np_id='{}' and np_communication_id='{}';",
            t.np_id.as_str(),
            t.np_communication_id.as_str()
        );
        self.simple_query_one_trait(query.as_str())
            .boxed_local()
            .compat()
            .into_actor(self)
            .then(move |r: Result<UserTrophySet, ResError>, act, _| {
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
                        ResError::DataBaseReadError => return ActorEither::A(actix::fut::err(e)),
                        ResError::ParseError => return ActorEither::A(actix::fut::err(e)),
                        _ => {}
                    },
                };
                ActorEither::B(act.update_user_trophy_set(&t).into_actor(act))
            })
    }

    fn update_user_trophy_set(
        &self,
        t: &UserTrophySet,
    ) -> impl Future01<Item = (), Error = ResError> {
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
            return Either::A(ft_err(ResError::BadRequest));
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

        Either::B(
            self.simple_query_row_trait(query.as_str())
                .map_ok(|_| ())
                .boxed_local()
                .compat(),
        )
    }

    fn update_user_trophy_titles(
        &mut self,
        t: &[UserTrophyTitle],
    ) -> impl Future01<Item = (), Error = ResError> {
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

        futures::future::join_all(v)
            .map(|r| {
                for r in r {
                    let _ = r?;
                }
                Ok(())
            })
            .boxed()
            .compat()
    }

    fn check_token(&self) -> impl ActorFuture<Item = (), Actor = Self, Error = ResError> {
        if self.psn.should_refresh() {
            ActorEither::A(
                PSN::new()
                    .add_refresh_token(
                        self.psn
                            .get_refresh_token()
                            .map(String::from)
                            .unwrap_or_else(|| "".to_owned()),
                    )
                    .auth()
                    .from_err()
                    .into_actor(self)
                    .map(|p, act, _| {
                        act.psn = p;
                    }),
            )
        } else {
            ActorEither::B(actix::fut::ok(()))
        }
    }
}

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

        self.query_multi_trait(&st, &[&np_id], Vec::with_capacity(20))
            .await
    }

    pub(crate) fn get_trophy_set(
        &self,
        np_id: &str,
        np_communication_id: &str,
    ) -> impl Future<Output = Result<UserTrophySet, ResError>> {
        let query = format!(
            "SELECT * FROM psn_user_trophy_sets WHERE np_id='{}' and np_communication_id='{}'",
            np_id, np_communication_id
        );

        self.simple_query_one_trait::<UserTrophySet>(query.as_str())
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
