//! This module provides a set of abstractions for defining and using operators in the larger
//! system for processing incoming data from `IoT` devices. The module defines a custom error type
//! (`OpError`) that is returned by operators when an input is not valid, and uses the `time`
//! crate to work with dates and times.
//!
//! The module exports the `Operator` trait, which defines the interface for all operators used in
//! the system, and includes two structs (`Gauge` and `Accumulator`) that implement the `Operator`
//! trait. The `Gauge` operator updates the current state of an actor with the most recent value of
//! a given index, while the `Accumulator` operator accumulates the sum of all previously reported
//! values for that index. The `Operator` trait also defines a `apply` method that applies an
//! operator to an actor's current state to produce a new state, along with a custom error type
//! (`OperatorError`) that is returned when an input is not valid for the operation, usually an
//! invalid index.
//!
//! The `OperatorResult` type is also defined in the module, which is used as the result type for
//! all `apply` methods of operators, and the `Gauge` and `Accumulator` structs implement the
//! `Operator` trait using this type. The `OperatorResult` is a type alias for a `Result` with a
//! value of type `T` and an error of type `OpError`. The module also defines the `State` type,
//! which is used as the state of actors in the system and is passed as an argument to the `apply`
//! method of all operators.
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
                // init val - idx does not exist yet
                Ok(value)
            },
            |old_val| {
                let new_val = *old_val + value;
                Ok(new_val)
            },
        )
    }
}
