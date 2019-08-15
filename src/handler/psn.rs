use std::time::Duration;

use actix::{ActorFuture, AsyncContext, Context, WrapFuture};
use futures::Future;
use psn_api_rs::{PSN, PSNRequest};

use crate::handler::cache::{CacheService, GetQueue, FromCacheSingle};
use crate::model::{
    actors::PSNService,
    errors::ResError,
    psn::UserPSNProfile,
};

const PSN_TIME_GAP: Duration = Duration::from_millis(1000);

impl PSNService {
    pub fn start_interval(&self, ctx: &mut Context<Self>) {
        self.process_psn_request(ctx);
    }

    pub fn auth(uuid: String, two_step: String) -> impl Future<Item=PSN, Error=ResError> {
        PSN::new()
            .add_uuid(uuid)
            .add_two_step(two_step)
            .auth()
            .from_err()
    }

    fn process_psn_request(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(PSN_TIME_GAP, move |act, ctx| {
            if act.is_active == true {
                ctx.spawn(
                    act.get_queue("psn_queue")
                        .into_actor(act)
                        .map_err(|_, _, _| ())
                        .map(|_, _, _| ()),
                );
            };
        });
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