use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GeneType {
    Accum,
    Gauge,
    Default,
}
impl fmt::Display for GeneType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let display_text = match self {
            Self::Accum => "[Accum Gene]",
            _ => "[Gauge Gene]",
        };
        write!(f, "{display_text}")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TimeScope {
    Forever,
    Year,
    Month,
    Day,
    HalfDay,
    QuarterDay,
    Hour,
    QuarterHour,
    TenMinutes,
    Minute,
}
impl fmt::Display for TimeScope {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let display_text = match self {
            Self::Forever => "[Forever Time Scope]",
            Self::Year => "[Year Time Scope]",
            Self::Month => "[Month Time Scope]",
            Self::Day => "[Day Time Scope]",
            Self::HalfDay => "[Half Day Time Scope]",
            Self::QuarterDay => "[Quarter Day Time Scope]",
            Self::Hour => "[Hour Time Scope]",
            Self::TenMinutes => "[Ten Minute Time Scope]",
            _ => "[Minute Time Scope]",
        };
        write!(f, "{display_text}")
    }
}
