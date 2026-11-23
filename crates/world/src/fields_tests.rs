use super::*;
use timeline::SystemNumber;

fn world() -> World {
    let body = Designation::star(SystemNumber::new(12_345).unwrap());
    World::new(Grid::new(2), body, 0, EngineVersion(1), EngineVersion(1))
}

fn id(text: &'static str) -> FieldId {
    FieldId(Code::of(text))
}

fn floats(len: usize) -> Field {
    Field {
        values: Values::F32(vec![0.0; len]),
        categorical: false,
        storage: Storage::Exact,
    }
}

#[test]
fn a_field_of_the_wrong_length_is_refused() {
    let mut w = world();
    assert_eq!(w.grid().cell_count(), 24);
    let err = w.insert(id("relief.height"), floats(23)).unwrap_err();
    assert!(matches!(
        err,
        WorldError::WrongLength {
            expected: 24,
            found: 23,
            ..
        }
    ));
    assert!(w.field(&id("relief.height")).is_none());
}

#[test]
fn a_field_is_inserted_once() {
    let mut w = world();
    w.insert(id("relief.height"), floats(24)).unwrap();
    let err = w.insert(id("relief.height"), floats(24)).unwrap_err();
    assert_eq!(err, WorldError::Duplicate(Code::of("relief.height")));
}

#[test]
fn a_categorical_field_is_integer() {
    let mut w = world();
    let mut f = floats(24);
    f.categorical = true;
    let err = w.insert(id("biome.kind"), f).unwrap_err();
    assert_eq!(err, WorldError::Categorical(Code::of("biome.kind")));
    let ok = Field {
        values: Values::U8(vec![3; 24]),
        categorical: true,
        storage: Storage::Exact,
    };
    w.insert(id("biome.kind"), ok).unwrap();
    assert_eq!(w.field(&id("biome.kind")).unwrap().values.get_f32(5), 3.0);
}

#[test]
fn u16_storage_needs_a_range() {
    let mut w = world();
    let mut f = floats(24);
    f.storage = Storage::U16 { min: 5.0, max: 5.0 };
    let err = w.insert(id("relief.height"), f).unwrap_err();
    assert_eq!(err, WorldError::Storage(Code::of("relief.height")));
    let int = Field {
        values: Values::U16(vec![0; 24]),
        categorical: false,
        storage: Storage::U16 { min: 0.0, max: 1.0 },
    };
    let err = w.insert(id("relief.index"), int).unwrap_err();
    assert_eq!(err, WorldError::Storage(Code::of("relief.index")));
    let mut good = floats(24);
    good.storage = Storage::U16 {
        min: -1.0,
        max: 1.0,
    };
    w.insert(id("relief.height"), good).unwrap();
}

#[test]
fn fields_iterate_in_code_order() {
    let mut w = world();
    w.insert(id("relief.height"), floats(24)).unwrap();
    w.insert(id("climate.temperature"), floats(24)).unwrap();
    let codes: Vec<&str> = w.fields().map(|(i, _)| i.0.as_str()).collect();
    assert_eq!(codes, ["climate.temperature", "relief.height"]);
}

#[test]
fn u16_storage_refuses_a_non_finite_value() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut w = world();
        let mut values = vec![0.5; 24];
        values[3] = bad;
        let field = Field {
            values: Values::F32(values),
            categorical: false,
            storage: Storage::U16 { min: 0.0, max: 1.0 },
        };
        assert_eq!(
            w.insert(id("climate.wetness"), field).unwrap_err(),
            WorldError::Storage(Code::of("climate.wetness")),
            "{bad}"
        );
    }
}
