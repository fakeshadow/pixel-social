use std::time::Duration;

use futures::Future;

use actix::{ActorFuture, AsyncContext, Context, WrapFuture};
use psn_api_rs::{models::PSNUser, PSNRequest, PSN};

use crate::handler::cache::GetQueue;
use crate::model::{actors::PSNService, errors::ResError};

const PSN_TIME_GAP: Duration = Duration::from_millis(1000);

impl PSNService {
    pub fn start_interval(&self, ctx: &mut Context<Self>) {
        self.process_psn_request(ctx);
    }

    pub fn auth(uuid: String, two_step: String) -> impl Future<Item = PSN, Error = ResError> {
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
