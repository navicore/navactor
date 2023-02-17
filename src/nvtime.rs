use serde::{Deserialize, Serialize};
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

#[must_use]
pub fn extract_datetime(datetime_str: &str) -> OffsetDateTime {
    match OffsetDateTime::parse(datetime_str, &Iso8601::DEFAULT) {
        Ok(d) => d,
        Err(e) => {
            log::warn!("can not parse datetime {} due to: {}", datetime_str, e);
            // TODO: TimeError
            OffsetDateTime::now_utc()
        }
    }
}

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
                // TODO: TimeError
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
