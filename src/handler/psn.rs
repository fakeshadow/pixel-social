use std::fmt::Write;
use std::str::FromStr;
use std::time::Duration;

use actix::{
    ActorFuture,
    AsyncContext,
    Context,
    fut::Either,
    WrapFuture,
};
use futures::Future;
use psn_api_rs::{PSN, PSNRequest};

use crate::handler::cache::{CacheService, GetQueue, GetSharedConn};
use crate::handler::db::{DatabaseService, SimpleQuery};
use crate::model::{
    actors::PSNService,
    errors::ResError,
    psn::{PSNTrophyRequest, UserPSNProfile, UserTrophyTitle},
};
use crate::model::psn::{PSNActivationRequest, PSNAuthRequest, PSNProfileRequest, PSNUserLib, TrophyTitleLib};

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
                            if let Some(req) = serde_json::from_str::<PSNProfileRequest>(q.as_str()).ok() {
                                return Either::A(
                                    Either::A(
                                        act.handle_profile_request(req)
                                    )
                                );
                            };
                            if let Some(req) = serde_json::from_str::<PSNTrophyRequest>(q.as_str()).ok() {
                                return Either::A(
                                    Either::B(
                                        act.psn
                                            .add_online_id(req.online_id)
                                            .get_profile()
                                            .into_actor(act)
                                            .map_err(|_, _, _| ())
                                            .and_then(|u: PSNUserLib, act, _| {
                                                act.update_profile_cache(u.into());
                                                actix::fut::ok(())
                                            })
                                    )
                                );
                            }
                            if let Some(req) = PSNActivationRequest::from_str(q.as_str()).ok() {
                                return Either::B(
                                    Either::A(
                                        act.psn
                                            .add_online_id(req.online_id)
                                            .get_profile()
                                            .into_actor(act)
                                            .map_err(|_, _, _| ())
                                            .and_then(|u: PSNUserLib, act, _| {
                                                act.update_profile_cache(u.into());
                                                actix::fut::ok(())
                                            })
                                    )
                                );
                            }
                            let req = serde_json::from_str::<PSNAuthRequest>(q.as_str())
                                .unwrap_or_else(|_| PSNAuthRequest::default());
                            Either::B(
                                Either::B(
                                    act.handle_auth_request(req)
                                )
                            )
                        })
                );
            };
        });
    }

    fn handle_auth_request(
        &mut self,
        req: PSNAuthRequest,
    ) -> impl ActorFuture<Item=(), Actor=Self, Error=()> {
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

    fn handle_profile_request(
        &mut self,
        req: PSNProfileRequest,
    ) -> impl ActorFuture<Item=(), Actor=Self, Error=()> {
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



    fn update_profile_cache(
        &self,
        p: UserPSNProfile,
    ) {
        actix_rt::spawn(
            crate::handler::cache::build_hmsets(
                self.get_conn(),
                vec![p],
                "user_psn",
                false,
            )
        );
    }
}

impl DatabaseService {
    // trophy is not frequent query. use simple query for less prepared statement.
    pub fn get_trophy_titles(
        &self,
        np_id: &str,
        page: u32,
    ) -> impl Future<Item=Vec<UserTrophyTitle>, Error=ResError> {
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
    ) -> impl Future<Item=Vec<UserTrophyTitle>, Error=ResError> {
        let page = req.page.as_ref().unwrap_or(&1);
        let online_id = req.online_id.as_str();

        let query = format!(
            "SELECT * FROM psn_user_trophy_titles WHERE np_id = {} ORDER BY last_update_date DESC LIMIT = 20",
            1
        );

        self.simple_query_multi_trait::<UserTrophyTitle>(query.as_str(), Vec::with_capacity(20))
    }

    fn add_user_trophy_titles(
        &self,
        np_id: &str,
        t: &[TrophyTitleLib],
    ) -> impl Future<Item=(), Error=ResError> {
        let mut query = String::new();

        for t in t.iter() {
            let d = &t.title_detail;
            write!(&mut query,
                   "INSERT INTO psn_user_trophy_titles
                    (np_id, np_communication_id, progress, earned_trophies, last_update_date)
                    VALUES ({}, {}, {}, ARRAY [{}, {}, {}, {}], {});
                    ON CONFLICT (np_id, np_communication_id)
                        DO UPDATE SET
                            progress = EXCLUDED.progress,
                            earned_trophies = EXCLUDED.earned_trophies,
                            last_update_date = EXCLUDED.last_update_date;
                   ",
                   np_id,
                   t.np_communication_id.as_str(),
                   d.progress,
                   d.earned_trophies.platinum,
                   d.earned_trophies.gold,
                   d.earned_trophies.silver,
                   d.earned_trophies.bronze,
                   d.last_update_date.as_str()
            );
        }

        self.simple_query_row_trait(query.as_str())
            .map(|_| ())
    }
}

impl CacheService {
    pub fn get_psn_profile(
        &self,
        online_id: &[u8],
    ) -> impl Future<Item=UserPSNProfile, Error=ResError> {
        use crate::handler::cache::FromCacheSingle;
        self.from_cache_single(online_id, "user_psn")
    }
}
