//! Giant planets as worlds of their own (backlog item 14, step B;
//! `docs/analysis/12-gas-giants.md`): no ground, but the top of the clouds,
//! banded by the planet's spin.
//!
//! **Bands.** Fast rotation turns the flow into zonal jets; their width
//! follows the Rhines scale `√(2U/β)` (Scott & Polvani 2007), so the number
//! of eastward jets in each hemisphere grows as `√(ΩR/U)`: about five for
//! Jupiter. The wind `U` is an outcome: it grows with the flux the giant
//! radiates (its internal heat, cooled with the age of its system, plus the
//! starlight it absorbs) and falls with the spin ([`wind_speed`]), so a
//! Saturn-like giant, colder and slower, gets as many jets as Jupiter, an
//! ice giant (fast and broad winds: retrograde at the equator, prograde at
//! high latitudes) one or two, a locked hot Jupiter (its day is days long)
//! one or two. Between the jets, pale zones (rising air, high ammonia
//! clouds) alternate with dark belts (sinking air, deeper clouds), each
//! hemisphere drawn on its own. Which clouds are on top, and so the colours
//! and the height between zones and belts, come from the decks that
//! condense (`crate::giant_clouds`).
//!
//! **Texture.** The latitude a pixel samples is shifted by noise stretched
//! along the bands, more where neighbouring jets shear (festoons and
//! wavy edges); long-lived vortices swirl the bands around them (a great
//! red spot on some gas giants, white ovals, dark spots with bright
//! companion clouds on ice giants).
//!
//! **Poles.** Poleward of about 64° the bands break down (Orton et al.
//! 2017). Each pole has its own style: a cluster of cyclones (Juno found a
//! central cyclone ringed by eight at Jupiter's north pole and five at its
//! south, each 5 000 to 8 000 km across, Adriani et al. 2018) on a dark,
//! bluish background of folded filaments and billowing white clouds; a
//! hexagonal jet around a polar vortex (Saturn's north, Godfrey 1988); or a
//! smooth haze with a bright cap and sometimes a single polar cyclone (the
//! ice giants).
//!
//! **Layers.** The biome layer holds the zones, belts, storms and polar
//! regions, in the giant's own colours; the elevation is the height of the
//! cloud tops (zones and storms high, belts low), above [`CLOUD_DECK`] so
//! that no pixel reads as sea; the temperature is that of the cloud tops (a
//! hot Jupiter's day side far hotter than its night side, the hot spot
//! shifted east); the moisture layer carries the eastward wind instead, as
//! a share of the strongest (`GiantInfo::wind_east`, `wind_west`: the
//! larger of the two is 1). The rendered colours go to `surface.png`
//! directly.
use crate::biome::Biome;
use crate::generation::{Generated, Stage};
use crate::giant_clouds::Clouds;
use crate::giants::{
    GiantKind, giant_radius, internal_temperature, surface_gravity, system_age_gyr,
};
use crate::noise::{fbm, smoothstep, stream, stream_seed};
use crate::star::StarInfo;
use crate::world::{World, WorldParams};
use noise::Perlin;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Elevation of the mean cloud deck: above any sea level, so that cloud
/// tops never read as water.
pub const CLOUD_DECK: f32 = 0.3;
/// Earth's radius (m) and `GM` (m³/s²).
const EARTH_RADIUS_M: f64 = 6.371e6;
const EARTH_GM: f64 = 3.986e14;
/// Jupiter's wind, m/s: the speed many of its jets reach ("exceed
/// 100 m s⁻¹", Ingersoll et al. 2004, the Cambridge chapter in the article's
/// sources) and the reference of [`wind_speed`]. Showman et al. 2010 give
/// peaks from about 150 m/s on Jupiter.
const JUPITER_WIND: f64 = 100.0;
/// Jupiter's total radiated flux, W/m²: the 6.25 it absorbs and the 7.485
/// it makes (Li et al. 2018, doi:10.1038/s41467-018-06107-2).
const JUPITER_FLUX: f64 = 13.7;
/// Jupiter's `β = 2Ω/R` at the equator, 1/(m s): 9.925 h, 71 492 km (NASA
/// Jupiter fact sheet).
const JUPITER_BETA: f64 = 2.0 * 1.7585e-4 / 7.1492e7;
/// An ice giant's reference wind, flux and `β`: calibration, no source. The
/// wind the jet count of Neptune needs (about 300 m/s: Showman et al. 2010
/// give peaks over 400 m/s), at the flux of an ice giant of effective
/// temperature 60 K, and Neptune's `β` (16.11 h, 24 764 km, NASA Neptune fact
/// sheet).
const ICE_WIND: f64 = 300.0;
const ICE_FLUX: f64 = 5.670_374e-8 * 60.0 * 60.0 * 60.0 * 60.0;
const ICE_BETA: f64 = 2.0 * 1.0834e-4 / 2.4764e7;
/// A hot Jupiter's superrotation, m/s at 1 500 K: calibration, no source (the
/// kilometres per second of the circulation models in Showman et al. 2010).
const HOT_WIND: f64 = 2000.0;
/// Jets per hemisphere at `√(ΩR/U) = 1`: Jupiter (`ΩR/U` ≈ 126) gets five.
const JET_FACTOR: f64 = 0.45;

/// Typical wind speed of a giant, m/s, from the energy it has to drive
/// winds and its spin: `U = U₀ (F/F₀)^(2/5) (β/β₀)^(−1/5)`, with `U₀`, `F₀`
/// and `β₀` of Jupiter (gas giants) or Neptune (ice giants).
///
/// The energy is the flux `F = σT_eff⁴` the giant radiates, the sum of the
/// sunlight it absorbs and its internal heat; a fixed share of it feeds the
/// winds, as on Earth, where about 1 % of the absorbed flux is converted
/// (Showman et al. 2010, section 3.1). In the turbulence that makes the jets
/// the energy input `ε ~ U³/L_β` is carried at the Rhines scale
/// `L_β = (U/β)^(1/2)` (Rhines 1975), which gives `U ∝ ε^(2/5) β^(−1/5)`:
/// the dimensional balance of Vallis & Maltrud (1993, doi:10.1175/1520-0485
/// (1993)023<1346:GOMFAJ>2.0.CO;2) for the jets of a forced flow on a beta
/// plane, derived here and not read from their paper. Showman et al. say
/// themselves that the winds of the giants are not understood this way
/// (Neptune has stronger winds than Jupiter with 4 % of the sunlight):
/// a Saturn-like giant comes out slower than Jupiter, not faster.
///
/// A hot giant is not driven this way: its wind is [`HOT_WIND`] scaled by
/// its equilibrium temperature.
fn wind_speed(
    kind: GiantKind,
    hot: bool,
    equilibrium_k: f32,
    effective_k: f32,
    omega: f64,
    radius_m: f64,
) -> f64 {
    if hot {
        return HOT_WIND * (f64::from(equilibrium_k) / 1500.0).clamp(0.6, 2.5);
    }
    let flux = 5.670_374e-8 * f64::from(effective_k).powi(4);
    let beta = 2.0 * omega / radius_m;
    let (u0, f0, b0) = match kind {
        GiantKind::Gas => (JUPITER_WIND, JUPITER_FLUX, JUPITER_BETA),
        GiantKind::Ice => (ICE_WIND, ICE_FLUX, ICE_BETA),
    };
    (u0 * (flux / f0).powf(0.4) * (beta / b0).powf(-0.2)).clamp(20.0, 3000.0)
}

