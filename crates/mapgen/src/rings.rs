//! The rings of a giant of a planetary system and the moons in and next to
//! them (backlog item 18; `docs/analysis/14-rings.md`).
//!
//! The system draws a giant's rings with [`crate::giant_world::draw_rings`]
//! (see `crate::system`); [`shape`] then makes them what the Solar System's
//! are:
//!
//! - **Bright rings end at the Roche limit.** Their outer edge is the fluid
//!   Roche limit of their icy particles (0.5 to 0.9 g/cm³) round the giant,
//!   `2.44 R (ρ_giant / ρ_particles)^(1/3)` (Murray & Dermott 1999): 2.15
//!   to 2.62 radii round Saturn, whose A ring ends at 2.27, and farther out
//!   round a denser giant. Only physics bounds it: the rings stay inside
//!   the giant's other moons (Hill stable from the innermost, Gladman
//!   1993) and above its upper atmosphere (1.11 radii, Saturn's D ring).
//! - **Gaps belong to moons.** A moonlet in the outer 40 % of the rings
//!   clears a gap round its orbit, 7 to 17 of its Hill radii wide, wider
//!   for the larger (Daphnis keeps the 40 km Keeler gap, Pan the 325 km
//!   Encke gap). It grew from a dense core until it filled its Roche lobe
//!   (Porco et al. 2007): its size follows from its core and its distance,
//!   its density is the critical density there, its shape the lobe's. A
//!   mid-sized icy moon outside the rings (Mimas) cuts an edge at its 2:1
//!   inner Lindblad resonance, placed with the giant's `J2`, and clears a
//!   division outward of it (the B ring's edge and the Cassini Division,
//!   Spitale & Porco 2010).
//! - **Edge moons and shepherds.** Small moons just beyond a bright ring's
//!   outer edge (Atlas, Prometheus, Pandora, Janus), larger the farther
//!   out (Charnoz et al. 2010), and a pair of 10 to 25 km on either side
//!   of a faint narrow ring (Cordelia and Ophelia round Uranus's ε ring).
//!   None is less dense than the critical density at its distance, up to
//!   solid ice's: those raised to it fill their Roche lobe; where it is
//!   higher, the moon is bare solid ice (0.9 g/cm³), held by its
//!   strength.
//! - **Dust sources.** Small moons beyond a faint ring (Jupiter's Amalthea
//!   and Thebe outside its main ring, Neptune's inner moons) and, on half
//!   the giants with faint rings, a small moon farther out (Uranus's Mab).
//!
//! - **Dust rings fed by moons** ([`dust_rings`]): a faint gossamer ring
//!   from the main rings out to each of the one or two outermost small
//!   moons beyond them, and a little past it, of the dust impacts knock
//!   off them (Jupiter's Amalthea and Thebe rings, Burns et al. 1999); a
//!   faint belt round a small moon farther out (Uranus's μ ring round
//!   Mab); and round an icy regular moon close in a broad ring of the ice
//!   its geysers blow out (Saturn's E ring round Enceladus). Which icy
//!   moon is active is drawn: its tidal heating is not computed.
//!
//! Everything comes from a stream of its own, so a system's other bodies
//! keep their draws. A giant whose regular moons would not fit (a hot one,
//! its Hill sphere too small) keeps its rings without moons.
use crate::giant_world::RingSystem;
use crate::noise::{stream, stream_seed};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// What a ring moon does to its giant's rings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum RingRole {
    /// Inside the rings, clearing a gap round its orbit (Pan, Daphnis).
    Embedded,
    /// Outside the rings, its 2:1 resonance cutting an edge in them and
    /// clearing a division outward of it (Mimas).
    Resonant,
    /// Just beyond the outer edge, keeping it sharp (Atlas, Prometheus).
    Edge,
    /// One of a pair confining a narrow ring (Cordelia, Ophelia).
    Shepherd,
    /// A small moon beyond a faint ring, whose impact dust feeds a faint
    /// dust ring (Amalthea, Thebe, Mab).
    Source,
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

/// The giant whose rings [`shape`] shapes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Giant {
    /// Mean density, g/cm³, taken from the radius the rings are measured
    /// in (its equatorial radius), so that the Roche limit comes out in
    /// that unit.
    pub density: f64,
    /// Mass, kg.
    pub mass_kg: f64,
    /// Its `J2`, the oblateness of its gravity field (see [`j2`]).
    pub j2: f64,
    /// How far out its rings and ring moons may reach, in its radii:
    /// Hill stable from its innermost other moon, or within the share of
    /// its Hill sphere moons keep to.
    pub room: f64,
    /// Whether it keeps ring moons (false for a hot giant, or one whose
    /// Hill sphere is too small).
    pub moons: bool,
}

