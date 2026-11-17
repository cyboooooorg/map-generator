//! Measures every built-in archetype asked alone in a whole system, one
//! row per seed, so each can be compared with the real body it stands for.
//!
//! ```text
//! cargo run --release -p mapgen --example archetype_census -- [SEEDS=20] [ARCHETYPE...]
//! ```
//!
//! Each row prints the body's class, surface temperature, air (total and
//! main gases), surface liquid, surface age and crater rate, terrain units (renewal, age, share), mass, radius,
//! tidal heat and, for icy bodies, the ice shell and the geysers.
use std::collections::BTreeMap;

use mapgen::system::{SystemRequest, WholeSystemRequest, draw, generation_params};
use rand::SeedableRng;
use rand::rngs::StdRng;

const ARCHETYPES: [&str; 21] = [
    "mercury-like",
    "venus-like",
    "earth-like",
    "mars-like",
    "io-like",
    "europa-like",
    "ganymede-like",
    "callisto-like",
    "titan-like",
    "enceladus-like",
    "triton-like",
    "jupiter-like",
    "saturn-like",
    "uranus-like",
    "neptune-like",
    "hot-jupiter",
    "hot-neptune",
    "mini-neptune",
    "super-earth",
    "ocean-world",
    "lava-world",
];

fn main() {
    let mut args = std::env::args().skip(1);
    let seeds: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(20);
    let asked: Vec<String> = args.collect();
    let mut list: Vec<&str> = if asked.is_empty() {
        ARCHETYPES.to_vec()
    } else {
        asked.iter().map(String::as_str).collect()
    };
    list.dedup();
    for arch in list {
        println!("== {arch}");
        for seed in 1..=seeds {
            let req = SystemRequest::System(WholeSystemRequest {
                planets: Some(BTreeMap::from([(arch.to_string(), 1)])),
                seed: Some(seed),
                ..WholeSystemRequest::default()
            });
            let sys = match draw(&req, &mut StdRng::seed_from_u64(u64::from(seed))) {
                Ok(s) => s,
                Err(e) => {
                    println!("{seed:>3} draw failed: {e}");
                    continue;
                }
            };
            let Some(b) = sys
                .bodies
                .iter()
                .find(|b| b.params.archetype.as_deref() == Some(arch))
            else {
                println!("{seed:>3} no body with the archetype");
                continue;
            };
            let p = generation_params(b);
            let Some(m) = p.model.as_ref() else {
                println!("{seed:>3} {:?} {} no model", b.kind, b.id);
                continue;
            };
            let mut line = format!(
                "{seed:>3} {:<5} {:<18} base {:<10} star {:.2} Gyr M {:.3} R {:.3} g {:.2} age {:.2} Ga craters x{:.2} tidal {:.1e} W",
                format!("{:?}", b.kind).chars().take(5).collect::<String>(),
                m.class.key(),
                mapgen::planet::interim::base_of(m),
                p.age_gyr,
                m.inputs.mass_earth,
                m.interior.radius_earth,
                m.interior.gravity,
                p.surface_age_ga,
                p.impact_factor.unwrap_or(f32::NAN),
                m.heat.tidal_w,
            );
            let units: Vec<String> = m
                .terrain
                .units
                .iter()
                .map(|u| {
                    format!(
                        "{:?} {:.3} Ga {:.0} %",
                        u.renewal,
                        u.age_gyr,
                        u.share * 100.0
                    )
                })
                .collect();
            line += &format!(" | units [{}]", units.join(", "));
            if let Some(s) = m.surface.as_ref() {
                let g = &s.gases;
                line += &format!(
                    " | T {:.1} K P {:.3e} bar co2 {:.2e} n2 {:.2e} h2o {:.2e} ch4 {:.2e} so2 {:.2e} o2 {:.2e} | {:?} {:?} snow {} | sea {:.2}",
                    s.climate.temperature_k,
                    g.total(),
                    g.co2,
                    g.n2,
                    g.h2o,
                    g.ch4,
                    g.so2,
                    g.o2,
                    s.regime,
                    s.climate.liquid,
                    s.climate.snowball,
                    p.sea_level,
                );
            }
            if let Some(sh) = m.ice_shell {
                line += &format!(
                    " | shell {:.1}/{:.1} km ocean {:.1} km conv {}",
                    sh.shell_km, sh.polar_shell_km, sh.ocean_km, sh.convecting
                );
            }
            if let Some(gy) = m.geysers.as_ref() {
                line += &format!(" | geysers {}", gy.fractures.len());
            }
            println!("{line}");
        }
    }
}
