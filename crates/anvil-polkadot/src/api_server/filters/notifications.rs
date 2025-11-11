use futures::Stream;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use subxt::utils::H256;
use tokio::sync::broadcast;

#[derive(Debug)]
pub struct BlockNotifications {
    receiver: broadcast::Receiver<H256>,
}

impl BlockNotifications {
    pub fn new(receiver: broadcast::Receiver<H256>) -> Self {
        Self { receiver }
    }
}

impl Stream for BlockNotifications {
    type Item = H256;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.receiver.try_recv() {
            Ok(hash) => Poll::Ready(Some(hash)),
            Err(broadcast::error::TryRecvError::Empty) => Poll::Pending,
            Err(broadcast::error::TryRecvError::Lagged(_)) => {
                // We received more messages than our channel can hold.
                // Skip and try again 🤷
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(broadcast::error::TryRecvError::Closed) => Poll::Ready(None),
        }
    }
}
