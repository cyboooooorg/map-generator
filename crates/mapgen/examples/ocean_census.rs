//! Measures the ice shells and oceans of icy moons and planets.
//!
//! ```text
//! cargo run --release -p mapgen --example ocean_census -- [SEEDS=20] [SYSTEMS=300]
//! ```
//!
//! Prints, for each icy moon archetype asked alone in a whole system
//! (seeds 1 to `SEEDS`), then for the icy moons and icy planets of
//! `SYSTEMS` drawn systems (seeds 1 to `SYSTEMS`), the min, median and max
//! of every column of [`COLUMNS`] (the share of the bodies, for a yes-or-no
//! column); then the plume rings of the drawn systems and each body with
//! geysers, with the period ratio to its nearest moons in and out and its
//! eccentricity; then the share of the neighbouring pairs of round
//! disk-born moons of the drawn systems locked in a 2:1 resonance (period
//! ratio in `RESONANCE_BAND`) and the drawn moons with the most tidal
//! heat. A column whose field the model does not hold yet prints `-`.
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
/// `None` where the model does not hold it (printed `-`). A `share`
/// column reads 1 for yes and 0 for no, and prints the share of yes.
struct Column {
    name: &'static str,
    read: fn(&PlanetModel) -> Option<f64>,
    share: bool,
}

/// Every column, in print order. Add a field of the model here and the
/// tables show it.
const COLUMNS: [Column; 7] = [
    Column {
        name: "shell km",
        read: |m| m.ice_shell.map(|s| s.shell_km),
        share: false,
    },
    Column {
        name: "polar shell km",
        read: |m| m.ice_shell.map(|s| s.polar_shell_km),
        share: false,
    },
    Column {
        name: "ocean km",
        read: |m| m.ice_shell.map(|s| s.ocean_km),
        share: false,
    },
    Column {
        name: "tidal W",
        read: |m| Some(m.heat.tidal_w),
        share: false,
    },
    Column {
        name: "moment of inertia",
        read: |m| Some(m.moment_of_inertia),
        share: false,
    },
    Column {
        name: "convecting share",
        read: |m| m.ice_shell.map(|s| f64::from(u8::from(s.convecting))),
        share: true,
    },
    Column {
        name: "geyser share",
        read: |m| Some(f64::from(u8::from(m.geysers.is_some()))),
        share: true,
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

/// The share of yes (1) among the values, `-` when there are none.
fn share(v: &[f64]) -> String {
    if v.is_empty() {
        return "-".into();
    }
    let yes = v.iter().filter(|&&x| x > 0.5).count();
    format!(
        "{yes} of {} ({:.0} %)",
        v.len(),
        100.0 * yes as f64 / v.len() as f64
    )
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
        let shown = if c.share {
            share(&values)
        } else {
            spread(values)
        };
        println!("  {:<18} {shown}", c.name);
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
    resonances(&drawn);
    println!("\nbodies with geysers (period ratio to the next moon in and out, e):");
    for system in &drawn {
        for b in system.bodies.iter().filter(vents) {
            let siblings: Vec<&Body> = system
                .bodies
                .iter()
                .filter(|o| o.kind == BodyKind::Moon && o.parent.is_some() && o.parent == b.parent)
                .collect();
            let ratio = |outward: bool| {
                siblings
                    .iter()
                    .filter(|o| (o.orbit.period_days > b.orbit.period_days) == outward)
                    .filter(|o| o.id != b.id)
                    .map(|o| o.orbit.period_days / b.orbit.period_days)
                    .min_by(|x, y| (x - 1.0).abs().total_cmp(&(y - 1.0).abs()))
                    .map_or("-".into(), |r| format!("{r:.3}"))
            };
            let model = b.model.as_ref().expect("a body with geysers has a model");
            let shell = model.ice_shell.expect("a body with geysers has a shell");
            let fractures = model.geysers.as_ref().map_or(0, |g| g.fractures.len());
            println!(
                "  seed {:>3} {:<6} {:?} r {:.0} km, polar shell {:.1} km, tidal {}, \
                 {fractures} fractures, ratio in {} out {}, e {:.4}",
                system.seed,
                b.id,
                b.kind,
                b.physical.radius_km,
                shell.polar_shell_km,
                num(model.heat.tidal_w),
                ratio(false),
                ratio(true),
                b.orbit.eccentricity,
            );
        }
    }
}

/// The neighbouring pairs of round, disk-born, prograde moons of the
/// `drawn` systems (each planet's, nearest first), and how many of them
/// lie in `RESONANCE_BAND`; then the five drawn moons with the most tidal
/// heat, with their flux.
fn resonances(drawn: &[System]) {
    use mapgen::planet::resonance::RESONANCE_BAND;
    let (mut pairs, mut locked) = (0, 0);
    let mut heated: Vec<(f64, f64, u32, &str)> = Vec::new();
    for system in drawn {
        let mut parents: Vec<&str> = system
            .bodies
            .iter()
            .filter(|b| b.kind == BodyKind::Moon)
            .filter_map(|b| b.parent.as_deref())
            .collect();
        parents.dedup();
        for parent in parents {
            let mut chain: Vec<&Body> = system
                .bodies
                .iter()
                .filter(|b| b.kind == BodyKind::Moon && b.parent.as_deref() == Some(parent))
                .filter(|b| b.model.is_some() && b.disk_born && b.orbit.inclination_deg < 90.0)
                .collect();
            chain.sort_by(|a, b| a.orbit.period_days.total_cmp(&b.orbit.period_days));
            for w in chain.windows(2) {
                let ratio = w[1].orbit.period_days / w[0].orbit.period_days;
                pairs += 1;
                locked += usize::from(RESONANCE_BAND.0 < ratio && ratio < RESONANCE_BAND.1);
            }
            for b in chain {
                let tidal = b.model.as_ref().map_or(0.0, |m| m.heat.tidal_w);
                let r = b.physical.radius_km * 1000.0;
                heated.push((
                    tidal,
                    tidal / (4.0 * std::f64::consts::PI * r * r),
                    system.seed,
                    &b.id,
                ));
            }
        }
    }
    println!(
        "\nneighbouring round disk-born moons in 2:1 resonance: {locked} of {pairs} pairs ({:.0} %)",
        100.0 * locked as f64 / pairs.max(1) as f64
    );
    heated.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("most tidally heated drawn moons:");
    for (w, flux, seed, id) in heated.iter().take(5) {
        println!(
            "  seed {seed:>3} {id:<6} {} W, {} W/m²",
            num(*w),
            num(*flux)
        );
    }
}

/// Whether a body vents geysers.
fn vents(b: &&Body) -> bool {
    b.model.as_ref().is_some_and(|m| m.geysers.is_some())
}
