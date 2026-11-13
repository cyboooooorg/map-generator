//! Measures the physical planet model on drawn systems.
//!
//! ```text
//! cargo run --release -p mapgen --example planet_census -- [COUNT=500]
//! ```
//!
//! Draws `COUNT` whole systems (seeds 1 to `COUNT`, nothing written) and
//! prints, for the planets of them all:
//!
//! 1. the base preset each planet's world renders with
//!    (`planet::interim::base_of` of its model); a planet's type is its
//!    class, so the class mix of item 4 is the type mix;
//! 2. the share of planets in each radius bin of Fulton et al. (2017,
//!    doi:10.3847/1538-3881/aa80eb);
//! 3. the median age, mass, orbit, core mass fraction, water fraction,
//!    envelope fraction, surface temperature and surface pressure of each
//!    class (the last two over the planets that have a surface);
//! 4. the class mix, the surface liquid mix, the share with life and the
//!    tectonic regime mix, over the planets that have a surface, and how
//!    many runaways came from the insolation limit or from boiling their
//!    ocean (`Water::boiled`), all of them and the runaway-greenhouse
//!    class alone, and how many wet planets stay past 647 K (a boil whose
//!    steam would have cooled, so the first pass stands), and how many
//!    planets that are no giant (nor mini-Neptune) keep an envelope that
//!    hides their surface, by class (they render with their class's
//!    preset, see `planet::interim::base_of`);
//! 5. the radius histogram, in bins of 0.25 R_earth, of the planets with a
//!    period under 100 days (the radius valley of Fulton et al. 2017
//!    shows as a dip between 1.5 and 2 R_earth), and the class mix of the
//!    same planets (super-Earths and sub-Neptunes should be within a
//!    factor 1.5 of each other, and the close-in ice worlds are watched).
use std::collections::BTreeMap;

