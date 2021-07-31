use crate::config::{SnapshotVersion, Configuration};
use crate::snapshot::SnapshotSink;
use crate::errors::Error;

/// `DiscardSnapshotStore` is used to successfully snapshot while
/// always discarding the snapshot. This is useful for when the
/// log should be truncated but no snapshot should be retained.
/// This should never be used for production use, and is only
/// suitable for testing.
#[derive(Copy, Clone, Debug, Default)]
pub struct DiscardSnapshotStore;

// impl DiscardSnapshotStore {
//     pub fn create(_version: SnapshotVersion, _index: u64, _term: u64, _configuration: Configuration, _configuration_index: u64, _trans: Transport) -> Result<DiscardSnapshotSink, Error> {
//         Ok(DiscardSnapshotSink::default())
//     }
// }

/// `DiscardSnapshotSink` is used to fulfill the `SnapshotSink` interface
/// while always discarding the . This is useful for when the log
/// should be truncated but no snapshot should be retained. This
/// should never be used for production use, and is only suitable
/// for testing.
#[derive(Copy, Clone, Debug, Default)]
pub struct DiscardSnapshotSink;

// impl SnapshotSink for DiscardSnapshotSink {
//     fn id(&self) -> String {
//         todo!()
//     }
//
//     fn cancel(&self) -> Result<(), Error> {
//         todo!()
//     }
// }