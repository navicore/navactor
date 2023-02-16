use approx::assert_ulps_eq;
use navactor::actor::State;
use navactor::genes::Gene;
use navactor::genes::GuageAndAccumGene;
use navactor::message::Message;
use test_log::test;
use time::OffsetDateTime;

#[cfg_attr(feature = "cargo-clippy", allow(clippy::unwrap_used))]
#[test]
fn test_default_gene() {
    let mut state: State<f64> = State::new();
    state.insert(0, 1.9);
    state.insert(1, 2.7);
    state.insert(100, 2.91);
    state.insert(199, 3.2);

    let mut values: State<f64> = State::new();
    values.insert(0, 2.9);
    values.insert(199, 4.11);

    let g1 = GuageAndAccumGene {
        ..Default::default()
    };

    let msg = Message::Update {
        path: String::from("/"),
        datetime: OffsetDateTime::now_utc(),
        values,
    };

    let r = g1.apply_operators(state, msg);
    assert!(r.is_ok(), "{r:?}");
    let new_state = r.unwrap();
    // test guage
    assert_ulps_eq!(new_state.get(&0).unwrap(), &2.9, max_ulps = 4);
    assert_ulps_eq!(new_state.get(&1).unwrap(), &2.7, max_ulps = 4);
    // test accumulator
    let tv1: f64 = 2.91;
    assert_ulps_eq!(new_state.get(&100).unwrap(), &tv1, max_ulps = 4);
    assert_ulps_eq!(new_state.get(&199).unwrap(), &7.31, max_ulps = 4);
}
