use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use futures::{
    channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    lock::Mutex,
    FutureExt, SinkExt, StreamExt,
};
use tokio::{future::FutureExt as TokioFutureExt, timer::Interval};

use crate::model::errors::ResError;

/// channel address wraps the channel sender and passed to other threads.
pub struct ChannelAddress<T> {
    address: Arc<Mutex<UnboundedSender<T>>>,
}

impl<T> Clone for ChannelAddress<T> {
    fn clone(&self) -> Self {
        ChannelAddress {
            address: self.address.clone(),
        }
    }
}

impl<T> ChannelAddress<T>
where
    T: Send + 'static,
{
    /// spawn a future and send message to channel receiver.
    pub fn do_send(&self, req: T) {
        let lock = self.address.clone();
        tokio::spawn(async move {
            let mut sender = lock.lock().await;
            let _ = sender.send(req).await;
        });
    }
}

/// create and channel and return send in Arc<Mutex> and receiver.
pub trait ChannelCreate {
    type Message;

    fn create_channel() -> (
        ChannelAddress<Self::Message>,
        UnboundedReceiver<Self::Message>,
    ) {
        let (tx, rx) = unbounded::<Self::Message>();

        (
            ChannelAddress {
                address: Arc::new(Mutex::new(tx)),
            },
            rx,
        )
    }
}

/// spawn a future and iterate the channel receiver and inject message to queue.
pub trait SpawnQueueHandler<T: Send>: Send + Sized + 'static {
    type Error;

    fn receiver(&mut self) -> &mut UnboundedReceiver<T>;

    fn handle_message<'a>(
        &'a mut self,
        msg: T,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>;

    fn spawn_queue(mut self) {
        tokio::spawn(
            async move {
                while let Some(req) = self.receiver().next().await {
                    self.handle_message(req).await?;
                }
                Ok(())
            }
                // ToDo: add error handler
                .map(|_r: Result<(), Self::Error>| ()),
        );
    }
}

/// spawn handler for interval loop futures.
pub trait SpawnIntervalHandler: Sized + Send + Sync + 'static {
    fn handle<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<(), ResError>> + Send + 'a>>;

    fn spawn_interval(self, dur: Duration) {
        tokio::spawn(async move {
            let mut interval = Interval::new_interval(dur);
            loop {
                interval.next().await;
                let r = self.handle().timeout(dur).await;
                if let Ok(r) = r {
                    if let Err(e) = r {
                        // ToDo: handler error.
                        println!("{:?}", e.to_string());
                    }
                }
            }
        });
    }
}
