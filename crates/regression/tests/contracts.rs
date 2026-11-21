//! Contracts between the engines, asserted on the shared reference systems.
//! Placeholder until the reference systems land with slice 1, PR 2.

#[test]
fn workspace_builds() {
    // Links every engine, so tier 2 has a test to run and a broken
    // dependency graph fails here before PR 2 adds the real contracts.
    let _ = (regression::REFERENCE_SEEDS,);
}

#[test]
fn the_system_id_of_a_fixed_input_is_pinned() {
    // A regression net, not a source: it pins the current output. A
    // change means every stored system id changes, which orphans the
    // stored data and its caches: decide it on purpose.
    use system::{ArchetypeId, BodyRequest, INPUTS_SCHEMA, Inputs, Request};
    use timeline::{Code, Seed};

    let mut value = serde_json::to_value(Inputs {
        schema: INPUTS_SCHEMA,
        seed: Seed::new(7),
        present: None,
        request: Request::Body(BodyRequest {
            host_mass: None,
            metallicity: None,
            archetype: Some(ArchetypeId(Code::of("earth-like"))),
        }),
    })
    .unwrap();
    value["present"] = serde_json::json!(4.57);
    let inputs: Inputs = serde_json::from_value(value).unwrap();
    assert_eq!(
        catalogue::SystemId::of(&inputs).to_string(),
        "528d67d35f485f94"
    );
}
