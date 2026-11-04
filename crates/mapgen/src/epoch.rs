//! The surface of a world at another epoch, for the time slider
//! (`docs/analysis/08-geology.md`, section 6).
//!
//! The craters younger than the wet era were drawn after the erosion; the
//! elevation without them is kept in `raw_data/elevation_base.png`. At an
//! epoch, the young craters formed by then (those of the past older than
//! it, and in the future those formed in between) are drawn on it again,
//! the same way as at generation, worn for the time until the epoch, and
//! the biomes follow the new relief. Only worlds without liquid water
//! today offer epochs: on others the relief would also move lakes, rivers
//! and erosion, which are not recomputed. Moisture and the other layers
//! keep their present values; temperature follows the relief through the
//! lapse rate.
//!
//! The world is kept as loaded (16 bits per value, see [`LoadedWorld`]):
//! besides it, only the elevation and temperature are held in full
//! precision while an epoch is computed (about 12 bytes per pixel).
use crate::atmosphere::LIQUID_WATER_BAR;
use crate::biome::BiomeInput;
use crate::climate::temperature;
use crate::craters::{CraterField, Era, FUTURE_GA, base_min_diameter_km};
use crate::export::raw_data::{ELEVATION, Layer16, LoadedWorld, TEMPERATURE};
use crate::world::WorldParams;
use rayon::prelude::*;

/// The epochs a world can show, in Ga before present: from the future
/// (`-FUTURE_GA`) back to its surface age, or to the end of its wet era if
/// that is more recent. `None` for a world with liquid water today or
/// without craters.
pub fn epoch_range_ga(params: &WorldParams) -> Option<(f64, f64)> {
    range_ga(
        params.pressure_bar,
        params.surface_age_ga,
        params.planet.geology.wet_until,
    )
}

/// [`epoch_range_ga`] from the values a world's `world.json` holds: its
/// surface pressure (bar), surface age and the end of its preset's wet era
/// (Ga).
pub fn range_ga(pressure_bar: f32, surface_age_ga: f32, wet_until_ga: f32) -> Option<(f64, f64)> {
    if pressure_bar >= LIQUID_WATER_BAR || surface_age_ga <= 0.0 {
        return None;
    }
    Some((-FUTURE_GA, surface_age_ga.min(wet_until_ga) as f64))
}

/// `world` at `epoch_ga` (Ga before present, negative in the future):
/// `base` (its elevation without the young craters, see
/// [`crate::world::World::elevation_base`]) with the young craters formed
/// by then, its temperature and biomes following the new relief. At epoch
/// 0 it is the world itself.
pub fn loaded_at(world: LoadedWorld, base: &[f32], epoch_ga: f64) -> LoadedWorld {
    let p = &world.params;
    let grid = p.grid();
    let mut elevation = base.to_vec();
    CraterField::new_at(p, base_min_diameter_km(&grid), epoch_ga).apply(
        &mut elevation,
        &grid,
        Era::Dry,
    );
    let rules = &p.planet.rules;
    let above = |e: f32| (e - p.sea_level).clamp(-1.0, 1.0) - rules.waterline;
    let temperature: Vec<f32> = (0..elevation.len())
        .into_par_iter()
        .map(|i| {
            world.temperature.get(i) + temperature(0.0, above(elevation[i]))
                - temperature(0.0, above(world.elevation.get(i)))
        })
        .collect();
    let biomes = (0..elevation.len())
        .into_par_iter()
        .map(|i| {
            let input = BiomeInput {
                elevation: (elevation[i] - p.sea_level).clamp(-1.0, 1.0),
                moisture: world.moisture.get(i),
                temperature: temperature[i],
                volcanic_zone: world.volcanic_zone.get(i),
                coast_distance_km: world.coast_distance_km.get(i),
                // No liquid water: no lakes or rivers.
                lake: false,
                river: false,
                highland_offset: world.highland_offset.get(i),
            };
            p.planet.remap(rules.choose(input))
        })
        .collect();
    LoadedWorld {
        elevation: Layer16::quantise(&elevation, ELEVATION),
        temperature: Layer16::quantise(&temperature, TEMPERATURE),
        biomes,
        ..world
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::generate;
    use crate::request::WorldRequest;
    use crate::world::World;

    fn world(planet: &str) -> World {
        let planet = format!("{}/../../presets/{planet}", env!("CARGO_MANIFEST_DIR"));
        let request = WorldRequest {
            planet: Some(planet),
            seed: Some(4),
            sea_level: Some(0.15),
            surface_age_ga: Some(4.0),
            width: Some(512),
            height: Some(256),
            ..Default::default()
        };
        generate(&request.resolve(&mut rand::rng()).unwrap().params).world
    }

    #[test]
    fn worlds_without_liquid_water_offer_epochs() {
        let mercury = world("examples/mercury-like.toml");
        assert_eq!(epoch_range_ga(&mercury.params), Some((-FUTURE_GA, 4.0)));
        // Mars: back to the end of its wet era.
        let mars = world("examples/mars-like.toml");
        assert_eq!(epoch_range_ga(&mars.params), Some((-FUTURE_GA, 3.5)));
        let terran = world("terran.toml");
        assert_eq!(epoch_range_ga(&terran.params), None);
    }

    #[test]
    fn the_present_epoch_is_the_world_itself() {
        let w = world("examples/mercury-like.toml");
        let base = w.elevation_base.clone().unwrap();
        assert_ne!(base, w.elevation, "the world has young craters");
        let loaded = LoadedWorld::from_world(&w);
        let now = loaded_at(loaded.clone(), &base, 0.0);
        // The base is the final elevation minus the young craters' change,
        // in floating point: within two 16-bit steps (about 1.5 m).
        let worst = now
            .elevation
            .raw()
            .iter()
            .zip(loaded.elevation.raw())
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(worst <= 2, "{worst} steps");
        let same = now
            .biomes
            .iter()
            .zip(&w.biomes)
            .filter(|(a, b)| a == b)
            .count();
        assert!(same as f64 / w.biomes.len() as f64 > 0.999);
    }

    #[test]
    fn other_epochs_change_the_relief() {
        let w = world("examples/mercury-like.toml");
        let base = w.elevation_base.clone().unwrap();
        let loaded = LoadedWorld::from_world(&w);
        let changed = |epoch: f64| {
            let then = loaded_at(loaded.clone(), &base, epoch);
            let (a, b) = (then.elevation.raw(), loaded.elevation.raw());
            a.iter().zip(b).filter(|(a, b)| a != b).count()
        };
        // Back before most young craters, and a billion years ahead.
        assert!(changed(3.9) > 1000);
        assert!(changed(-1.0) > 100);
    }
}
