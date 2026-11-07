//! A system's small bodies (`docs/analysis/13-asteroids.md`): its belts and
//! the trojan swarms at its planets' L4 and L5 points, drawn once the
//! planets are placed, each from a stream of its own so that the planets
//! and moons of a seed do not move.
//!
//! **Inner belt.** Between the rocky planets and the first giant, where
//! the giant's resonances stopped a planet forming: from about 0.40 of its
//! distance (the secular resonance ν6 in the Solar System) to its 2:1 at
//! 0.63, with Kirkwood gaps at the 3:1, 5:2 and 7:3, and the Hildas
//! gathered in the stable 3:2. A system without a giant has none: its
//! planetesimals grew into planets.
//!
//! **Outer belt.** Beyond the outermost giant, from its 3:2 to its 2:1
//! (Neptune's 39 to 48 AU), or at the ice line's 11 to 18 times without a
//! giant; cold belts are seen round one FGK star in five, and ours would
//! not be detected, hence a probability of a half. The **scattered disk**
//! is the part a giant threw onto eccentric orbits.
//!
//! **Realism.** A belt is drawn only where the other planets leave it
//! room (5 Hill radii beyond their periapsis and apoapsis), and only if
//! what is left spans a ratio of 1.15 at least.
//!
//! **Trojans.** L4 and L5 are stable while the planet's mass share stays
//! under Routh's 0.0385; a swarm lasts unless a more massive giant orbits
//! within a period ratio of 3 (Saturn and Uranus have none; Jupiter and
//! Neptune have many; Mars, next to the heavier but rocky Earth, keeps
//! its own).
//!
//! **Craters.** Asteroids from the inner belt and comets from the outer
//! belt and the scattered disk strike the planets; [`impact_factor`]
//! weighs both against the Solar System's.

use crate::noise::{stream, stream_seed};
use crate::star::StarInfo;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

/// Earth masses in a Sun.
const SUN_EARTH: f64 = 332_946.0;
/// Jupiter's mass share round the Sun, for the gaps' widths.
const MU_JUPITER: f64 = 9.546e-4;
/// The Solar System's main belt (Pitjeva & Pitjev 2018), Earth masses.
pub const MAIN_BELT_EARTH: f64 = 4.0e-4;
/// The Solar System's Kuiper belt and scattered disk together, Earth
/// masses (Bernstein et al. 2004 for the classical belt, a few times more
/// for the rest).
pub const COMET_SOURCES_EARTH: f64 = 0.05;
/// Routh's limit on the mass share for stable L4 and L5 points.
pub const ROUTH: f64 = 0.038_520_896_504_551_1;
/// The inner belt's zone round its giant, as shares of its distance.
pub const INNER_ZONE: (f64, f64) = (0.40, 0.63);
/// Hill radii kept clear beyond a planet's periapsis and apoapsis.
const CLEARING_HILL: f64 = 5.0;
/// Smallest outer / inner ratio a belt is kept at.
const MIN_RATIO: f64 = 1.15;

/// What kind of belt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum BeltKind {
    /// Between the rocky planets and the first giant (the main belt).
    Inner,
    /// In the first giant's 3:2 resonance (the Hildas).
    Hildas,
    /// Beyond the outermost giant (the Kuiper belt).
    Outer,
    /// Thrown by the outermost giant onto eccentric orbits.
    Scattered,
}

/// What a belt's bodies are mostly made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum BeltComposition {
    /// Rock and metal, inside the ice line (S types).
    Rocky,
    /// Rocky inside the ice line, carbon-rich and icy beyond it.
    Mixed,
    /// Ices and rock, beyond the ice line.
    Icy,
}

