use crate::config::{Configuration, ConfigurationChangeRequest};
use crate::errors::Error;
use crate::log::Log;

use crossbeam::{
    channel::{bounded, Receiver, Sender, SendError, RecvError},
    select,
};
use crate::raft::Raft;


/// Future is used to represent an action that may occur in the future.
pub trait Future {
    /// Error blocks until the future arrives and then returns the error status
    /// of the future. This may be called any number of times - all calls will
    /// return the same value, however is not OK to call this method twice
    /// concurrently on the same Future instance.
    /// Error will only return generic errors related to raft, such
    /// as ErrLeadershipLost, or ErrRaftShutdown. Some operations, such as
    /// ApplyLog, may also return errors from other methods.
    fn error(&mut self) -> Result<Error, Box<dyn std::error::Error>>;
}

/// IndexFuture is used for future actions that can result in a raft log entry
/// being created.
pub trait IndexFuture: Future {
    /// Index holds the index of the newly applied log entry.
    /// This must not be called until after the Error method has returned.
    fn index(&self) -> u64;
}

/// ApplyFuture is used for Apply and can return the FSM response.
pub trait ApplyFuture<T>: IndexFuture {
    /// Response returns the FSM response as returned by the FSM.Apply method. This
    /// must not be called until after the Error method has returned.
    /// Note that if FSM.Apply returns an error, it will be returned by Response,
    /// and not by the Error method, so it is always important to check Response
    /// for errors from the FSM.
    fn response(&self) -> T;
}

/// ConfigurationFuture is used for GetConfiguration and can return the
/// latest configuration in use by Raft.
pub trait ConfigurationFuture: IndexFuture {
    /// Configuration contains the latest configuration. This must
    /// not be called until after the Error method has returned.
    fn configuration(&self) -> Configuration;
}

/// SnapshotFuture is used for waiting on a user-triggered snapshot to complete.
pub trait SnapshotFuture: Future {
    // TODO: implement snapshot future
    // fn open(&self) -> ;
}

/// LeadershipTransferFuture is used for waiting on a user-triggered leadership
/// transfer to complete.
pub trait LeadershipTransferFuture: Future {}

/// ErrorFuture is used to return a static error.
pub struct ErrorFuture {
    error: Error,
}

impl Future for ErrorFuture {
    fn error(&mut self) -> Result<Error, Box<dyn std::error::Error>> {
        Ok(self.error.clone())
    }
}

impl IndexFuture for ErrorFuture {
    fn index(&self) -> u64 {
        0
    }
}

impl ApplyFuture<()> for ErrorFuture {
    fn response(&self) -> () {
        ()
    }
}

/// DeferError can be embedded to allow a future
/// to provide an error in the future
pub struct DeferError {
    err: Option<Error>,
    err_tx_ch: Option<Sender<Error>>,
    err_rx_ch: Receiver<Error>,
    responded: bool,
    shutdown_tx_ch: Sender<()>,
    shutdown_rx_ch: Receiver<()>,
}

impl Default for DeferError {
    fn default() -> Self {
        let (etx, erx) = bounded::<Error>(1);
        let (stx, srx) = bounded::<()>(1);

        Self {
            err: None,
            err_tx_ch: Some(etx),
            err_rx_ch: erx,
            responded: false,
            shutdown_tx_ch: stx,
            shutdown_rx_ch: srx,
        }
    }
}

impl DeferError {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn respond(&mut self, e: Option<Error>) -> Result<(), Box<dyn std::error::Error>> {
        match self.err_tx_ch.clone() {
            None => Ok(()),
            Some(tx) => {
                if self.responded {
                    return Ok(());
                }
                match e {
                    None => {
                        tx.send(Error::None);
                        self.responded = true;
                        self.err_tx_ch = None;
                        Ok(())
                    }
                    Some(e) => {
                        match tx.send(e) {
                            Ok(_) => {
                                self.err_tx_ch = None;
                                self.responded = true;
                                Ok(())
                            },
                            Err(e) => {
                                Err(Box::new(e))
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Future for DeferError {
    fn error(&mut self) -> Result<Error, Box<dyn std::error::Error>> {
        let err = self.err.clone();

        match err {
            None => {
                select! {
                    recv(self.err_rx_ch) -> result => {
                        match result {
                            Ok(e) => {
                                self.err = Some(e);
                            },
                            Err(e) => {
                                return Err(Box::new(e));
                            },
                        }

                    },
                    recv(self.shutdown_rx_ch) -> _ => {
                        self.err = Some(Error::RaftShutdown);
                    },
                }
                Ok(self.err.clone().unwrap())
            },
            Some(e) => Ok(e),
        }
    }
}


/// There are several types of requests that cause a configuration entry to
/// be appended to the log. These are encoded here for leaderLoop() to process.
/// This is internal to a single server.
pub struct ConfigurationChangeFuture<T>  {
    lf: LogFuture<T>,
    req: ConfigurationChangeRequest,
}

/// `BootstrapFuture` is used to attempt a live bootstrap of the cluster. See the
/// `Raft` object's `BootstrapCluster` member function for more details.
pub struct BootStrapFuture  {
    defer_error: DeferError,

    /// configuration is the proposed bootstrap configuration to apply.
    configuration: Configuration
}

impl Future for BootStrapFuture {
    fn error(&mut self) -> Result<Error, Box<dyn std::error::Error>> {
        self.defer_error.error()
    }
}

impl LeadershipTransferFuture for BootStrapFuture {}

/// `LogFuture` is used to apply a log entry and waits until
/// the log is considered committed.
pub struct LogFuture<T> {
    defer_err: DeferError,
    log: Log,
    response: T,
    dispatch: std::time::Instant,
}

impl<T: Clone> Future for LogFuture<T> {
    fn error(&mut self) -> Result<Error, Box<dyn std::error::Error>> {
        self.defer_err.error()
    }
}

impl<T: Clone> IndexFuture for LogFuture<T> {
    fn index(&self) -> u64 {
        self.log.index
    }
}

impl<T: Clone> ApplyFuture<T> for LogFuture<T> {
    fn response(&self) -> T {
        self.response.clone()
    }
}

impl<T: Clone> LeadershipTransferFuture for LogFuture<T> {}

pub struct ShutdownFuture<'a, FSM, LF> {
    raft: Option<&'a Raft<FSM, LF>>
}

impl<'a, FSM, LF> Future for ShutdownFuture<'a, FSM, LF> {
    fn error(&mut self) -> Result<Error, Box<dyn std::error::Error>> {
        match self.raft {
            None => Err(Box::new(Error::None)),
            Some(mut r) => {
                r.wait_shutdown()

            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::future::{DeferError, Future};
    use crate::errors::Error;

    #[test]
    fn test_defer_error_future_success() {
        let mut de = DeferError::new();
        de.respond(None).unwrap();
        assert_eq!(Error::None, de.error().unwrap());
        assert_eq!(Error::None, de.error().unwrap());
    }

    #[test]
    fn test_defer_error_future_error() {
        let want = Error::NonVoter;
        let mut de = DeferError::new();
        de.respond(Some(want));
        assert_eq!(Error::NonVoter, de.error().unwrap());
        assert_eq!(Error::NonVoter, de.error().unwrap());
    }

    #[test]
    fn test_defer_error_future_concurrent() {
        let want = Some(Error::NonVoter);
        let mut de = DeferError::new();
        crossbeam::thread::scope(|s| {
            s.spawn(|_| {
                de.respond(want);
            });
        });

        assert_eq!(Error::NonVoter, de.error().unwrap());
    }
}