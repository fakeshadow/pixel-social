use std::convert::TryInto;
use std::fmt::Write;
use std::time::Duration;

use actix::{ActorFuture, AsyncContext, Context, fut::Either as ActorEither, WrapFuture};
use futures::{
    future::{Either, err as ft_err},
    Future, IntoFuture, Stream,
};
use psn_api_rs::{PSN, PSNRequest as PSNRequestLib};

use crate::handler::{
    cache::{CacheService, GetQueue, GetSharedConn},
    db::{DatabaseService, Query, SimpleQuery},
};
use crate::handler::cache::CheckCacheConn;
use crate::model::{
    actors::PSNService,
    errors::ResError,
    psn::{
        PSNRequest, PSNUserLib, TrophySetLib, TrophyTitlesLib, UserPSNProfile, UserTrophySet,
        UserTrophyTitle,
    },
};

const PSN_TIME_GAP: Duration = Duration::from_millis(3000);

impl PSNService {
    pub fn start_interval(&self, ctx: &mut Context<Self>) {
        self.process_psn_request(ctx);
    }

    fn process_psn_request(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(PSN_TIME_GAP, move |act, ctx| {
            ctx.spawn(act
                .check_token()
                .and_then(|_, act: &mut PSNService, _| {
                    //ToDo: db connection is not checked yet.
                    act.check_cache_conn()
                        .into_actor(act)
                        .and_then(|opt, act, _| {
                            act.if_replace_cache(opt)
                                .get_queue("psn_queue")
                                .into_actor(act)
                                .and_then(|q: String, act, _| {
                                    match serde_json::from_str::<PSNRequest>(q.as_str()) {
                                        Ok(req) => match req {
                                            PSNRequest::Profile {
                                                online_id
                                            } => {
                                                ActorEither::A(ActorEither::A(ActorEither::A(
                                                    act.handle_profile_request(online_id),
                                                )))
                                            }
                                            PSNRequest::TrophyTitles {
                                                online_id,
                                                ..
                                            } => {
                                                ActorEither::A(ActorEither::A(ActorEither::B(
                                                    act.handle_trophy_titles_request(online_id)
                                                        .and_then(|r: Vec<UserTrophyTitle>, act: &mut PSNService, _| {
                                                            act.update_user_trophy_titles(&r)
                                                                .into_actor(act)
                                                        }),
                                                )))
                                            }
                                            PSNRequest::TrophySet {
                                                online_id,
                                                np_communication_id,
                                            } => ActorEither::A(ActorEither::B(ActorEither::A(
                                                act.handle_trophy_set_request(online_id, np_communication_id)
                                                    .and_then(|r: UserTrophySet, act: &mut PSNService, _| {
                                                        act.query_update_user_trophy_set(r)
                                                    }),
                                            ))),
                                            PSNRequest::Auth {
                                                uuid,
                                                two_step,
                                                refresh_token
                                            } => {
                                                ActorEither::A(ActorEither::B(ActorEither::B(
                                                    act.handle_auth_request(uuid, two_step, refresh_token),
                                                )))
                                            }
                                            PSNRequest::Activation {
                                                user_id,
                                                online_id,
                                                code,
                                            } => ActorEither::B(ActorEither::A(
                                                act.psn
                                                    .add_online_id(online_id)
                                                    .get_profile()
                                                    .from_err()
                                                    .into_actor(act)
                                                    .and_then(move |u: PSNUserLib, act, _| {
                                                        if u.about_me == code {
                                                            let mut u = UserPSNProfile::from(u);
                                                            u.id = user_id;
                                                            act.update_profile_cache(u);
                                                            actix::fut::ok(())
                                                        } else {
                                                            // ToDo: add more error detail and send it through message to user.
                                                            actix::fut::err(ResError::Unauthorized)
                                                        }
                                                    }),
                                            )),
                                        },
                                        Err(_) => ActorEither::B(ActorEither::B(actix::fut::ok(()))),
                                    }
                                })
                        })
                })
                .map_err(|e: ResError, _, _| {
                    match e {
                        ResError::NoCache => (),
                        _ => println!("{:?}", e.to_string())
                    }
                })
            );
        });
    }