/// A belt of small bodies round the star.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Belt {
    pub kind: BeltKind,
    pub inner_au: f64,
    pub outer_au: f64,
    pub mass_earth: f64,
    /// Kirkwood gaps: centre and width, AU.
    pub gaps: Vec<[f64; 2]>,
    pub composition: BeltComposition,
    /// Typical eccentricity and inclination (to the star's equator) of its
    /// bodies.
    pub eccentricity: f64,
    pub inclination_deg: f64,
    /// The planet whose resonances shape it, by id.
    pub shaped_by: Option<String>,
}

/// A planet's trojans: bodies larger than 1 km round its L4 point (60°
/// ahead) and its L5 point (60° behind).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Trojans {
    pub l4: u32,
    pub l5: u32,
}

/// A planet as the small bodies see it.
#[derive(Clone, Copy, Debug)]
pub struct Track<'a> {
    pub id: &'a str,
    pub body_type: &'a str,
    pub mass_earth: f64,
    pub orbit_au: f64,
    pub eccentricity: f64,
}

impl Track<'_> {
    fn giant(&self) -> bool {
        matches!(self.body_type, "gas-giant" | "ice-giant" | "hot-jupiter")
    }

    /// The span it clears round the star, AU.
    fn clearing(&self, star_mass: f64) -> (f64, f64) {
        let hill = self.orbit_au * (self.mass_earth / (3.0 * star_mass * SUN_EARTH)).cbrt();
        (
            self.orbit_au * (1.0 - self.eccentricity) - CLEARING_HILL * hill,
            self.orbit_au * (1.0 + self.eccentricity) + CLEARING_HILL * hill,
        )
    }
}

/// The inner belt's zone round a giant at `giant_au`, AU: where no rocky
/// planet forms.
pub fn belt_zone(giant_au: f64) -> (f64, f64) {
    (INNER_ZONE.0 * giant_au, INNER_ZONE.1 * giant_au)
}

/// Where a body going round `j` times while the planet at `a` goes round
/// `k` times sits, AU.
fn resonance(a: f64, j: f64, k: f64) -> f64 {
    a * (k / j).powf(2.0 / 3.0)
}

/// Whether a planet can shape an inner belt: a giant that is not a hot
/// Jupiter, at half the ice line or beyond.
pub fn cold_giant(body_type: &str, orbit_au: f64, ice_au: f64) -> bool {
    matches!(body_type, "gas-giant" | "ice-giant") && orbit_au >= 0.5 * ice_au
}

/// The giant that shapes an inner belt: the innermost cold one (see
/// [`cold_giant`]).
pub fn first_cold_giant<'a>(planets: &'a [Track<'a>], ice_au: f64) -> Option<&'a Track<'a>> {
    planets
        .iter()
        .filter(|p| cold_giant(p.body_type, p.orbit_au, ice_au))
        .min_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au))
}

/// The widest part of `[lo, hi]` that `planets` (but `except`) leave
/// clear, if its outer / inner ratio is [`MIN_RATIO`] at least.
fn clear_span(
    lo: f64,
    hi: f64,
    planets: &[Track],
    except: Option<&str>,
    star_mass: f64,
) -> Option<(f64, f64)> {
    let mut pieces = vec![(lo, hi)];
    for p in planets.iter().filter(|p| Some(p.id) != except) {
        let (a, b) = p.clearing(star_mass);
        pieces = pieces
            .into_iter()
            .flat_map(|(x, y)| {
                if b <= x || a >= y {
                    vec![(x, y)]
                } else {
                    [(x, a.min(y)), (b.max(x), y)]
                        .into_iter()
                        .filter(|(u, v)| v > u)
                        .collect()
                }
            })
            .collect();
    }
    pieces
        .into_iter()
        .filter(|(x, y)| *x > 0.0 && y / x >= MIN_RATIO)
        .max_by(|a, b| (a.1 / a.0).total_cmp(&(b.1 / b.0)))
}

fn composition(inner: f64, outer: f64, ice: f64) -> BeltComposition {
    if outer <= ice {
        BeltComposition::Rocky
    } else if inner >= ice {
        BeltComposition::Icy
    } else {
        BeltComposition::Mixed
    }
}

