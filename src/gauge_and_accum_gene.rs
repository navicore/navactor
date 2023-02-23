use crate::actor::State;
use crate::gene::Gene;
use crate::gene::TimeScope;
use crate::message::Message;
use crate::operator::{Accumulator, Gauge, OpError, Operator, OperatorResult};
use std::ops::Add;
use time::OffsetDateTime;

/// the most basic common state are either gauges or accumulators.  This
/// gene allows both of those operators to be applied to ranges of indexes.
pub struct GaugeAndAccumGene {
    pub guage_first_idx: i32,
    pub guage_slots: i32,
    pub accumulator_first_idx: i32,
    pub accumulator_slots: i32,
    pub time_scope: TimeScope,
    pub base_time: OffsetDateTime,
}

impl GaugeAndAccumGene {
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
            Gauge::apply(&state, idx, in_val, datetime)?
        } else if (self.accumulator_first_idx..self.accumulator_first_idx + self.accumulator_slots)
            .contains(&idx)
        {
            // this is an accumulator
            Accumulator::apply(&state, idx, in_val, datetime)?
        } else {
            return Err(OpError {
                reason: format!("unsupported idx: {idx}"),
            });
        };

        state.insert(idx, new_val);
        Ok(state)
    }
}

impl<T: Add<Output = T> + Copy> Gene<T> for GaugeAndAccumGene {
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
                    state = self.update_state_with_val(*in_val, idx, state, datetime)?;
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

impl Default for GaugeAndAccumGene {
    fn default() -> Self {
        Self {
            guage_first_idx: 0,
            guage_slots: 100,
            accumulator_first_idx: 100,
            accumulator_slots: 100,
            time_scope: TimeScope::Forever,
            base_time: OffsetDateTime::now_utc(),
        }
    }
}