/// One band of latitude, from `south` to `north` (degrees).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Band {
    south: f32,
    north: f32,
    zone: bool,
    /// Brightness shift of this band, `[-1, 1]`.
    tint: f32,
    /// How far its colour leans away from the palette's (redder belts,
    /// whiter zones), `[0, 1]`.
    hue: f32,
}

/// A long-lived vortex.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Storm {
    lon: f32,
    lat: f32,
    /// Half-widths in degrees, east–west and north–south.
    half_lon: f32,
    half_lat: f32,
    colour: [f32; 3],
    /// A bright companion cloud on its poleward side (ice giants' dark spots).
    companion: bool,
}

/// The colours of a giant, sRGB in `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Palette {
    pub zone: [f32; 3],
    pub belt: [f32; 3],
    pub storm: [f32; 3],
    pub polar: [f32; 3],
}

/// The look of a giant's pole.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum PoleStyle {
    /// A smooth haze, a bright cap and sometimes one polar cyclone (the ice
    /// giants).
    #[default]
    Haze,
    /// A central cyclone ringed by others (Jupiter's: eight in the north,
    /// five in the south), on dark folded filaments.
    Cyclones,
    /// A hexagonal jet around a polar vortex (Saturn's north).
    Hexagon,
}

/// A pole as drawn: its style, its cyclones (count of the ring, their
/// distance from the pole and size, degrees), the ring's turn, whether a
/// cyclone sits on the pole itself.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pole {
    style: PoleStyle,
    ring: u8,
    ring_colat: f32,
    radius: f32,
    turn: f32,
    central: bool,
}

/// The ring system of a giant, as `world.json` stores it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct RingSystem {
    /// Inner and outer edges, in planet radii (equatorial).
    pub inner: f32,
    pub outer: f32,
    /// Gaps: their centres and widths, in planet radii.
    pub gaps: Vec<[f32; 2]>,
    /// Opacity of the densest part, `[0, 1]`.
    pub opacity: f32,
    /// Colour, sRGB in `[0, 1]`.
    pub colour: [f32; 3],
    /// Bright (broad, icy) or faint (narrow, dusty).
    pub bright: bool,
    /// The moon that opens each gap (the same order as `gaps`), by its id
    /// in its planetary system: one that clears it or whose resonance sets
    /// it (see `crate::rings`). Empty on rings drawn otherwise, whose gaps
    /// are tied to no moon.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub gap_moons: Vec<String>,
}

/// A giant's rings as its planetary system drew them, or none (see
/// `WorldParams::given_rings`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GivenRings {
    pub rings: Option<RingSystem>,
    /// Faint dust rings its moons feed (see `crate::rings::dust_rings`),
    /// far wider than the main rings and seen only against the light.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<Vec<crate::rings::DustRing>>", optional)
    )]
    pub dust: Vec<crate::rings::DustRing>,
}

/// A giant's rings, drawn from `rng` with the odds of bright and of
/// faint rings (its preset's `[giant] rings`): bright ones broad and icy
/// (Saturn's), faint ones narrow and dusty (Uranus's and Neptune's), or
/// none.
pub fn draw_rings(rng: &mut impl RngExt, rings_odds: [f32; 2]) -> Option<RingSystem> {
    let r = rng.random::<f32>();
    if r < rings_odds[0] {
        let inner = rng.random_range(1.2..1.5);
        let outer = rng.random_range(2.1..2.5);
        let gaps = (0..rng.random_range(1..4))
            .map(|_| {
                [
                    rng.random_range(inner + 0.1..outer - 0.1),
                    rng.random_range(0.015..0.06),
                ]
            })
            .collect();
        Some(RingSystem {
            inner,
            outer,
            gaps,
            opacity: rng.random_range(0.6..0.9),
            colour: [0.86, 0.8, 0.68],
            bright: true,
            gap_moons: Vec::new(),
        })
    } else if r < rings_odds[0] + rings_odds[1] {
        let inner = rng.random_range(1.5..1.9);
        Some(RingSystem {
            inner,
            outer: inner + rng.random_range(0.05..0.3),
            gaps: Vec::new(),
            opacity: rng.random_range(0.08..0.2),
            colour: [0.45, 0.43, 0.42],
            bright: false,
            gap_moons: Vec::new(),
        })
    } else {
        None
    }
}

/// What a giant world is, as `world.json` stores it (under `giant`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GiantInfo {
    pub kind: GiantKind,
    /// A hot giant (`Teq` over 1 000 K): locked to its star, inflated.
    pub hot: bool,
    /// Mass and equatorial radius, Earth units; mean density, g/cm³;
    /// gravity at the cloud tops, Earth's = 1.
    pub mass_earth: f32,
    pub radius_earth: f32,
    pub density: f32,
    pub gravity: f32,
    /// Length of its day, hours (its year when locked).
    pub rotation_hours: f32,
    /// Flattening of the globe, `1 − polar / equatorial radius`.
    pub flattening: f32,
    /// Equilibrium temperature (Earth's albedo), effective temperature with
    /// its internal heat, K, and the heat it emits over what it absorbs.
    pub equilibrium_k: f32,
    pub effective_k: f32,
    pub heat_ratio: f32,
    /// Cloud class of Sudarsky et al. (2000), 1 to 5.
    pub class: u8,
    /// Eastward jets in each hemisphere (the Rhines count). The wind profile
    /// has `2 × jets − 1` alternating jets in each hemisphere, `jets` of them
    /// eastward, besides the equatorial one.
    pub jets: u32,
    /// Strongest eastward and westward winds, m/s.
    pub wind_east: f32,
    pub wind_west: f32,
    /// The eastward wind every 2° of latitude from the south pole (91
    /// values), for the views' differential rotation.
    pub wind_profile: Vec<f32>,
    /// Whether it has a great spot (gas) or a dark spot (ice), and a polar
    /// hexagon.
    pub great_spot: bool,
    pub hexagon: bool,
    /// The look of the north and south poles, and the cyclones ringing
    /// each (0 when none).
    #[serde(default)]
    pub poles: [PoleStyle; 2],
    #[serde(default)]
    pub polar_cyclones: [u8; 2],
    pub palette: Palette,
    pub rings: Option<RingSystem>,
}

/// Everything drawn for a giant world: its info and the bands and storms
/// the pixels sample.
#[derive(Clone, Debug)]
pub struct Look {
    pub info: GiantInfo,
    bands: Vec<Band>,
    storms: Vec<Storm>,
    /// Latitude where the polar regions start, degrees.
    polar: f32,
    /// The north and south poles.
    poles: [Pole; 2],
    /// Typical wind speed, m/s, and the equatorial jet (gas giants).
    wind: f32,
    equatorial_jet: f32,
    /// Jets at the band edges: latitude, signed speed, half-width.
    jets_at: Vec<(f32, f32, f32)>,
    /// Eastward shift of a hot giant's hot spot from the substellar point, degrees.
    hot_spot_shift: f32,
    /// Height between zones and belts relative to Jupiter's (see
    /// [`Clouds::relief`]).
    relief: f32,
}