/// Density of the icy particles of bright rings, g/cm³ (porous ice).
const PARTICLE_DENSITY: (f64, f64) = (0.5, 0.9);
/// The lowest a ring reaches, in the giant's radii: below it the upper
/// atmosphere drags the grains in (Saturn's D ring ends at 1.11, NASA
/// Saturnian rings fact sheet; Mitchell et al. 2018,
/// doi:10.1126/science.aat2236). A bright ring whose Roche limit leaves no
/// room above it is not kept.
const ATMOSPHERE_EDGE: f64 = 1.11;
/// The narrowest a bright ring is kept, in the giant's radii: its inner
/// edge moves in when its Roche limit lies closer than that to it
/// (calibration, no source; Saturn's span 1.03).
const BRIGHT_SPAN: f64 = 0.3;
/// The volume of a moon's Roche lobe over the cube of its Hill radius,
/// `2π ln(2 + √3) / (3√3)` (Porco et al. 2007, eq. 1 and 2,
/// doi:10.1126/science.1143977).
pub const LOBE_VOLUME: f64 = 1.592_5;
/// Density of a moonlet's core, g/cm³: solid ice (Porco et al. 2007,
/// doi:10.1126/science.1143977).
const CORE_DENSITY: f64 = 0.9;
/// Share of a moonlet's accreted mantle that is ice, the rest voids: ring
/// particles packed as in the simulations of Porco et al. (2007; 65 %
/// voids, doi:10.1126/science.1143977).
const MANTLE_PACKING: f64 = 0.35;
/// Radius of a moonlet's core, km, log-uniform (calibration on Daphnis
/// and Pan). Round a Saturn a core grows 1.9 times to its long semi-axis
/// ([`grown`]), so its mean radius ends at about 1.35 times the core's:
/// these cores make moonlets of about 2 to 14 km in mean radius, Daphnis
/// (3.8 km) to Pan (14 km), each core about 0.74 of its moon's mean
/// radius and 0.54 of its long semi-axis, in line with the cores of a
/// third to a half of the moons' size Porco et al. (2007,
/// doi:10.1126/science.1143977) find.
const CORE_KM: (f64, f64) = (1.5, 10.0);
/// Semi-axes over the mean radius of a moon filling its Roche lobe, in the
/// lobe's ratio 3 : 2 : 2 (Porco et al. 2007, doi:10.1126/science.1143977):
/// its long axis points at the giant.
pub const LOBE_SHAPE: [f64; 3] = [1.310_371, 0.873_580_7, 0.873_580_7];
/// Mean radius above which an icy moon is round, km (Lineweaver & Norman
/// 2010, arXiv:1004.1091: about 200 km for ice, 300 km for rock).
const ROUND_ICE_KM: f64 = 200.0;
/// A gap's width in Hill radii of its moonlet, from the lightest moonlet
/// (Daphnis, 8 × 10¹³ kg: about 7.5) to the heaviest (Pan, 5 × 10¹⁵ kg:
/// about 17), interpolated on the logarithm of the mass.
const GAP_HILL: (f64, f64) = (7.5, 17.0);
const GAP_MASS_KG: (f64, f64) = (8e13, 5e15);
/// Odds of a mid-sized moon cutting a resonance edge (Mimas): most bright
/// rings have one.
const RESONANT_ODDS: f64 = 0.6;
/// Width of the division outward of a resonance edge, in the giant's radii
/// (the Cassini Division: 4,700 km, 0.078).
const RESONANCE_GAP: (f64, f64) = (0.04, 0.09);
/// Odds of a shepherd pair round a faint ring.
const SHEPHERD_ODDS: f64 = 0.7;
/// Where the small dust-source moons beyond a faint ring orbit, in the
/// giant's radii: Jupiter's Amalthea (2.54) and Thebe (3.10, NASA Jovian
/// satellite fact sheet), Neptune's Despina (2.13) to Larissa (2.98).
const SOURCE_SPAN: (f64, f64) = (2.0, 3.2);
/// Their mean radius, km, log-uniform: from moons of a few km, below what
/// the probes could see round most giants, to Thebe (49) and Amalthea
/// (84, NASA Jovian satellite fact sheet).
const SOURCE_KM: (f64, f64) = (4.0, 90.0);
/// Odds of a small moon farther out, which keeps a faint belt round its
/// orbit (Mab and Uranus's μ ring; calibration, no source: one giant of
/// the two with narrow rings), where (Mab: 3.82 radii, Showalter &
/// Lissauer 2006, doi:10.1126/science.1122882), and how large, km (Mab:
/// about 12).
const OUTER_SOURCE_ODDS: f64 = 0.5;
const OUTER_SOURCE_SPAN: (f64, f64) = (3.4, 4.2);
const OUTER_SOURCE_KM: (f64, f64) = (5.0, 20.0);
/// Gravitational constant, SI.
const G: f64 = 6.674e-11;

/// The fluid Roche limit of particles of `density` round a giant of
/// `giant_density` (g/cm³), in its radii (Murray & Dermott 1999, §4.7,
/// doi:10.1017/CBO9781139174817).
pub fn fluid_roche(giant_density: f64, density: f64) -> f64 {
    2.44 * (giant_density / density).cbrt()
}

/// The rigid Roche limit: where a solid moonlet of `density` holds
/// (Murray & Dermott 1999, §4.7, doi:10.1017/CBO9781139174817).
pub fn rigid_roche(giant_density: f64, density: f64) -> f64 {
    1.26 * (giant_density / density).cbrt()
}

/// The critical density at `a_radii` round a giant of `giant_density`
/// (g/cm³): a moon that dense fills its Roche lobe, one less dense cannot
/// keep what it gathers. `ρ_crit = 3M / (γ a³)` (Porco et al. 2007, eq. 2,
/// doi:10.1126/science.1143977), with `M = 4/3 π ρ R³` and `a` in `R`:
/// 0.45 g/cm³ at Pan's distance round Saturn.
pub fn critical_density(giant_density: f64, a_radii: f64) -> f64 {
    4.0 * PI * giant_density / (LOBE_VOLUME * a_radii.powi(3))
}

/// A giant's `J2` from its flattening `f` and its spin parameter
/// `q = Ω²R³/GM`, by the first-order relation of a fluid planet
/// `f = 3/2 J2 + q/2` (Murray & Dermott 1999, §4.5,
/// doi:10.1017/CBO9781139174817): Saturn's real `f` and `q` (0.098 and
/// 0.155) give 0.014, against 0.0163 measured.
pub fn j2(flattening: f64, q: f64) -> f64 {
    ((2.0 * flattening - q) / 3.0).max(0.0)
}

/// The 2:1 resonance of a moon lies at `2^(-2/3)` of its distance round a
/// point mass (Kepler's third law); see [`inner_lindblad`] for an oblate
/// giant.
pub const RESONANCE_FACTOR: f64 = 0.629_960_5;

