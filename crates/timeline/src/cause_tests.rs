use super::*;
use science::units::{EarthFlux, Kelvin};

#[test]
fn a_recorded_value_finds_its_node_and_its_sources() {
    let mut trace = Trace::new();
    let flux = trace
        .leaf(Code::of("draw.flux"), EarthFlux(1.0), Provenance::Drawn)
        .unwrap();
    let t = trace
        .record(
            Code::of("climate.equilibrium"),
            Kelvin(255.0),
            &[flux.node()],
        )
        .unwrap();
    let node = trace.node(t.node()).unwrap();
    assert_eq!(
        node.origin,
        Origin::Derived {
            from: vec![flux.node()]
        }
    );
    assert_eq!(*t.value(), Kelvin(255.0));
}

#[test]
fn non_finite_values_are_refused_with_their_code() {
    let mut trace = Trace::new();
    let err = trace
        .record(Code::of("climate.t"), Kelvin(f64::NAN), &[])
        .unwrap_err();
    assert_eq!(
        err,
        ContractError::NotFinite {
            code: "climate.t".into()
        }
    );
    assert!(
        trace
            .leaf(Code::of("x"), Kelvin(f64::INFINITY), Provenance::Input)
            .is_err()
    );
}

#[test]
fn an_id_past_the_end_of_the_trace_is_refused() {
    let mut other = Trace::new();
    let mut trace = Trace::new();
    let far = {
        other
            .leaf(Code::of("a"), Kelvin(1.0), Provenance::Input)
            .unwrap();
        other
            .leaf(Code::of("b"), Kelvin(1.0), Provenance::Input)
            .unwrap()
    };
    assert!(matches!(
        trace.record(Code::of("c"), Kelvin(1.0), &[far.node()]),
        Err(ContractError::UnknownNode { .. })
    ));
}

#[test]
fn codes_are_dotted_lowercase() {
    assert!(Code::new("climate.surface-temperature".into()).is_ok());
    for bad in ["", "Climate", "climate.", ".x", "a b", "a..b", "1a"] {
        assert!(Code::new(bad.into()).is_err(), "{bad}");
    }
}

#[test]
#[should_panic]
fn an_invalid_code_literal_panics() {
    let _ = Code::of("Bad Code");
}

#[test]
fn a_code_is_validated_when_deserialised() {
    assert_eq!(
        serde_json::from_str::<Code>("\"a.b\"").unwrap(),
        Code::of("a.b")
    );
    assert!(serde_json::from_str::<Code>("\"A.b\"").is_err());
}

struct State {
    y: Traced<Kelvin>,
    extra: Vec<Option<Traced<Kelvin>>>,
}

impl TracedFields for State {
    fn for_each_node(&mut self, f: &mut dyn FnMut(&mut NodeId)) {
        self.y.for_each_node(f);
        self.extra.for_each_node(f);
    }
}

fn sample() -> (Trace, State) {
    let mut trace = Trace::new();
    let a = trace
        .leaf(Code::of("a"), Kelvin(1.0), Provenance::Input)
        .unwrap();
    let _u = trace
        .leaf(Code::of("u"), Kelvin(9.0), Provenance::Observed)
        .unwrap();
    let b = trace
        .leaf(Code::of("b"), Kelvin(2.0), Provenance::Imposed)
        .unwrap();
    let x = trace
        .record(Code::of("x"), Kelvin(3.0), &[a.node()])
        .unwrap();
    let y = trace
        .record(Code::of("y"), Kelvin(5.0), &[x.node(), b.node()])
        .unwrap();
    (
        trace,
        State {
            y,
            extra: vec![None],
        },
    )
}

#[test]
fn freezing_keeps_only_the_reachable_nodes_renumbered() {
    let (trace, mut state) = sample();
    let graph = trace.freeze(&mut state).unwrap();
    let codes: Vec<&str> = graph.nodes.iter().map(|n| n.code.as_str()).collect();
    assert_eq!(codes, ["a", "b", "x", "y"]);
    assert_eq!(graph.validate(), Ok(()));
    // y is the last node, and its sources resolve to x (2) and b (1).
    assert_eq!(state.y.node(), NodeId(3));
    assert_eq!(
        graph.node(state.y.node()).unwrap().origin,
        Origin::Derived {
            from: vec![NodeId(2), NodeId(1)]
        }
    );
    assert_eq!(
        graph.node(NodeId(2)).unwrap().origin,
        Origin::Derived {
            from: vec![NodeId(0)]
        }
    );
    assert_eq!(
        graph.node(NodeId(0)).unwrap().origin,
        Origin::Leaf(Provenance::Input)
    );
}

#[test]
fn freezing_an_id_past_the_end_of_the_trace_is_refused() {
    let (_, mut state) = sample();
    let empty = Trace::new();
    assert!(matches!(
        empty.freeze(&mut state),
        Err(ContractError::UnknownNode { .. })
    ));
}

#[test]
fn a_frozen_graph_round_trips_through_json() {
    let (trace, mut state) = sample();
    let graph = trace.freeze(&mut state).unwrap();
    let json = serde_json::to_string(&graph).unwrap();
    let back: CauseGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(back, graph);
    assert_eq!(back.validate(), Ok(()));
}

#[test]
fn validate_reports_a_dangling_source() {
    let json = r#"{"nodes":[
        {"code":"a","value":{"Flag":true},"origin":{"Leaf":"Input"}},
        {"code":"b","value":{"Flag":true},"origin":{"Derived":{"from":[7]}}}]}"#;
    let graph: CauseGraph = serde_json::from_str(json).unwrap();
    assert_eq!(
        graph.validate(),
        Err(ContractError::UnknownNode { node: 7 })
    );
}

#[test]
fn validate_reports_a_forward_source() {
    let json = r#"{"nodes":[
        {"code":"a","value":{"Flag":true},"origin":{"Derived":{"from":[1]}}},
        {"code":"b","value":{"Flag":true},"origin":{"Leaf":"Input"}}]}"#;
    let graph: CauseGraph = serde_json::from_str(json).unwrap();
    assert_eq!(
        graph.validate(),
        Err(ContractError::UnknownNode { node: 1 })
    );
}

#[test]
fn node_values_carry_the_unit_symbol() {
    assert_eq!(
        Kelvin(1.5).node_value(),
        NodeValue::Number {
            value: 1.5,
            unit: "K".into()
        }
    );
    assert_eq!(true.node_value(), NodeValue::Flag(true));
}

#[test]
fn numbers_round_trip_through_json_bit_for_bit() {
    // serde_json's default float parser can be off by one ulp (0.0048
    // came back as 0.0048000000000000004); the workspace enables
    // `float_roundtrip`.
    for x in [
        0.0048_f64,
        4.8e-3 * 0.737_1,
        1.867_5e15,
        0.1 + 0.2,
        6.02e23,
        5e-324,
    ] {
        let back: f64 = serde_json::from_str(&serde_json::to_string(&x).unwrap()).unwrap();
        assert_eq!(back.to_bits(), x.to_bits(), "{x:e}");
    }
}
