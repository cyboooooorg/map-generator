//! Measures the moons of drawn systems.
//!
//! ```text
//! cargo run --release -p mapgen --example moon_census -- [COUNT=500]
//! ```
//!
//! Draws `COUNT` whole systems (seeds 1 to `COUNT`, nothing written) and
//! prints:
//!
//! 1. for each planet type, how many planets there are and how many of
//!    them have moons;
//! 2. the giants (every type with "giant", "jupiter" or "neptune" in its
//!    name) by equilibrium temperature, which sets their disk's ice line
//!    (`moons::disk_ice_line_radii`);
//! 3. for each host type, the moons by type (rock or ice) and radius bin,
//!    the bins named after the Solar System's moons of that size;
//! 4. how many moons are over 2300 km (Titan, Ganymede) and how many
//!    systems hold one, then the moons over 1200 km, host and orbit, the
//!    first 40 of them;
//! 5. the round moons (those with a model): their class by host type, the
//!    largest one, and their model's radius over the radius their bulk's
//!    uncompressed density gave them (`MoonSystem::draw`);
//! 6. the round moons over 2000 km in radius around a host under 100 K:
//!    their surface temperature, pressure, CH₄ share, surface liquid and
//!    class, and why each that is no hydrocarbon world misses it.
//!
//! Part 4 of the planet model (#423) measured the moon draw with it before
//! and after drawing a giant's moons from its disk's mass, shape and
//! temperature.
use std::collections::BTreeMap;

use mapgen::star::StarInfo;
use mapgen::system::{BodyKind, SystemRequest, WholeSystemRequest, draw};
use rand::SeedableRng;
use rand::rngs::StdRng;

const BINS: [(f64, &str); 7] = [
    (0.0, "<200"),
    (200.0, "200-600"),
    (600.0, "600-1200"),
    (1200.0, "1200-1700 (Europe/Lune)"),
    (1700.0, "1700-2300 (Io/Triton..)"),
    (2300.0, "2300-3000 (Titan/Ganymede)"),
    (3000.0, ">3000 (Mars+)"),
];

fn bin(r: f64) -> &'static str {
    BINS.iter().rev().find(|(lo, _)| r >= *lo).unwrap().1
}

/// The radius, km, of a moon line of the "over 1200 km" list.
fn radius_of(line: &str) -> f64 {
    let after = line.split(" -> ").nth(1).unwrap();
    after.split_whitespace().nth(1).unwrap().parse().unwrap()
}