fn log_uniform(lo: f64, hi: f64, rng: &mut impl RngExt) -> f64 {
    (lo.ln() + rng.random::<f64>() * (hi / lo).ln()).exp()
}

/// The chance of an outer belt round a star of this spectral class.
fn outer_odds(star: &StarInfo) -> f64 {
    match star.kind.chars().next() {
        Some('F' | 'G' | 'K') => 0.5,
        Some('M') => 0.25,
        _ => 0.3,
    }
}

/// The belts of a system round `star` (ice line `ice_au`), from its
/// planets, nearest first.
pub fn draw_belts(star: &StarInfo, ice_au: f64, planets: &[Track], seed: u32) -> Vec<Belt> {
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(seed, stream::BELTS)));
    let star_mass = f64::from(star.mass);
    let mut belts = Vec::new();
    // Draws in a fixed order, whatever is kept, so one belt's presence does
    // not move another's.
    let inner_mass = log_uniform(1e-5, 1e-2, &mut rng);
    let outer_roll = rng.random::<f64>();
    let outer_mass = log_uniform(1e-3, 1.0, &mut rng);
    let scattered_share = log_uniform(0.3, 3.0, &mut rng);

    if let Some(giant) = first_cold_giant(planets, ice_au) {
        let (lo, hi) = belt_zone(giant.orbit_au);
        let floor = 0.05 * f64::from(star.luminosity).sqrt();
        if let Some((inner, outer)) =
            clear_span(lo.max(floor), hi, planets, Some(giant.id), star_mass)
        {
            let mu = giant.mass_earth / (star_mass * SUN_EARTH + giant.mass_earth);
            let gaps = [(3.0, 1.0), (5.0, 2.0), (7.0, 3.0)]
                .iter()
                .map(|&(j, k)| resonance(giant.orbit_au, j, k))
                .filter(|&c| c > inner && c < outer)
                .map(|c| [c, 0.012 * c * (mu / MU_JUPITER).sqrt()])
                .collect();
            belts.push(Belt {
                kind: BeltKind::Inner,
                inner_au: inner,
                outer_au: outer,
                mass_earth: inner_mass,
                gaps,
                composition: composition(inner, outer, ice_au),
                eccentricity: 0.15,
                inclination_deg: 10.0,
                shaped_by: Some(giant.id.to_string()),
            });
            // The Hildas, in the giant's 3:2, where nothing else orbits.
            let centre = resonance(giant.orbit_au, 3.0, 2.0);
            let (h_lo, h_hi) = (0.97 * centre, 1.035 * centre);
            let crossed = planets.iter().filter(|p| p.id != giant.id).any(|p| {
                let (a, b) = p.clearing(star_mass);
                a < h_hi && b > h_lo
            });
            if !crossed {
                belts.push(Belt {
                    kind: BeltKind::Hildas,
                    inner_au: h_lo,
                    outer_au: h_hi,
                    mass_earth: 1e-3 * inner_mass,
                    gaps: Vec::new(),
                    composition: composition(h_lo, h_hi, ice_au),
                    eccentricity: 0.15,
                    inclination_deg: 8.0,
                    shaped_by: Some(giant.id.to_string()),
                });
            }
        }
    }

    if outer_roll < outer_odds(star) {
        let last = planets
            .iter()
            .filter(|p| p.giant() && p.body_type != "hot-jupiter")
            .max_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au));
        let (lo, hi) = match last {
            Some(giant) => (
                resonance(giant.orbit_au, 2.0, 3.0),
                resonance(giant.orbit_au, 1.0, 2.0),
            ),
            None => (11.0 * ice_au, 18.0 * ice_au),
        };
        if let Some((inner, outer)) = clear_span(lo, hi, planets, None, star_mass) {
            belts.push(Belt {
                kind: BeltKind::Outer,
                inner_au: inner,
                outer_au: outer,
                mass_earth: outer_mass,
                gaps: Vec::new(),
                composition: composition(inner, outer, ice_au),
                eccentricity: 0.08,
                inclination_deg: 6.0,
                shaped_by: last.map(|g| g.id.to_string()),
            });
            if let Some(giant) = last {
                belts.push(Belt {
                    kind: BeltKind::Scattered,
                    inner_au: giant.orbit_au,
                    outer_au: 4.0 * giant.orbit_au,
                    mass_earth: outer_mass * scattered_share,
                    gaps: Vec::new(),
                    composition: composition(giant.orbit_au, 4.0 * giant.orbit_au, ice_au),
                    eccentricity: 0.3,
                    inclination_deg: 15.0,
                    shaped_by: Some(giant.id.to_string()),
                });
            }
        }
    }
    belts
}