/// Where the `m:(m−1)` inner Lindblad resonance of a moon at `a_moon` (in
/// the giant's radii) lies round a giant of `j2`, in its radii: where
/// `m (Ω − n_moon) = κ`, with the orbital and epicyclic frequencies to
/// first order in `J2`, `Ω² = GM/r³ (1 + 3/2 J2 / r²)` and
/// `κ² = GM/r³ (1 − 3/2 J2 / r²)` (Murray & Dermott 1999,
/// doi:10.1017/CBO9781139174817). For Mimas this gives the edge of
/// Saturn's B ring 650 km outward of Kepler's, at 117,532 km: 24 km short
/// of the 117,556 Spitale & Porco (2010, doi:10.1088/0004-6256/140/6/1747)
/// find with the higher terms of Saturn's field.
pub fn inner_lindblad(a_moon: f64, m: u32, j2: f64) -> f64 {
    let m = f64::from(m);
    let omega = |r: f64| (r.powi(-3) * (1.0 + 1.5 * j2 / (r * r))).sqrt();
    let kappa = |r: f64| (r.powi(-3) * (1.0 - 1.5 * j2 / (r * r))).sqrt();
    let target = m * omega(a_moon);
    let kepler = a_moon * ((m - 1.0) / m).powf(2.0 / 3.0);
    // `m Ω − κ` falls outwards: halve the bracket round Kepler's.
    let (mut lo, mut hi) = (0.8 * kepler, 1.2 * kepler);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if m * omega(mid) - kappa(mid) > target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// A moonlet grown in the rings at `a_radii` from a core of `core_km`
/// (solid ice) under a mantle of ring particles of `mantle` g/cm³: its
/// mean radius, km, density and semi-axes. It gathers particles until its
/// mean density falls to the critical density `crit` there, filling its
/// Roche lobe, whose long semi-axis is then
/// `R_c (4π/(3γ) (ρ_c − ρ_m) / (ρ_crit − ρ_m))^(1/3)` (Porco et al. 2007,
/// eq. 3, doi:10.1126/science.1143977): Pan's 1.9 times its core. Where
/// the critical density is above the core's, the core stays bare. Inside
/// the fluid Roche limit of the particles the critical density is at least
/// 0.54 of their density, above the mantle's 0.35: the growth stays finite
/// (2.7 times at most).
fn grown(core_km: f64, mantle: f64, crit: f64, draws: [f64; 2]) -> (f64, f64, [f64; 3]) {
    if crit >= CORE_DENSITY {
        return (core_km, CORE_DENSITY, irregular(core_km, draws));
    }
    debug_assert!(crit > mantle, "a moonlet beyond the Roche limit");
    let growth =
        (4.0 * PI / (3.0 * LOBE_VOLUME) * (CORE_DENSITY - mantle) / (crit - mantle)).cbrt();
    let hill_km = core_km * growth;
    let radius_km = hill_km * (3.0 * LOBE_VOLUME / (4.0 * PI)).cbrt();
    let density = mantle + (CORE_DENSITY - mantle) * (core_km / radius_km).powi(3);
    (radius_km, density, LOBE_SHAPE)
}

/// A small moon of `density` drawn at `a_radii`: no less dense than the
/// critical density there, below which it would lose what it gathers
/// (Porco et al. 2007, doi:10.1126/science.1143977), and filling its Roche
/// lobe when raised to it; irregular otherwise. Where the critical density
/// exceeds solid ice, no loose mantle holds: the moon is bare solid ice
/// (0.9 g/cm³), irregular, kept by its strength outside its rigid Roche
/// limit (Metis and Adrastea inside Jupiter's Roche zone), as a moonlet's
/// bare core is ([`grown`]). Its density and semi-axes.
fn lobe_bound(
    giant_density: f64,
    a_radii: f64,
    density: f64,
    radius_km: f64,
    draws: [f64; 2],
) -> (f64, [f64; 3]) {
    let crit = critical_density(giant_density, a_radii);
    if crit > CORE_DENSITY {
        (CORE_DENSITY, irregular(radius_km, draws))
    } else if crit >= density {
        (crit, LOBE_SHAPE)
    } else {
        (density, irregular(radius_km, draws))
    }
}

/// Log-uniform between `range.0` and `range.1` from `u` in `[0, 1)`.
fn log_uniform(range: (f64, f64), u: f64) -> f64 {
    (range.0.ln() + u * (range.1 / range.0).ln()).exp()
}

/// A giant's `rings` (as `draw_rings` drew them) shaped by the `giant`,
/// with the moons that make them: the rings with their edges and gaps set
/// (none when a bright ring finds no room under its Roche limit), and the
/// moons, nearest first. `seed` is the giant's.
pub fn shape(rings: RingSystem, giant: &Giant, seed: u32) -> (Option<RingSystem>, Vec<RingMoon>) {
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(seed, stream::RING_MOONS)));
    let mut rings = rings;
    let mut out = Vec::new();
    // Each moon's own seed, from the stream's seed: no other stream's.
    let base = stream_seed(seed, stream::RING_MOONS);
    // In the order drawn, a moon dropped later keeping its number: the
    // others' seeds do not depend on it.
    let mut drawn = 0;
    let mut next_seed = || {
        drawn += 1;
        stream_seed(base, drawn - 1)
    };
    let density_g = giant.density;
    if rings.bright {
        let particles = rng.random_range(PARTICLE_DENSITY.0..PARTICLE_DENSITY.1);
        let outer = fluid_roche(density_g, particles).min(giant.room);
        let inner = f64::from(rings.inner).min(outer - BRIGHT_SPAN);
        if inner < ATMOSPHERE_EDGE {
            return (None, out);
        }
        rings.inner = inner as f32;
        rings.outer = outer as f32;
        rings.gaps.clear();
        if !giant.moons {
            return (Some(rings), out);
        }
        // The particles a moonlet gathers, packed loosely.
        let mantle = particles * MANTLE_PACKING;
        // Moonlets in the outer 40 %, each with its gap.
        let count = match rng.random::<f64>() {
            r if r < 0.3 => 0,
            r if r < 0.75 => 1,
            _ => 2,
        };
        for _ in 0..count {
            let core_km = log_uniform(CORE_KM, rng.random::<f64>());
            let at = rng.random::<f64>();
            let shape_draws = [rng.random::<f64>(), rng.random::<f64>()];
            // Its core, solid, must hold against the tides.
            let from = rigid_roche(density_g, CORE_DENSITY).max(inner + 0.6 * (outer - inner));
            let to = outer - 0.03;
            if from >= to {
                continue;
            }
            let a = from + at * (to - from);
            let crit = critical_density(density_g, a);
            let (radius_km, density, shape) = grown(core_km, mantle, crit, shape_draws);
            let mass = 4.0 / 3.0 * PI * (radius_km * 1e3).powi(3) * density * 1e3;
            let hill = hill_radii(a, mass, giant.mass_kg);
            let k = ((mass.ln() - GAP_MASS_KG.0.ln()) / (GAP_MASS_KG.1 / GAP_MASS_KG.0).ln())
                .clamp(0.0, 1.0);
            let width = hill * (GAP_HILL.0 + k * (GAP_HILL.1 - GAP_HILL.0));
            if overlaps(&rings.gaps, a, width) {
                continue;
            }
            rings.gaps.push([a as f32, width as f32]);
            out.push(RingMoon {
                role: RingRole::Embedded,
                a_radii: a,
                radius_km,
                shape,
                density,
                seed: next_seed(),
                gap: Some(rings.gaps.len() - 1),
            });
        }
        // A mid-sized moon outside: its 2:1 resonance cuts the edge of the
        // dense inner ring, the division outward of it, more ring beyond.
        let resonant = rng.random::<f64>() < RESONANT_ODDS;
        let radius_km = rng.random_range(150.0..250.0);
        let density = rng.random_range(1.1..1.3);
        let at = rng.random::<f64>();
        let width = rng.random_range(RESONANCE_GAP.0..RESONANCE_GAP.1);
        let shape_draws = [rng.random::<f64>(), rng.random::<f64>()];
        let from = (outer + 0.5).max(fluid_roche(density_g, density) + 0.1);
        let to = ((outer - 0.1 - width) / RESONANCE_FACTOR).min(giant.room);
        let mut resonant_at = None;
        if resonant && from < to {
            let a = from + at * (to - from);
            let edge = inner_lindblad(a, 2, giant.j2);
            let centre = edge + width / 2.0;
            if edge > inner + 0.1 && edge + width < outer - 0.05 {
                // The division is its own: a moonlet whose gap it would
                // overlap is not kept (the moonlets alone, with their
                // gaps, are in `out` and `rings.gaps` so far, in the same
                // order).
                let mut k = 0;
                while k < rings.gaps.len() {
                    if overlaps(&rings.gaps[k..=k], centre, width) {
                        rings.gaps.remove(k);
                        out.remove(k);
                    } else {
                        k += 1;
                    }
                }
                for (k, moon) in out.iter_mut().enumerate() {
                    moon.gap = Some(k);
                }
                rings.gaps.push([centre as f32, width as f32]);
                resonant_at = Some(a);
                out.push(RingMoon {
                    role: RingRole::Resonant,
                    a_radii: a,
                    radius_km,
                    shape: if radius_km < ROUND_ICE_KM {
                        irregular(radius_km, shape_draws)
                    } else {
                        [1.0; 3]
                    },
                    density,
                    seed: next_seed(),
                    gap: Some(rings.gaps.len() - 1),
                });
            }
        }
        // Small moons just beyond the outer edge, the larger farther out:
        // the rings spread past their Roche limit, and the moonlets born
        // there merge as they move out (Charnoz et al. 2010,
        // doi:10.1038/nature09096; Crida & Charnoz 2012,
        // doi:10.1126/science.1226477).
        let edge_count = rng.random_range(1..4);
        let mut edges: Vec<(f64, f64, f64, [f64; 2])> = (0..edge_count)
            .map(|k| {
                let radius_km = log_uniform((10.0, 100.0), rng.random::<f64>());
                let density = rng.random_range(0.4..0.7);
                let a = outer * rng.random_range(1.01..1.25) * (1.0 + 0.02 * k as f64);
                (
                    a,
                    radius_km,
                    density,
                    [rng.random::<f64>(), rng.random::<f64>()],
                )
            })
            .collect();
        let mut sizes: Vec<f64> = edges.iter().map(|e| e.1).collect();
        sizes.sort_by(f64::total_cmp);
        edges.sort_by(|x, y| x.0.total_cmp(&y.0));
        for ((a, _, density, draws), radius_km) in edges.into_iter().zip(sizes) {
            let (density, shape) = lobe_bound(density_g, a, density, radius_km, draws);
            if resonant_at.is_some_and(|m| a > m - 0.15)
                || a < rigid_roche(density_g, density)
                || a > giant.room
            {
                continue;
            }
            out.push(RingMoon {
                role: RingRole::Edge,
                a_radii: a,
                radius_km,
                shape,
                density,
                seed: next_seed(),
                gap: None,
            });
        }
    } else if giant.moons {
        if rng.random::<f64>() < SHEPHERD_ODDS {
            // A pair confining the narrow ring.
            for side in [-1.0, 1.0] {
                let radius_km = rng.random_range(10.0..25.0);
                let density = rng.random_range(0.5..1.0);
                let offset = rng.random_range(0.01..0.04);
                let shape_draws = [rng.random::<f64>(), rng.random::<f64>()];
                let edge = if side < 0.0 { rings.inner } else { rings.outer };
                let a = f64::from(edge) + side * offset;
                let (density, shape) = lobe_bound(density_g, a, density, radius_km, shape_draws);
                if a < rigid_roche(density_g, density) || a > giant.room {
                    continue;
                }
                out.push(RingMoon {
                    role: RingRole::Shepherd,
                    a_radii: a,
                    radius_km,
                    shape,
                    density,
                    seed: next_seed(),
                    gap: None,
                });
            }
        }
        // Small moons beyond the narrow ring, sources of dust rings: one
        // or two close in (Amalthea and Thebe beyond Jupiter's main ring),
        // and on some giants one farther out (Mab). Burns et al. (1999,
        // doi:10.1126/science.284.5417.1146) suggest that faint rings may
        // accompany all small inner moons.
        let count = rng.random_range(1..3);
        let mut sources: Vec<(f64, f64, f64, [f64; 2])> = (0..count)
            .map(|_| {
                (
                    rng.random_range(SOURCE_SPAN.0..SOURCE_SPAN.1),
                    log_uniform(SOURCE_KM, rng.random::<f64>()),
                    rng.random_range(0.4..0.9),
                    [rng.random::<f64>(), rng.random::<f64>()],
                )
            })
            .collect();
        let far = rng.random::<f64>() < OUTER_SOURCE_ODDS;
        let far_moon = (
            rng.random_range(OUTER_SOURCE_SPAN.0..OUTER_SOURCE_SPAN.1),
            log_uniform(OUTER_SOURCE_KM, rng.random::<f64>()),
            rng.random_range(0.4..0.9),
            [rng.random::<f64>(), rng.random::<f64>()],
        );
        if far {
            sources.push(far_moon);
        }
        let mut kept: Vec<f64> = Vec::new();
        for (a, radius_km, density, draws) in sources {
            let (density, shape) = lobe_bound(density_g, a, density, radius_km, draws);
            if a < f64::from(rings.outer) + 0.15
                || a < rigid_roche(density_g, density)
                || a > giant.room
                || kept.iter().any(|k| (k - a).abs() < 0.1)
            {
                continue;
            }
            kept.push(a);
            out.push(RingMoon {
                role: RingRole::Source,
                a_radii: a,
                radius_km,
                shape,
                density,
                seed: next_seed(),
                gap: None,
            });
        }
    }
    out.sort_by(|x, y| x.a_radii.total_cmp(&y.a_radii));
    (Some(rings), out)
}

