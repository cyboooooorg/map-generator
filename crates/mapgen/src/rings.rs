//! The rings of a giant of a planetary system and the moons in and next to
//! them (backlog item 18; `docs/analysis/14-rings.md`).
//!
//! The system draws a giant's rings with [`crate::giant_world::draw_rings`]
//! (see `crate::system`); [`shape`] then makes them what the Solar System's
//! are:
//!
//! - **Bright rings end at the Roche limit.** Their outer edge is the fluid
//!   Roche limit of their icy particles (0.5 to 0.9 g/cm³) round the giant,
//!   `2.44 R (ρ_giant / ρ_particles)^(1/3)`, kept within 2.0 to 2.5 radii
//!   (Saturn's A ring ends at 2.27).
//! - **Gaps belong to moons.** A moonlet of 4 to 30 km in the outer part of
//!   the rings clears a gap round its orbit, 7 to 17 of its Hill radii wide,
//!   wider for the larger (Daphnis keeps the 40 km Keeler gap, Pan the
//!   325 km Encke gap). A mid-sized icy moon just outside the rings (Mimas)
//!   opens a broad gap at its 2:1 resonance, `0.63` of its distance (the
//!   Cassini Division).
//! - **Edge moons and shepherds.** Small moons of 10 to 100 km just beyond
//!   a bright ring's outer edge (Atlas, Prometheus, Pandora, Janus), and a
//!   pair of 10 to 25 km on either side of a faint narrow ring (Cordelia and
//!   Ophelia round Uranus's ε ring).
//!
//! Everything comes from a stream of its own, so a system's other bodies
//! keep their draws. A giant whose regular moons would not fit (a hot one,
//! its Hill sphere too small) keeps its rings without moons.
use crate::giant_world::RingSystem;
use crate::noise::{stream, stream_seed};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

/// What a ring moon does to its giant's rings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum RingRole {
    /// Inside the rings, clearing a gap round its orbit (Pan, Daphnis).
    Embedded,
    /// Outside the rings, opening a gap at its 2:1 resonance (Mimas).
    Resonant,
    /// Just beyond the outer edge, keeping it sharp (Atlas, Prometheus).
    Edge,
    /// One of a pair confining a narrow ring (Cordelia, Ophelia).
    Shepherd,
}

/// A ring moon as the system stores it, before its orbit and its world are
/// filled in.
#[derive(Clone, Debug, PartialEq)]
pub struct RingMoon {
    pub role: RingRole,
    /// Distance from the giant's centre, in its radii.
    pub a_radii: f64,
    /// Mean radius, km, and semi-axes over it (all 1 when round).
    pub radius_km: f64,
    pub shape: [f64; 3],
    /// g/cm³.
    pub density: f64,
    pub seed: u32,
    /// The gap it opens, an index into the rings' `gaps`.
    pub gap: Option<usize>,
}

/// Density of the icy particles of bright rings, g/cm³ (porous ice).
const PARTICLE_DENSITY: (f64, f64) = (0.5, 0.9);
/// Where a bright ring's outer edge may lie, in the giant's radii.
const OUTER_EDGE: (f64, f64) = (2.0, 2.5);
/// Diameter of a gap-clearing moonlet, km (Daphnis 8, Pan 28).
const MOONLET_KM: (f64, f64) = (4.0, 30.0);
/// A gap's width in Hill radii of its moonlet, from the lightest moonlet
/// (Daphnis, 8 × 10¹³ kg: about 7.5) to the heaviest (Pan, 5 × 10¹⁵ kg:
/// about 17), interpolated on the logarithm of the mass.
const GAP_HILL: (f64, f64) = (7.5, 17.0);
const GAP_MASS_KG: (f64, f64) = (8e13, 5e15);
/// Odds of a mid-sized moon opening a resonance gap (Mimas): most bright
/// rings have one.
const RESONANT_ODDS: f64 = 0.6;
/// Width of a resonance gap, in the giant's radii (the Cassini Division:
/// 4,700 km, 0.078).
const RESONANCE_GAP: (f64, f64) = (0.04, 0.09);
/// Odds of a shepherd pair round a faint ring.
const SHEPHERD_ODDS: f64 = 0.7;
/// Gravitational constant, SI.
const G: f64 = 6.674e-11;