fn main() {
    let count: u64 = std::env::args().nth(1).map_or(500, |a| a.parse().unwrap());
    // host group -> (moon type, radius bin) -> n
    let mut hist: BTreeMap<String, BTreeMap<(String, &str), u32>> = BTreeMap::new();
    let mut hosts: BTreeMap<String, (u32, u32)> = BTreeMap::new(); // planets, with moons
    let mut giant_teq: BTreeMap<&str, u32> = BTreeMap::new();
    let mut big: Vec<String> = Vec::new();
    // host group -> round moon's class -> n
    let mut classes: BTreeMap<String, BTreeMap<String, u32>> = BTreeMap::new();
    let mut largest_round = (0.0_f64, String::new());
    let mut radius_ratios: Vec<f64> = Vec::new();
    // Cold large moons: (line, reason)
    let mut cold_large: Vec<(String, &'static str)> = Vec::new();
    for seed in 1..=count {
        let req = SystemRequest::System(WholeSystemRequest {
            seed: Some(seed as u32),
            ..Default::default()
        });
        let sys = draw(&req, &mut StdRng::seed_from_u64(seed)).unwrap();
        for p in sys.bodies.iter().filter(|b| b.kind == BodyKind::Planet) {
            let moons: Vec<_> = sys
                .bodies
                .iter()
                .filter(|b| b.parent.as_deref() == Some(&p.id))
                .collect();
            let pp = &p.params;
            let teq =
                StarInfo::new(pp.star, pp.orbit_au, pp.eccentricity, pp.age_gyr).equilibrium_k;
            let group = p.body_type.to_string();
            let e = hosts.entry(group.clone()).or_default();
            e.0 += 1;
            if !moons.is_empty() {
                e.1 += 1;
            }
            if p.body_type.contains("giant")
                || p.body_type.contains("jupiter")
                || p.body_type.contains("neptune")
            {
                let t = if teq > 400.0 {
                    "Teq>400"
                } else if teq > 150.0 {
                    "150-400"
                } else if teq > 60.0 {
                    "60-150"
                } else {
                    "<60"
                };
                *giant_teq.entry(t).or_default() += 1;
            }
            let drawn = mapgen::moons::MoonSystem::draw(pp).map_or_else(Vec::new, |s| s.moons);
            for m in moons {
                let r = m.physical.radius_km;
                if let Some(model) = &m.model {
                    if let Some(d) = drawn.iter().find(|d| d.seed == m.params.seed) {
                        radius_ratios.push(r / f64::from(d.radius_km));
                    }
                    if r > 2000.0 && teq < 100.0 {
                        let s = model.surface.as_ref().expect("a moon has a surface");
                        let p_bar = s.gases.total();
                        let ch4 = if p_bar > 0.0 {
                            s.gases.ch4 / p_bar
                        } else {
                            0.0
                        };
                        let t = s.climate.temperature_k;
                        let reason = why_not_hydrocarbon(model, p_bar, t);
                        cold_large.push((
                            format!(
                                "seed {seed:3} {:5} host {:11} Teq {teq:3.0} K  r {r:4.0} km  T {t:5.1} K  P {p_bar:9.2e} bar  CH4 {:5.1} %  CH4 in {:8.2e}  dynamo {:5}  {:?} -> {}",
                                m.id,
                                p.body_type,
                                100.0 * ch4,
                                model.inputs.volatiles.ch4,
                                s.core.dynamo,
                                s.climate.liquid,
                                m.body_type
                            ),
                            reason,
                        ));
                    }
                    *classes
                        .entry(group.clone())
                        .or_default()
                        .entry(m.body_type.clone())
                        .or_default() += 1;
                    if r > largest_round.0 {
                        largest_round = (r, format!("seed {seed} {} ({})", m.id, m.body_type));
                    }
                }
                *hist
                    .entry(group.clone())
                    .or_default()
                    .entry((m.body_type.clone(), bin(r)))
                    .or_default() += 1;
                if r > 1200.0 {
                    big.push(format!("seed {seed} {} ({} {:.0} ME, {:.2} AU, Teq {:.0} K) -> {} {:.0} km a={:.0} radii",
                        m.id, p.body_type, p.physical.mass_earth, pp.orbit_au, teq, m.body_type, r,
                        m.orbit.semi_major_axis_km / p.physical.radius_km));
                }
            }
        }
    }
    println!("== hosts: planets, with moons");
    for (k, (n, w)) in &hosts {
        println!("{k:20} {n:5} {w:5}");
    }
    println!("== giants by Teq");
    for (k, n) in &giant_teq {
        println!("{k:10} {n}");
    }
    println!("== moons by host class, type, radius");
    for (g, h) in &hist {
        println!("-- {g}");
        for ((t, b), n) in h {
            println!("   {t:10} {b:28} {n}");
        }
    }
    let titans: Vec<&String> = big
        .iter()
        .filter(|l| l.contains(" -> ") && radius_of(l) > 2300.0)
        .collect();
    let mut systems: Vec<&str> = titans
        .iter()
        .map(|l| l.split_whitespace().nth(1).unwrap())
        .collect();
    systems.dedup();
    println!(
        "== moons over 2300 km: {} in {} systems of {count}",
        titans.len(),
        systems.len()
    );
    println!("== moons over 1200 km: {}", big.len());
    for l in big.iter().take(40) {
        println!("{l}");
    }
    println!("== round moons by host class, moon class");
    let mut all: BTreeMap<&str, u32> = BTreeMap::new();
    for (g, h) in &classes {
        println!("-- {g}");
        for (c, n) in h {
            println!("   {c:22} {n}");
            *all.entry(c).or_default() += n;
        }
    }
    println!("-- all hosts");
    for (c, n) in &all {
        println!("   {c:22} {n}");
    }
    println!(
        "== largest round moon: {:.0} km, {}",
        largest_round.0, largest_round.1
    );
    println!(
        "== round moons over 2000 km around a host under 100 K: {}",
        cold_large.len()
    );
    let mut reasons: BTreeMap<&str, u32> = BTreeMap::new();
    for (line, reason) in &cold_large {
        println!("{line}  [{reason}]");
        *reasons.entry(reason).or_default() += 1;
    }
    println!("-- why");
    for (reason, n) in &reasons {
        println!("   {reason:60} {n}");
    }
    radius_ratios.sort_by(f64::total_cmp);
    if let (Some(lo), Some(hi)) = (radius_ratios.first(), radius_ratios.last()) {
        println!(
            "== round moons' model radius / drawn radius: {lo:.3} to {hi:.3}, median {:.3} ({} moons)",
            radius_ratios[radius_ratios.len() / 2],
            radius_ratios.len()
        );
    }
}

/// Why a cold large moon of `model` (surface pressure `p_bar`, temperature
/// `t`) is no hydrocarbon world (liquid hydrocarbons under more than
/// 1 bar, `planet::classify`): its air lost (its own dynamo ignored inside
/// its giant's magnetosphere, or no field to lose), no methane in its
/// inputs (a giant-impact moon of a large world is dry rock), its methane
/// destroyed by its star's Lyman α faster than it outgassed it
/// (`atmosphere::methane_photolysis`), too cold or too warm for liquid
/// methane, or air under 1 bar.
fn why_not_hydrocarbon(model: &mapgen::planet::PlanetModel, p_bar: f64, t: f64) -> &'static str {
    use mapgen::planet::{PlanetClass, SurfaceLiquid};
    let s = model.surface.as_ref().unwrap();
    if model.class == PlanetClass::HydrocarbonWorld {
        "is one"
    } else if model.inputs.volatiles.ch4 <= 0.0 {
        "no CH4 in its inputs (dry giant-impact moon)"
    } else if p_bar < 1e-3 {
        if s.core.dynamo {
            "no air kept: own dynamo ignored (magnetosphere rule)"
        } else {
            "no air kept: escape without a field (shoreline)"
        }
    } else if s.gases.ch4 <= 0.0 {
        "air kept, its CH4 photolysed (Lyman alpha outran outgassing)"
    } else if t < 90.7 {
        "air kept, too cold for liquid CH4 (< 90.7 K)"
    } else if s.climate.liquid != SurfaceLiquid::Hydrocarbons {
        "air kept, too warm for liquid CH4 (ethane boils)"
    } else if p_bar <= 1.0 {
        "liquid CH4 but air under DENSE_AIR_BAR (1 bar)"
    } else {
        "other (class rule)"
    }
}
