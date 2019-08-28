use std::sync::Mutex;

use futures::{compat::Future01CompatExt, FutureExt};
use futures01::{
    future::{ExecuteError as ExecuteError01, Executor as Executor01},
    Future as Future01,
};
use tokio_executor::Executor;

/*
    took from https://github.com/mitsuhiko/redis-rs/pull/229
*/

pub struct Executor03As01<Ex> {
    inner: Mutex<Ex>,
}

impl<Ex> Executor03As01<Ex> {
    pub fn new(inner: Ex) -> Self {
        Executor03As01 {
            inner: Mutex::new(inner),
        }
    }
}

impl<Ex, Fut> Executor01<Fut> for Executor03As01<Ex>
where
    Ex: Executor,
    Fut: Future01<Item = (), Error = ()> + Send + 'static,
{
    fn execute(&self, f01: Fut) -> Result<(), ExecuteError01<Fut>> {
        let f03 = f01.compat().map(|_| ());
        let pin = Box::pin(f03);

        let mut g = self.inner.lock().unwrap();
        (&mut g)
            .spawn(pin)
            .expect("unable to spawn future from Compat executor");
        Ok(())
    }
}
