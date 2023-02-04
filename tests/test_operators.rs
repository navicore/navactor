use approx::assert_ulps_eq;
use navactor::actor::State;
use navactor::genes::AccumOperator;
use navactor::genes::GuageOperator;
use navactor::genes::Operator;
use test_log::test;
use time::OffsetDateTime;

#[test]
fn test_guage() {
    let mut state: State<f64> = State::new();
    state.insert(0, 1.9);

    let r = GuageOperator::apply(&state, 0, 5.0, OffsetDateTime::now_utc());
    assert_eq!(r.ok(), Some(5.0));
}

#[test]
fn test_accumulator() {
    let mut state: State<f64> = State::new();
    state.insert(0, 1.9);

    let r = AccumOperator::apply(&state, 0, 5.0, OffsetDateTime::now_utc());
    assert_eq!(r.ok(), Some(6.9));
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::unwrap_used))]
#[test]
fn test_accumulator_with_dec() {
    let mut state: State<f64> = State::new();
    state.insert(0, 3.2);

    let r = AccumOperator::apply(&state, 0, 4.11, OffsetDateTime::now_utc());
    assert_ulps_eq!(r.ok().unwrap(), &7.31, max_ulps = 4);
}
