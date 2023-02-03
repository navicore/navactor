use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Serialize, Deserialize)]
pub struct OffsetDateTimeWrapper {
    pub datetime_i64: i64,
}

impl OffsetDateTimeWrapper {
    #[must_use]
    pub fn to_ts(&self) -> OffsetDateTime {
        match OffsetDateTime::from_unix_timestamp(self.datetime_i64) {
            Ok(ts) => ts,
            Err(e) => {
                log::error!("can not get ts: {e}");
                OffsetDateTime::now_utc()
            }
        }
    }

    #[must_use]
    pub const fn new(timestamp: OffsetDateTime) -> Self {
        Self {
            datetime_i64: timestamp.unix_timestamp(),
        }
    }
}
