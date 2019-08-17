use std::convert::TryInto;
use std::fmt::Write;
use std::str::FromStr;
use std::time::Duration;

use actix::{fut::Either, ActorFuture, AsyncContext, Context, WrapFuture};
use futures::Future;
use psn_api_rs::{PSNRequest, PSN};

use crate::handler::cache::{CacheService, GetQueue, GetSharedConn};
use crate::handler::db::{DatabaseService, SimpleQuery};
use crate::model::{
    actors::PSNService,
    errors::ResError,
    psn::{
        PSNActivationRequest, PSNAuthRequest, PSNProfileRequest, PSNTrophyRequest, PSNUserLib,
        TrophySetLib, TrophyTitleLib, TrophyTitlesLib, UserPSNProfile, UserTrophySet,
        UserTrophyTitle,
    },
};

const PSN_TIME_GAP: Duration = Duration::from_millis(1000);

impl PSNService {
    pub fn start_interval(&self, ctx: &mut Context<Self>) {
        self.process_psn_request(ctx);
    }

    fn process_psn_request(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(PSN_TIME_GAP, move |act, ctx| {
            if act.is_active {
                ctx.spawn(
                    act.get_queue("psn_queue")
                        .into_actor(act)
                        .map_err(|_, _, _| ())
                        .and_then(|q: String, act, _| {
                            // what a disaster.
                            if let Some(req) =
                                serde_json::from_str::<PSNProfileRequest>(q.as_str()).ok()
                            {
                                return Either::A(Either::A(act.handle_profile_request(req)));
                            };
                            if let Some(req) =
                                serde_json::from_str::<PSNTrophyRequest>(q.as_str()).ok()
                            {
                                return Either::A(Either::B(match req.np_communication_id {
                                    Some(np_cid) => Either::A(
                                        act.handle_trophy_titles_request(req.online_id)
                                            .map(|_, _, _| ()),
                                    ),
                                    None => Either::B(
                                        act.handle_trophy_titles_request(req.online_id).and_then(
                                            |r: Vec<UserTrophyTitle>, act: &mut PSNService, _| {
                                                act.add_user_trophy_titles(&r)
                                                    .map_err(|_| ())
                                                    .into_actor(act)
                                            },
                                        ),
                                    ),
                                }));
                            }
                            if let Some(req) = PSNActivationRequest::from_str(q.as_str()).ok() {
                                return Either::B(Either::A(
                                    act.psn
                                        .add_online_id(req.online_id)
                                        .get_profile()
                                        .into_actor(act)
                                        .map_err(|_, _, _| ())
                                        .and_then(|u: PSNUserLib, act, _| {
                                            act.update_profile_cache(u.into());
                                            actix::fut::ok(())
                                        }),
                                ));
                            }
                            let req = serde_json::from_str::<PSNAuthRequest>(q.as_str())
                                .unwrap_or_else(|_| PSNAuthRequest::default());
                            Either::B(Either::B(act.handle_auth_request(req)))
                        }),
                );
            };
        });
    }

    fn handle_auth_request(
        &mut self,
        req: PSNAuthRequest,
    ) -> impl ActorFuture<Item = (), Actor = Self, Error = ()> {
        PSN::new()
            .add_uuid(req.uuid)
            .add_two_step(req.two_step)
            .auth()
            .map_err(|_| ())
            .into_actor(self)
            .map(|p, act, _| {
                act.psn = p;
                act.is_active = true;
            })
    }