/// Where the great spot of a gas giant sits: the centre latitude and the
/// width (degrees) of a zone of the southern hemisphere between 12° and 45°
/// (the Great Red Spot lies at 17° to 27.5° S, Ingersoll et al. 2004),
/// chosen by `pick` in `[0, 1)`; the zone nearest 22° S when no zone is in
/// that range.
fn great_spot_site(bands: &[Band], pick: f32) -> (f32, f32) {
    let zones: Vec<&Band> = bands
        .iter()
        .filter(|b| b.zone && b.north < 0.0 && (12.0..45.0).contains(&(-0.5 * (b.south + b.north))))
        .collect();
    let band = if zones.is_empty() {
        bands
            .iter()
            .filter(|b| b.north < 0.0)
            .min_by(|a, b| {
                let off = |x: &Band| (-0.5 * (x.south + x.north) - 22.0).abs();
                off(a).total_cmp(&off(b))
            })
            .unwrap_or(&bands[0])
    } else {
        zones[((pick * zones.len() as f32) as usize).min(zones.len() - 1)]
    };
    (0.5 * (band.south + band.north), band.north - band.south)
}

/// Radius of a giant world, Earth radii, from its parameters.
pub fn radius_earth(p: &WorldParams, mass_earth: f32) -> f32 {
    let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity);
    giant_radius(mass_earth, star.equilibrium_k)
}

/// The colours of a giant of `kind` and `clouds`, varied by `t` and `u` in
/// `[0, 1]`: the palette of its class, blended with that of the next class
/// by the share its top deck is thin (see [`Clouds::thin`]), so that a
/// giant crossing a class bound changes colour gradually.
fn palette(kind: GiantKind, clouds: Clouds, t: f32, u: f32) -> Palette {
    let own = class_palette(kind, clouds.class, t, u);
    if clouds.thin <= 0.0 {
        return own;
    }
    let next = class_palette(kind, clouds.class + 1, t, u);
    let blend = |a: [f32; 3], b: [f32; 3]| [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * clouds.thin);
    Palette {
        zone: blend(own.zone, next.zone),
        belt: blend(own.belt, next.belt),
        storm: blend(own.storm, next.storm),
        polar: blend(own.polar, next.polar),
    }
}

/// The colours of a giant of `kind` and cloud `class`, varied by `t` and
/// `u` in `[0, 1]`: Jupiter's cream and ochre to Saturn's pale gold, cyan
/// to deep blue ice giants (methane absorbing the red, a tinge of it left
/// on the warmer ones, classes II and III), white water clouds, azure clear
/// skies, dark roasters, and the pale silicate clouds of class V, a Bond
/// albedo of about 0.5 against 0.03 for class IV (Sudarsky et al. 2000).
fn class_palette(kind: GiantKind, class: u8, t: f32, u: f32) -> Palette {
    let mix = |a: [f32; 3], b: [f32; 3], f: f32| [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * f);
    let (zone, belt, storm, polar) = match (kind, class) {
        (GiantKind::Ice, 1) => (
            mix([0.72, 0.89, 0.92], [0.5, 0.66, 0.93], t),
            mix([0.62, 0.82, 0.88], [0.32, 0.46, 0.84], t),
            mix([0.2, 0.3, 0.5], [0.1, 0.16, 0.42], t),
            mix([0.78, 0.9, 0.93], [0.56, 0.7, 0.92], t),
        ),
        (_, 1) => (
            mix([0.94, 0.89, 0.79], [0.93, 0.86, 0.68], t),
            mix([0.68, 0.48, 0.32], [0.82, 0.7, 0.5], t),
            mix([0.78, 0.4, 0.26], [0.86, 0.62, 0.42], t),
            mix([0.56, 0.53, 0.5], [0.58, 0.62, 0.68], t),
        ),
        (_, 2) => (
            [0.97, 0.97, 0.95],
            [0.86, 0.87, 0.89],
            [0.99, 0.99, 0.98],
            [0.9, 0.91, 0.93],
        ),
        (_, 3) => (
            mix([0.48, 0.67, 0.93], [0.4, 0.6, 0.9], t),
            [0.3, 0.48, 0.84],
            [0.85, 0.9, 0.97],
            [0.42, 0.56, 0.82],
        ),
        (_, 4) => (
            [0.23, 0.25, 0.33],
            [0.13, 0.13, 0.17],
            [0.3, 0.28, 0.32],
            [0.18, 0.18, 0.22],
        ),
        _ => (
            [0.78, 0.74, 0.68],
            [0.6, 0.56, 0.5],
            [0.88, 0.83, 0.76],
            [0.7, 0.67, 0.62],
        ),
    };
    // Methane stays in the gas of a warmer ice giant and still absorbs red.
    let [zone, belt, storm, polar] = if kind == GiantKind::Ice && (2..=3).contains(&class) {
        [zone, belt, storm, polar].map(crate::giant_clouds::methane_tint)
    } else {
        [zone, belt, storm, polar]
    };
    // Some giants are more uniform than others (Saturn's muted bands).
    let belt = mix(belt, zone, 0.45 * u);
    Palette {
        zone,
        belt,
        storm,
        polar,
    }
}

