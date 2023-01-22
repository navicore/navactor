use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Serialize, Deserialize)]
pub struct OffsetDateTimeWrapper {
    pub datetime: OffsetDateTime,
}
