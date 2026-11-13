//! Transitional mapping of a [`PlanetModel`] onto today's presets, so that
//! a planet the model describes renders with the rules, palette and remap
//! of the built-in preset closest to it. Deleted by #424, when archetypes
//! replace the presets.
use super::{PlanetClass, PlanetModel, SurfaceLiquid};
use crate::geology::Regime;

/// Mean surface temperature under which a dry body with water renders
/// frozen, K: water's freezing point.
const FREEZING_K: f64 = 273.0;

/// Least share of water and ices of a dry, cold body that renders frozen:
/// the ice world's threshold of [`super::classify`] (Europa, about 8 %,
/// is one; Earth, 0.02 %, is not).
const FROZEN_MIN_WATER: f64 = 0.05;

/// The built-in preset whose rules and remap render `model`, the first
/// rule that holds:
///
/// 1. a giant class: `gas-giant`, `hot-jupiter` or `ice-giant` (a
///    mini-Neptune renders as an ice giant);
/// 2. a lava world or a heat-pipe regime: `volcanic`;
/// 3. a runaway greenhouse: `caustic` (the Venus look; the spec names
///    barren or volcanic, caustic is closer to Venus and exists);
/// 4. liquid hydrocarbons: `frozen`;
/// 5. liquid water: `terran`;
/// 6. no liquid, a mean temperature under 273 K and at least 5 % water:
///    `frozen`;
/// 7. otherwise `barren` (a body with no surface under a thin envelope
///    too: the model gives it no temperature at the surface).
pub fn base_of(model: &PlanetModel) -> &'static str {
    match model.class {
        PlanetClass::GasGiant => return "gas-giant",
        PlanetClass::HotJupiter => return "hot-jupiter",
        PlanetClass::IceGiant | PlanetClass::MiniNeptune => return "ice-giant",
        _ => {}
    }
    let surface = model.surface.as_ref();
    let liquid = surface.map_or(SurfaceLiquid::None, |s| s.climate.liquid);
    if model.class == PlanetClass::LavaWorld
        || liquid == SurfaceLiquid::Lava
        || surface.is_some_and(|s| s.regime == Regime::HeatPipe)
    {
        return "volcanic";
    }
    if model.class == PlanetClass::RunawayGreenhouse {
        return "caustic";
    }
    let Some(surface) = surface else {
        return "barren";
    };
    match liquid {
        SurfaceLiquid::Hydrocarbons => "frozen",
        SurfaceLiquid::Water => "terran",
        _ if surface.climate.temperature_k < FREEZING_K
            && model.inputs.water_fraction >= FROZEN_MIN_WATER =>
        {
            "frozen"
        }
        _ => "barren",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::planet_model;
    use crate::planet::reference::{model_inputs, reference, sun_at};

    /// The model of the reference body `name` at its orbit round the Sun.
    fn model_of(name: &str) -> PlanetModel {
        let body = reference(name);
        planet_model(&sun_at(body.orbit_au), model_inputs(body))
    }

    #[test]
    fn solar_bodies_render_with_their_look() {
        // Earth, Venus, Mars, Mercury and Titan's inputs from
        // planet::reference at their orbits round the Sun.
        let expect = [
            ("Earth", "terran"),
            ("Venus", "caustic"),
            ("Mars", "barren"),
            ("Mercury", "barren"),
            ("Titan", "frozen"),
        ];
        for (name, base) in expect {
            assert_eq!(base_of(&model_of(name)), base, "{name}");
        }
    }

    #[test]
    fn giants_render_as_giants() {
        for (name, base) in [
            ("Jupiter", "gas-giant"),
            ("Saturn", "gas-giant"),
            ("Uranus", "ice-giant"),
            ("Neptune", "ice-giant"),
        ] {
            assert_eq!(base_of(&model_of(name)), base, "{name}");
        }
        let mut mini = model_of("Neptune");
        mini.class = PlanetClass::MiniNeptune;
        assert_eq!(base_of(&mini), "ice-giant");
        let mut hot = model_of("Jupiter");
        hot.class = PlanetClass::HotJupiter;
        assert_eq!(base_of(&hot), "hot-jupiter");
    }
}
