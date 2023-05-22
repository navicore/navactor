//! This module provides the implementation for the `AccumGene` struct, which represents a gene that
//! primarily uses the `Accumulator` operator to update the state of actors based on incoming data
//! from `IoT` devices. The module implements the `Gene` trait to define how to apply the gene's
//! operators to update an actor's state and to provide the gene's time scope. The module also
//! defines a custom error type (`OpError`) that is returned by the `Accumulator` operator when an
//! input is not valid, and uses the `time` crate to work with dates and times. Additionally, the
//! `AccumGene` struct includes a default implementation that sets the gene's time scope to
//! `Forever` and its base time to the current UTC time.
//!
//! The module exports the `AccumGene` struct, which can be used in the larger system for processing
//! incoming data from `IoT` devices.
use crate::actor::State;
use crate::gene::Gene;
use crate::gene::TimeScope;
use crate::message::Message;
use crate::operator::{Accumulator, OpError, Operator, OperatorResult};
use std::ops::Add;
use time::OffsetDateTime;
use tracing::trace;

pub struct AccumGene {
    pub time_scope: TimeScope,
    pub base_time: OffsetDateTime,
}

fn update_state_with_val<T: Add<Output = T> + Copy>(
    in_val: T,
    idx: i32,
    mut state: State<T>,
    datetime: OffsetDateTime,
) -> OperatorResult<State<T>> {
    let new_val = Accumulator::apply(&state, idx, in_val, datetime)?;
    state.insert(idx, new_val);
    Ok(state)
}

impl<T: Add<Output = T> + Copy> Gene<T> for AccumGene {
    fn apply_operators(&self, mut state: State<T>, update: Message<T>) -> OperatorResult<State<T>> {
        match update {
            Message::Observations {
                path: _,
                datetime,
                values,
            } => {
                for &idx in values.keys() {
                    let in_val = *values.get(&idx).ok_or_else(|| OpError {
                        reason: format!("unsupported idx: {idx}"),
                    })?;
                    let len = state.keys().len();
                    trace!("updating key {idx} of keys {len}");
                    state = update_state_with_val(in_val, idx, state, datetime)?;
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

impl Default for AccumGene {
    fn default() -> Self {
        Self {
            time_scope: TimeScope::Forever,
            base_time: OffsetDateTime::now_utc(),
        }
    }
}
