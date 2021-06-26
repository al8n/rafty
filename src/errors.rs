use std::string::FromUtf8Error;
use thiserror::Error;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum Errors {
    #[error("log not found")]
    LogNotFound,

    #[error("not found")]
    NotFound,

    #[error("cannot convert UTF8 to String")]
    FromUtf8Error(#[from] FromUtf8Error),
}
