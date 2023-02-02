use approx::assert_ulps_eq;
use navactor::actor::ActorState;
use navactor::genes::DefaultGene;
use navactor::genes::Gene;
use navactor::message::Message;
use test_log::test;
use time::OffsetDateTime;

#[test]
fn test_default_gene() {
    let mut state: ActorState<f64> = ActorState::new();
    state.insert(0, 1.9);
    state.insert(1, 2.7);
    state.insert(100, 2.91);
    state.insert(199, 3.2);

    let mut values: ActorState<f64> = ActorState::new();
    values.insert(0, 2.9);
    values.insert(199, 4.11);

    let g1 = DefaultGene::new();

    let msg = Message::Update {
        path: String::from("/"),
        datetime: OffsetDateTime::now_utc(),
        values,
    };

    let r = g1.apply_operators(state, msg);
    assert!(r.is_ok(), "{r:?}");
    let new_state = r.unwrap();
    // test guage
    assert_eq!(new_state.get(&0).unwrap(), &2.9);
    assert_eq!(new_state.get(&1).unwrap(), &2.7);
    // test accumulator
    assert_eq!(new_state.get(&100).unwrap(), &2.91);
    assert_ulps_eq!(new_state.get(&199).unwrap(), &7.31, max_ulps = 4);
}
