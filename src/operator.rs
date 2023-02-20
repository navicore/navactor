use crate::actor::State;
use std::fmt;
use std::ops::Add;
use time::OffsetDateTime;

pub(crate) type OperatorResult<T> = Result<T, OpError>;

/// Returned when an operator can not return a result.
#[derive(Debug, Clone)]
pub struct OpError {
    pub(crate) reason: String,
}

impl fmt::Display for OpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "unsuccessful operation: {}", self.reason)
    }
}

/// The Operator encapsulates logic that operates on all new incoming data to
/// advance the state of the actor or DT
pub trait Operator<T: Add<Output = T>>: Sync + Send {
    /// Returns a result with a value of type T
    ///
    /// # Arguments
    ///
    /// * `state`       - the current state of the actor
    /// * `idx`         - the index of the state being operated on
    /// * `value`       - the value from outside the actor to be
    ///                 considered and applied to the current state
    /// * `datetime`    - the datetime of the incoming observation
    ///
    /// # Errors
    ///
    /// Returns [`OperatorError`](../genes/struct.OperatorError.html) if the
    /// input is not valid for the operation - usually an invalid
    /// index
    fn apply(state: &State<T>, idx: i32, value: T, datetime: OffsetDateTime) -> OperatorResult<T>;
}

/// The simplest Operator is a gauge.  Every new observation replaces the
/// previously reported one for an index as the new current state for that index
pub struct Gauge {}
impl<T: Add<Output = T>> Operator<T> for Gauge {
    fn apply(_: &State<T>, _: i32, value: T, _: OffsetDateTime) -> OperatorResult<T> {
        Ok(value)
    }
}

/// `AccumOperator` sums all reports.  The current state for an accum index is the
/// sum of all previously reported values.  This works best when the actor state
/// and identity is time-based, like a daily or hourly or monthly scope.  Variations
/// of the accum operator can be the sum of fixed time ranges or last n reports.
pub struct Accumulator {}
impl<T: Add<Output = T> + Copy> Operator<T> for Accumulator {
    fn apply(state: &State<T>, idx: i32, value: T, _: OffsetDateTime) -> OperatorResult<T> {
        state.get(&idx).map_or_else(
            || {
                Err(OpError {
                    reason: String::from("idx invalid"),
                })
            },
            |old_val| {
                let new_val = *old_val + value;
                Ok(new_val)
            },
        )
    }
}
