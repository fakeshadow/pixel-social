use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::{
    channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    lock::Mutex,
    FutureExt, SinkExt, StreamExt,
};

/// channel address wraps the channel sender and passed to other threads.
pub struct ChannelAddress<T> {
    inner: Arc<Mutex<UnboundedSender<T>>>,
}

impl<T> Clone for ChannelAddress<T> {
    fn clone(&self) -> Self {
        ChannelAddress {
            inner: self.inner.clone(),
        }
    }
}

impl<T> ChannelAddress<T>
where
    T: Send + 'static,
{
    /// spawn a future and send message to channel receiver.
    pub fn do_send(&self, req: T) {
        let lock = self.inner.clone();
        tokio::spawn(async move {
            let mut sender = lock.lock().await;
            let _ = sender.send(req).await;
        });
    }
}

/// create and channel and return send in Arc<Mutex> and receiver.
pub trait ChannelGenerator {
    type Message;

    fn create_channel() -> (
        ChannelAddress<Self::Message>,
        UnboundedReceiver<Self::Message>,
    ) {
        let (tx, rx) = unbounded::<Self::Message>();

        (
            ChannelAddress {
                inner: Arc::new(Mutex::new(tx)),
            },
            rx,
        )
    }
}

/// spawn a future and iterate the channel receiver and inject message to queue.
pub trait InjectQueue<T>: Send + Sized + 'static
where
    T: Send,
{
    type Error;

    fn receiver(&mut self) -> &mut UnboundedReceiver<T>;

    fn handle_message<'a>(
        &'a mut self,
        msg: T,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>;

    fn handle_inject(mut self) {
        tokio::spawn(
            async move {
                while let Some(req) = self.receiver().next().await {
                    self.handle_message(req).await?;
                }
                Ok(())
                // ToDo: add error handler
            }
                .map(|r: Result<(), Self::Error>| ()),
        );
    }
}