/// Whether a gap `width` wide at `a` (giant radii) would overlap one of
/// `gaps`, or come within 0.02 radii of it.
fn overlaps(gaps: &[[f32; 2]], a: f64, width: f64) -> bool {
    gaps.iter()
        .any(|g| (f64::from(g[0]) - a).abs() < (f64::from(g[1]) + width) / 2.0 + 0.02)
}

/// What feeds a dust ring.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum DustKind {
    /// Dust knocked off a small moon by impacts (Jupiter's gossamer rings,
    /// Uranus's μ ring).
    Gossamer,
    /// Ice grains from a moon's geysers (Saturn's E ring).
    Plume,
}

/// A faint dust ring, in the giant's equatorial plane.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DustRing {
    pub kind: DustKind,
    /// Edges, in the giant's radii.
    pub inner: f32,
    pub outer: f32,
    /// Where it is densest, in the giant's radii: its source's orbit.
    pub peak: f32,
    /// Opacity where densest, `[0, 1]`: a few hundredths at most.
    pub opacity: f32,
    /// Colour, sRGB in `[0, 1]`.
    pub colour: [f32; 3],
    /// The moon that feeds it, by its id in the system.
    pub source: String,
}

/// A moon of a giant as [`dust_rings`] needs it.
#[derive(Clone, Debug, PartialEq)]
pub struct DustSource {
    pub id: String,
    /// Distance from the giant's centre, in its radii.
    pub a_radii: f64,
    /// Its part in the rings, none for a regular moon.
    pub role: Option<RingRole>,
    pub icy: bool,
}

