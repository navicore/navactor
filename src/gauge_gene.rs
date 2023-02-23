use crate::actor::State;
use crate::gene::Gene;
use crate::gene::TimeScope;
use crate::message::Message;
use crate::operator::{Gauge, OpError, Operator, OperatorResult};
use std::ops::Add;
use time::OffsetDateTime;

pub struct GaugeGene {
    pub time_scope: TimeScope,
    pub base_time: OffsetDateTime,
}

fn update_state_with_val<T: Add<Output = T>>(
    in_val: T,
    idx: i32,
    mut state: State<T>,
    datetime: OffsetDateTime,
) -> OperatorResult<State<T>> {
    let new_val = Gauge::apply(&state, idx, in_val, datetime)?;
    state.insert(idx, new_val);
    Ok(state)
}

//pub trait Gene<T: Add<Output = T>> {
impl<T: Add<Output = T> + Copy> Gene<T> for GaugeGene {
    fn apply_operators(&self, mut state: State<T>, update: Message<T>) -> OperatorResult<State<T>> {
        match update {
            Message::Update {
                path: _,
                datetime,
                values,
            } => {
                for &idx in values.keys() {
                    let in_val = values.get(&idx).ok_or_else(|| OpError {
                        reason: format!("unsupported idx: {idx}"),
                    })?;
                    state = update_state_with_val(*in_val, idx, state, datetime)?;
                }
            }
            _ => {
                return Err(OpError {
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

impl Default for GaugeGene {
    fn default() -> Self {
        Self {
            time_scope: TimeScope::Forever,
            base_time: OffsetDateTime::now_utc(),
        }
    }
}