    fn handle_auth_request(
        &mut self,
        uuid: Option<String>,
        two_step: Option<String>,
        refresh_token: Option<String>,
    ) -> impl ActorFuture<Item=(), Actor=Self, Error=ResError> {
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
            act.is_active = true;
        })
    }

    fn handle_trophy_titles_request(
        &mut self,
        online_id: String,
    ) -> impl ActorFuture<Item=Vec<UserTrophyTitle>, Actor=Self, Error=ResError> {
        // get profile before and after getting titles and check if the user's np_id remains unchanged.
        self.psn
            .add_online_id(online_id)
            .get_profile()
            .from_err()
            .into_actor(self)
            .and_then(|u: PSNUserLib, act, _| {
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
                        futures::future::join_all(f)
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
                                act.psn.get_profile().from_err().into_actor(act).and_then(
                                    move |uu: PSNUserLib, _, _| {
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
                                    },
                                )
                            })
                    },
                )
            })
    }

    fn handle_trophy_set_request(
        &mut self,
        online_id: String,
        np_communication_id: String,
    ) -> impl ActorFuture<Item=UserTrophySet, Actor=Self, Error=ResError> {
        self.psn
            .add_online_id(online_id)
            .get_profile()
            .from_err()
            .into_actor(self)
            .and_then(|u: PSNUserLib, act, _| {
                act.psn
                    .add_np_communication_id(np_communication_id.clone())
                    .get_trophy_set::<TrophySetLib>()
                    .from_err()
                    .into_actor(act)
                    .and_then(|set, act, _| {
                        act.psn.get_profile().from_err().into_actor(act).and_then(
                            move |uu: PSNUserLib, _, _| {
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
                            },
                        )
                    })
            })
    }

    fn handle_profile_request(
        &mut self,
        online_id: String,
    ) -> impl ActorFuture<Item=(), Actor=Self, Error=ResError> {
        self.psn
            .add_online_id(online_id)
            .get_profile()
            .from_err()
            .into_actor(self)
            .and_then(|u: PSNUserLib, act, _| {
                act.update_profile_cache(u.into());
                actix::fut::ok(())
            })
    }

    fn update_profile_cache(&self, p: UserPSNProfile) {
        actix::spawn(crate::handler::cache::build_hmsets(
            self.get_conn(),
            &[p],
            crate::handler::cache::USER_PSN_U8,
            false,
        ));
    }

    // a costly update for updating existing trophy set.
    // The purpose is to flag people who have a changed trophy timestamp on the trophy already earned
    // by comparing the The first_earned_date with the earned_date
    fn query_update_user_trophy_set(
        &self,
        mut t: UserTrophySet,
    ) -> impl ActorFuture<Item=(), Actor=Self, Error=ResError> {
        let query = format!(
            "SELECT * FROM psn_user_trophy_sets WHERE np_id='{}' and np_communication_id='{}';",
            t.np_id.as_str(),
            t.np_communication_id.as_str()
        );
        self.simple_query_one_trait(query.as_str())
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
    ) -> impl Future<Item=(), Error=ResError> {
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

        Either::B(self.simple_query_row_trait(query.as_str()).map(|_| ()))
    }

    fn update_user_trophy_titles(
        &mut self,
        t: &[UserTrophyTitle],
    ) -> impl Future<Item=(), Error=ResError> {
        let mut v = Vec::with_capacity(t.len());

        for t in t.iter() {
            let f = self
                .get_client()
                .execute(
                    &self.insert_trophy_title.as_ref().unwrap().borrow(),
                    &[
                        &t.np_id,
                        &t.np_communication_id,
                        &i32::from(t.progress),
                        &i32::from(t.earned_platinum),
                        &i32::from(t.earned_gold),
                        &i32::from(t.earned_silver),
                        &i32::from(t.earned_bronze),
                        &t.last_update_date,
                    ],
                )
                .into_future()
                .from_err();

            v.push(f);
        }

        futures::stream::futures_unordered(v)
            .into_future()
            .map_err(|(e, _)| e)
            .map(|_| ())
    }

    fn check_token(&self) -> impl ActorFuture<Item=(), Actor=Self, Error=ResError> {
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
    // trophy is not frequent query. use simple query for less prepared statement.
    pub fn get_trophy_titles(
        &self,
        np_id: &str,
        page: u32,
    ) -> impl Future<Item=Vec<UserTrophyTitle>, Error=ResError> {
        let query = format!(
            "SELECT * FROM psn_user_trophy_titles WHERE np_id='{}' ORDER BY last_update_date DESC OFFSET {} LIMIT 20",
            np_id,
            (page - 1) * 20
        );

        self.simple_query_multi_trait::<UserTrophyTitle>(query.as_str(), Vec::with_capacity(20))
            .and_then(|v| {
                if v.is_empty() {
                    Err(ResError::NotFound)
                } else {
                    Ok(v)
                }
            })
    }

    pub fn get_trophy_set(
        &self,
        np_id: &str,
        np_communication_id: &str,
    ) -> impl Future<Item=UserTrophySet, Error=ResError> {
        let query = format!(
            "SELECT * FROM psn_user_trophy_sets WHERE np_id='{}' and np_communication_id='{}'",
            np_id, np_communication_id
        );

        self.simple_query_one_trait::<UserTrophySet>(query.as_str())
    }

    //    pub fn update_trophy_meta(
    //        &self,
    //        test: &str
    //    ) -> impl Future<Item= () , Error = ResError> {
    //        let mut query = String::new();
    //
    //        self.simple_query_one_trait::<UserTrophySet>(query.as_str())
    //    }
}

impl CacheService {
    pub fn get_psn_profile(
        &self,
        online_id: &str,
    ) -> impl Future<Item=UserPSNProfile, Error=ResError> {
        use crate::handler::cache::FromCacheSingle;
        self.from_cache_single(online_id, "user_psn")
    }
}
