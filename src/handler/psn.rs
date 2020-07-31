use std::{collections::VecDeque, convert::TryInto, fmt::Write, time::Duration};

use chrono::Utc;
use futures::TryFutureExt;
use psn_api_rs::types::PSNInner;
use psn_api_rs::{psn::PSN, traits::PSNRequest as PSNRequestLib};
use tokio_postgres::types::ToSql;

use crate::handler::{
    cache::MyRedisPool,
    db::{MyPostgresPool, ParseRowStream},
    messenger::{ErrReportMsg, ErrReportServiceAddr},
};
use crate::model::{
    common::{dur, dur_as_sec},
    errors::ResError,
    psn::{
        PSNTrophyArgumentRequest, PSNUserLib, TrophySetLib, TrophyTitlesLib, UserPSNProfile,
        UserTrophySet, UserTrophyTitle,
    },
};

use actix_send::prelude::*;

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

#[actor]
pub struct PSNService {
    db_pool: MyPostgresPool,
    cache_pool: MyRedisPool,
    psn: Option<PSN>,
    queue: VecDeque<PSNRequest>,
    // stores all reqs' timestamp goes to PSN.
    // profile request use <online_id> as key,
    // trophy_list request use <online_id:::titles> as key,
    // trophy_set request use <online_id:::np_communication_id> as key
    // chrono::Utc::now().timestamp is score
    time_gate: hashbrown::HashMap<Vec<u8>, i64>,
    rep_addr: Option<ErrReportServiceAddr>,
}

pub(crate) type PSNServiceAddr = Address<PSNService>;

pub struct PSNServiceMsg {
    req: PSNRequest,
    is_front: bool,
}

#[handler_v2]
impl PSNService {
    async fn handle_msg(&mut self, msg: PSNServiceMsg) {
        let req = msg.req;
        let is_front = msg.is_front;

        if self.should_add_queue(&req) {
            self.update_time_stamp(&req);
            self.add_to_queue(req, is_front);
        };
    }
}

pub(crate) async fn init_psn_service(
    db_pool: MyPostgresPool,
    cache_pool: MyRedisPool,
    rep_addr: Option<ErrReportServiceAddr>,
) -> Address<PSNService> {
    let builder = PSNService::builder(move || {
        let rep_addr = rep_addr.clone();
        let db_pool = db_pool.clone();
        let cache_pool = cache_pool.clone();
        async {
            PSNService {
                db_pool,
                cache_pool,
                psn: None,
                queue: Default::default(),
                time_gate: Default::default(),
                rep_addr,
            }
        }
    });

    let addr: Address<PSNService> = builder.start().await;

    // Process queue in an interval
    addr.run_interval(PSN_REQ_INTERVAL, |service| {
        Box::pin(service.interval_req_task())
    })
    .await
    .expect("Failed to start PSNService Interval task");

    addr
}

impl PSNService {
    async fn interval_req_task(&mut self) {
        if let Some(msg) = self.queue.pop_front() {
            if let Err(e) = self.handle_request(msg).await {
                if let Some(addr) = self.rep_addr.as_ref() {
                    let _ = addr.send(ErrReportMsg(e)).await;
                }
            }
        };
    }
}

impl PSNService {
    fn psn(&self) -> &PSN {
        self.psn.as_ref().unwrap()
    }

    async fn handle_request(&mut self, req: PSNRequest) -> Result<(), ResError> {
        match req {
            PSNRequest::Profile { online_id } => self.handle_profile_request(&online_id).await,
            PSNRequest::TrophyTitles { online_id, .. } => {
                let r = self.handle_trophy_titles_request(&online_id).await?;
                // only check db connection when update user trophy titles.
                self.db_pool.update_user_trophy_titles(&r).await
            }
            PSNRequest::TrophySet {
                online_id,
                np_communication_id,
            } => {
                let r = self
                    .handle_trophy_set_request(&online_id, &np_communication_id)
                    .await?;
                self.db_pool.query_update_user_trophy_set(r).await
            }
            PSNRequest::Auth {
                npsso,
                refresh_token,
            } => self.handle_auth_request(npsso, refresh_token).await,
            PSNRequest::Activation {
                user_id,
                online_id,
                code,
            } => {
                self.handle_activation_request(user_id, &online_id, code)
                    .await
            }
        }
    }

    // handle_xxx_request are mostly the network request to PSN.
    // update_xxx are mostly postgres and redis write operation.
    async fn handle_auth_request(
        &mut self,
        npsso: Option<String>,
        refresh_token: Option<String>,
    ) -> Result<(), ResError> {
        let mut psn = PSNInner::new();
        let client = PSN::new_client()?;

        if let Some(npsso) = npsso {
            psn.add_npsso(npsso);
        };

        if let Some(refresh_token) = refresh_token {
            psn.add_refresh_token(refresh_token);
        }

        let inner = psn.auth(client).await?;
        let psn = PSN::new(vec![inner]).await;
        self.psn = Some(psn);

        Ok(())
    }

    async fn handle_activation_request(
        &self,
        user_id: Option<u32>,
        online_id: &str,
        code: String,
    ) -> Result<(), ResError> {
        let u: PSNUserLib = self.psn().get_profile(online_id).await?;

        if u.about_me == code {
            let mut u = UserPSNProfile::from(u);
            u.id = user_id;
            self.cache_pool
                .build_sets(&[u], crate::handler::cache::USER_PSN_U8, false)
                .await
        } else {
            // ToDo: add more error detail and send it through message to user.
            Err(ResError::Unauthorized)
        }
    }