/// Odds of a gossamer ring from a small moon beyond the main rings.
const GOSSAMER_ODDS: f64 = 0.8;
/// How far past its moon a gossamer ring reaches, as shares of the moon's
/// distance: from the moon's own orbit (Jupiter's Amalthea ring ends at
/// Amalthea, 2.54 radii) to Thebe's extension, out to 3.92 radii from
/// Thebe's 3.10 (NASA Jovian rings fact sheet; Burns et al. 1999,
/// doi:10.1126/science.284.5417.1146).
const GOSSAMER_REACH: (f64, f64) = (1.0, 1.26);
/// Beyond this, in the giant's radii, a small source moon keeps its dust
/// round its orbit rather than spreading it in to the main rings:
/// Jupiter's gossamer rings run in from moons up to Thebe's 3.10 radii,
/// Uranus's μ ring stays round Mab at 3.82 (calibration on those two).
const GOSSAMER_FAR: f64 = 3.3;
/// The span of a belt round a far small moon, as shares of its distance
/// (Uranus's μ ring, 17,000 km wide round 97,700 km, NASA Uranian rings
/// fact sheet; de Pater, Hammel, Gibbard & Showalter 2006,
/// doi:10.1126/science.1125110).
const BELT_SPAN: (f64, f64) = (0.913, 1.087);
/// Odds of an active icy moon, and how far out it may be, in the giant's
/// radii (Enceladus: 3.95).
const PLUME_ODDS: f64 = 0.25;
const PLUME_REACH: f64 = 12.0;
/// The span of a plume ring as shares of its moon's distance (the E ring:
/// about 3 to 8 radii round Enceladus at 3.95).
const PLUME_SPAN: (f64, f64) = (0.75, 2.0);

