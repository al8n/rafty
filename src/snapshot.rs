use crate::errors::Errors;
use anyhow::Result;

pub trait SnapshotSink {
    fn id(&self) -> String;
    fn cancel(&self) -> Result<(), Errors>;
}