/// The trojans of each planet of `planets` round a star of `star_mass`
/// Suns, in the same order (none where they cannot last).
pub fn draw_trojans(star_mass: f64, planets: &[Track], seed: u32) -> Vec<Option<Trojans>> {
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(seed, stream::TROJANS)));
    let outermost = planets
        .iter()
        .filter(|p| p.giant())
        .map(|p| p.orbit_au)
        .fold(0.0f64, f64::max);
    planets
        .iter()
        .map(|p| {
            // Two draws per planet, kept or not, so that one planet's swarm
            // does not move another's.
            let (u, r) = (rng.random::<f64>(), 0.7 + 0.75 * rng.random::<f64>());
            let mu = p.mass_earth / (star_mass * SUN_EARTH + p.mass_earth);
            // Only a heavier giant empties the points: Jupiter Saturn's,
            // Saturn Uranus's. A rocky neighbour does not (the Earth leaves
            // Mars its trojans, Christou et al. 2025).
            let perturbed = planets.iter().any(|q| {
                q.id != p.id && q.giant() && q.mass_earth > p.mass_earth && {
                    let ratio = (q.orbit_au / p.orbit_au).max(p.orbit_au / q.orbit_au);
                    ratio.powf(1.5) < 3.0
                }
            });
            if mu >= ROUTH || perturbed {
                return None;
            }
            let between = |lo: f64, hi: f64| (lo.ln() + u * (hi / lo).ln()).exp();
            let count = match p.body_type {
                "gas-giant" => between(1e4, 1e6),
                "ice-giant" if p.orbit_au >= outermost => between(1e4, 1e6),
                "ice-giant" => between(1e3, 1e5),
                "hot-jupiter" => between(1.0, 1e3) - 1.0,
                _ => between(1.0, 101.0) - 1.0,
            }
            .round() as u32;
            let l4 = (f64::from(count) * r / (1.0 + r)).round() as u32;
            Some(Trojans { l4, l5: count - l4 })
        })
        .collect()
}

