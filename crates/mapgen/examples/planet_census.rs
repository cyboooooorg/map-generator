//! Measures the physical planet model on drawn systems.
//!
//! ```text
//! cargo run --release -p mapgen --example planet_census -- [COUNT=500]
//! ```
//!
//! Draws `COUNT` whole systems (seeds 1 to `COUNT`, nothing written) and
//! prints, for the planets of them all:
//!
//! 1. the current `body_type` against the class the model derives;
//! 2. the share of planets in each radius bin of Fulton et al. (2017,
//!    doi:10.3847/1538-3881/aa80eb);
//! 3. the median age, core mass fraction, water fraction and envelope
//!    fraction of each class.
use std::collections::BTreeMap;

use mapgen::planet::body_model;
use mapgen::system::{BodyKind, SystemRequest, WholeSystemRequest, draw};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Radius bins of Fulton et al. 2017, Earth radii: lower edge and label.
const BINS: [(f64, &str); 9] = [
    (0.0, "<1"),
    (1.0, "1-1.5"),
    (1.5, "1.5-2"),
    (2.0, "2-3"),
    (3.0, "3-4"),
    (4.0, "4-6"),
    (6.0, "6-10"),
    (10.0, "10-22"),
    (22.0, ">22"),
];

#[derive(Default)]
struct ClassStats {
    age: Vec<f64>,
    cmf: Vec<f64>,
    water: Vec<f64>,
    envelope: Vec<f64>,
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n % 2 == 1 {
        values[n / 2]
    } else {
        0.5 * (values[n / 2 - 1] + values[n / 2])
    }
}

fn main() {
    let count: u32 = std::env::args()
        .nth(1)
        .map_or(500, |a| a.parse().expect("COUNT must be a number"));

    // (body_type, class) -> planets
    let mut cross: BTreeMap<(String, &'static str), u32> = BTreeMap::new();
    let mut bins = [0u32; BINS.len()];
    let mut stats: BTreeMap<&'static str, ClassStats> = BTreeMap::new();
    let mut planets = 0u32;

    for seed in 1..=count {
        let request = SystemRequest::System(WholeSystemRequest {
            seed: Some(seed),
            ..WholeSystemRequest::default()
        });
        let system = draw(&request, &mut StdRng::seed_from_u64(u64::from(seed)))
            .unwrap_or_else(|e| panic!("seed {seed}: {e:?}"));
        for body in system.bodies.iter().filter(|b| b.kind == BodyKind::Planet) {
            let model = body_model(&system, body).expect("a planet has a model");
            let class = model.class.key();
            planets += 1;
            *cross.entry((body.body_type.clone(), class)).or_default() += 1;
            let radius = model.interior.radius_earth;
            let bin = BINS
                .iter()
                .rposition(|&(low, _)| radius >= low)
                .unwrap_or(0);
            bins[bin] += 1;
            let s = stats.entry(class).or_default();
            s.age.push(f64::from(body.params.age_gyr));
            s.cmf.push(model.inputs.core_mass_fraction);
            s.water.push(model.inputs.water_fraction);
            s.envelope.push(model.inputs.envelope_fraction);
        }
    }

    println!("{count} systems, {planets} planets\n");

    println!("body_type against derived class (planets)");
    let classes: Vec<&'static str> = stats.keys().copied().collect();
    print!("{:<12}", "body_type");
    for c in &classes {
        print!("{c:>13}");
    }
    println!("{:>8}", "total");
    let types: Vec<String> = {
        let mut t: Vec<String> = cross.keys().map(|(t, _)| t.clone()).collect();
        t.dedup();
        t
    };
    for t in &types {
        print!("{t:<12}");
        let mut total = 0;
        for c in &classes {
            let n = cross.get(&(t.clone(), *c)).copied().unwrap_or(0);
            total += n;
            print!("{n:>13}");
        }
        println!("{total:>8}");
    }

    println!("\nplanets per radius bin (Fulton et al. 2017 bins, R_earth)");
    for (i, (_, label)) in BINS.iter().enumerate() {
        println!(
            "{label:>6}  {:>6.1} %  ({})",
            100.0 * f64::from(bins[i]) / f64::from(planets),
            bins[i]
        );
    }

    println!("\nmedians per class");
    println!(
        "{:<13}{:>7}{:>10}{:>7}{:>9}{:>10}",
        "class", "planets", "age Gyr", "CMF", "water", "envelope"
    );
    for (class, s) in &mut stats {
        println!(
            "{class:<13}{:>7}{:>10.2}{:>7.3}{:>9.4}{:>10.4}",
            s.age.len(),
            median(&mut s.age),
            median(&mut s.cmf),
            median(&mut s.water),
            median(&mut s.envelope),
        );
    }
}
