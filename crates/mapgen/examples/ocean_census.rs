//! Measures the ice shells and oceans of icy moons and planets.
//!
//! ```text
//! cargo run --release -p mapgen --example ocean_census -- [SEEDS=20] [SYSTEMS=300]
//! ```
//!
//! Prints, for each icy moon archetype asked alone in a whole system
//! (seeds 1 to `SEEDS`), then for the icy moons and icy planets of
//! `SYSTEMS` drawn systems (seeds 1 to `SYSTEMS`), the min, median and max
//! of every column of [`COLUMNS`]; then the plume rings of the drawn
//! systems. A column whose field the model does not hold yet prints `-`.
//!
//! Climate pass lot 3 (part A) measured the oceans with it before and
//! after deriving the shell from the heat flux, the tides and the polar
//! shell; a later task adds a column in one place, [`COLUMNS`].
use std::collections::BTreeMap;

use mapgen::planet::PlanetModel;
use mapgen::rings::DustKind;
use mapgen::system::{Body, BodyKind, System, SystemRequest, WholeSystemRequest, draw};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// The icy moon archetypes asked alone.
const ARCHETYPES: [&str; 6] = [
    "enceladus-like",
    "europa-like",
    "ganymede-like",
    "titan-like",
    "callisto-like",
    "io-like",
];

/// A column of the tables: its name and what it reads off a body's model,
/// `None` where the model does not hold it (printed `-`).
struct Column {
    name: &'static str,
    read: fn(&PlanetModel) -> Option<f64>,
}

/// Every column, in print order. Add a field of the model here and the
/// tables show it.
const COLUMNS: [Column; 7] = [
    Column {
        name: "shell km",
        read: |m| m.ice_shell.map(|s| s.shell_km),
    },
    Column {
        name: "polar shell km",
        read: |_| None,
    },
    Column {
        name: "ocean km",
        read: |m| m.ice_shell.map(|s| s.ocean_km),
    },
    Column {
        name: "tidal W",
        read: |m| Some(m.heat.tidal_w),
    },
    Column {
        name: "moment of inertia",
        read: |m| Some(m.moment_of_inertia),
    },
    Column {
        name: "convecting share",
        read: |_| None,
    },
    Column {
        name: "geyser share",
        read: |_| None,
    },
];

fn whole(seed: u32, archetype: Option<&str>) -> System {
    let request = SystemRequest::System(WholeSystemRequest {
        metallicity: Some(0.0),
        star: archetype.map(|_| "G2V".into()),
        planets: archetype.map(|a| BTreeMap::from([(a.to_string(), 1)])),
        seed: Some(seed),
        width: Some(64),
        height: Some(32),
        ..Default::default()
    });
    draw(&request, &mut StdRng::seed_from_u64(u64::from(seed))).expect("a drawn system")
}

/// `min / median / max` of the values, `-` when there are none.
fn spread(mut v: Vec<f64>) -> String {
    if v.is_empty() {
        return "-".into();
    }
    v.sort_by(f64::total_cmp);
    let n = v.len();
    let med = if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    };
    format!("{} / {} / {}", num(v[0]), num(med), num(v[n - 1]))
}

fn num(x: f64) -> String {
    if x != 0.0 && (x.abs() >= 1e5 || x.abs() < 1e-2) {
        format!("{x:.3e}")
    } else {
        format!("{x:.2}")
    }
}

/// One table: a row per column, `min / median / max` over the bodies.
fn table(title: &str, bodies: &[&Body]) {
    println!("\n== {title} ({} bodies)", bodies.len());
    for c in &COLUMNS {
        let values: Vec<f64> = bodies
            .iter()
            .filter_map(|b| b.model.as_ref().and_then(c.read))
            .collect();
        println!("  {:<18} {}", c.name, spread(values));
    }
}

/// Whether a body holds an ice shell over water.
fn icy(b: &&Body) -> bool {
    b.model.as_ref().is_some_and(|m| m.ice_shell.is_some())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let seeds: u32 = args.next().map_or(20, |a| a.parse().unwrap());
    let systems: u32 = args.next().map_or(300, |a| a.parse().unwrap());

    for name in ARCHETYPES {
        let drawn: Vec<System> = (1..=seeds).map(|s| whole(s, Some(name))).collect();
        let asked: Vec<&Body> = drawn
            .iter()
            .flat_map(|s| &s.bodies)
            .filter(|b| b.kind == BodyKind::Moon && b.params.archetype.as_deref() == Some(name))
            .collect();
        table(&format!("{name}, seeds 1 to {seeds}"), &asked);
    }

    let drawn: Vec<System> = (1..=systems).map(|s| whole(s, None)).collect();
    let of_kind = |kind: BodyKind| -> Vec<&Body> {
        drawn
            .iter()
            .flat_map(|s| &s.bodies)
            .filter(|b| b.kind == kind)
            .filter(icy)
            .collect()
    };
    table(
        &format!("drawn icy moons, {systems} systems"),
        &of_kind(BodyKind::Moon),
    );
    table(
        &format!("drawn icy planets, {systems} systems"),
        &of_kind(BodyKind::Planet),
    );
    let plumes: usize = drawn
        .iter()
        .flat_map(|s| &s.bodies)
        .filter_map(|b| b.params.given_rings.as_ref())
        .flat_map(|g| &g.dust)
        .filter(|d| d.kind == DustKind::Plume)
        .count();
    println!("\nplume rings in {systems} systems: {plumes}");
}
