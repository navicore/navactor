use crate::actor::State;
use crate::message::Message;
use std::fmt;
use std::ops::Add;
use time::OffsetDateTime;

type OperatorResult<T> = Result<T, OperatorError>;

/// Returned when an operator can not return a result.
#[derive(Debug, Clone)]
pub struct OperatorError {
    reason: String,
}

impl fmt::Display for OperatorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "unsuccessful operation: {}", self.reason)
    }
}

//TODO: generics
/// The Operator encapsulates logic that operates on all new incoming data to
/// advance the state of the actor or DT
pub trait Operator<T: Add<Output = T>>: Sync + Send {
    /// Returns a result with a value of type i64
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
pub struct GuageOperator {}
impl<T: Add<Output = T>> Operator<T> for GuageOperator {
    fn apply(_: &State<T>, _: i32, value: T, _: OffsetDateTime) -> OperatorResult<T> {
        Ok(value)
    }
}

/// `AccumOperator` sums all reports.  The current state for an accum index is the
/// sum of all previously reported values.  This works best when the actor state
/// and identity is time-based, like a daily or hourly or monthly scope.  Variations
/// of the accum operator can be the sum of fixed time ranges or last n reports.
pub struct AccumOperator {}
impl<T: Add<Output = T> + Copy> Operator<T> for AccumOperator {
    fn apply(state: &State<T>, idx: i32, value: T, _: OffsetDateTime) -> OperatorResult<T> {
        state.get(&idx).map_or_else(
            || {
                Err(OperatorError {
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

/// A Gene is a collection of config information - mostly operators - that
/// are applied to a path of actors.  A gene may apply to
/// `/org/location/floor` where all reports for a floor have the some gene
/// or could be `/make/model` or for an individual machine like
/// `/devices/12345`.
pub trait Gene<T: Add<Output = T>> {
    /// Applying all operators for a update message - many new observations
    /// arrive bundled together in single packages of update messages.  This
    /// function is a convenience function for the Operator apply function
    /// for individual indexes.
    ///
    /// # Errors
    ///
    /// Returns [`OperatorError`](../genes/struct.OperatorError.html) if the
    /// input is not valid for the operation - usually an invalid
    /// index
    fn apply_operators(
        &self,
        state: State<T>,
        update: Message<T>,
    ) -> OperatorResult<State<T>>;
    fn get_time_scope(&self) -> &TimeScope;
}

/// the most basic common state are either guages or accumulators.  This
/// gene allows both of those operators to be applied to ranges of indexes.
pub struct GuageAndAccumGene {
    pub guage_first_idx: i32,
    pub guage_slots: i32,
    pub accumulator_first_idx: i32,
    pub accumulator_slots: i32,
    pub time_scope: TimeScope,
}

impl GuageAndAccumGene {
    fn update_state_with_val<T: Add<Output = T> + Copy>(
        &self,
        in_val: T,
        idx: i32,
        mut state: State<T>,
        datetime: OffsetDateTime,
    ) -> OperatorResult<State<T>> {
        let new_val = if (self.guage_first_idx..self.guage_first_idx + self.guage_slots)
            .contains(&idx)
        {
            // this is a guage
            GuageOperator::apply(&state, idx, in_val, datetime)?
        } else if (self.accumulator_first_idx..self.accumulator_first_idx + self.accumulator_slots)
            .contains(&idx)
        {
            // this is an accumulator
            AccumOperator::apply(&state, idx, in_val, datetime)?
        } else {
            return Err(OperatorError {
                reason: format!("unsupported idx: {idx}"),
            });
        };

        state.insert(idx, new_val);
        Ok(state)
    }
}

impl<T: Add<Output = T> + Copy> Gene<T> for GuageAndAccumGene {
    fn apply_operators(&self, mut state: State<T>, update: Message<T>) -> OperatorResult<State<T>> {
        match update {
            Message::Update {
                path: _,
                datetime,
                values,
            } => {
                for &idx in values.keys() {
                    let in_val = values.get(&idx).ok_or_else(|| OperatorError {
                        reason: format!("unsupported idx: {idx}"),
                    })?;
                    state = self.update_state_with_val(*in_val, idx, state, datetime)?;
                }
            }
            _ => {
                return Err(OperatorError {
                    reason: "unsupported message type".to_string(),
                })
            }
        };
        Ok(state)
    }
    fn get_time_scope(&self) -> &TimeScope {
        &self.time_scope
    }
}

impl Default for GuageAndAccumGene {
    fn default() -> Self {
        Self {
            guage_first_idx: 0,
            guage_slots: 100,
            accumulator_first_idx: 100,
            accumulator_slots: 100,
            time_scope: TimeScope::Forever,
        }
    }
}

#[derive(Debug, Clone)]
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
