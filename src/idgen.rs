use chrono::Utc;
use parking_lot::{Mutex, MutexGuard, RawMutex};
use sonyflake::{Error, Sonyflake};
use std::sync::Arc;

pub struct IDGen {
    sf: Sonyflake,
}

impl IDGen {
    pub fn new() -> IDGen {
        IDGen {
            sf: Sonyflake::builder()
                .start_time(Utc::now())
                .finalize()
                .unwrap(),
        }
    }

    /// `next_id` returns the next unique id. After the Sonyflake time overflows, next_id returns an error.
    pub fn next_id(&mut self) -> u64 {
        match self.sf.next_id() {
            Ok(id) => id,
            Err(_) => {
                self.sf = new_sf();
                self.next_id()
            }
        }
    }
}

/// `new_sf` returns a Sonyflake base on current time.
fn new_sf() -> Sonyflake {
    Sonyflake::builder()
        .start_time(Utc::now())
        .finalize()
        .unwrap()
}

#[cfg(test)]
mod test {
    use crate::idgen::IDGen;

    #[test]
    fn test_next_id() {
        let v = IDGen::new().next_id();
        assert!(v > 0);
    }
}
