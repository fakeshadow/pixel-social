//use actix::Handler;
//use diesel::prelude::*;
//use actix_redis::{Command, RedisActor as CacheExecutor};
//
//use crate::app::AppState;
//use crate::model::cache::CacheQueryResult;
//use crate::model::errors::ServiceError;
//use crate::model::{cache::*, post::Post, user::SlimUser, db::DbExecutor};
//use crate::schema::{topics, posts, users};
//
//const LIMIT: i64 = 20;
//
//impl Handler<CacheQuery> for CacheExecutor {
//    type Result = Result<CacheQueryResult, ServiceError>;
//
//    fn handle(&mut self, message: CacheQuery, _: &mut Self::Context) -> Self::Result {
//
//        let conn = &self
//
//
//        match message {
//            CacheQuery::UpdateCategoryTopicCache(topics) => {
//                self::Command(resp_array!["SET", "mydomain:one", info.one])
//
//                Ok(CacheQueryResult::UpdatedCache)
//            }
//            _=> Ok(CacheQueryResult::UpdatedCache)
//        }
//    }
//}