    fn handle_trophy_titles_request(
        &mut self,
        online_id: String,
    ) -> impl ActorFuture<Item = Vec<UserTrophyTitle>, Actor = Self, Error = ()> {
        // get profile before and after getting titles and check if the user's np_id remains unchanged.
        self.psn
            .add_online_id(online_id)
            .get_profile()
            .map_err(|_| ())
            .into_actor(self)
            .and_then(|u: PSNUserLib, act, _| {
                act.psn
                    .get_titles(0)
                    .map_err(|_| ())
                    .into_actor(act)
                    .and_then(|r: TrophyTitlesLib, act, _| {
                        let total = r.total_results;
                        let page = total / 100;
                        let mut f = Vec::with_capacity(page as usize);
                        for i in 0..page {
                            f.push(
                                act.psn
                                    .get_titles::<TrophyTitlesLib>((i + 1) * 100)
                                    .then(|r| match r {
                                        Ok(r) => Ok(Some(r)),
                                        Err(_) => Ok(None),
                                    }),
                            )
                        }
                        futures::future::join_all(f)
                            .map_err(|e: psn_api_rs::PSNError| ())
                            .into_actor(act)
                    })
                    .and_then(move |titles: Vec<Option<TrophyTitlesLib>>, act, _| {
                        let mut v: Vec<UserTrophyTitle> = Vec::with_capacity(titles.len());
                        for titles in titles.into_iter() {
                            if let Some(titles) = titles {
                                for title in titles.trophy_titles.into_iter() {
                                    if let Some(title) = title.try_into().ok() {
                                        v.push(title)
                                    }
                                }
                            }
                        }
                        act.psn
                            .get_profile()
                            .map_err(|_| ())
                            .into_actor(act)
                            .and_then(move |uu: PSNUserLib, act, _| {
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
                                    actix::fut::err(())
                                }
                            })
                    })
            })
    }

    fn handle_trophy_set_request(
        &mut self,
        online_id: String,
        np_communication_id: String,
    ) -> impl ActorFuture<Item = UserTrophySet, Actor = Self, Error = ()> {
        self.psn
            .add_online_id(online_id)
            .get_profile()
            .map_err(|_| ())
            .into_actor(self)
            .and_then(|u: PSNUserLib, act, _| {
                act.psn
                    .add_np_communication_id(np_communication_id)
                    .get_trophy_set::<TrophySetLib>()
                    .map_err(|_| ())
                    .into_actor(act)
                    .and_then(|set, act, _| {
                        act.psn
                            .get_profile()
                            .map_err(|_| ())
                            .into_actor(act)
                            .and_then(move |uu: PSNUserLib, act, _| {
                                if u.np_id == uu.np_id && u.online_id == uu.online_id {
                                    actix::fut::ok(UserTrophySet {
                                        id: 0,
                                        online_id: u.online_id,
                                        np_id: uu.np_id,
                                        titles: set.trophies.iter().map(|t| t.into()).collect(),
                                    })
                                } else {
                                    // ToDo: handle potential attacker here as the np_id of user doesn't match.
                                    actix::fut::err(())
                                }
                            })
                    })
            })
    }

    fn handle_profile_request(
        &mut self,
        req: PSNProfileRequest,
    ) -> impl ActorFuture<Item = (), Actor = Self, Error = ()> {
        self.psn
            .add_online_id(req.online_id)
            .get_profile()
            .into_actor(self)
            .map_err(|_, _, _| ())
            .and_then(|u: PSNUserLib, act, _| {
                act.update_profile_cache(u.into());
                actix::fut::ok(())
            })
    }

    fn update_profile_cache(&self, p: UserPSNProfile) {
        actix_rt::spawn(crate::handler::cache::build_hmsets(
            self.get_conn(),
            vec![p],
            "user_psn",
            false,
        ));
    }

    fn add_user_trophy_titles(
        &self,
        t: &[UserTrophyTitle],
    ) -> impl Future<Item = (), Error = ResError> {
        let mut query = String::new();

        for t in t.iter() {
            let _ = write!(
                &mut query,
                "INSERT INTO psn_user_trophy_titles
                    (np_id, np_communication_id, progress, earned_platinum, earned_gold, earned_silver, earned_bronze, last_update_date)
                    VALUES ({}, {}, {}, {}, {}, {}, {}, {});
                    ON CONFLICT (np_id, np_communication_id)
                        DO UPDATE SET
                            progress = EXCLUDED.progress,
                            earned_trophies = EXCLUDED.earned_trophies,
                            last_update_date = EXCLUDED.last_update_date;
                ",
                t.np_id.as_str(),
                t.np_communication_id.as_str(),
                t.progress,
                t.earned_platinum,
                t.earned_gold,
                t.earned_silver,
                t.earned_bronze,
                t.last_update_date
            );
        }

        self.simple_query_row_trait(query.as_str()).map(|_| ())
    }
}

impl DatabaseService {
    // trophy is not frequent query. use simple query for less prepared statement.
    pub fn get_trophy_titles(
        &self,
        np_id: &str,
        page: u32,
    ) -> impl Future<Item = Vec<UserTrophyTitle>, Error = ResError> {
        let query = format!(
            "SELECT * FROM psn_user_trophy_titles WHERE np_id = {} ORDER BY last_update_date DESC OFFSET= {} LIMIT = 20",
            np_id,
            page
        );

        self.simple_query_multi_trait::<UserTrophyTitle>(query.as_str(), Vec::with_capacity(20))
    }

    pub fn get_trophy_set(
        &self,
        req: &PSNTrophyRequest,
    ) -> impl Future<Item = Vec<UserTrophyTitle>, Error = ResError> {
        let page = req.page.as_ref().unwrap_or(&1);
        let online_id = req.online_id.as_str();

        let query = format!(
            "SELECT * FROM psn_user_trophy_titles WHERE np_id = {} ORDER BY last_update_date DESC LIMIT = 20",
            1
        );

        self.simple_query_multi_trait::<UserTrophyTitle>(query.as_str(), Vec::with_capacity(20))
    }
}

impl CacheService {
    pub fn get_psn_profile(
        &self,
        online_id: &[u8],
    ) -> impl Future<Item = UserPSNProfile, Error = ResError> {
        use crate::handler::cache::FromCacheSingle;
        self.from_cache_single(online_id, "user_psn")
    }
}
