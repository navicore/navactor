use crate::actor::ActorState;
use crate::message::Message;
use std::fmt;
use time::OffsetDateTime;

type OperatorResult<T> = std::result::Result<T, OperatorError>;

#[derive(Debug, Clone)]
pub struct OperatorError {
    reason: String,
}

impl fmt::Display for OperatorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "unsuccessful operation: {}", self.reason)
    }
}

pub trait Operator {
    fn apply(
        state: &ActorState<f64>,
        idx: i32,
        value: f64,
        datetime: OffsetDateTime,
    ) -> OperatorResult<f64>;
}

pub struct GuageOperator {}
impl Operator for GuageOperator {
    fn apply(_: &ActorState<f64>, _: i32, value: f64, _: OffsetDateTime) -> OperatorResult<f64> {
        Ok(value)
    }
}

pub struct AccumOperator {}
impl Operator for AccumOperator {
    fn apply(
        state: &ActorState<f64>,
        idx: i32,
        value: f64,
        _: OffsetDateTime,
    ) -> OperatorResult<f64> {
        if let Some(old_val) = state.get(&idx) {
            Ok(old_val + value)
        } else {
            Err(OperatorError {
                reason: String::from("idx invalid"),
            })
        }
    }
}

pub trait Gene {
    fn apply_operators(
        &self,
        state: ActorState<f64>,
        update: crate::genes::Message,
    ) -> OperatorResult<ActorState<f64>>;
}

pub struct DefaultGene {}
impl Gene for DefaultGene {
    fn apply_operators(
        &self,
        mut state: ActorState<f64>,
        update: Message,
    ) -> OperatorResult<ActorState<f64>> {
        if let Message::Update {
            path: _,
            datetime,
            values,
        } = update
        {
            for &idx in values.keys() {
                if let Some(in_val) = values.get(&idx) {
                    match idx {
                        i if (0..100).contains(&i) => {
                            // this is a guage
                            match GuageOperator::apply(&state, i, *in_val, datetime) {
                                Ok(new_val) => {
                                    state.insert(i, new_val);
                                }
                                Err(e) => return Err(e),
                            }
                        }
                        i if (100..200).contains(&i) => {
                            // this is an accumulator
                            match AccumOperator::apply(&state, i, *in_val, datetime) {
                                Ok(new_val) => {
                                    state.insert(i, new_val);
                                }
                                Err(e) => return Err(e),
                            }
                        }
                        i => {
                            return Err(OperatorError {
                                reason: format!("unsupported idx: {i}"),
                            })
                        }
                    }
                } else {
                    return Err(OperatorError {
                        reason: String::from("cannot read input value"),
                    });
                }
            }
        }
        Ok(state)
    }
}

impl DefaultGene {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for DefaultGene {
    fn default() -> Self {
        Self::new()
    }
}
