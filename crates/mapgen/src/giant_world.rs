//! Giant planets as worlds of their own (backlog item 14, step B;
//! `docs/analysis/12-gas-giants.md`): no ground, but the top of the clouds,
//! banded by the planet's spin.
//!
//! **Bands.** Fast rotation turns the flow into zonal jets; their width
//! follows the Rhines scale `√(2U/β)` (Scott & Polvani 2007), so the number
//! of jets in each hemisphere grows as `√(ΩR/U)`: about five for Jupiter,
//! four for Saturn, one for an ice giant (whose winds are fast and broad:
//! retrograde at the equator, prograde at high latitudes), one or two for a
//! locked hot Jupiter (its day is days long). Between the jets, pale zones
//! (rising air, high ammonia clouds) alternate with dark belts (sinking
//! air, deeper clouds), each hemisphere drawn on its own.
//!
//! **Texture.** The latitude a pixel samples is shifted by noise stretched
//! along the bands, more where neighbouring jets shear (festoons and
//! wavy edges); long-lived vortices swirl the bands around them (a great
//! red spot on some gas giants, white ovals, dark spots with bright
//! companion clouds on ice giants); the poles are hazier and mottled,
//! with a hexagon on some (Saturn's).
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
use crate::giants::{GiantKind, cloud_class, giant_radius};
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
/// Typical wind speeds setting the band count (m/s): Jupiter's jets, the
/// broader ones of ice giants, a hot Jupiter's superrotation.
const GAS_WIND: f64 = 100.0;
const ICE_WIND: f64 = 300.0;
const HOT_WIND: f64 = 2000.0;
/// Jets per hemisphere at `√(ΩR/U) = 1`: Jupiter (`ΩR/U` ≈ 126) gets five.
const JET_FACTOR: f64 = 0.45;

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
    /// Jets in each hemisphere (the Rhines count).
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
    /// Typical wind speed, m/s, and the equatorial jet (gas giants).
    wind: f32,
    equatorial_jet: f32,
    /// Jets at the band edges: latitude, signed speed, half-width.
    jets_at: Vec<(f32, f32, f32)>,
    /// Eastward shift of a hot giant's hot spot from the substellar point, degrees.
    hot_spot_shift: f32,
}

/// Radius of a giant world, Earth radii, from its parameters.
pub fn radius_earth(p: &WorldParams, mass_earth: f32) -> f32 {
    let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity);
    giant_radius(mass_earth, star.equilibrium_k)
}

/// The colours of a giant of `kind` and cloud `class`, varied by `t` and
/// `u` in `[0, 1]`: Jupiter's cream and ochre to Saturn's pale gold, cyan
/// to deep blue ice giants, white water clouds, azure clear skies, dark
/// roasters, grey silicate clouds.
fn palette(kind: GiantKind, class: u8, t: f32, u: f32) -> Palette {
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
            [0.4, 0.38, 0.34],
            [0.25, 0.22, 0.2],
            [0.48, 0.42, 0.36],
            [0.3, 0.28, 0.26],
        ),
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
        let internal = match kind {
            GiantKind::Gas => 100.0 * (mass_earth / 317.8).powf(0.4),
            GiantKind::Ice => 40.0,
        };
        let effective = (equilibrium.powi(4) + internal.powi(4)).powf(0.25);
        let class = cloud_class(effective);

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

        let wind = match (hot, kind) {
            (true, _) => HOT_WIND * (f64::from(equilibrium) / 1500.0).clamp(0.6, 2.5),
            (_, GiantKind::Gas) => GAS_WIND,
            (_, GiantKind::Ice) => ICE_WIND,
        };
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
        let equatorial_jet = match (hot, kind) {
            (false, GiantKind::Gas) => rng.random_range(0.8..4.0) * wind as f32,
            _ => 0.0,
        };

        // Storms: a great spot on some gas giants and white ovals on most;
        // dark spots with bright companions on some ice giants.
        let mut storms = Vec::new();
        let colours = palette(kind, class, rng.random(), rng.random());
        let mut great_spot = false;
        if !hot {
            match kind {
                GiantKind::Gas => {
                    if rng.random::<f32>() < 0.45 {
                        great_spot = true;
                        let size = rng.random_range(0.7..1.3);
                        storms.push(Storm {
                            lon: rng.random_range(0.0..360.0),
                            lat: -rng.random_range(17.0..26.0),
                            half_lon: 11.0 * size,
                            half_lat: 5.5 * size,
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

        let rings = {
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
                })
            } else {
                None
            }
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
                palette: colours,
                rings,
            },
            bands,
            storms,
            polar,
            wind: wind as f32,
            equatorial_jet,
            jets_at,
            hot_spot_shift: if hot {
                rng.random_range(10.0..35.0)
            } else {
                0.0
            },
        };
        look.info.wind_profile = (0..91)
            .map(|k| look.wind_at(-90.0 + 2.0 * k as f32))
            .collect();
        look.info.wind_east = look.info.wind_profile.iter().copied().fold(0.0, f32::max);
        look.info.wind_west = -look.info.wind_profile.iter().copied().fold(0.0, f32::min);
        look
    }

    /// The eastward wind at latitude `lat` (degrees), m/s.
    pub fn wind_at(&self, lat: f32) -> f32 {
        let polar_fade = 1.0 - smoothstep(80.0, 90.0, lat.abs());
        if self.info.hot {
            return self.wind * (-(lat / 25.0).powi(2)).exp() * polar_fade;
        }
        match self.info.kind {
            GiantKind::Ice => {
                let s = (lat.to_radians() * 1.3).sin();
                self.wind * (1.6 * s * s - 0.8) * polar_fade
            }
            GiantKind::Gas => {
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
    let mut elevation = if band.zone { 0.06 } else { -0.06 };
    // Polar regions: mottled haze and small cyclones, fading in.
    let haze = smoothstep(look.polar - 10.0, look.polar + 4.0, warped.abs());
    if haze > 0.0 {
        let mottle = fbm(noise, x * 8.0, y * 8.0, z * 8.0, 4);
        let polar = palette.polar.map(|c| c * (1.0 + 0.2 * mottle));
        colour = mix(colour, polar, haze);
        elevation = elevation * (1.0 - haze) + 0.02 * mottle * haze;
        if haze > 0.5 {
            biome = Biome::GiantPolar;
        }
    }
    // A hexagonal jet around the north pole.
    if look.info.hexagon && lat > 70.0 {
        let angle = (lon.to_radians()).rem_euclid(std::f32::consts::FRAC_PI_3)
            - std::f32::consts::FRAC_PI_6;
        let hex = 90.0 - (90.0 - 78.0) * (std::f32::consts::FRAC_PI_6.cos() / angle.cos());
        let ring = (-((lat - hex) / 1.2).powi(2)).exp();
        colour = [0, 1, 2].map(|k| colour[k] * (1.0 - 0.35 * ring) + palette.belt[k] * 0.35 * ring);
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
}
