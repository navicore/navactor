use approx::assert_ulps_eq;
use navactor::accum_gene::AccumGene;
use navactor::actor::State;
use navactor::gene::Gene;
use navactor::message::Message;
use time::OffsetDateTime;

#[cfg_attr(feature = "cargo-clippy", allow(clippy::unwrap_used))]
#[test]
fn test_accum_gene() {
    let mut state: State<f64> = State::new();
    state.insert(0, 1.9);
    state.insert(1, 2.7);
    state.insert(100, 2.91);
    state.insert(199, 3.2);

    let mut values: State<f64> = State::new();
    values.insert(0, 2.9);
    values.insert(199, 4.11);

    let g1 = AccumGene {
        ..Default::default()
    };

    let msg = Message::Observations {
        path: String::from("/"),
        datetime: OffsetDateTime::now_utc(),
        values,
    };

    let r = g1.apply_operators(state, msg);
    assert!(r.is_ok(), "{r:?}");
    let new_state = r.unwrap();

    // accum is sum: 1.9 + 2.9 == 4.8
    assert_ulps_eq!(new_state.get(&0).unwrap(), &4.8, max_ulps = 4);
    // accum is sum: 2.7 is untouched because there is no '1' idx entry in values
    assert_ulps_eq!(new_state.get(&1).unwrap(), &2.7, max_ulps = 4);

    // in accum gene, every idx is summed with new values
    let tv1: f64 = 2.91;
    // accum is sum: 2.91 is untouched because there is no '100' idx entry in values
    assert_ulps_eq!(new_state.get(&100).unwrap(), &tv1, max_ulps = 4);
    // accum is sum: 3.2 + 4.11 == 7.3.1
    assert_ulps_eq!(new_state.get(&199).unwrap(), &7.31, max_ulps = 4);
}