/// The fluid Roche limit of particles of `density` round a giant of
/// `giant_density` (g/cm³), in its radii.
pub fn fluid_roche(giant_density: f64, density: f64) -> f64 {
    2.44 * (giant_density / density).cbrt()
}

/// The rigid Roche limit: where a solid moonlet of `density` holds.
pub fn rigid_roche(giant_density: f64, density: f64) -> f64 {
    1.26 * (giant_density / density).cbrt()
}

/// A giant's `rings` (as `draw_rings` drew them) shaped by its density
/// (g/cm³) and its mass (kg), with the moons that make
/// them: the rings with their outer edge and gaps set, and the moons,
/// nearest first. `moons` is false when the giant keeps no moons. `seed` is
/// the giant's.
pub fn shape(
    rings: RingSystem,
    giant_density: f64,
    giant_mass_kg: f64,
    moons: bool,
    seed: u32,
) -> (RingSystem, Vec<RingMoon>) {
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(seed, stream::RING_MOONS)));
    let mut rings = rings;
    let mut out = Vec::new();
    // Each moon's own seed, from the stream's seed: no other stream's.
    let base = stream_seed(seed, stream::RING_MOONS);
    let moon_seed = |k: u32| stream_seed(base, k);
    if rings.bright {
        let particles = rng.random_range(PARTICLE_DENSITY.0..PARTICLE_DENSITY.1);
        let outer = fluid_roche(giant_density, particles).clamp(OUTER_EDGE.0, OUTER_EDGE.1);
        let inner = f64::from(rings.inner);
        rings.outer = outer as f32;
        rings.gaps.clear();
        if !moons {
            return (rings, out);
        }
        // Moonlets in the outer third, each with its gap.
        let count = match rng.random::<f64>() {
            r if r < 0.3 => 0,
            r if r < 0.75 => 1,
            _ => 2,
        };
        for _ in 0..count {
            let diameter = (MOONLET_KM.0.ln()
                + rng.random::<f64>() * (MOONLET_KM.1 / MOONLET_KM.0).ln())
            .exp();
            let density = rng.random_range(0.35..0.6);
            let from = rigid_roche(giant_density, density).max(inner + 0.6 * (outer - inner));
            let to = outer - 0.03;
            let at = rng.random::<f64>();
            let shape_draws = [rng.random::<f64>(), rng.random::<f64>()];
            if from >= to {
                continue;
            }
            let a = from + at * (to - from);
            let radius_km = diameter / 2.0;
            let mass = 4.0 / 3.0 * std::f64::consts::PI * (radius_km * 1e3).powi(3) * density * 1e3;
            let hill = a * (mass / (3.0 * giant_mass_kg)).cbrt();
            let k = ((mass.ln() - GAP_MASS_KG.0.ln()) / (GAP_MASS_KG.1 / GAP_MASS_KG.0).ln())
                .clamp(0.0, 1.0);
            let width = hill * (GAP_HILL.0 + k * (GAP_HILL.1 - GAP_HILL.0));
            // Gaps do not overlap.
            if rings
                .gaps
                .iter()
                .any(|g| (f64::from(g[0]) - a).abs() < (f64::from(g[1]) + width) / 2.0 + 0.02)
            {
                continue;
            }
            rings.gaps.push([a as f32, width as f32]);
            out.push(RingMoon {
                role: RingRole::Embedded,
                a_radii: a,
                radius_km,
                shape: irregular(radius_km, shape_draws),
                density,
                seed: moon_seed(out.len() as u32),
                gap: Some(rings.gaps.len() - 1),
            });
        }
        // A mid-sized moon outside, its 2:1 resonance inside the rings.
        let resonant = rng.random::<f64>() < RESONANT_ODDS;
        let radius_km = rng.random_range(150.0..250.0);
        let density = rng.random_range(1.1..1.3);
        let at = rng.random::<f64>();
        let width = rng.random_range(RESONANCE_GAP.0..RESONANCE_GAP.1);
        let from = (outer + 0.5).max(fluid_roche(giant_density, density) + 0.1);
        let to = (outer - 0.1) / RESONANCE_FACTOR;
        let mut resonant_at = None;
        if resonant && from < to {
            let a = from + at * (to - from);
            let gap_at = a * RESONANCE_FACTOR;
            if gap_at - width / 2.0 > inner + 0.1 {
                rings.gaps.push([gap_at as f32, width as f32]);
                resonant_at = Some(a);
                out.push(RingMoon {
                    role: RingRole::Resonant,
                    a_radii: a,
                    radius_km,
                    shape: [1.0; 3],
                    density,
                    seed: moon_seed(out.len() as u32),
                    gap: Some(rings.gaps.len() - 1),
                });
            }
        }
        // Small moons just beyond the outer edge.
        let edge_count = rng.random_range(1..4);
        for k in 0..edge_count {
            let radius_km = (10f64.ln() + rng.random::<f64>() * 10f64.ln()).exp();
            let density = rng.random_range(0.4..0.7);
            let a = outer * rng.random_range(1.01..1.25) * (1.0 + 0.02 * k as f64);
            let shape_draws = [rng.random::<f64>(), rng.random::<f64>()];
            if resonant_at.is_some_and(|m| a > m - 0.15) || a < rigid_roche(giant_density, density)
            {
                continue;
            }
            out.push(RingMoon {
                role: RingRole::Edge,
                a_radii: a,
                radius_km,
                shape: irregular(radius_km, shape_draws),
                density,
                seed: moon_seed(out.len() as u32),
                gap: None,
            });
        }
    } else if moons && rng.random::<f64>() < SHEPHERD_ODDS {
        // A pair confining the narrow ring.
        for side in [-1.0, 1.0] {
            let radius_km = rng.random_range(10.0..25.0);
            let density = rng.random_range(0.5..1.0);
            let offset = rng.random_range(0.01..0.04);
            let shape_draws = [rng.random::<f64>(), rng.random::<f64>()];
            let edge = if side < 0.0 { rings.inner } else { rings.outer };
            let a = f64::from(edge) + side * offset;
            if a < rigid_roche(giant_density, density) {
                continue;
            }
            out.push(RingMoon {
                role: RingRole::Shepherd,
                a_radii: a,
                radius_km,
                shape: irregular(radius_km, shape_draws),
                density,
                seed: moon_seed(out.len() as u32),
                gap: None,
            });
        }
    }
    out.sort_by(|x, y| x.a_radii.total_cmp(&y.a_radii));
    (rings, out)
}