impl Look {
    /// The look of the giant world `p` of mass `mass_earth`.
    pub fn draw(p: &WorldParams, mass_earth: f32) -> Self {
        let kind = p.planet.giant.as_ref().map_or(GiantKind::Gas, |g| g.kind);
        let rings_odds = p.planet.giant.as_ref().map_or([0.0, 0.0], |g| g.rings);
        let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(p.seed, stream::GIANT_LOOK)));
        let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity);
        let equilibrium = star.equilibrium_k;
        let hot = equilibrium > 1000.0;
        let radius_earth = giant_radius(mass_earth, equilibrium);
        let internal = internal_temperature(kind, mass_earth, system_age_gyr(star.mass));
        let effective = (equilibrium.powi(4) + internal.powi(4)).powf(0.25);
        let clouds = Clouds::of(effective, surface_gravity(mass_earth, radius_earth));
        let class = clouds.class;

        // Spin: Jupiter's and Saturn's 10 hours, the ice giants' 16 to 17;
        // a hot giant turns once a year, locked to its star.
        let rotation_hours = if hot || star.tidally_locked {
            star.year_days * 24.0
        } else {
            match kind {
                GiantKind::Gas => rng.random_range(9.0..12.0),
                GiantKind::Ice => rng.random_range(14.0..19.0),
            }
        };
        let omega = 2.0 * PI / (f64::from(rotation_hours) * 3600.0);
        let radius_m = f64::from(radius_earth) * EARTH_RADIUS_M;
        let gm = f64::from(mass_earth) * EARTH_GM;
        let q = omega * omega * radius_m.powi(3) / gm;
        let flattening = (0.7 * q).min(0.12) as f32;

        let wind = wind_speed(kind, hot, equilibrium, effective, omega, radius_m);
        let jets = ((JET_FACTOR * (omega * radius_m / wind).sqrt()).round() as u32).clamp(1, 8);

        // The bands of each hemisphere, from the equator out to the polar
        // regions: an equatorial zone, then belts and zones in turn.
        let polar = match kind {
            GiantKind::Gas => rng.random_range(58.0..70.0),
            GiantKind::Ice => rng.random_range(62.0..74.0),
        };
        let equator = if hot {
            rng.random_range(18.0..30.0)
        } else {
            match kind {
                GiantKind::Gas => rng.random_range(6.0..11.0),
                GiantKind::Ice => rng.random_range(18.0..28.0),
            }
        };
        let mut bands = vec![Band {
            south: -equator,
            north: equator,
            zone: true,
            tint: rng.random_range(-0.3..0.3),
            hue: rng.random(),
        }];
        for sign in [1.0f32, -1.0] {
            let count = (2 * jets - 1) as usize;
            let weights: Vec<f32> = (0..count)
                .map(|k| rng.random_range(0.5..1.5) * (1.0 - 0.55 * k as f32 / count as f32))
                .collect();
            let total: f32 = weights.iter().sum();
            let mut edge = equator;
            for (k, w) in weights.iter().enumerate() {
                let next = edge + (polar - equator) * w / total;
                let (south, north) = if sign > 0.0 {
                    (edge, next)
                } else {
                    (-next, -edge)
                };
                bands.push(Band {
                    south,
                    north,
                    zone: k % 2 == 1,
                    tint: rng.random_range(-0.5..0.5),
                    hue: rng.random(),
                });
                edge = next;
            }
        }
        bands.sort_by(|a, b| a.south.total_cmp(&b.south));

        // Jets at the band edges, alternating in sign, weaker poleward;
        // gas giants add a prograde equatorial jet.
        let mut jets_at = Vec::new();
        let mut edges: Vec<f32> = bands.iter().map(|b| b.north).collect();
        edges.pop();
        for &lat in &edges {
            let rank = bands.iter().position(|b| b.north == lat).unwrap_or(0) as i32
                - (bands.len() / 2) as i32;
            let sign = if (rank.rem_euclid(2) == 0) == (lat > 0.0) {
                1.0
            } else {
                -1.0
            };
            let strength = rng.random_range(0.4..1.4) * (1.0 - lat.abs() / 90.0).sqrt();
            jets_at.push((
                lat,
                sign * strength * wind as f32,
                rng.random_range(2.0..4.0),
            ));
        }
        // An ice giant with a single jet keeps the smooth profile of
        // `wind_at`; with more it gets its jets at the band edges too, and
        // its equatorial jet blows against the spin (Showman et al. 2010;
        // Scott & Polvani 2007).
        let equatorial_jet = match (hot, kind) {
            (false, GiantKind::Gas) => rng.random_range(0.8..4.0) * wind as f32,
            (false, GiantKind::Ice) if jets > 1 => -rng.random_range(0.8..2.0) * wind as f32,
            _ => 0.0,
        };

        // Storms: a great spot on some gas giants and white ovals on most;
        // dark spots with bright companions on some ice giants.
        let mut storms = Vec::new();
        let colours = palette(kind, clouds, rng.random(), rng.random());
        let mut great_spot = false;
        if !hot {
            match kind {
                GiantKind::Gas => {
                    if rng.random::<f32>() < 0.45 {
                        great_spot = true;
                        let size = rng.random_range(0.7..1.3);
                        let lon = rng.random_range(0.0..360.0);
                        // In a zone of the south, between two jets: the
                        // zones are anticyclonic (Ingersoll et al. 2004).
                        let (lat, width) = great_spot_site(&bands, rng.random());
                        // Shrunk to fit a narrow zone, but never under the
                        // white ovals, so that it stays the largest storm.
                        let fit = (0.45 * width / (5.5 * size)).clamp(0.7, 1.0);
                        storms.push(Storm {
                            lon,
                            lat,
                            half_lon: 11.0 * size * fit,
                            half_lat: 5.5 * size * fit,
                            colour: colours.storm,
                            companion: false,
                        });
                    }
                    for _ in 0..rng.random_range(2..9) {
                        let lat = rng.random_range(20.0..50.0)
                            * if rng.random::<bool>() { 1.0 } else { -1.0 };
                        let size = rng.random_range(1.0..3.0);
                        storms.push(Storm {
                            lon: rng.random_range(0.0..360.0),
                            lat,
                            half_lon: size * 1.6,
                            half_lat: size,
                            colour: [0.97, 0.95, 0.9],
                            companion: false,
                        });
                    }
                }
                GiantKind::Ice => {
                    if rng.random::<f32>() < 0.5 {
                        great_spot = true;
                        let size = rng.random_range(0.6..1.2);
                        storms.push(Storm {
                            lon: rng.random_range(0.0..360.0),
                            lat: -rng.random_range(18.0..45.0),
                            half_lon: 14.0 * size,
                            half_lat: 7.0 * size,
                            colour: colours.storm,
                            companion: true,
                        });
                    }
                }
            }
        }
        let hexagon = kind == GiantKind::Gas && !hot && rng.random::<f32>() < 0.3;

        // Drawn whatever the system gave, so that the rest of the look keeps
        // its draws; the system's rings, when it drew them, take its place.
        let drawn = draw_rings(&mut rng, rings_odds);
        let rings = match &p.given_rings {
            Some(given) => given.rings.clone(),
            None => drawn,
        };

        let mut look = Look {
            info: GiantInfo {
                kind,
                hot,
                mass_earth,
                radius_earth,
                density: (5.51 * f64::from(mass_earth) / f64::from(radius_earth).powi(3)) as f32,
                gravity: mass_earth / (radius_earth * radius_earth),
                rotation_hours,
                flattening,
                equilibrium_k: equilibrium,
                effective_k: effective,
                heat_ratio: (effective / equilibrium).powi(4),
                class,
                jets,
                wind_east: 0.0,
                wind_west: 0.0,
                wind_profile: Vec::new(),
                great_spot,
                hexagon,
                poles: [PoleStyle::Haze; 2],
                polar_cyclones: [0; 2],
                palette: colours,
                rings,
            },
            bands,
            storms,
            polar,
            poles: [Pole {
                style: PoleStyle::Haze,
                ring: 0,
                ring_colat: 0.0,
                radius: 0.0,
                turn: 0.0,
                central: false,
            }; 2],
            wind: wind as f32,
            equatorial_jet,
            jets_at,
            hot_spot_shift: if hot {
                rng.random_range(10.0..35.0)
            } else {
                0.0
            },
            relief: clouds.relief,
        };
        // The poles, drawn last so the rest keeps its draws: each its own
        // style, the cyclones sized like Juno's (5 000 to 8 000 km across).
        let radius_km = f64::from(radius_earth) * EARTH_RADIUS_M / 1000.0;
        for k in 0..2 {
            let r: f32 = rng.random();
            let style = match (hot, kind) {
                (true, _) => PoleStyle::Haze,
                (_, GiantKind::Ice) => {
                    if r < 0.3 {
                        PoleStyle::Cyclones
                    } else {
                        PoleStyle::Haze
                    }
                }
                (_, GiantKind::Gas) => {
                    // About 15 % of poles get a hexagon, 55 % cyclones.
                    if k == 0 && hexagon && r < 0.4 {
                        PoleStyle::Hexagon
                    } else if r < 0.55 {
                        PoleStyle::Cyclones
                    } else if r < 0.65 {
                        PoleStyle::Hexagon
                    } else {
                        PoleStyle::Haze
                    }
                }
            };
            let diameter_km = rng.random_range(5000.0..8000.0);
            // Their spiral arms reach out about 1.4 times the core seen in
            // the infrared.
            let radius =
                ((1.4 * diameter_km / 2.0 / radius_km).to_degrees() as f32).clamp(2.0, 11.0);
            // An ice giant's lone polar cyclone; a gas giant's ring of five
            // to nine, packed edge to edge around the central one.
            let ring = if kind == GiantKind::Gas && style == PoleStyle::Cyclones {
                rng.random_range(5u8..10)
            } else {
                0
            };
            let ring_colat = if ring > 0 {
                (radius * 2.1 / (std::f32::consts::PI / f32::from(ring)).sin() / 2.0)
                    .max(radius * 2.2)
            } else {
                0.0
            };
            look.poles[k] = Pole {
                style,
                ring,
                ring_colat,
                radius,
                turn: rng.random::<f32>() * std::f32::consts::TAU,
                central: style == PoleStyle::Cyclones,
            };
            look.info.poles[k] = style;
            look.info.polar_cyclones[k] = ring;
        }
        look.info.hexagon = look.info.poles.contains(&PoleStyle::Hexagon);
        look.info.wind_profile = (0..91)
            .map(|k| look.wind_at(-90.0 + 2.0 * k as f32))
            .collect();
        look.info.wind_east = look.info.wind_profile.iter().copied().fold(0.0, f32::max);
        look.info.wind_west = -look.info.wind_profile.iter().copied().fold(0.0, f32::min);
        look
    }

    /// The latitudes (degrees) of the edges of the bands, from the south
    /// edge of the southern polar region to the north edge of the northern
    /// one: one more than there are bands, ascending. The band between the
    /// `k`-th and the next edge is a zone when `k` is odd (the equatorial
    /// zone sits in the middle, at an odd rank).
    pub fn band_edges(&self) -> Vec<f32> {
        let mut edges = vec![self.bands[0].south];
        edges.extend(self.bands.iter().map(|b| b.north));
        edges
    }

    /// The eastward wind at latitude `lat` (degrees), m/s.
    pub fn wind_at(&self, lat: f32) -> f32 {
        let polar_fade = 1.0 - smoothstep(80.0, 90.0, lat.abs());
        if self.info.hot {
            return self.wind * (-(lat / 25.0).powi(2)).exp() * polar_fade;
        }
        match self.info.kind {
            // Three broad jets, westward at the equator, eastward near 69°:
            // the profile of Uranus and Neptune scaled by the giant's wind.
            GiantKind::Ice if self.info.jets <= 1 => {
                let s = (lat.to_radians() * 1.3).sin();
                self.wind * (1.6 * s * s - 0.8) * polar_fade
            }
            GiantKind::Ice | GiantKind::Gas => {
                let jets: f32 = self
                    .jets_at
                    .iter()
                    .map(|&(at, speed, width)| speed * (-((lat - at) / width).powi(2)).exp())
                    .sum();
                (self.equatorial_jet * (-(lat / 12.0).powi(2)).exp() + jets) * polar_fade
            }
        }
    }

    /// The band at latitude `lat`, its index and the share of the way
    /// across it (0 south edge, 1 north edge); `None` in the polar regions.
    fn band_at(&self, lat: f32) -> Option<(usize, f32)> {
        self.bands
            .iter()
            .position(|b| lat >= b.south && lat < b.north)
            .map(|i| {
                let b = &self.bands[i];
                (i, (lat - b.south) / (b.north - b.south))
            })
    }

    /// How strongly jets shear at `lat`: 1 on a band edge, 0 mid-band.
    fn shear(&self, lat: f32) -> f32 {
        self.bands
            .iter()
            .map(|b| (-((lat - b.north) / 2.5).powi(2)).exp())
            .fold(0.0, f32::max)
    }
}

