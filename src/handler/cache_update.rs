use std::time::Duration;

use actix::prelude::{ActorFuture, AsyncContext, Context, Handler, Message, WrapFuture};
use chrono::Utc;
use futures::{
    future::{join_all, Either},
    Future,
};
use redis::aio::SharedConnection;

use crate::handler::cache::{
    update_list, update_post_count, AddPostCache, AddTopicCache, BulkUpdateCache,
    CategoriesFromCache, CheckCacheConn, FromCache, GetSharedConn, IdsFromList,
};
use crate::model::cache_update::FailedType;
use crate::model::errors::ResError;
use crate::model::{actors::CacheUpdateService, post::Post, topic::Topic, user::User};

// list_pop update interval time gap in seconds
const LIST_TIME_DUR: Duration = Duration::from_secs(5);
// time interval for retry adding failed cache to redis.
const FAILED_TIME_DUR: Duration = Duration::from_secs(3);

impl CacheUpdateService {
    pub fn start_interval(&mut self, ctx: &mut Context<Self>) {
        self.update_list_pop(ctx);
        self.update_failed_cache(ctx);
    }

    // use only this interval to reconnect redis if the connection is lost.
    fn update_list_pop(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(LIST_TIME_DUR, move |act, ctx| {
            ctx.spawn(
                act.check_cache_conn()
                    .into_actor(act)
                    .and_then(|opt, act, _| {
                        act.if_replace_cache(opt)
                            .categories_from_cache()
                            .into_actor(act)
                            .and_then(|cat, act, _| {
                                let conn = act.get_conn();
                                let yesterday =
                                    Utc::now().naive_utc().timestamp_millis() - 86_400_000;
                                let mut vec = Vec::new();

                                for c in cat.iter() {
                                    // update_list will also update topic count new.
                                    vec.push(Either::A(update_list(
                                        Some(c.id),
                                        yesterday,
                                        conn.clone(),
                                    )));
                                    vec.push(Either::B(update_post_count(
                                        c.id,
                                        yesterday,
                                        conn.clone(),
                                    )));
                                }
                                vec.push(Either::A(update_list(None, yesterday, conn)));

                                join_all(vec).map(|_| ()).into_actor(act)
                            })
                    })
                    .map_err(|_e: ResError, _, _| ()),
            );
        });
    }

    // ToDo: right now every failed cache is update individually. Could use a giant pipeline to reduce some traffic if there are major lost connection to redis occur often.
    fn update_failed_cache(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(FAILED_TIME_DUR, move |act, ctx| {
            let mut v = Vec::new();

            let mut u_t = Vec::new();
            let mut u_p = Vec::new();
            let mut u_u = Vec::new();

            let mut uids = Vec::new();
            let mut pids = Vec::new();
            let mut tids = Vec::new();

            if let Ok(l) = act.failed_collection.lock() {
                for (t, typ) in l.topic.iter() {
                    tids.push(t.id);
                    match *typ {
                        FailedType::New => v.push(act.add_topic_cache(t)),
                        FailedType::Update => u_t.push(t),
                    };
                }

                for (p, typ) in l.post.iter() {
                    pids.push(p.id);
                    match *typ {
                        FailedType::New => v.push(act.add_post_cache(&p)),
                        FailedType::Update => u_p.push(p),
                    };
                }

                for (u, _) in l.user.iter() {
                    uids.push(u.id);
                    u_u.push(u)
                }

                v.push(act.bulk_add_update_cache(u_t, u_p, u_u));
            };

            if !v.is_empty() {
                ctx.spawn(join_all(v).map_err(|_| ()).into_actor(act).and_then(
                    move |_, act, _| {
                        if let Ok(mut l) = act.failed_collection.lock() {
                            l.remove_by_pids(&pids);
                            l.remove_by_tids(&tids);
                            l.remove_by_uids(&uids);
                        }
                        actix::fut::ok(())
                    },
                ));
            }
        });
    }
}

impl GetSharedConn for CacheUpdateService {
    fn get_conn(&self) -> SharedConnection {
        self.cache.as_ref().unwrap().borrow().clone()
    }
}

impl IdsFromList for CacheUpdateService {}

impl FromCache for CacheUpdateService {}

impl CategoriesFromCache for CacheUpdateService {}

impl AddTopicCache for CacheUpdateService {}

impl AddPostCache for CacheUpdateService {}

impl BulkUpdateCache for CacheUpdateService {}

impl CheckCacheConn for CacheUpdateService {
    fn self_url(&self) -> String {
        self.url.to_owned()
    }

    fn replace_cache(&self, c: SharedConnection) {
        self.cache.as_ref().map(|s| s.replace(c));
    }
}

// cache service will push data failed to insert into redis to cache update service.
// we will just keep retrying to add them to redis.
#[derive(Message)]
pub enum CacheFailedMessage {
    FailedTopic(Topic),
    FailedPost(Post),
    FailedUser(Vec<User>),
    FailedTopicUpdate(Vec<Topic>),
    FailedPostUpdate(Vec<Post>),
}

impl Handler<CacheFailedMessage> for CacheUpdateService {
    type Result = ();
    fn handle(&mut self, msg: CacheFailedMessage, _: &mut Context<Self>) {
        match msg {
            CacheFailedMessage::FailedPost(p) => {
                if let Ok(mut l) = self.failed_collection.lock() {
                    l.add_post_new(p);
                }
            }
            CacheFailedMessage::FailedTopic(t) => {
                if let Ok(mut l) = self.failed_collection.lock() {
                    l.add_topic_new(t);
                }
            }
            CacheFailedMessage::FailedTopicUpdate(t) => {
                if let Ok(mut l) = self.failed_collection.lock() {
                    l.add_topic_update(t);
                }
            }
            CacheFailedMessage::FailedPostUpdate(p) => {
                if let Ok(mut l) = self.failed_collection.lock() {
                    l.add_post_update(p);
                }
            }
            CacheFailedMessage::FailedUser(t) => {
                if let Ok(mut l) = self.failed_collection.lock() {
                    l.add_user(t);
                }
            }
        }
    }
}
