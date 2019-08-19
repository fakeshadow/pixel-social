use std::convert::TryInto;
use std::fmt::Write;
use std::time::Duration;

use actix::{ActorFuture, AsyncContext, Context, fut::Either as ActorEither, WrapFuture, WrapStream};
use futures::{
    future::{Either, err as ft_err},
    Future, IntoFuture, Stream,
};
use psn_api_rs::{PSN, PSNRequest as PSNRequestLib};

use crate::handler::{
    cache::{CacheService, GetQueue, GetSharedConn},
    db::{DatabaseService, Query, SimpleQuery},
};
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
                    act.get_queue("psn_queue")
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
                                        page: _
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
                                            .and_then(|u: PSNUserLib, act, _| {
                                                act.update_profile_cache(u.into());
                                                actix::fut::ok(())
                                            }),
                                    )),
                                },
                                Err(_) => ActorEither::B(ActorEither::B(actix::fut::ok(()))),
                            }
                        })
                }).map_err(|e: ResError, _, _| {
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
            vec![p],
            "user_psn",
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
            .then(move |r: Result<UserTrophySet, _>, act, _| {
                if let Ok(tt) = r {
                    for t in t.trophies.iter_mut() {
                        for tt in tt.trophies.iter() {
                            if t.trophy_id == tt.trophy_id {
                                if t.earned_date.is_some() && tt.earned_date.is_none() {
                                    t.first_earned_date = t.earned_date;
                                }
                                break;
                            }
                        }
                    }
                };

                act.update_user_trophy_set(&t).into_actor(act)
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
                    trophy_set = EXCLUDED.trophy_set
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
                    self.insert_trophy_title.as_ref().unwrap(),
                    &[
                        &t.np_id,
                        &t.np_communication_id,
                        &(t.progress as i32),
                        &(t.earned_platinum as i32),
                        &(t.earned_gold as i32),
                        &(t.earned_silver as i32),
                        &(t.earned_bronze as i32),
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
                            .unwrap_or("".to_owned()),
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
