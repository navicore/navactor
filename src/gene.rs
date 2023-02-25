//! This module provides a set of abstractions for defining and manipulating genes that are used in
//! a larger system for processing incoming data from `IoT` devices. Specifically, a gene represents
//! a set of configuration information (primarily in the form of operators) that are applied to a
//! path of actors. The gene defines how incoming data is processed and used to update the state of
//! the actors.
//!
//! The module includes several structs and traits that define operators and genes, and provides
//! functionality for applying operators to update messages containing new observations. Two of the
//! most important operators defined in the module are the `GaugeOperator` and `AccumOperator`,
//! which respectively update the current state of an actor with the most recent value of a given
//! index or accumulate the sum of all previously reported values for that index.
//!
//! Additionally, the module defines a custom error type (`OperatorError`) that is returned by
//! operators when an input is not valid, along with a `TimeScope` enum that specifies the time
//! granularity of the gene.

use crate::actor::State;
use crate::message::Message;
use crate::operator::OperatorResult;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Add;

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
    fn apply_operators(&self, state: State<T>, update: Message<T>) -> OperatorResult<State<T>>;
    fn get_time_scope(&self) -> &TimeScope;
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GeneType {
    Accum,
    Gauge,
    GaugeAndAccum,
    Default,
}

impl fmt::Display for GeneType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let display_text = match self {
            Self::Accum => "[Accum Gene]",
            Self::Gauge => "[GaugeGene]",
            Self::GaugeAndAccum => "[GaugeAndAccum Gene]",
            Self::Default => "[Default GaugeGene]",
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