/// One pixel of a giant world.
struct Pixel {
    colour: [f32; 3],
    biome: Biome,
    elevation: f32,
    temperature: f32,
    wind: f32,
}

/// Samples the giant at longitude `lon` and latitude `lat` (degrees).
fn sample(look: &Look, noise: &Perlin, fine: &Perlin, lon: f32, lat: f32) -> Pixel {
    let palette = &look.info.palette;
    // Vortices swirl the bands around them.
    let (mut s_lon, mut s_lat) = (lon, lat);
    let mut storm_hit: Option<(&Storm, f32)> = None;
    for storm in &look.storms {
        let d_lon =
            ((lon - storm.lon + 540.0).rem_euclid(360.0) - 180.0) * lat.to_radians().cos().max(0.2);
        let d_lat = lat - storm.lat;
        let (u, v) = (d_lon / storm.half_lon, d_lat / storm.half_lat);
        let r = (u * u + v * v).sqrt();
        if r < 1.8 {
            // Anticyclones turn clockwise in the north, counter-clockwise
            // in the south.
            let turn = 3.0 * (-(r * r) * 1.5).exp() * storm.lat.signum();
            let (sin, cos) = turn.sin_cos();
            let (ru, rv) = (u * cos - v * sin, u * sin + v * cos);
            s_lon = storm.lon + ru * storm.half_lon / lat.to_radians().cos().max(0.2);
            s_lat = storm.lat + rv * storm.half_lat;
            if storm_hit.is_none_or(|(_, best)| r < best) {
                storm_hit = Some((storm, r));
            }
        }
    }

    // Turbulence stretched along the bands, stronger where jets shear.
    let (lon_r, lat_r) = (f64::from(s_lon).to_radians(), f64::from(s_lat).to_radians());
    let (x, y, z) = (
        lat_r.cos() * lon_r.cos(),
        lat_r.sin(),
        lat_r.cos() * lon_r.sin(),
    );
    let shear = look.shear(s_lat);
    let wobble = fbm(noise, x * 2.5, y * 14.0, z * 2.5, 5);
    let meander = fbm(noise, x * 1.1 + 5.0, y * 5.0, z * 1.1, 3);
    let streak = fbm(fine, x * 9.0, y * 70.0, z * 9.0, 4);
    let warped = s_lat + (1.2 + 4.0 * shear) * wobble + 1.5 * meander + 0.4 * streak;

    let mix = |a: [f32; 3], b: [f32; 3], f: f32| [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * f);
    // The band colour, also under the polar haze, which fades in.
    let inside = warped.clamp(-look.polar + 0.01, look.polar - 0.01);
    let (i, across) = look.band_at(inside).unwrap_or((0, 0.5));
    let band = &look.bands[i];
    let base = if band.zone {
        mix(palette.zone, [0.99, 0.98, 0.95], 0.4 * band.hue)
    } else {
        mix(palette.belt, palette.storm, 0.35 * band.hue)
    };
    let shade = 1.0 + 0.14 * band.tint + 0.1 * streak;
    let mut colour = base.map(|c| c * shade);
    // Soft edges: blend towards the neighbour near each edge.
    let edge = smoothstep(0.0, 0.2, across.min(1.0 - across));
    let other = if band.zone {
        palette.belt
    } else {
        palette.zone
    };
    colour = mix(other, colour, 0.5 + 0.5 * edge);
    let mut biome = if band.zone {
        Biome::GiantZone
    } else {
        Biome::GiantBelt
    };
    let mut elevation = if band.zone { 0.06 } else { -0.06 } * look.relief;
    // Polar regions: the bands give way to the pole's own look.
    let haze = smoothstep(look.polar - 10.0, look.polar + 4.0, warped.abs());
    if haze > 0.0 {
        let pole = &look.poles[usize::from(lat < 0.0)];
        let (polar, storm, lift) = polar_look(look, pole, lon, lat, noise, fine);
        colour = mix(colour, polar, haze);
        elevation = elevation * (1.0 - haze) + lift * haze;
        if haze > 0.5 {
            biome = if storm {
                Biome::GiantStorm
            } else {
                Biome::GiantPolar
            };
        }
    }
    elevation += 0.025 * wobble + 0.01 * streak;

    if let Some((storm, r)) = storm_hit {
        let core = 1.0 - smoothstep(0.55, 1.0, r);
        let collar = (-((r - 1.05) / 0.12).powi(2)).exp();
        colour = [0, 1, 2]
            .map(|k| colour[k] * (1.0 - core) + storm.colour[k] * core * (0.9 + 0.1 * streak));
        colour = colour.map(|c| c * (1.0 + 0.15 * collar));
        if storm.companion {
            // Bright methane clouds over the poleward rim.
            let poleward = (lat - storm.lat) * storm.lat.signum();
            let cloud = (-((r - 1.0) / 0.25).powi(2)).exp() * smoothstep(0.0, 3.0, poleward);
            colour = colour.map(|c| c + (1.0 - c) * 0.8 * cloud);
        }
        if core > 0.5 {
            biome = Biome::GiantStorm;
        }
        elevation += 0.12 * core;
    }

    // Cloud tops: belts deeper and warmer; a hot giant's day side far
    // hotter, its hot spot carried east.
    let mut temperature =
        look.info.effective_k + if biome == Biome::GiantBelt { 3.0 } else { -3.0 };
    if look.info.hot {
        let from_spot = (lon - 180.0 - look.hot_spot_shift).to_radians();
        temperature =
            look.info.effective_k * (1.0 + 0.28 * from_spot.cos() * lat.to_radians().cos());
        // Hotter clouds glow red on the night side in the views; here the
        // day side is a touch lighter where clouds thin out.
        colour = colour.map(|c| c * (1.0 + 0.08 * from_spot.cos()));
    }
    Pixel {
        colour: colour.map(|c| c.clamp(0.0, 1.0)),
        biome,
        elevation,
        temperature: temperature - 273.15,
        wind: look.wind_at(lat),
    }
}