/// The crater rate of a body at `orbit_au` relative to the Solar System's:
/// the giant beyond it stirring the impactors (`giants::impact_factor`),
/// times the impactors there are, asteroids for four fifths of the
/// Solar System's and comets for the rest.
pub fn impact_factor(belts: &[Belt], planets: &[Track], orbit_au: f64) -> f32 {
    let giant = planets
        .iter()
        .filter(|p| p.giant() && p.orbit_au > orbit_au)
        .min_by(|a, b| a.orbit_au.total_cmp(&b.orbit_au))
        .map(|g| (g.mass_earth / f64::from(crate::giants::JUPITER_EARTH_MASSES)) as f32);
    let curve = crate::giants::impact_factor(giant);
    let mass = |kinds: &[BeltKind]| -> f64 {
        belts
            .iter()
            .filter(|b| kinds.contains(&b.kind))
            .map(|b| b.mass_earth)
            .sum()
    };
    let asteroids = (mass(&[BeltKind::Inner, BeltKind::Hildas]) / MAIN_BELT_EARTH).clamp(0.0, 10.0);
    let comets =
        (mass(&[BeltKind::Outer, BeltKind::Scattered]) / COMET_SOURCES_EARTH).clamp(0.0, 10.0);
    curve * (0.8 * asteroids + 0.2 * comets).max(0.05) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sun() -> StarInfo {
        StarInfo::new(crate::star::parse_type("G2V").unwrap(), 1.0, 0.0)
    }

    fn solar() -> Vec<Track<'static>> {
        vec![
            Track {
                id: "b",
                body_type: "barren",
                mass_earth: 0.055,
                orbit_au: 0.387,
                eccentricity: 0.2,
            },
            Track {
                id: "c",
                body_type: "terran",
                mass_earth: 0.815,
                orbit_au: 0.723,
                eccentricity: 0.007,
            },
            Track {
                id: "d",
                body_type: "terran",
                mass_earth: 1.0,
                orbit_au: 1.0,
                eccentricity: 0.017,
            },
            Track {
                id: "e",
                body_type: "barren",
                mass_earth: 0.107,
                orbit_au: 1.524,
                eccentricity: 0.093,
            },
            Track {
                id: "f",
                body_type: "gas-giant",
                mass_earth: 317.8,
                orbit_au: 5.2,
                eccentricity: 0.049,
            },
            Track {
                id: "g",
                body_type: "gas-giant",
                mass_earth: 95.2,
                orbit_au: 9.58,
                eccentricity: 0.057,
            },
            Track {
                id: "h",
                body_type: "ice-giant",
                mass_earth: 14.5,
                orbit_au: 19.2,
                eccentricity: 0.046,
            },
            Track {
                id: "i",
                body_type: "ice-giant",
                mass_earth: 17.1,
                orbit_au: 30.1,
                eccentricity: 0.01,
            },
        ]
    }

    #[test]
    fn a_solar_system_has_its_main_belt_with_kirkwood_gaps_and_hildas() {
        let belts = draw_belts(&sun(), 2.7, &solar(), 1);
        let inner = belts
            .iter()
            .find(|b| b.kind == BeltKind::Inner)
            .expect("an inner belt");
        assert!((2.0..2.2).contains(&inner.inner_au), "{}", inner.inner_au);
        assert!((3.2..3.4).contains(&inner.outer_au), "{}", inner.outer_au);
        let centres: Vec<f64> = inner.gaps.iter().map(|g| g[0]).collect();
        for (want, got) in [2.50, 2.82, 2.95].iter().zip(&centres) {
            assert!((want - got).abs() < 0.02, "{centres:?}");
        }
        // About 0.03 AU wide at the 3:1, as observed.
        assert!((0.02..0.04).contains(&inner.gaps[0][1]));
        assert_eq!(inner.shaped_by.as_deref(), Some("f"));
        assert_eq!(inner.composition, BeltComposition::Mixed);
        let hildas = belts
            .iter()
            .find(|b| b.kind == BeltKind::Hildas)
            .expect("the Hildas");
        assert!((hildas.inner_au + hildas.outer_au) / 2.0 > 3.9);
    }

    #[test]
    fn no_giant_no_inner_belt() {
        let rocky: Vec<Track> = solar().into_iter().filter(|p| !p.giant()).collect();
        for seed in 0..50 {
            assert!(
                draw_belts(&sun(), 2.7, &rocky, seed)
                    .iter()
                    .all(|b| b.kind == BeltKind::Outer)
            );
        }
    }

    #[test]
    fn a_planet_in_the_zone_leaves_the_belt_only_what_it_does_not_clear() {
        let mut planets = solar();
        planets.push(Track {
            id: "x",
            body_type: "terran",
            mass_earth: 1.0,
            orbit_au: 2.4,
            eccentricity: 0.0,
        });
        let inner = draw_belts(&sun(), 2.7, &planets, 1)
            .into_iter()
            .find(|b| b.kind == BeltKind::Inner)
            .unwrap();
        assert!(inner.inner_au > 2.4, "{inner:?}");
        // A planet in the middle of a narrow zone leaves nothing wide enough.
        let mut planets = solar();
        planets.retain(|p| p.id != "e");
        planets.push(Track {
            id: "x",
            body_type: "terran",
            mass_earth: 5.0,
            orbit_au: 2.65,
            eccentricity: 0.15,
        });
        assert!(
            draw_belts(&sun(), 2.7, &planets, 1)
                .iter()
                .all(|b| b.kind != BeltKind::Inner)
        );
    }

    #[test]
    fn the_outer_belt_lies_between_the_last_giants_three_two_and_two_one() {
        let seed = (0..100)
            .find(|&s| {
                draw_belts(&sun(), 2.7, &solar(), s)
                    .iter()
                    .any(|b| b.kind == BeltKind::Outer)
            })
            .expect("half the seeds have one");
        let belts = draw_belts(&sun(), 2.7, &solar(), seed);
        let outer = belts.iter().find(|b| b.kind == BeltKind::Outer).unwrap();
        assert!(
            (outer.inner_au - 39.4).abs() < 0.5 && (outer.outer_au - 47.8).abs() < 0.5,
            "{outer:?}"
        );
        assert!(belts.iter().any(|b| b.kind == BeltKind::Scattered));
        assert_eq!(outer.composition, BeltComposition::Icy);
    }

    #[test]
    fn trojans_need_stable_points_and_no_heavier_giant() {
        let trojans = draw_trojans(1.0, &solar(), 3);
        // Jupiter and Neptune keep theirs; Saturn and Uranus lose them to
        // a heavier giant.
        assert!(trojans[4].is_some_and(|t| t.l4 + t.l5 >= 10_000));
        assert!(trojans[5].is_none() && trojans[6].is_none());
        assert!(trojans[7].is_some_and(|t| t.l4 + t.l5 >= 10_000));
        // The rocky planets keep a handful at most, Mars too: its heavier
        // neighbour, the Earth, is no giant.
        for (rocky, swarm) in trojans.iter().enumerate().take(4) {
            assert!(swarm.is_some_and(|t| t.l4 + t.l5 <= 100), "{rocky}");
        }
        // A giant of 13 Jupiters round a small red dwarf passes Routh's limit.
        let heavy = [Track {
            id: "b",
            body_type: "gas-giant",
            mass_earth: 4131.0,
            orbit_au: 1.0,
            eccentricity: 0.0,
        }];
        assert_eq!(draw_trojans(0.08, &heavy, 3), vec![None]);
    }

    #[test]
    fn the_solar_system_strikes_at_its_own_rate() {
        let belts = vec![
            Belt {
                kind: BeltKind::Inner,
                inner_au: 2.1,
                outer_au: 3.3,
                mass_earth: MAIN_BELT_EARTH,
                gaps: Vec::new(),
                composition: BeltComposition::Mixed,
                eccentricity: 0.15,
                inclination_deg: 10.0,
                shaped_by: None,
            },
            Belt {
                kind: BeltKind::Outer,
                inner_au: 39.0,
                outer_au: 48.0,
                mass_earth: COMET_SOURCES_EARTH,
                gaps: Vec::new(),
                composition: BeltComposition::Icy,
                eccentricity: 0.08,
                inclination_deg: 6.0,
                shaped_by: None,
            },
        ];
        assert!((impact_factor(&belts, &solar(), 1.0) - 1.0).abs() < 1e-5);
        // Without belts, only a twentieth of the impactors are left.
        assert!((impact_factor(&[], &solar(), 1.0) - 0.05).abs() < 1e-5);
    }

    #[test]
    fn the_same_seed_draws_the_same_small_bodies() {
        assert_eq!(
            draw_belts(&sun(), 2.7, &solar(), 9),
            draw_belts(&sun(), 2.7, &solar(), 9)
        );
        assert_eq!(
            draw_trojans(1.0, &solar(), 9),
            draw_trojans(1.0, &solar(), 9)
        );
    }
}