use mapgen::planet::{body_model, interim};
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
    mass: Vec<f64>,
    orbit: Vec<f64>,
    temperature: Vec<f64>,
    pressure: Vec<f64>,
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

    // base preset rendered -> planets
    let mut bases: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut bins = [0u32; BINS.len()];
    let mut stats: BTreeMap<&'static str, ClassStats> = BTreeMap::new();
    let mut planets = 0u32;
    let mut with_surface = 0u32;
    // Planets no giant whose envelope hides the surface, by class.
    let mut enveloped: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut liquids: BTreeMap<String, u32> = BTreeMap::new();
    let mut regimes: BTreeMap<String, u32> = BTreeMap::new();
    let mut life = 0u32;
    // [by the insolation limit, boiled]: all runaways, then the class.
    let mut runaways = [0u32; 2];
    let mut runaway_class = [0u32; 2];
    let mut hot_and_wet = 0u32;
    let mut oxygenated = 0u32;
    let mut close_in = [0u32; 20];
    let mut close_in_total = 0u32;
    let mut close_in_class: BTreeMap<&'static str, u32> = BTreeMap::new();

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
            *bases.entry(interim::base_of(&model)).or_default() += 1;
            let radius = model.interior.radius_earth;
            let bin = BINS
                .iter()
                .rposition(|&(low, _)| radius >= low)
                .unwrap_or(0);
            bins[bin] += 1;
            if model.surface.is_none() && !model.class.is_giant() {
                *enveloped.entry(class).or_default() += 1;
            }
            if let Some(surface) = &model.surface {
                with_surface += 1;
                *liquids
                    .entry(format!("{:?}", surface.climate.liquid))
                    .or_default() += 1;
                *regimes.entry(format!("{:?}", surface.regime)).or_default() += 1;
                life += u32::from(surface.life.arose);
                hot_and_wet += u32::from(
                    !surface.water.runaway
                        && surface.water.kept_kg > 0.0
                        && surface.climate.temperature_k >= 647.096,
                );
                if surface.water.runaway {
                    let source = usize::from(surface.water.boiled);
                    runaways[source] += 1;
                    if model.class == mapgen::planet::PlanetClass::RunawayGreenhouse {
                        runaway_class[source] += 1;
                    }
                }
                oxygenated += u32::from(surface.life.oxygenated);
            }
            if body.orbit.period_days < 100.0 {
                close_in_total += 1;
                *close_in_class.entry(class).or_default() += 1;
                let i = ((radius / 0.25) as usize).min(close_in.len() - 1);
                close_in[i] += 1;
            }
            let s = stats.entry(class).or_default();
            s.age.push(f64::from(body.params.age_gyr));
            s.mass.push(model.inputs.mass_earth);
            s.orbit.push(f64::from(body.params.orbit_au));
            if let Some(surface) = &model.surface {
                s.temperature.push(surface.climate.temperature_k);
                s.pressure.push(surface.gases.total());
            }
            s.cmf.push(model.inputs.core_mass_fraction);
            s.water.push(model.inputs.water_fraction);
            s.envelope.push(model.inputs.envelope_fraction);
        }
    }

    println!("{count} systems, {planets} planets\n");

    println!("base preset of the worlds (planets)");
    for (base, n) in &bases {
        println!(
            "{base:<12}{n:>8}  {:>5.1} %",
            100.0 * f64::from(*n) / f64::from(planets)
        );
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
        "{:<20}{:>7}{:>9}{:>9}{:>8}{:>7}{:>9}{:>10}{:>8}{:>11}",
        "class",
        "planets",
        "age Gyr",
        "mass M⊕",
        "a AU",
        "CMF",
        "water",
        "envelope",
        "T K",
        "P bar"
    );
    for (class, s) in &mut stats {
        println!(
            "{class:<20}{:>7}{:>9.2}{:>9.3}{:>8.3}{:>7.3}{:>9.4}{:>10.4}{:>8.0}{:>11.3}",
            s.age.len(),
            median(&mut s.age),
            median(&mut s.mass),
            median(&mut s.orbit),
            median(&mut s.cmf),
            median(&mut s.water),
            median(&mut s.envelope),
            median(&mut s.temperature),
            median(&mut s.pressure),
        );
    }

    let share = |n: u32, of: u32| 100.0 * f64::from(n) / f64::from(of.max(1));

    println!("\nclass mix ({planets} planets)");
    let mut mix: Vec<(&&str, u32)> = stats.iter().map(|(c, s)| (c, s.age.len() as u32)).collect();
    mix.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    for (class, n) in mix {
        println!("{class:<20}{:>6.1} %  ({n})", share(n, planets));
    }

    println!("\nplanets with a surface: {with_surface}");
    let hidden: u32 = enveloped.values().sum();
    println!(
        "no giant, surface hidden by an envelope: {hidden} ({:.1} %)",
        share(hidden, planets)
    );
    for (class, n) in &enveloped {
        println!("  {class:<18}{n:>6}");
    }
    println!("\nsurface liquid ({with_surface} planets with a surface)");
    for (liquid, n) in &liquids {
        println!("{liquid:<20}{:>6.1} %  ({n})", share(*n, with_surface));
    }
    println!("\ntectonic regime ({with_surface} planets with a surface)");
    for (regime, n) in &regimes {
        println!("{regime:<20}{:>6.1} %  ({n})", share(*n, with_surface));
    }
    println!(
        "\nrunaways: {} by the insolation limit, {} boiled; \
         of the runaway-greenhouse class: {} by the limit, {} boiled",
        runaways[0], runaways[1], runaway_class[0], runaway_class[1]
    );
    println!("wet, not in runaway and past 647 K (a boil not kept): {hot_and_wet}");
    println!(
        "\nlife: {:.1} % of planets with a surface ({life}), {:.1} % oxygenated ({oxygenated})",
        share(life, with_surface),
        share(oxygenated, with_surface)
    );

    println!("\nradius histogram, period under 100 days ({close_in_total} planets)");
    for (i, n) in close_in.iter().enumerate() {
        let low = 0.25 * i as f64;
        let last = if i + 1 == close_in.len() { "+" } else { "" };
        println!(
            "{low:>5.2}-{:<5.2}{last:<2}{:>6.1} %  ({n})",
            low + 0.25,
            share(*n, close_in_total)
        );
    }
    println!("\nclasses, period under 100 days ({close_in_total} planets)");
    for (class, n) in &close_in_class {
        println!("{class:<20}{:>6.1} %  ({n})", share(*n, close_in_total));
    }
}
