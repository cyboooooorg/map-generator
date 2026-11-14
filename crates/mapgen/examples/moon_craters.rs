//! Measures the surface age and the craters of the moon archetypes.
//!
//! ```text
//! cargo run --release -p mapgen --example moon_craters -- [SEEDS=3] [AGE_GYR]
//! ```
//!
//! For each moon archetype (`titan-like` to `triton-like`), asked alone in
//! a whole system round a `G2V` at seeds 1 to `SEEDS` (its age `AGE_GYR`,
//! drawn when not given), prints its regime,
//! its internal heat flux (tidal and radiogenic, mW/m²), its surface age
//! (Ga), its surface pressure and gravity, the smallest crater its air
//! lets through, and the density of its surviving craters of 5 and 20 km
//! or more per million km² that erosion and diffusion have not worn out,
//! counted over the whole sphere (not on a map).
//!
//! Part 4 of the planet model (#423) measured the icy moons' craters with
//! it before and after making the surface age of a stagnant ice lid follow
//! its heat flux and the air's screening follow its column.
use mapgen::craters::{CraterField, WORN_OUT};
use mapgen::system::{SystemRequest, WholeSystemRequest, draw, generation_params};
use rand::SeedableRng;
use rand::rngs::StdRng;

const MOONS: [&str; 7] = [
    "titan-like",
    "europa-like",
    "ganymede-like",
    "callisto-like",
    "enceladus-like",
    "io-like",
    "triton-like",
];

fn main() {
    let seeds: u32 = std::env::args().nth(1).map_or(3, |a| a.parse().unwrap());
    let age_gyr: Option<f32> = std::env::args().nth(2).map(|a| a.parse().unwrap());
    println!(
        "{:<15} {:>4} {:<9} {:>8} {:>8} {:>7} {:>7} {:>6} {:>7} {:>9} {:>9}",
        "archetype",
        "seed",
        "regime",
        "tide mW",
        "radio mW",
        "age Ga",
        "P bar",
        "g",
        "flux",
        "N5/1e6",
        "N20/1e6"
    );
    for name in MOONS {
        for seed in 1..=seeds {
            let request = SystemRequest::System(WholeSystemRequest {
                star: Some("G2V".into()),
                age_gyr,
                metallicity: Some(0.0),
                planets: Some([(name.to_string(), 1)].into_iter().collect()),
                seed: Some(seed),
                width: Some(64),
                height: Some(32),
            });
            let system = draw(&request, &mut StdRng::seed_from_u64(u64::from(seed))).unwrap();
            let Some(moon) = system
                .bodies
                .iter()
                .find(|b| b.params.archetype.as_deref() == Some(name))
            else {
                println!("{name:<15} {seed:>4} not drawn");
                continue;
            };
            let model = moon.model.as_ref().expect("a round moon has a model");
            let r = model.interior.radius_earth * 6.371e6;
            let area_m2 = 4.0 * std::f64::consts::PI * r * r;
            let p = generation_params(moon);
            let field = CraterField::new(&p, 5.0);
            let area_km2 = area_m2 / 1e6;
            // Those the base map would show: not worn out (by the wet era's
            // erosion or by diffusion), whatever the sea.
            let n = |d: f64| {
                field
                    .craters
                    .iter()
                    .filter(|c| c.diameter_km >= d && field.wear(c) >= WORN_OUT)
                    .count() as f64
            };
            let rates = mapgen::impacts::rates(&p);
            println!(
                "{:<15} {:>4} {:<9} {:>8.2} {:>8.2} {:>7.3} {:>7.3} {:>6.2} {:>7.2} {:>9.1} {:>9.1}",
                name,
                seed,
                format!("{:?}", model.surface.as_ref().map(|s| s.regime)),
                model.heat.tidal_w / area_m2 * 1e3,
                model.heat.radiogenic_w / area_m2 * 1e3,
                p.surface_age_ga,
                p.pressure_bar,
                model.interior.gravity,
                rates.flux,
                n(5.0) / area_km2 * 1e6,
                n(20.0) / area_km2 * 1e6,
            );
        }
    }
}
