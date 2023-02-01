use navactor::actor::ActorState;
use navactor::genes::AccumOperator;
use navactor::genes::GuageOperator;
use navactor::genes::Operator;
use test_log::test;
use time::OffsetDateTime;

#[test]
fn test_guage() {
    let mut state: ActorState<f64> = ActorState::new();
    state.insert(0, 1.9);

    let r = GuageOperator::apply(&state, 0, 5.0, OffsetDateTime::now_utc());
    assert_eq!(r.ok(), Some(5.0));
}

#[test]
fn test_accumulator() {
    let mut state: ActorState<f64> = ActorState::new();
    state.insert(0, 1.9);

    let r = AccumOperator::apply(&state, 0, 5.0, OffsetDateTime::now_utc());
    assert_eq!(r.ok(), Some(6.9));
}

#[test]
fn test_accumulator_with_dec() {
    let mut state: ActorState<f64> = ActorState::new();
    state.insert(0, 1.91);

    let r = AccumOperator::apply(&state, 0, 5.0, OffsetDateTime::now_utc());
    assert_eq!(r.ok(), Some(6.91));
}