/// The 2:1 resonance of a moon lies at `2^(-2/3)` of its distance.
pub const RESONANCE_FACTOR: f64 = 0.629_960_5;

/// Semi-axes over the mean radius of a small icy moon of `radius_km`, as
/// the moons of rocky planets are drawn (`crate::moons`): the smaller, the
/// more unequal; Pan and Atlas are flattened by ring material on their
/// equator, which this does not model.
fn irregular(radius_km: f64, draws: [f64; 2]) -> [f64; 3] {
    let limit = 400.0;
    let irregular = (1.0 - 2.0 * radius_km / limit).clamp(0.2, 1.0);
    let b = 1.0 - irregular * (0.05 + 0.25 * draws[0]);
    let c = b - irregular * (0.05 + 0.25 * draws[1]);
    let scale = 1.0 / (b * c).cbrt();
    [scale, scale * b, scale * c]
}

/// The Hill radius of a moon of `mass_kg` at `a_radii` round a giant of
/// `giant_mass_kg`, in the giant's radii.
pub fn hill_radii(a_radii: f64, mass_kg: f64, giant_mass_kg: f64) -> f64 {
    a_radii * (mass_kg / (3.0 * giant_mass_kg)).cbrt()
}

/// A moon's orbital period round a giant, days.
pub fn period_days(a_km: f64, giant_mass_kg: f64) -> f64 {
    let a = a_km * 1e3;
    2.0 * std::f64::consts::PI * (a.powi(3) / (G * giant_mass_kg)).sqrt() / 86_400.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bright() -> RingSystem {
        RingSystem {
            inner: 1.25,
            outer: 2.3,
            gaps: vec![[2.0, 0.03]],
            opacity: 0.8,
            colour: [0.86, 0.8, 0.68],
            bright: true,
            gap_moons: Vec::new(),
        }
    }

    const SATURN_KG: f64 = 5.683e26;
    const SATURN_KM: f64 = 60_268.0;

    #[test]
    fn bright_rings_end_at_the_roche_limit_and_their_gaps_have_moons() {
        let (mut embedded, mut resonant, mut edges) = (0, 0, 0);
        for seed in 0..300 {
            let (rings, moons) = shape(bright(), 0.69, SATURN_KG, true, seed);
            assert!((2.0..=2.5).contains(&rings.outer), "{}", rings.outer);
            assert!(rings.gaps.len() <= 4);
            // Every gap is some moon's.
            let mut owned: Vec<usize> = moons.iter().filter_map(|m| m.gap).collect();
            owned.sort_unstable();
            assert_eq!(owned, (0..rings.gaps.len()).collect::<Vec<_>>());
            for m in &moons {
                match m.role {
                    RingRole::Embedded => {
                        embedded += 1;
                        let gap = rings.gaps[m.gap.unwrap()];
                        assert!((f64::from(gap[0]) - m.a_radii).abs() < 1e-5);
                        assert!(
                            m.a_radii < f64::from(rings.outer)
                                && m.a_radii > f64::from(rings.inner)
                        );
                        // Tens to a few hundred km wide.
                        let km = f64::from(gap[1]) * SATURN_KM;
                        assert!((5.0..800.0).contains(&km), "{km}");
                    }
                    RingRole::Resonant => {
                        resonant += 1;
                        let gap = rings.gaps[m.gap.unwrap()];
                        assert!((f64::from(gap[0]) - m.a_radii * RESONANCE_FACTOR).abs() < 1e-5);
                        assert!(m.a_radii > f64::from(rings.outer) + 0.49);
                    }
                    RingRole::Edge => {
                        edges += 1;
                        assert!(m.a_radii > f64::from(rings.outer));
                    }
                    RingRole::Shepherd => panic!("no shepherds round bright rings"),
                }
            }
            assert!(moons.windows(2).all(|w| w[0].a_radii <= w[1].a_radii));
        }
        assert!(
            embedded > 200 && resonant > 100 && edges > 300,
            "{embedded} {resonant} {edges}"
        );
    }

    #[test]
    fn the_gaps_of_pan_and_daphnis_come_out_of_their_masses() {
        // Pan: 28 km across at 2.216 R; Daphnis: 8 km at 2.265 R.
        let width = |diameter_km: f64, density: f64, a: f64| {
            let mass = 4.0 / 3.0
                * std::f64::consts::PI
                * (diameter_km / 2.0 * 1e3).powi(3)
                * density
                * 1e3;
            let k = ((mass.ln() - GAP_MASS_KG.0.ln()) / (GAP_MASS_KG.1 / GAP_MASS_KG.0).ln())
                .clamp(0.0, 1.0);
            hill_radii(a, mass, SATURN_KG)
                * (GAP_HILL.0 + k * (GAP_HILL.1 - GAP_HILL.0))
                * SATURN_KM
        };
        let pan = width(28.0, 0.42, 2.216);
        let daphnis = width(5.3, 0.34, 2.265);
        assert!((250.0..400.0).contains(&pan), "{pan}");
        assert!((25.0..60.0).contains(&daphnis), "{daphnis}");
    }

    #[test]
    fn faint_rings_get_shepherds_and_moonless_giants_none() {
        let faint = RingSystem {
            inner: 1.9,
            outer: 1.97,
            gaps: Vec::new(),
            opacity: 0.1,
            colour: [0.45, 0.43, 0.42],
            bright: false,
            gap_moons: Vec::new(),
        };
        let mut pairs = 0;
        for seed in 0..200 {
            let (rings, moons) = shape(faint.clone(), 1.27, 8.68e25, true, seed);
            assert_eq!(rings, faint);
            if !moons.is_empty() {
                pairs += 1;
                assert!(moons.iter().all(|m| m.role == RingRole::Shepherd));
                assert!(moons[0].a_radii < 1.9 && moons.last().unwrap().a_radii > 1.97);
            }
            let (_, none) = shape(bright(), 0.69, SATURN_KG, false, seed);
            assert!(none.is_empty());
        }
        assert!((100..180).contains(&pairs), "{pairs}");
    }
}