/// The dust rings fed by the moons of a giant (`moons`, any order) with
/// main `rings` (or none), from the giant's `seed`.
pub fn dust_rings(rings: Option<&RingSystem>, moons: &[DustSource], seed: u32) -> Vec<DustRing> {
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(seed, stream::DUST_RINGS)));
    let mut out = Vec::new();
    if let Some(rings) = rings {
        // Gossamer rings: the one or two outermost small moons beyond the
        // main rings close in, each its own, from the rings' edge out to
        // its orbit and a little past it.
        let far = |m: &DustSource| m.role == Some(RingRole::Source) && m.a_radii > GOSSAMER_FAR;
        let mut small: Vec<&DustSource> = moons
            .iter()
            .filter(|m| {
                matches!(m.role, Some(RingRole::Edge | RingRole::Source))
                    && m.a_radii > f64::from(rings.outer)
                    && !far(m)
            })
            .collect();
        small.sort_by(|a, b| b.a_radii.total_cmp(&a.a_radii));
        for moon in small.into_iter().take(2) {
            let on = rng.random::<f64>() < GOSSAMER_ODDS;
            let opacity = rng.random_range(0.01..0.03);
            let reach = rng.random_range(GOSSAMER_REACH.0..GOSSAMER_REACH.1);
            if !on {
                continue;
            }
            out.push(DustRing {
                kind: DustKind::Gossamer,
                inner: rings.outer,
                outer: (moon.a_radii * reach) as f32,
                peak: moon.a_radii as f32,
                opacity: opacity as f32,
                colour: [0.62, 0.52, 0.44],
                source: moon.id.clone(),
            });
        }
        // A belt round a far small moon (the μ ring, bluish: its grains
        // are small).
        for moon in moons.iter().filter(|m| far(m)) {
            let on = rng.random::<f64>() < GOSSAMER_ODDS;
            let opacity = rng.random_range(0.01..0.02);
            if !on {
                continue;
            }
            out.push(DustRing {
                kind: DustKind::Gossamer,
                inner: (moon.a_radii * BELT_SPAN.0) as f32,
                outer: (moon.a_radii * BELT_SPAN.1) as f32,
                peak: moon.a_radii as f32,
                opacity: opacity as f32,
                colour: [0.55, 0.65, 0.85],
                source: moon.id.clone(),
            });
        }
    }
    // A plume ring round the innermost icy regular moon close in, with
    // drawn odds: no tidal heating is computed.
    let active = rng.random::<f64>() < PLUME_ODDS;
    let opacity = rng.random_range(0.01..0.025);
    let geyser = moons
        .iter()
        .filter(|m| m.role.is_none() && m.icy && m.a_radii < PLUME_REACH)
        .min_by(|a, b| a.a_radii.total_cmp(&b.a_radii));
    if let (true, Some(moon)) = (active, geyser) {
        out.push(DustRing {
            kind: DustKind::Plume,
            inner: (moon.a_radii * PLUME_SPAN.0) as f32,
            outer: (moon.a_radii * PLUME_SPAN.1) as f32,
            peak: moon.a_radii as f32,
            opacity: opacity as f32,
            colour: [0.82, 0.88, 0.96],
            source: moon.id.clone(),
        });
    }
    out
}

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
    2.0 * PI * (a.powi(3) / (G * giant_mass_kg)).sqrt() / 86_400.0
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

    fn faint() -> RingSystem {
        RingSystem {
            inner: 1.72,
            outer: 1.8,
            gaps: Vec::new(),
            opacity: 0.1,
            colour: [0.45, 0.43, 0.42],
            bright: false,
            gap_moons: Vec::new(),
        }
    }

    const SATURN_KG: f64 = 5.683e26;
    const SATURN_KM: f64 = 60_268.0;
    /// Saturn's mean density over its equatorial radius, g/cm³.
    const SATURN_DENSITY: f64 = 0.6198;

    fn saturn() -> Giant {
        Giant {
            density: SATURN_DENSITY,
            mass_kg: SATURN_KG,
            j2: 0.016_3,
            room: 8.0,
            moons: true,
        }
    }

    fn jupiter() -> Giant {
        // 1.898e27 kg over an equatorial radius of 71,492 km.
        Giant {
            density: 1.240,
            mass_kg: 1.898e27,
            j2: 0.014_7,
            room: 8.0,
            moons: true,
        }
    }

    #[test]
    fn the_roche_limits_of_ring_ice_round_saturn_are_in_its_equatorial_radii() {
        // Murray & Dermott's fluid limit with Saturn's density taken from
        // its equatorial radius: 2.155 radii for ice of 0.9, 2.62 for 0.5.
        assert!((fluid_roche(SATURN_DENSITY, 0.9) - 2.155).abs() < 0.005);
        assert!((fluid_roche(SATURN_DENSITY, 0.5) - 2.62).abs() < 0.005);
    }

    #[test]
    fn bright_rings_end_at_the_roche_limit_and_their_gaps_have_moons() {
        let (mut embedded, mut resonant, mut edges) = (0, 0, 0);
        let giant = saturn();
        for seed in 0..300 {
            let (rings, moons) = shape(bright(), &giant, seed);
            let rings = rings.expect("room for Saturn's rings");
            let roche = (
                fluid_roche(SATURN_DENSITY, PARTICLE_DENSITY.1),
                fluid_roche(SATURN_DENSITY, PARTICLE_DENSITY.0),
            );
            let outer = f64::from(rings.outer);
            assert!(
                outer >= roche.0 - 1e-5 && outer <= roche.1 + 1e-5,
                "{outer}"
            );
            assert!(rings.gaps.len() <= 4);
            // Every gap is some moon's.
            let mut owned: Vec<usize> = moons.iter().filter_map(|m| m.gap).collect();
            owned.sort_unstable();
            assert_eq!(owned, (0..rings.gaps.len()).collect::<Vec<_>>());
            for m in &moons {
                // None is less dense than it takes to keep what it gathered.
                let crit = critical_density(SATURN_DENSITY, m.a_radii);
                assert!(m.density >= crit - 1e-9 || m.role == RingRole::Resonant);
                match m.role {
                    RingRole::Embedded => {
                        embedded += 1;
                        let gap = rings.gaps[m.gap.unwrap()];
                        assert!((f64::from(gap[0]) - m.a_radii).abs() < 1e-5);
                        assert!(
                            m.a_radii < f64::from(rings.outer)
                                && m.a_radii > f64::from(rings.inner)
                        );
                        // Grown to fill its Roche lobe, at the critical density.
                        assert_eq!(m.shape, LOBE_SHAPE);
                        assert!((m.density - crit).abs() < 1e-9);
                        assert!((1.5..16.0).contains(&m.radius_km), "{}", m.radius_km);
                        // Tens to a few hundred km wide.
                        let km = f64::from(gap[1]) * SATURN_KM;
                        assert!((5.0..800.0).contains(&km), "{km}");
                    }
                    RingRole::Resonant => {
                        resonant += 1;
                        // Its resonance is the division's inner edge.
                        let gap = rings.gaps[m.gap.unwrap()];
                        let edge = f64::from(gap[0]) - f64::from(gap[1]) / 2.0;
                        assert!((edge - inner_lindblad(m.a_radii, 2, giant.j2)).abs() < 1e-5);
                        assert!(f64::from(gap[0] + gap[1] / 2.0) < f64::from(rings.outer));
                        assert!(m.a_radii > f64::from(rings.outer) + 0.49);
                        assert_eq!(m.shape == [1.0; 3], m.radius_km >= ROUND_ICE_KM);
                    }
                    RingRole::Edge => {
                        edges += 1;
                        assert!(m.a_radii > f64::from(rings.outer));
                    }
                    RingRole::Shepherd | RingRole::Source => {
                        panic!("no shepherds or dust sources round bright rings")
                    }
                }
            }
            assert!(moons.windows(2).all(|w| w[0].a_radii <= w[1].a_radii));
            // Edge moons grow with their distance.
            let edge: Vec<&RingMoon> = moons.iter().filter(|m| m.role == RingRole::Edge).collect();
            assert!(edge.windows(2).all(|w| w[0].radius_km <= w[1].radius_km));
        }
        assert!(
            embedded > 200 && resonant > 100 && edges > 300,
            "{embedded} {resonant} {edges}"
        );
    }

    #[test]
    fn a_dense_giant_has_wider_rings_and_its_moons_stay_in_the_room_they_have() {
        let giant = jupiter();
        for seed in 0..200 {
            let (rings, _) = shape(bright(), &giant, seed);
            let outer = f64::from(rings.unwrap().outer);
            // 2.72 to 3.30 radii: past the old cap of 2.5.
            assert!((2.70..3.31).contains(&outer), "{outer}");
        }
        let tight = Giant {
            room: 3.0,
            ..jupiter()
        };
        for seed in 0..200 {
            let (rings, moons) = shape(bright(), &tight, seed);
            assert!(f64::from(rings.unwrap().outer) <= 3.0 + 1e-6);
            assert!(moons.iter().all(|m| m.a_radii <= 3.0));
        }
    }

    #[test]
    fn a_bright_ring_with_no_room_above_the_atmosphere_is_not_kept() {
        // An inflated giant of 0.08 g/cm³: its Roche limit for ice of 0.5
        // lies at 1.33 radii, too close above 1.11 for a ring 0.3 wide.
        let puffy = Giant {
            density: 0.08,
            ..saturn()
        };
        for seed in 0..50 {
            let (rings, moons) = shape(bright(), &puffy, seed);
            assert!(rings.is_none() && moons.is_empty());
        }
    }

    #[test]
    fn gaps_do_not_overlap_and_no_moon_sits_in_another_ones_gap() {
        for seed in 0..2000 {
            let (rings, moons) = shape(bright(), &saturn(), seed);
            let rings = rings.unwrap();
            for (i, g) in rings.gaps.iter().enumerate() {
                assert!(
                    !overlaps(&rings.gaps[i + 1..], f64::from(g[0]), f64::from(g[1])),
                    "seed {seed}: {:?}",
                    rings.gaps
                );
            }
            for m in moons.iter().filter(|m| m.role == RingRole::Embedded) {
                for (k, g) in rings.gaps.iter().enumerate() {
                    if Some(k) != m.gap {
                        assert!(
                            (f64::from(g[0]) - m.a_radii).abs() > f64::from(g[1]) / 2.0,
                            "seed {seed}"
                        );
                    }
                }
            }
            let mut seeds: Vec<u32> = moons.iter().map(|m| m.seed).collect();
            seeds.sort_unstable();
            seeds.dedup();
            assert_eq!(seeds.len(), moons.len(), "seed {seed}");
        }
    }

    #[test]
    fn the_gaps_of_pan_and_daphnis_come_out_of_their_masses() {
        // Pan: 28 km across at 2.216 R; Daphnis: 8 km at 2.265 R.
        let width = |diameter_km: f64, density: f64, a: f64| {
            let mass = 4.0 / 3.0 * PI * (diameter_km / 2.0 * 1e3).powi(3) * density * 1e3;
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
    fn the_critical_density_is_that_of_saturns_small_moons() {
        // Porco et al. 2007: 0.45 g/cm³ at Pan's distance, 0.38 at Pandora's.
        let pan = critical_density(SATURN_DENSITY, 133_584.0 / SATURN_KM);
        let pandora = critical_density(SATURN_DENSITY, 141_720.0 / SATURN_KM);
        assert!((pan - 0.45).abs() < 0.01, "{pan}");
        assert!((pandora - 0.38).abs() < 0.01, "{pandora}");
        // Round a giant of Jupiter's density, about 0.7 at 2.4 radii.
        assert!((critical_density(1.24, 2.4) - 0.71).abs() < 0.02);
    }

    #[test]
    fn a_moonlet_grows_from_its_core_until_it_fills_its_roche_lobe() {
        // A core of solid ice at Pan's distance under a mantle of 0.14
        // g/cm³ ends at 1.9 times its size (Porco et al. 2007, eq. 3).
        let a = 133_584.0 / SATURN_KM;
        let crit = critical_density(SATURN_DENSITY, a);
        let (radius_km, density, shape) = grown(9.0, 0.14, crit, [0.5, 0.5]);
        let long = radius_km * shape[0] / LOBE_SHAPE[0] / (3.0 * LOBE_VOLUME / (4.0 * PI)).cbrt();
        assert!((long / 9.0 - 1.9).abs() < 0.05, "{}", long / 9.0);
        assert!((density - crit).abs() < 1e-9 && shape == LOBE_SHAPE);
        // Its lobe, from its mass: the long semi-axis is its Hill radius.
        let mass = 4.0 / 3.0 * PI * (radius_km * 1e3).powi(3) * density * 1e3;
        let hill_km = hill_radii(a, mass, SATURN_KG) * SATURN_KM;
        assert!((hill_km - long).abs() / long < 1e-3, "{hill_km} {long}");
        // Where the critical density is above solid ice, the core stays bare.
        let (bare, density, shape) = grown(5.0, 0.2, 1.0, [0.5, 0.5]);
        assert!(bare == 5.0 && density == CORE_DENSITY && shape != LOBE_SHAPE);
        // The lobe's shape keeps the mean radius: 3 : 2 : 2 for a volume of 1.
        assert!((LOBE_SHAPE.iter().product::<f64>() - 1.0).abs() < 1e-6);
        assert!((LOBE_SHAPE[1] / LOBE_SHAPE[0] - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn mimass_resonance_with_saturns_j2_sets_the_edge_of_the_b_ring() {
        // Mimas at 185,539 km; J2 0.01633 in Saturn's equatorial radius of
        // 60,268 km: 117,532 km, 650 km outward of Kepler's (Spitale &
        // Porco 2010, with J4 and beyond: 117,556 km).
        let a = 185_539.0 / SATURN_KM;
        let edge = inner_lindblad(a, 2, 0.016_33) * SATURN_KM;
        let kepler = a * RESONANCE_FACTOR * SATURN_KM;
        assert!((edge - 117_532.3).abs() < 1.0, "{edge}");
        assert!((edge - kepler - 650.0).abs() < 1.0, "{}", edge - kepler);
        assert!((edge - 117_556.0).abs() < 30.0, "{edge}");
        // Without J2, Kepler's.
        assert!((inner_lindblad(a, 2, 0.0) - a * RESONANCE_FACTOR).abs() < 1e-6);
        // Saturn's real flattening and spin give about its J2.
        assert!((j2(0.098, 0.155) - 0.0137).abs() < 1e-3);
    }

    #[test]
    fn faint_rings_get_shepherds_and_dust_sources_and_moonless_giants_none() {
        let faint = RingSystem {
            inner: 1.9,
            outer: 1.97,
            ..faint()
        };
        let uranus = Giant {
            density: 1.24,
            mass_kg: 8.68e25,
            j2: 0.0035,
            room: 8.0,
            moons: true,
        };
        let (mut pairs, mut far, mut fed) = (0, 0, 0);
        for seed in 0..200 {
            let (rings, moons) = shape(faint.clone(), &uranus, seed);
            assert_eq!(rings.as_ref(), Some(&faint));
            let shepherds: Vec<&RingMoon> = moons
                .iter()
                .filter(|m| m.role == RingRole::Shepherd)
                .collect();
            if !shepherds.is_empty() {
                pairs += 1;
                assert!(shepherds[0].a_radii < 1.9 && shepherds.last().unwrap().a_radii > 1.97);
            }
            let sources: Vec<&RingMoon> = moons
                .iter()
                .filter(|m| m.role == RingRole::Source)
                .collect();
            assert!(sources.len() <= 3);
            fed += usize::from(!sources.is_empty());
            assert!(
                sources
                    .iter()
                    .all(|m| m.a_radii > 1.97 + 0.15 && m.a_radii < 4.2)
            );
            far += usize::from(sources.iter().any(|m| m.a_radii > GOSSAMER_FAR));
            assert!(
                moons
                    .iter()
                    .all(|m| m.role == RingRole::Shepherd || m.role == RingRole::Source)
            );
            let none = Giant {
                moons: false,
                ..saturn()
            };
            assert!(shape(bright(), &none, seed).1.is_empty());
            assert!(shape(faint.clone(), &none, seed).1.is_empty());
        }
        assert!((100..180).contains(&pairs), "{pairs}");
        assert!((70..130).contains(&far), "{far}");
        assert!(fed > 180, "{fed}");
    }

    #[test]
    fn small_moons_round_a_dense_giant_are_never_rubble_below_the_critical_density() {
        // A super-Jupiter of 4 g/cm³: the critical density exceeds solid
        // ice out to 3.27 radii. A moon there is bare solid ice; farther
        // out, no less dense than the critical density.
        let dense = Giant {
            density: 4.0,
            mass_kg: 5.7e27,
            j2: 0.01,
            room: 8.0,
            moons: true,
        };
        let mut solid = 0;
        for seed in 0..300 {
            for rings in [faint(), bright()] {
                let (_, moons) = shape(rings, &dense, seed);
                for m in moons.iter().filter(|m| m.role != RingRole::Resonant) {
                    let crit = critical_density(dense.density, m.a_radii);
                    assert!(
                        m.density >= crit.min(CORE_DENSITY) - 1e-9,
                        "seed {seed}: {:?} at {:.2} radii, {:.2} g/cm³ against {crit:.2}",
                        m.role,
                        m.a_radii,
                        m.density
                    );
                    if crit > CORE_DENSITY && m.role != RingRole::Embedded {
                        solid += 1;
                        assert!(m.density == CORE_DENSITY && m.shape != LOBE_SHAPE);
                    }
                }
            }
        }
        assert!(solid > 200, "{solid}");
    }

    #[test]
    fn dust_rings_follow_their_moons() {
        let moon = |id: &str, a: f64, role: Option<RingRole>, icy: bool| DustSource {
            id: id.to_string(),
            a_radii: a,
            role,
            icy,
        };
        let moons = [
            moon("c-1", 2.2, Some(RingRole::Embedded), true),
            moon("c-2", 2.5, Some(RingRole::Edge), true),
            moon("c-3", 2.9, Some(RingRole::Edge), true),
            moon("c-4", 3.1, Some(RingRole::Edge), true),
            moon("c-5", 7.0, None, true),
            moon("c-6", 11.0, None, true),
        ];
        let rings = bright();
        let (mut gossamer, mut plumes, mut past) = (0, 0, 0);
        for seed in 0..400 {
            for ring in dust_rings(Some(&rings), &moons, seed) {
                assert!(ring.opacity < 0.04 && ring.inner < ring.peak && ring.peak <= ring.outer);
                match ring.kind {
                    DustKind::Gossamer => {
                        gossamer += 1;
                        // The two outermost small moons, never the moonlet.
                        assert!(
                            ring.source == "c-3" || ring.source == "c-4",
                            "{}",
                            ring.source
                        );
                        assert_eq!(ring.inner, rings.outer);
                        // Out to its moon, and up to 1.26 times past it (Thebe's).
                        let reach = ring.outer / ring.peak;
                        assert!((1.0..=1.26).contains(&reach), "{reach}");
                        past += usize::from(reach > 1.13);
                    }
                    DustKind::Plume => {
                        plumes += 1;
                        assert_eq!(ring.source, "c-5");
                        assert!(
                            (ring.inner - 5.25).abs() < 1e-4 && (ring.outer - 14.0).abs() < 1e-4
                        );
                    }
                }
            }
        }
        assert!((560..720).contains(&gossamer), "{gossamer}");
        assert!(past > gossamer / 3, "{past}");
        assert!((60..140).contains(&plumes), "{plumes}");
        // Shepherds make none.
        let shepherds = [
            moon("e-1", 1.85, Some(RingRole::Shepherd), true),
            moon("e-2", 2.02, Some(RingRole::Shepherd), true),
        ];
        assert!((0..100).all(|seed| dust_rings(Some(&rings), &shepherds, seed).is_empty()));
        // No main rings, no small moons: no gossamer ring; a rocky moon has no geysers.
        let rocky = [moon("d-1", 6.0, None, false)];
        assert!((0..100).all(|seed| dust_rings(None, &rocky, seed).is_empty()));
    }

    #[test]
    fn a_jupiter_has_only_dust_rings_and_a_uranus_a_belt_round_its_mab() {
        let moon = |id: &str, a: f64| DustSource {
            id: id.to_string(),
            a_radii: a,
            role: Some(RingRole::Source),
            icy: true,
        };
        // Amalthea and Thebe beyond Jupiter's faint main ring; Mab far out.
        let moons = [moon("e-1", 2.54), moon("e-2", 3.10), moon("e-3", 3.82)];
        let rings = faint();
        let (mut gossamer, mut belts) = (0, 0);
        for seed in 0..200 {
            for ring in dust_rings(Some(&rings), &moons, seed) {
                assert_eq!(ring.kind, DustKind::Gossamer);
                if ring.source == "e-3" {
                    belts += 1;
                    // Round its orbit, as the μ ring round Mab.
                    assert!((ring.inner / ring.peak - 0.913).abs() < 1e-3);
                    assert!((ring.outer / ring.peak - 1.087).abs() < 1e-3);
                } else {
                    gossamer += 1;
                    // From the main ring's edge out to its moon and past it.
                    assert_eq!(ring.inner, rings.outer);
                    assert!(ring.outer >= ring.peak);
                }
            }
        }
        assert!((280..360).contains(&gossamer), "{gossamer}");
        assert!((140..180).contains(&belts), "{belts}");
    }
}
