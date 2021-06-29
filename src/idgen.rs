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
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_next_id() {
        let mut g = IDGen::new();
        let v = g.next_id();
        println!("{}", v);
        assert!(v > 0);

        std::thread::sleep(Duration::from_millis(100));
        let v1 = g.next_id();
        println!("{}", v);
        assert!(v1 > 0);

        std::thread::sleep(Duration::from_millis(100));
        let v2 = new_sf().next_id().unwrap();
        println!("{}", v2);
        assert!(v2 > 0)
    }
}