/// The colour of a giant's polar region at `lon`, `lat` (degrees), whether
/// it lies in a cyclone's core, and the height of its cloud tops: drawn in
/// an azimuthal view of the pole (distance from it, `colat`, and longitude).
fn polar_look(
    look: &Look,
    pole: &Pole,
    lon: f32,
    lat: f32,
    noise: &Perlin,
    fine: &Perlin,
) -> ([f32; 3], bool, f32) {
    let palette = &look.info.palette;
    let mix = |a: [f32; 3], b: [f32; 3], f: f32| {
        [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * f.clamp(0.0, 1.0))
    };
    let colat = 90.0 - lat.abs();
    let theta = lon.to_radians() * if lat < 0.0 { -1.0 } else { 1.0 };
    let (px, py) = (colat * theta.cos(), colat * theta.sin());
    // Noise on the polar plane (a seam-free patch around each pole).
    let side = if lat < 0.0 { 40.0 } else { 0.0 };
    let n = |s: f64, octaves| {
        fbm(
            noise,
            f64::from(px) * s + side,
            f64::from(py) * s,
            3.7,
            octaves,
        )
    };
    let mottle = n(0.25, 4);
    let mut colour = palette.polar.map(|c| c * (1.0 + 0.2 * mottle));
    let mut lift = 0.02 * mottle;
    let mut storm = false;
    match pole.style {
        PoleStyle::Haze => {
            // A bright cap of haze over the pole.
            let cap = 1.0 - smoothstep(6.0, 18.0, colat);
            colour = mix(colour, [0.97, 0.98, 1.0], 0.3 * cap);
        }
        PoleStyle::Hexagon => {
            // The jet's six straight sides, at about 78° (12° from the pole).
            let angle = (theta + pole.turn).rem_euclid(std::f32::consts::FRAC_PI_3)
                - std::f32::consts::FRAC_PI_6;
            let hex = 12.0 * std::f32::consts::FRAC_PI_6.cos() / angle.cos();
            let jet = (-((colat - hex) / 1.0).powi(2)).exp();
            colour = mix(colour, palette.belt, 0.55 * jet);
            // Inside, a darker cap round the polar vortex's bright-rimmed eye.
            let inside = 1.0 - smoothstep(hex - 1.0, hex + 0.5, colat);
            colour = mix(
                colour,
                mix(palette.polar, [0.3, 0.42, 0.62], 0.35),
                0.4 * inside,
            );
            let eye = 1.0 - smoothstep(0.8, 1.6, colat);
            let rim = (-((colat - 1.8) / 0.4).powi(2)).exp();
            colour = mix(
                mix(colour, [0.2, 0.22, 0.28], eye),
                [0.95, 0.9, 0.82],
                0.6 * rim,
            );
            lift += 0.05 * jet;
        }
        PoleStyle::Cyclones => {
            // A dark, bluish background of folded filaments and billowing
            // white clouds (Juno's JunoCam).
            let wx = f64::from(px) + 3.0 * f64::from(n(0.15, 3));
            let wy = f64::from(py) + 3.0 * f64::from(n(0.15, 3) + 0.5);
            let folds = fbm(fine, wx * 0.7, wy * 0.7, 1.3, 4);
            let filament = smoothstep(0.6, 0.92, 1.0 - (folds * 3.5).abs());
            let billow = smoothstep(0.2, 0.55, fbm(fine, wx * 0.3 + 9.0, wy * 0.3, 2.1, 5));
            let tawny = smoothstep(0.1, 0.45, n(0.09, 3));
            colour = mix(palette.polar, [0.24, 0.31, 0.44], 0.65);
            colour = mix(colour, [0.52, 0.43, 0.36], 0.45 * tawny);
            colour = mix(colour, [0.94, 0.92, 0.86], 0.7 * filament + 0.55 * billow);
            lift = 0.03 * billow;
            // The cyclones: one on the pole, the others in a ring.
            let mut centres: Vec<(f32, f32)> = Vec::new();
            if pole.central {
                centres.push((0.0, 0.0));
            }
            for k in 0..pole.ring {
                let a = pole.turn + std::f32::consts::TAU * f32::from(k) / f32::from(pole.ring);
                centres.push((pole.ring_colat * a.cos(), pole.ring_colat * a.sin()));
            }
            for (k, &(cx, cy)) in centres.iter().enumerate() {
                let (dx, dy) = (px - cx, py - cy);
                let rho = (dx * dx + dy * dy).sqrt() / pole.radius;
                if rho >= 1.2 {
                    continue;
                }
                // Spiral arms winding in: cream on a dark core, some
                // cyclones filled with cloud, others open.
                let phi = dy.atan2(dx);
                let wind = (rho + 0.04).ln();
                let swirl = n(0.9, 3);
                let arms = (4.0 * phi + 8.0 * wind + 1.5 * swirl).sin() * 0.65
                    + (9.0 * phi + 14.0 * wind + 2.0 * n(1.7, 2)).sin() * 0.35;
                let filled = (k as f32 * 1.618 + pole.turn).fract() < 0.4;
                let bright = if filled {
                    0.72 + 0.28 * arms
                } else {
                    0.45 + 0.55 * arms
                };
                let core_colour = mix([0.3, 0.36, 0.47], [0.96, 0.93, 0.86], bright);
                // A dark eye, and a dark collar where the arms end.
                let eye = 1.0 - smoothstep(0.04, 0.12, rho);
                let collar = (-((rho - 1.0) / 0.1).powi(2)).exp();
                let core_colour = mix(core_colour, [0.36, 0.3, 0.26], 0.7 * eye);
                let weight = 1.0 - smoothstep(0.8, 1.15, rho);
                colour = mix(colour, core_colour, weight);
                colour = mix(colour, [0.18, 0.22, 0.3], 0.35 * collar);
                lift += 0.08 * weight * (1.0 - rho).max(0.0);
                if rho < 0.75 {
                    storm = true;
                }
            }
        }
    }
    (colour, storm, lift)
}

