use rand::prelude::*;
use std::{
    sync::atomic::{AtomicU16, AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

pub trait IdGen<T> {
    fn next(&mut self) -> T;
}

pub struct TimestampRandIdGen {
    rand_seed_cnt: u16,
    timestamp_cnt: u64,
}

impl IdGen<u64> for TimestampRandIdGen {
    fn next(&mut self) -> u64 {
        self.rand_seed_cnt += 1;
        self.timestamp_cnt = u64::max(unix_timestamp(), self.timestamp_cnt + 1);

        let random = rand::rng().random::<u16>();
        self.timestamp_cnt << 24
            | (((self.rand_seed_cnt & 0x0FFF_u16) as u64) << 12)
            | ((random & 0x0FFF_u16) as u64)
    }
}

impl TimestampRandIdGen {
    pub fn new() -> Self {
        Self {
            rand_seed_cnt: rand::rng().random::<u16>(),
            timestamp_cnt: unix_timestamp(),
        }
    }
}

#[allow(dead_code)]
pub struct AtomicTimestampRandIdGen {
    rand_seed_cnt: AtomicU16,
    timestamp_cnt: AtomicU64,
}

#[allow(dead_code)]
impl AtomicTimestampRandIdGen {
    pub fn new() -> AtomicTimestampRandIdGen {
        AtomicTimestampRandIdGen {
            rand_seed_cnt: AtomicU16::new(rand::rng().random::<u16>()),
            timestamp_cnt: AtomicU64::new(unix_timestamp()),
        }
    }
}

impl IdGen<u64> for AtomicTimestampRandIdGen {
    fn next(&mut self) -> u64 {
        let rand_seed_cnt = self.rand_seed_cnt.fetch_add(1, Ordering::Relaxed);
        let timestamp = self
            .timestamp_cnt
            .fetch_max(unix_timestamp(), Ordering::Relaxed);
        let random = rand::rng().random::<u16>();
        timestamp << 24
            | (((rand_seed_cnt & 0x0FFF_u16) as u64) << 12)
            | ((random & 0x0FFF_u16) as u64)
    }
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
