use tokio::sync::mpsc::{Sender, OwnedPermit};
cfg_not_default!(
    use crossbeam::channel::{Sender, Receiver, bounded, unbounded, SendError, RecvError, TrySendError, TryRecvError, SendTimeoutError, RecvTimeoutError, Iter, TryIter};
    use std::time::{Duration, Instant};
    use std::panic::{UnwindSafe, RefUnwindSafe};

    #[derive(Display, FromStr, Debug, Copy, Clone, Eq, PartialEq)]
    #[display(style = "CamelCase")]
    pub enum ChannelType {
        Bounded,
        Unbounded,
    }

    #[derive(Clone, Debug)]
    pub struct Channel<T> {
        tx: Sender<T>,
        rx: Receiver<T>,
        typ: ChannelType,
    }

    unsafe impl<T: Send> Send for Channel<T> {}
    unsafe impl<T: Send> Sync for Channel<T> {}

    impl<T> UnwindSafe for Channel<T> {}
    impl<T> RefUnwindSafe for Channel<T> {}

    impl<T> Channel<T> {
        pub fn bounded(cap: usize) -> Self {
            let (tx, rx) = bounded::<T>(cap);
            Self {
                tx,
                rx,
                typ: ChannelType::Bounded
            }
        }

        pub fn unbounded() -> Self {
            let (tx, rx) = unbounded::<T>();
            Self {
                tx,
                rx,
                typ: ChannelType::Unbounded
            }
        }

        pub fn get_type(&self) -> ChannelType {
            self.typ
        }

        pub fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
            self.tx.try_send(msg)
        }

        pub fn send(&self, msg: T) -> Result<(), SendError<T>> {
            self.tx.send(msg)
        }

        pub fn send_timeout(&self, msg: T, timeout: Duration) -> Result<(), SendTimeoutError<T>> {
            self.tx.send_timeout(msg, timeout)
        }

        pub fn send_deadline(&self, msg: T, deadline: Instant) -> Result<(), SendTimeoutError<T>> {
            self.tx.send_deadline(msg, deadline)
        }

        pub fn is_tx_empty(&self) -> bool {
            self.tx.is_empty()
        }

        pub fn is_tx_full(&self) -> bool {
            self.tx.is_full()
        }

        pub fn tx_len(&self) -> usize {
            self.tx.len()
        }

        pub fn tx_capacity(&self) -> Option<usize> {
            self.tx.capacity()
        }

        pub fn tx_same_channel(&self, other: &Sender<T>) -> bool {
            self.tx.same_channel(other)
        }

        pub fn try_recv(&self) -> Result<T, TryRecvError> {
            self.rx.try_recv()
        }

        pub fn recv(&mut self) -> Result<T, RecvError> {
            self.rx.recv()
        }

        pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
            self.rx.recv_timeout(timeout)
        }

        pub fn recv_deadline(&self, deadline: Instant) -> Result<T, RecvTimeoutError> {
            self.rx.recv_deadline(deadline)
        }

        pub fn is_rx_empty(&self) -> bool {
            self.rx.is_empty()
        }

        pub fn is_rx_full(&self) -> bool {
            self.rx.is_full()
        }

        pub fn rx_len(&self) -> usize {
            self.rx.len()
        }

        pub fn rx_capacity(&self) -> Option<usize> {
            self.rx.capacity()
        }

        pub fn iter(&self) -> Iter<'_, T> {
            self.rx.iter()
        }

        pub fn try_iter(&self) -> TryIter<'_, T> {
            self.rx.try_iter()
        }

        pub fn rx_same_channel(&self, other: &Receiver<T>) -> bool {
            self.rx.same_channel(other)
        }
    }
);

use tokio::{
    sync::{mpsc, broadcast, oneshot, watch},

};
use std::time::Duration;
use std::task::{Context, Poll};

pub enum ChannelType {
    BoundedMPSC,
    UnboundedMPSC,
    Broadcast,
    OneShot,
    Watch,
}

pub struct BoundedMPSCChannel<T> {
    typ: ChannelType,
    tx: mpsc::Sender<T>,
    rx: mpsc::Receiver<T>,
}

impl<T> BoundedMPSCChannel<T> {
    pub fn new(cap: usize) -> Self {
        let (tx, rx) = mpsc::channel::<T>(cap);
        Self {
            typ: ChannelType::BoundedMPSC,
            tx,
            rx,
        }
    }

    pub fn rx_close(&mut self) {
        self.rx.close()
    }

    pub async fn recv(&mut self) -> Option<T> {
        self.rx.recv().await
    }

    pub fn blocking_recv(&mut self) -> Option<T> {
        self.rx.blocking_recv()
    }

    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        self.rx.poll_recv(cx)
    }

    pub async fn tx_closed(&self) {
        self.tx.closed().await
    }

    pub fn is_tx_closed(&self) -> bool {
        self.tx.is_closed()
    }

    pub fn try_send(&self, msg: T) -> Result<(), mpsc::error::TrySendError<T>> {
        self.tx.try_send(msg)
    }

    pub async fn send(&self, msg: T) -> Result<(), mpsc::error::SendError<T>> {
        self.tx.send(msg).await
    }

    pub async fn send_timeout(
        &self,
        value: T,
        timeout: Duration,
    ) -> Result<(), mpsc::error::SendTimeoutError<T>> {
        self.tx.send_timeout(value, timeout).await
    }

    pub fn blocking_send(&self, value: T) -> Result<(), mpsc::error::SendError<T>> {
        self.tx.blocking_send(value)
    }

    pub async fn reserve(&self) -> Result<mpsc::Permit<'_, T>, mpsc::error::SendError<()>> {
        self.tx.reserve().await
    }

    pub async fn reserve_owned(self) -> Result<mpsc::OwnedPermit<T>, mpsc::error::SendError<()>> {
        self.tx.reserve_owned().await
    }

    pub fn try_reserve(&self) -> Result<mpsc::Permit<'_, T>, mpsc::error::TrySendError<()>> {
        self.tx.try_reserve()
    }

    pub fn try_reserve_owned(self) -> Result<OwnedPermit<T>, mpsc::error::TrySendError<Sender<T>>> {
        self.tx.try_reserve_owned()
    }

    pub fn tx_same_channel(&self, other: &mpsc::Sender<T>) -> bool {
        self.tx.same_channel(other)
    }

    pub fn tx_capacity(&self) -> usize {
        self.tx.capacity()
    }
}

pub struct UnboundedMPSCChannel<T> {
    typ: ChannelType,
    tx: mpsc::UnboundedSender<T>,
    rx: mpsc::UnboundedReceiver<T>,
}

impl<T> UnboundedMPSCChannel<T> {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel::<T>();
        Self {
            typ: ChannelType::BoundedMPSC,
            tx,
            rx,
        }
    }

    pub fn rx_close(&mut self) {
        self.rx.close()
    }

    pub async fn recv(&mut self) -> Option<T> {
        self.rx.recv().await
    }

    pub fn blocking_recv(&mut self) -> Option<T> {
        self.rx.blocking_recv()
    }

    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        self.rx.poll_recv(cx)
    }

    pub async fn tx_closed(&self) {
        self.tx.closed().await
    }

    pub fn is_tx_closed(&self) -> bool {
        self.tx.is_closed()
    }

    pub fn send(&self, msg: T) -> Result<(), mpsc::error::SendError<T>> {
        self.tx.send(msg)
    }

    pub fn tx_same_channel(&self, other: &mpsc::UnboundedSender<T>) -> bool {
        self.tx.same_channel(other)
    }
}