use crate::errors::Errors;
use crate::log::Log;
use crate::snapshot::SnapshotSink;
use anyhow::Result;
use chrono::Utc;
use std::io::Error;

/// `FSM` provides an trait that can be implemented by
/// clients to make use of the replicated log.
pub trait FSM<T> {
    /// `apply` log is invoked once a log entry is committed.
    /// It returns a value which will be made available in the
    /// `apply_future` returned by `raft::apply` method if that
    /// method was called on the same `raft` node as the `FSM`.
    fn apply(&self, l: Log) -> T;

    /// `snapshot` is used to support log compaction. This call should
    /// return an `FSMSnapshot` which can be used to save a point-in-time
    /// snapshot of the `FSM`. `apply` and `snapshot` are not called in multiple
    /// threads, but `apply` will be called concurrently with Persist. This means
    /// the `FSM` should be implemented in a fashion that allows for concurrent
    /// updates while a snapshot is happening
    fn snapshot(&self) -> dyn FSMSnapshot;

    /// `restore` is used to restore an `FSM` from a snapshot. It is not called
    /// concurrently with any other command. The `FSM` must discard all previous state.
    fn restore(&self, r: Box<dyn tokio::io::AsyncRead>) -> Result<(), std::io::Error>;
}

/// `BatchingFSM` extends the `FSM` interface to add an `apply_batch` function. This can
/// optionally be implemented by clients to enable multiple logs to be applied to
/// the `FSM` in batches. Up to MaxAppendEntries could be sent in a batch.
pub trait BatchingFSM<T>: FSM<T> {
    /// `apply_batch` is invoked once a batch of log entries has been committed and
    /// are ready to be applied to the `FSM`. `apply_batch` will take in an array of
    /// log entries. These log entries will be in the order they were committed,
    /// will not have gaps, and could be of a few log types. Clients should check
    /// the log type prior to attempting to decode the data attached. Presently
    /// the `LogCommand` and `LogConfiguration` types will be sent.
    ///
    /// The returned slice must be the same length as the input and each response
    /// should correlate to the log at the same index of the input. The returned
    /// values will be made available in the `ApplyFuture` returned by `Raft::apply`
    /// method if that method was called on the same Raft node as the `FSM`.
    fn apply_batch(&self, logs: Vec<Log>) -> Vec<T>;
}

/// `FSMSnapshot` is returned by an `FSM` in response to a Snapshot
/// It must be safe to invoke `FSMSnapshot` methods with concurrent
/// calls to Apply.
pub trait FSMSnapshot {
    /// `persist` should dump all necessary state to the WriteCloser 'sink',
    /// and call `sink.close` when finished or call `sink.cancel` on error.
    fn persist(&self, sink: dyn SnapshotSink) -> Result<(), Errors>;

    /// `release` is invoked when we are finished with the snapshot.
    fn release(&self);
}

// TODO: fsm.go line 69: func(r *Raft) runFSM() {}

/// `fsm_restore_and_measure` wraps the `restore` call on an `FSM` to consistently measure
/// and report timing metrics. The caller is still responsible for calling Close
/// on the source in all cases.
fn fsm_restore_and_measure<T>(
    fsm: Box<dyn FSM<T>>,
    source: Box<dyn tokio::io::AsyncRead>,
) -> Result<()> {
    let start = Utc::now();
    fsm.restore(source)?;

    metrics::gauge!("raft.fsm.restore", start.timestamp_millis() as f64);
    let duration = Utc::now().signed_duration_since(start).num_milliseconds() as f64;
    metrics::gauge!("raft.fsm.last.restore.duration", duration);
    Ok(())
}