    async fn handle_trophy_titles_request(
        &self,
        online_id: &str,
    ) -> Result<Vec<UserTrophyTitle>, ResError> {
        // get profile before and after getting titles and check if the user's np_id remains unchanged.
        let u = self.psn().get_profile::<PSNUserLib>(online_id).await?;
        let titles_first = self
            .psn()
            .get_titles::<TrophyTitlesLib>(online_id, 0)
            .await?;

        let total = titles_first.total_results;
        let page = total / 100;
        let mut f = Vec::with_capacity(page as usize);
        for i in 0..page {
            f.push(
                self.psn()
                    .get_titles::<TrophyTitlesLib>(online_id, (i + 1) * 100)
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

        let uu: PSNUserLib = self.psn().get_profile(online_id).await?;

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
        &self,
        online_id: &str,
        np_communication_id: &str,
    ) -> Result<UserTrophySet, ResError> {
        let u = self.psn().get_profile::<PSNUserLib>(online_id).await?;

        let set = self
            .psn()
            .get_trophy_set::<TrophySetLib>(online_id, np_communication_id)
            .await?;

        let uu = self.psn().get_profile::<PSNUserLib>(online_id).await?;

        if u.np_id == uu.np_id && u.online_id == uu.online_id {
            Ok(UserTrophySet {
                np_id: uu.np_id,
                np_communication_id: np_communication_id.into(),
                is_visible: true,
                trophies: set.trophies.iter().map(|t| t.into()).collect(),
            })
        } else {
            Err(ResError::Unauthorized)
        }
    }

    async fn handle_profile_request(&mut self, online_id: &str) -> Result<(), ResError> {
        let u: UserPSNProfile = self
            .psn()
            .get_profile::<PSNUserLib>(online_id)
            .await?
            .into();

        self.cache_pool
            .build_sets(&[u], crate::handler::cache::USER_PSN_U8, false)
            .await
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
        npsso: Option<String>,
        refresh_token: Option<String>,
    },
    Activation {
        user_id: Option<u32>,
        online_id: String,
        code: String,
    },
}

impl PSNRequest {
    pub(crate) fn into_msg(self, is_front: bool) -> PSNServiceMsg {
        PSNServiceMsg {
            req: self,
            is_front,
        }
    }

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

impl MyPostgresPool {
    // a costly update for updating existing trophy set.
    // The purpose is to flag people who have a changed trophy timestamp on the trophy already earned
    // by comparing the The first_earned_date with the earned_date
    async fn query_update_user_trophy_set(&self, mut t: UserTrophySet) -> Result<(), ResError> {
        let pool = self.get().await?;
        let (cli, _) = &*pool;

        let st = cli.prepare(PSN_SET_BY_NPID).await?;
        let params: [&(dyn ToSql + Sync); 2] = [&t.np_id.as_str(), &t.np_communication_id.as_str()];
        let r = cli
            .query_raw(&st, params.iter().map(|s| *s as _))
            .await?
            .parse_row::<UserTrophySet>()
            .await;

        drop(pool);

        match r {
            Ok(mut t_old) => {
                let t_old = t_old.pop().ok_or(ResError::PostgresError)?;
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
                ResError::PostgresError => return Err(e),
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

        let pool = self.get().await?;
        let (cli, _) = &*pool;

        cli.simple_query(query.as_str())
            .map_ok(|_| ())
            .err_into()
            .await
    }

    async fn update_user_trophy_titles(&self, t: &[UserTrophyTitle]) -> Result<(), ResError> {
        let pool = self.get().await?;
        let (cli, _) = &*pool;

        let st = cli.prepare(INSERT_TITLES).await?;
        for t in t.iter() {
            let _f = cli
                .execute(
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
                )
                .await;
        }
        Ok(())
    }

    pub(crate) async fn get_trophy_titles(
        &self,
        np_id: &str,
        page: u32,
    ) -> Result<Vec<UserTrophyTitle>, ResError> {
        let pool = self.get().await?;
        let (cli, _) = &*pool;

        let offset = (page - 1) * 20;
        let st = cli.prepare(PSN_TITLES_NY_TIME).await?;
        let params: [&(dyn ToSql + Sync); 2] = [&np_id, &offset];

        cli.query_raw(&st, params.iter().map(|s| *s as _))
            .await?
            .parse_row()
            .await
    }

    pub(crate) async fn get_trophy_set(
        &self,
        np_id: &str,
        np_communication_id: &str,
    ) -> Result<Vec<UserTrophySet>, ResError> {
        let pool = self.get().await?;
        let (cli, _) = &*pool;

        let st = cli.prepare(PSN_TITLES_NY_TIME).await?;
        let params: [&(dyn ToSql + Sync); 2] = [&np_id, &np_communication_id];

        cli.query_raw(&st, params.iter().map(|s| *s as _))
            .await?
            .parse_row()
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

impl MyRedisPool {
    pub(crate) async fn get_psn_profile(
        &self,
        online_id: &str,
    ) -> Result<UserPSNProfile, ResError> {
        self.get_cache_single(online_id, "user_psn").await
    }
}
