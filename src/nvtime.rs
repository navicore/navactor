use serde::{Deserialize, Serialize};
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

pub type TimeResult = Result<OffsetDateTime, TimeError>;

#[derive(Debug, Clone)]
pub struct TimeError {
    pub reason: String,
}

impl std::fmt::Display for TimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

#[must_use]
pub fn extract_datetime(datetime_str: &str) -> TimeResult {
    match OffsetDateTime::parse(datetime_str, &Iso8601::DEFAULT) {
        Ok(d) => Ok(d),
        Err(e) => {
            log::warn!("can not parse datetime {} due to: {}", datetime_str, e);
            Err(TimeError {
                reason: format!("{e}"),
            })
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct OffsetDateTimeWrapper {
    pub datetime_num: i64,
}

impl OffsetDateTimeWrapper {
    #[must_use]
    pub fn to_ts(&self) -> TimeResult {
        match OffsetDateTime::from_unix_timestamp(self.datetime_num) {
            Ok(ts) => Ok(ts),
            Err(e) => {
                log::error!("can not get ts: {e}");
                Err(TimeError {
                    reason: format!("{e}"),
                })
            }
        }
    }

    #[must_use]
    pub const fn new(timestamp: OffsetDateTime) -> Self {
        Self {
            datetime_num: timestamp.unix_timestamp(),
        }
    }
}