/// Generates the giant world `p`: its cloud bands as the layers of a world.
pub fn generate(p: &WorldParams, progress: &dyn Fn(Stage)) -> Generated {
    let mass = p.giant_mass_earth.unwrap_or(317.8);
    let look = Look::draw(p, mass);
    progress(Stage::Noise);
    let (w, h) = (p.width, p.height);
    let noise = Perlin::new(stream_seed(p.seed, stream::GIANT_LOOK));
    let fine = Perlin::new(stream_seed(p.seed, stream::GIANT_LOOK).wrapping_add(1));
    let grid = p.grid();
    let rows: Vec<Vec<Pixel>> = (0..h)
        .into_par_iter()
        .map(|y| {
            let lat = grid.latitude(y).to_degrees() as f32;
            (0..w)
                .map(|x| {
                    let lon = (x as f32 + 0.5) / w as f32 * 360.0;
                    sample(&look, &noise, &fine, lon, lat)
                })
                .collect()
        })
        .collect();
    progress(Stage::Biomes);
    let n = (w * h) as usize;
    let mut world = World {
        params: p.clone(),
        width: w,
        height: h,
        elevation: Vec::with_capacity(n),
        elevation_base: None,
        moisture: Vec::with_capacity(n),
        temperature: Vec::with_capacity(n),
        flow_km2: vec![0.0; n],
        volcanic_zone: vec![0.0; n],
        coast_distance_km: vec![crate::generation::COAST_DISTANCE_CAP_KM; n],
        floodplain: vec![0.0; n],
        highland_offset: vec![0.0; n],
        biomes: Vec::with_capacity(n),
        rivers: Vec::new(),
        albedo: Vec::new(),
        clouds: Vec::new(),
        surface: Vec::with_capacity(n),
    };
    let reach = look.info.wind_east.max(look.info.wind_west).max(1.0);
    for pixel in rows.into_iter().flatten() {
        world.elevation.push(CLOUD_DECK + pixel.elevation);
        world.temperature.push(pixel.temperature);
        world.biomes.push(pixel.biome);
        world.moisture.push(pixel.wind / reach);
        world
            .surface
            .push(pixel.colour.map(|c| (c * 255.0).round() as u8));
    }
    Generated {
        world,
        river_coverage: vec![0.0; n],
    }
}

/// The colours of the band biomes of the giant world `p`, sRGB bytes, for
/// its `world.json` biome table; `None` for a rocky world.
pub fn biome_colours(p: &WorldParams) -> Option<[(Biome, [u8; 3]); 4]> {
    let mass = p.giant_mass_earth?;
    let palette = Look::draw(p, mass).info.palette;
    let byte = |c: [f32; 3]| c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
    Some([
        (Biome::GiantZone, byte(palette.zone)),
        (Biome::GiantBelt, byte(palette.belt)),
        (Biome::GiantStorm, byte(palette.storm)),
        (Biome::GiantPolar, byte(palette.polar)),
    ])
}

