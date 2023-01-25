use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Serialize, Deserialize)]
pub struct OffsetDateTimeWrapper {
    pub datetime_i64: i64,
}

impl OffsetDateTimeWrapper {
    pub fn to_ts(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.datetime_i64).unwrap()
    }

    pub fn new(timestamp: OffsetDateTime) -> Self {
        Self {
            datetime_i64: timestamp.unix_timestamp(),
        }
    }
}
