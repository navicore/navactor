//!A module that provides functions to work with time in the `OffsetDateTime` format. It contains a
//!struct `TimeError` that represents errors that can occur when working with time.

use serde::{Deserialize, Serialize};
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;
use tracing::error;
use tracing::warn;

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

/// extract a datetime from an ISO8601 string
///
/// # Errors
///
/// Returns [`TimeError`](../struct.TimeError.html) if the
/// string can not be parsed into datetime
pub fn extract_datetime(datetime_str: &str) -> TimeResult {
    match OffsetDateTime::parse(datetime_str, &Iso8601::DEFAULT) {
        Ok(d) => Ok(d),
        Err(e) => {
            warn!("can not parse datetime {} due to: {}", datetime_str, e);
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
    /// extract a datetime from a persistable unix timestamp
    ///
    /// # Errors
    ///
    /// Returns [`TimeError`](../struct.TimeError.html) if the
    /// string can not be unmarshalled into a datetime
    pub fn to_ts(&self) -> TimeResult {
        match OffsetDateTime::from_unix_timestamp(self.datetime_num) {
            Ok(ts) => Ok(ts),
            Err(e) => {
                error!("can not get ts: {e}");
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
