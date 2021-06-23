extern crate lazy_static;
use std::fmt::{Display, Formatter, Result as FormatResult};

pub mod pb;

#[derive(Debug, Copy, Clone)]
pub enum State {
    Follower,
    Leader,
    Candidate,
    PreCandidate,
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
        match self {
            State::Follower => write!(f, "Follower"),
            State::Leader => write!(f, "Leader"),
            State::Candidate => write!(f, "Candidate"),
            State::PreCandidate => write!(f, "PreCandidate"),
        }
    }
}
