//! Export of a world and its PNG, in temporary directories only.

use std::fs;

use adapters::{ExportError, Ramp, decode, export};
use tempfile::TempDir;
use timeline::{Code, Designation, EngineVersion, SystemNumber};
use world::{Field, FieldId, Grid, Storage, Values, World};

fn id(code: &str) -> FieldId {
    FieldId(Code::new(code.to_string()).unwrap())
}

fn sample() -> World {
    let grid = Grid::new(4);
    let cells = grid.cell_count();
    let body = Designation::planet(SystemNumber::new(10_001).unwrap(), 1).unwrap();
    let mut world = World::new(grid, body, 3, EngineVersion(1), EngineVersion(1));
    let heights = (0..cells)
        .map(|k| if k < cells / 2 { -1000.0 } else { 1000.0 })
        .collect();
    let field = Field {
        values: Values::F32(heights),
        categorical: false,
        storage: Storage::Exact,
    };
    world.insert(id("relief.height"), field).unwrap();
    world
}

#[test]
fn writes_the_world_and_the_png() {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("out");
    let world = sample();
    let exported = export(&out, &world, &id("relief.height"), 16, &Ramp::relief()).unwrap();
    assert_eq!(
        exported.world.file_name().unwrap().to_str().unwrap(),
        "MGS-10001-c-3.world"
    );
    assert_eq!(
        exported.png.file_name().unwrap().to_str().unwrap(),
        "MGS-10001-c-3-relief.height.png"
    );
    assert_eq!(decode(&fs::read(&exported.world).unwrap()).unwrap(), world);
    let decoder = png::Decoder::new(std::io::Cursor::new(fs::read(&exported.png).unwrap()));
    let reader = decoder.read_info().unwrap();
    let info = reader.info();
    assert_eq!((info.width, info.height), (16, 8));
}

#[test]
fn a_missing_field_creates_nothing() {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("out");
    let result = export(&out, &sample(), &id("relief.nope"), 16, &Ramp::relief());
    assert!(matches!(result, Err(ExportError::World(_))));
    assert!(!out.exists());
}

#[test]
fn the_relief_ramp_is_blue_under_water_and_green_on_land() {
    let ramp = Ramp::relief();
    let [r, g, b] = ramp.colour(-1.0);
    assert!(b > r && b > g);
    let [r, g, b] = ramp.colour(1.0);
    assert!(g > r && g > b);
}
