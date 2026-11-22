//! Tier 3: PNG previews of the relief of Earth, Mars and Venus at 60 km
//! cells, written under `target/relief-previews/<name>/` for the eye.

use adapters::{Ramp, export};
use science::units::Metre;
use timeline::{Designation, PlanetState, Seed, SystemNumber, Trace};
use world::fixtures::{earth_relief_fixture, mars_relief_fixture, venus_relief_fixture};
use world::relief::{HEIGHT, generate};
use world::{FieldId, Grid, World};

const CELL: Metre = Metre(60e3);
const WIDTH: u32 = 2048;

fn preview(name: &str, rank: u32, state: &PlanetState) {
    let grid = Grid::for_radius(Metre::from(*state.bulk.radius.value()), CELL);
    let field = generate(state, Seed::new(1), &grid).unwrap();
    let body = Designation::planet(SystemNumber::new(10_001).unwrap(), rank).unwrap();
    let mut world = World::new(grid, body, 0, system::ENGINE_VERSION, world::ENGINE_VERSION);
    let id = FieldId(timeline::Code::new(HEIGHT.to_string()).unwrap());
    world.insert(id.clone(), field).unwrap();
    let out = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target"))
        .join("relief-previews")
        .join(name);
    let exported = export(&out, &world, &id, WIDTH, &Ramp::relief()).unwrap();
    assert!(
        exported.png.exists(),
        "{name}: no PNG at {:?}",
        exported.png
    );
    eprintln!("{name}: {}", exported.png.display());
}

#[test]
#[ignore = "heavy: three 60 km worlds written as PNG previews"]
fn relief_previews() {
    let mut trace = Trace::new();
    preview("earth", 1, &earth_relief_fixture(&mut trace));
    preview("mars", 2, &mars_relief_fixture(&mut trace, None));
    preview("venus", 3, &venus_relief_fixture(&mut trace));
}