/// What a giant world is, for `world.json`; `None` for a rocky world.
pub fn info(p: &WorldParams) -> Option<GiantInfo> {
    p.giant_mass_earth.map(|mass| Look::draw(p, mass).info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::Preset;

    fn params(preset: &str, seed: u32, mass: f32, orbit_au: f32) -> WorldParams {
        WorldParams {
            width: 128,
            height: 64,
            seed,
            planet: Preset::load(preset).unwrap(),
            sea_level: 0.0,
            volcanic_intensity: 0.0,
            circumference_km: 40_075.0,
            axial_tilt_deg: 3.0,
            erosion: 0.0,
            pressure_bar: 1.0,
            surface_age_ga: 0.0,
            star: crate::star::SUN,
            orbit_au,
            eccentricity: 0.0,
            drift_ma: 0.0,
            moons: crate::moons::MoonsMode::None,
            shape: None,
            metallicity: 0.0,
            giants: crate::giants::GiantsMode::None,
            giant_mass_earth: Some(mass),
            host: crate::exomoon::HostMode::None,
            given_rings: None,
            impact_factor: None,
        }
    }

    #[test]
    fn jupiter_gets_jovian_bands_and_neptune_three_jets() {
        let jupiter = Look::draw(&params("gas-giant", 1, 317.8, 5.2), 317.8).info;
        assert_eq!(jupiter.class, 1);
        assert!((4..=6).contains(&jupiter.jets), "{}", jupiter.jets);
        assert!((jupiter.radius_earth - 11.2).abs() < 0.1);
        assert!((jupiter.density - 1.24).abs() < 0.1, "{}", jupiter.density);
        assert!(
            (0.04..0.09).contains(&jupiter.flattening),
            "{}",
            jupiter.flattening
        );
        let neptune = Look::draw(&params("ice-giant", 2, 17.1, 30.0), 17.1).info;
        assert_eq!(neptune.jets, 1);
        // Retrograde at the equator, prograde at high latitudes.
        let profile = &neptune.wind_profile;
        assert!(
            profile[45] < -100.0 && profile[45 + 30] > 100.0,
            "{profile:?}"
        );
    }

    #[test]
    fn a_saturn_like_giant_gets_saturns_jets_and_the_wind_is_an_outcome() {
        // Saturn: about 20 jets in all, 4J − 1 for J per hemisphere: five
        // to seven, not the four the fixed wind of 100 m/s gave.
        for seed in 0..20 {
            let saturn = Look::draw(&params("gas-giant", seed, 95.16, 9.5), 95.16).info;
            assert!((5..=7).contains(&saturn.jets), "{} {}", seed, saturn.jets);
        }
        // The wind falls with the energy: the same giant far from its star
        // and a young-looking one near it do not blow alike.
        let wind = |mass: f32, au: f32| {
            let i = Look::draw(&params("gas-giant", 1, mass, au), mass).info;
            (i.wind_east.max(i.wind_west), i.effective_k)
        };
        let (cold, cold_k) = wind(317.8, 40.0);
        let (warm, warm_k) = wind(317.8, 2.0);
        assert!(warm_k > cold_k && warm > 1.5 * cold, "{cold} {warm}");
    }

    #[test]
    fn an_ice_giants_winds_follow_its_energy_and_spin_not_one_fixed_profile() {
        let peak = |mass: f32, au: f32, seed: u32| {
            Look::draw(&params("ice-giant", seed, mass, au), mass).wind
        };
        // Neptune-like: close to the 300 m/s the jet count was made with.
        let neptune = peak(17.1, 30.0, 2);
        assert!((200.0..400.0).contains(&neptune), "{neptune}");
        // Colder, further out: slower. Warmer: faster.
        assert!(peak(17.1, 90.0, 2) < 0.9 * neptune);
        assert!(peak(17.1, 8.0, 2) > 1.1 * neptune);
        // The spin matters too: different seeds spin differently.
        let spins: Vec<f32> = (0..30).map(|s| peak(17.1, 30.0, s)).collect();
        let (lo, hi) = spins
            .iter()
            .fold((f32::MAX, 0f32), |(l, h), &w| (l.min(w), h.max(w)));
        assert!(hi > 1.05 * lo, "{lo} {hi}");
    }

    #[test]
    fn the_great_spot_sits_in_a_southern_zone_and_the_edges_follow_the_bands() {
        let mut spots = 0;
        for seed in 0..200 {
            let look = Look::draw(&params("gas-giant", seed, 317.8, 5.2), 317.8);
            let edges = look.band_edges();
            assert_eq!(edges.len(), look.bands.len() + 1);
            assert!(edges.windows(2).all(|w| w[0] < w[1]), "{edges:?}");
            assert!((edges[0] + look.polar).abs() < 1e-3);
            // Zones are the odd bands, the equatorial zone in the middle.
            assert!(
                look.bands
                    .iter()
                    .enumerate()
                    .all(|(i, b)| b.zone == (i % 2 == 1))
            );
            if let Some(spot) = look.storms.first().filter(|_| look.info.great_spot) {
                spots += 1;
                assert!(spot.lat < -10.0, "{}", spot.lat);
                let (i, across) = look.band_at(spot.lat).unwrap();
                assert!(look.bands[i].zone, "seed {seed}: lat {}", spot.lat);
                assert!((0.2..0.8).contains(&across), "{across}");
            }
        }
        assert!(spots > 50, "{spots}");
    }

    #[test]
    fn warmer_ice_giants_keep_a_tinge_of_methane_blue_and_class_v_is_pale() {
        let red = |kind: GiantKind, class: u8| class_palette(kind, class, 0.5, 0.5).zone[0];
        for class in [2, 3] {
            assert!(
                red(GiantKind::Ice, class) < red(GiantKind::Gas, class),
                "{class}"
            );
        }
        // Class V reflects far more than class IV (Sudarsky et al. 2000).
        let lum = |class: u8| {
            let z = class_palette(GiantKind::Gas, class, 0.5, 0.5).zone;
            z[0] + z[1] + z[2]
        };
        assert!(lum(5) > 2.5 * lum(4), "{} {}", lum(5), lum(4));
    }

    #[test]
    fn a_hot_jupiter_is_locked_inflated_and_hot_on_its_day_side() {
        let p = params("hot-jupiter", 3, 300.0, 0.03);
        let look = Look::draw(&p, 300.0);
        assert!(look.info.hot);
        assert!(look.info.radius_earth > 11.8, "{}", look.info.radius_earth);
        assert!(look.info.rotation_hours > 24.0);
        let world = generate(&p, &|_| {}).world;
        let row = (world.height / 2 * world.width) as usize;
        let (noon, midnight) = (world.temperature[row + 64], world.temperature[row]);
        assert!(noon > midnight + 300.0, "{noon} {midnight}");
    }

    #[test]
    fn every_pixel_is_a_band_biome_in_the_giants_colours() {
        let p = params("gas-giant", 5, 120.0, 6.0);
        let world = generate(&p, &|_| {}).world;
        assert_eq!(world.surface.len(), world.biomes.len());
        assert!(world.biomes.iter().all(|b| matches!(
            b,
            Biome::GiantZone | Biome::GiantBelt | Biome::GiantStorm | Biome::GiantPolar
        )));
        let count = |b: Biome| world.biomes.iter().filter(|&&x| x == b).count();
        assert!(
            count(Biome::GiantZone) > 0 && count(Biome::GiantBelt) > 0,
            "{:?}",
            [
                Biome::GiantZone,
                Biome::GiantBelt,
                Biome::GiantStorm,
                Biome::GiantPolar
            ]
            .map(count)
        );
        // The same parameters give the same world.
        assert_eq!(generate(&p, &|_| {}).world.surface, world.surface);
    }

    #[test]
    fn poles_vary_and_cyclones_ring_them_like_jupiters() {
        let mut styles = std::collections::HashMap::new();
        for seed in 0..300 {
            let look = Look::draw(&params("gas-giant", seed, 317.8, 5.2), 317.8);
            for k in 0..2 {
                *styles.entry(look.info.poles[k]).or_insert(0) += 1;
                if look.info.poles[k] == PoleStyle::Cyclones {
                    assert!((5..=9).contains(&look.info.polar_cyclones[k]));
                    // Packed round the pole, within its polar region.
                    let pole = look.poles[k];
                    assert!(pole.ring_colat + pole.radius < 90.0 - look.polar + 10.0);
                } else {
                    assert_eq!(look.info.polar_cyclones[k], 0);
                }
            }
            assert_eq!(
                look.info.hexagon,
                look.info.poles.contains(&PoleStyle::Hexagon)
            );
        }
        let share = |s: PoleStyle| f64::from(styles.get(&s).copied().unwrap_or(0)) / 600.0;
        assert!(
            (0.4..0.7).contains(&share(PoleStyle::Cyclones)),
            "{styles:?}"
        );
        assert!(
            (0.08..0.3).contains(&share(PoleStyle::Hexagon)),
            "{styles:?}"
        );
        // Ice giants: haze or a lone cyclone, no ring; hot giants: haze.
        let ice = Look::draw(&params("ice-giant", 4, 17.1, 30.0), 17.1).info;
        assert_eq!(ice.polar_cyclones, [0, 0]);
        let hot = Look::draw(&params("hot-jupiter", 4, 300.0, 0.03), 300.0).info;
        assert_eq!(hot.poles, [PoleStyle::Haze; 2]);
    }
}
