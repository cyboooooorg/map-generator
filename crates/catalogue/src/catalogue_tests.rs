use super::*;
use system::{ArchetypeId, BodyRequest, INPUTS_SCHEMA, Inputs, Request};
use timeline::{Code, EngineVersion, Seed, fnv1a64};

/// The seed-7 earth-like inputs of the system tests, `present` given as
/// JSON so that the catalogue needs no unit type.
fn inputs(seed: u64, present: &str, archetype: &'static str) -> Inputs {
    let mut value = serde_json::to_value(Inputs {
        schema: INPUTS_SCHEMA,
        seed: Seed::new(seed),
        present: None,
        request: Request::Body(BodyRequest {
            host_mass: None,
            metallicity: None,
            archetype: Some(ArchetypeId(Code::of(archetype))),
        }),
    })
    .unwrap();
    value["present"] = serde_json::from_str(present).unwrap();
    serde_json::from_value(value).unwrap()
}

#[test]
fn the_id_is_stable() {
    let inputs = inputs(7, "4.57", "earth-like");
    let json = serde_json::to_string(&inputs).unwrap();
    let id = SystemId::of(&inputs);
    assert_eq!(id.to_string(), format!("{:016x}", fnv1a64(json.as_bytes())));
    assert_eq!(id, SystemId::of(&inputs));
}

#[test]
fn any_input_change_changes_the_id() {
    let base = SystemId::of(&inputs(7, "4.57", "earth-like"));
    let seed = SystemId::of(&inputs(8, "4.57", "earth-like"));
    let present = SystemId::of(&inputs(7, "4.5", "earth-like"));
    let archetype = SystemId::of(&inputs(7, "4.57", "ocean-world"));
    let all = [base, seed, present, archetype];
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

#[test]
fn a_drawn_present_differs_from_a_fixed_one() {
    let drawn = SystemId::of(&inputs(7, "null", "earth-like"));
    let fixed = SystemId::of(&inputs(7, "4.57", "earth-like"));
    assert_ne!(drawn, fixed);
}

#[test]
fn the_id_round_trips_as_text() {
    let id = SystemId::of(&inputs(7, "4.57", "earth-like"));
    let text = id.to_string();
    assert_eq!(text.len(), 16);
    assert_eq!(text.parse::<SystemId>().unwrap(), id);
    assert_eq!(
        serde_json::from_str::<SystemId>(&serde_json::to_string(&id).unwrap()).unwrap(),
        id
    );
    assert!(matches!(
        "xyz".parse::<SystemId>(),
        Err(CatalogueError::BadId(_))
    ));
    assert!(matches!(
        "0123456789abcde".parse::<SystemId>(),
        Err(CatalogueError::BadId(_))
    ));
}

#[test]
fn an_unknown_schema_is_refused() {
    let mut stored = StoredSystem::new(inputs(7, "4.57", "earth-like"), EngineVersion(1));
    assert_eq!(stored.check_schema(), Ok(()));
    stored.inputs.schema = INPUTS_SCHEMA + 1;
    assert_eq!(
        stored.check_schema(),
        Err(CatalogueError::UnknownSchema {
            found: INPUTS_SCHEMA + 1,
            expected: INPUTS_SCHEMA
        })
    );
}
