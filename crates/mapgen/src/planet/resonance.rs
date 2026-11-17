//! Moons in mean-motion resonance and the heat their host's tides pump
//! into them (spec "Climate pass lot 3", part A, tidal heating).
//!
//! The regular moons of a giant form chains, each period twice the last
//! ([`chains`]), as Io, Europa and Ganymede, or Enceladus and Dione: the
//! neighbours whose orbits converged on the commensurability as the
//! host's tides pushed them out ([`capture`]).
//! The host's tides push the inner moon outward; the resonance hands
//! part of that push to the outer ones and holds their eccentricities
//! up, and the moons' own tides turn the difference into heat. In
//! equilibrium that heat is set by the host's dissipation and the
//! orbits, not by the moons' own `k₂/Q` (Meyer & Wisdom 2007,
//! doi:10.1016/j.icarus.2007.03.001; [`equilibrium_heat_w`]); a moon's
//! heat today is the larger of that and its own tide ([`todays_heat_w`]). The host's
//! dissipation comes from resonance locking (Fuller, Luan & Quataert
//! 2016, doi:10.1093/mnras/stw609; [`host_k2_over_q`]), which gives
//! Saturn its 20-fold larger `k₂/Q` than Jupiter's without a value per
//! planet.

use std::f64::consts::PI;
use std::ops::Range;

use crate::exomoon::G;

/// Neighbouring period ratio, outer over inner, of two moons locked in a
/// 2:1 resonance (open interval): within 1 % of 2. Locked moons sit at
/// the commensurability, off it only by the precession of their
/// pericentres the resonance forces and by their libration: Io–Europa
/// 2.007, Europa–Ganymede 2.015, Enceladus–Dione 1.997 (JPL mean
/// orbital elements); a libration of a first-order resonance spans
/// about 1e-3 of the semi-major axis at these masses and
/// eccentricities (Murray & Dermott 1999, *Solar System Dynamics*,
/// §8.6, not checked). A pair further off is not in resonance: Dione
/// and Rhea, at 1.67, are not. The moons [`capture`] places in a
/// resonance sit at 2.
pub const RESONANCE_BAND: (f64, f64) = (1.98, 2.02);

/// Ratio of the semi-major axes of two moons at the 2:1
/// commensurability, outer over inner: `2^(2/3)` (Kepler's third law).
pub const COMMENSURATE_A_RATIO: f64 = 1.587_401_051_968_199_4;

/// Time the inner moon of a chain takes to migrate outward, `a/ȧ`, in
/// ages of its system, under resonance locking (Fuller, Luan & Quataert
/// 2016, doi:10.1093/mnras/stw609: a moon locked to a mode of its
/// planet migrates on a time of order the planet's age). Calibration:
/// the migrations measured from the drift of the moons' orbits, about
/// 3.6 Gyr for Io (Lainey et al. 2009, doi:10.1038/nature08108) and
/// 4.4 Gyr for Enceladus (Lainey et al. 2012,
/// doi:10.1088/0004-637X/752/1/14), at the Solar System's 4.57 Gyr.
pub const MIGRATION_AGES: f64 = 1.0;

/// Most a host's locked `k₂/Q` may reach: the strongest dissipation
/// measured in a giant, Saturn's at Titan's tidal frequency, `Q` about
/// 124 with `k₂` about 0.38 (Lainey et al. 2020, *Nature Astronomy*,
/// doi:10.1038/s41550-020-1120-5), so `k₂/Q` about 3e-3. Locking on a
/// light or distant inner moon would otherwise ask a dissipation no
/// giant has been seen to reach.
pub const MAX_HOST_K2_OVER_Q: f64 = 3e-3;

/// Seconds in a gigayear (Julian years).
const GYR_S: f64 = 1e9 * 365.25 * 86_400.0;

/// A moon in a host's chain, for [`resonant_heat`]: its mass, its orbit
/// (semi-major axis, period and eccentricity) and what its own tides
/// respond with (its radius and `k₂/Q`, see [`super::heat::k2_over_q`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbiter {
    /// Mass, kg.
    pub mass_kg: f64,
    /// Semi-major axis around the host, m.
    pub a_m: f64,
    /// Orbital period, s.
    pub period_s: f64,
    /// Eccentricity (0 circular).
    pub eccentricity: f64,
    /// Radius, m.
    pub radius_m: f64,
    /// Its own tidal response `k₂/Q`.
    pub k2_over_q: f64,
}

/// Mean motion of `moon`, rad/s: `2π / P`.
fn mean_motion(moon: &Orbiter) -> f64 {
    2.0 * PI / moon.period_s
}

/// Torque, N·m, the tide `moon` raises on a host of radius
/// `host_radius_m` with a response `host_k2_over_q` exerts back on the
/// moon's orbit: `T = (3/2)(k₂/Q) G m² R⁵ / a⁶` (Murray & Dermott 1999,
/// *Solar System Dynamics*, eq. 4.161, not checked; the form Meyer &
/// Wisdom 2007, doi:10.1016/j.icarus.2007.03.001, use). The host's mass
/// enters only through the orbit.
fn torque(host_radius_m: f64, host_k2_over_q: f64, moon: &Orbiter) -> f64 {
    1.5 * host_k2_over_q * G * moon.mass_kg * moon.mass_kg * host_radius_m.powi(5)
        / moon.a_m.powi(6)
}

/// Orbital angular momentum of `moon`, kg·m²/s: `L = m n a²`.
fn angular_momentum(moon: &Orbiter) -> f64 {
    moon.mass_kg * mean_motion(moon) * moon.a_m * moon.a_m
}

/// Rate, 1/s, at which the tide `moon` raises on a host of `host_kg` and
/// radius `host_radius_m` responding with `host_k2_over_q` pushes its
/// orbit out, `ȧ/a = 2T/L` ([`torque`], with `L = m √(G M a)`): it goes
/// as `m a^(-13/2)`, so a light or distant moon migrates slowly.
fn migration_rate(host_kg: f64, host_radius_m: f64, host_k2_over_q: f64, moon: &Orbiter) -> f64 {
    let momentum = moon.mass_kg * (G * host_kg * moon.a_m).sqrt();
    2.0 * torque(host_radius_m, host_k2_over_q, moon) / momentum
}

/// Which neighbouring pairs of a host's disk-born `moons` (nearest
/// first, at the orbits they formed on) the host's tides drove into a
/// 2:1 resonance over `age_gyr`, around a host of `host_kg` and radius
/// `host_radius_m`, with each captured outer moon moved onto the
/// commensurability: one flag per pair, `true` when its two moons are
/// locked.
///
/// Tides push each moon out at its own rate ([`migration_rate`], under
/// the host's locked [`host_k2_over_q`] on the innermost moon, as
/// [`resonant_heat`]). A pair is captured only when its orbits
/// converge, the inner moon migrating out faster than the outer one, and
/// reach the commensurability: a convergent pair meeting a first-order
/// resonance slowly, at the small eccentricities of regular moons, is
/// captured (Goldreich 1965, doi:10.1093/mnras/130.3.159; Peale 1976,
/// doi:10.1146/annurev.aa.14.090176.001243; Murray & Dermott 1999,
/// *Solar System Dynamics*, §8.12, not checked), a divergent one never.
/// The period ratio of the pair falls by `(3/2) Δ(ȧ/a)` times the age:
/// a pair formed wide of 2 by less than that reaches it; one formed
/// inside 2 (Dione and Rhea, at 1.67) moves away from it. The locked
/// moons then migrate together, at `2 ΣT / ΣL`, and may catch the next
/// one out: Io drove Europa, and the two Ganymede, into the Laplace
/// resonance (Yoder 1979, doi:10.1038/279767a0). Today's rates stand for
/// the whole age, a rough measure.
///
/// A moon placed ([`Placement`]: asked by archetype, or a partner placed
/// with one) is not moved: its pair is locked when it already sits in
/// [`RESONANCE_BAND`]. An asked moon is locked only on the sides its
/// archetype names a partner: Callisto, outside the Laplace chain, and
/// Ganymede, its outermost moon, are locked with no drawn moon outside
/// them, and an asked moon in the band of a neighbour it names none with
/// is moved just out of it. A drawn moon that is captured is moved to
/// [`COMMENSURATE_A_RATIO`] of its inner neighbour (period twice its),
/// and one not captured that lies in the band is moved just out of it,
/// so that the orbits say which pairs are locked ([`chains`]).
pub fn capture(
    host_kg: f64,
    host_radius_m: f64,
    age_gyr: f64,
    moons: &mut [Orbiter],
    placed: &[Placement],
) -> Vec<bool> {
    let Some(innermost) = moons.first() else {
        return Vec::new();
    };
    let k2q = host_k2_over_q(host_kg, host_radius_m, innermost, age_gyr);
    let age_s = age_gyr * GYR_S;
    let rate = |m: &Orbiter| migration_rate(host_kg, host_radius_m, k2q, m);
    let momentum = |m: &Orbiter| m.mass_kg * (G * host_kg * m.a_m).sqrt();
    // The torque and angular momentum of the locked group the next moon
    // out meets.
    let mut group = (
        rate(innermost) * momentum(innermost) / 2.0,
        momentum(innermost),
    );
    let mut locked = Vec::with_capacity(moons.len().saturating_sub(1));
    for i in 0..moons.len().saturating_sub(1) {
        let inner = moons[i];
        let outer = &mut moons[i + 1];
        let ratio = outer.period_s / inner.period_s;
        let in_band = RESONANCE_BAND.0 < ratio && ratio < RESONANCE_BAND.1;
        let barred = !placed[i].outward() || !placed[i + 1].inward();
        let captured = if barred {
            false
        } else if placed[i + 1] != Placement::Drawn {
            in_band
        } else {
            let closing = 1.5 * (2.0 * group.0 / group.1 - rate(outer)) * age_s;
            closing > 0.0 && ratio > RESONANCE_BAND.0 && (ratio / 2.0).ln() <= closing
        };
        if placed[i + 1] == Placement::Drawn || barred {
            let to = if captured {
                Some(2.0)
            } else if in_band {
                // Just out of the band, on the side it lies.
                Some(if ratio >= 2.0 {
                    RESONANCE_BAND.1 * 1.005
                } else {
                    RESONANCE_BAND.0 / 1.005
                })
            } else {
                None
            };
            if let Some(to) = to {
                let a_ratio = if captured {
                    COMMENSURATE_A_RATIO
                } else {
                    f64::powf(to, 2.0 / 3.0)
                };
                outer.a_m = inner.a_m * a_ratio;
                outer.period_s = inner.period_s * to;
            }
        }
        let (torque, l) = (rate(outer) * momentum(outer) / 2.0, momentum(outer));
        group = if captured {
            (group.0 + torque, group.1 + l)
        } else {
            (torque, l)
        };
        locked.push(captured);
    }
    locked
}

/// How a moon enters [`capture`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Drawn: moved onto the commensurability when captured, off it when
    /// not.
    Drawn,
    /// A partner an asked moon brings: kept where it is, locked with the
    /// neighbour in whose band it sits.
    Partner,
    /// Asked by archetype: kept where it is, locked only with a neighbour
    /// on a side its archetype names a partner on (`[moon.resonance]`).
    Asked {
        /// It names one inside it.
        inward: bool,
        /// It names one outside it.
        outward: bool,
    },
}

impl Placement {
    /// Whether it may be locked with its inner neighbour.
    fn inward(self) -> bool {
        match self {
            Self::Asked { inward, .. } => inward,
            _ => true,
        }
    }

    /// Whether it may be locked with its outer neighbour.
    fn outward(self) -> bool {
        match self {
            Self::Asked { outward, .. } => outward,
            _ => true,
        }
    }
}

/// The chains among moons of periods `periods_s`, sorted by distance from
/// their host: the runs of consecutive moons whose neighbouring period
/// ratio lies in [`RESONANCE_BAND`] (placed there by [`capture`] or by
/// their archetype), as index ranges into `periods_s`. A moon without
/// such a neighbour is in none (runs of one are left out).
pub fn chains(periods_s: &[f64]) -> Vec<Range<usize>> {
    let locked = |i: usize| {
        let ratio = periods_s[i + 1] / periods_s[i];
        RESONANCE_BAND.0 < ratio && ratio < RESONANCE_BAND.1
    };
    let mut runs = Vec::new();
    let mut start = 0;
    for i in 0..periods_s.len() {
        let ends = i + 1 == periods_s.len() || !locked(i);
        if ends {
            if i > start {
                runs.push(start..i + 1);
            }
            start = i + 1;
        }
    }
    runs
}

/// The tidal response `k₂/Q` of a host of `host_kg` and radius
/// `host_radius_m`, `age_gyr` old, under resonance locking: the host
/// dissipates so that `inner`, the inner moon of a chain, migrates
/// outward on `a/ȧ` = [`MIGRATION_AGES`] times the age (Fuller, Luan &
/// Quataert 2016, doi:10.1093/mnras/stw609). Its orbit's angular
/// momentum `L = m √(G M a)` grows at the torque `T` ([`torque`]), so
/// `ȧ/a = 2T/L`, and `k₂/Q = n a⁸ / (3 t G m R⁵)` with `t = a/ȧ`. At
/// 4.57 Gyr, Jupiter with Io gets 8.5e-6 (measured 1.1e-5, Lainey et al.
/// 2009, doi:10.1038/nature08108), Saturn with Enceladus 2.2e-4
/// (measured 2.3e-4, Lainey et al. 2012,
/// doi:10.1088/0004-637X/752/1/14). At most [`MAX_HOST_K2_OVER_Q`].
pub fn host_k2_over_q(host_kg: f64, host_radius_m: f64, inner: &Orbiter, age_gyr: f64) -> f64 {
    let migration_s = MIGRATION_AGES * age_gyr * GYR_S;
    let momentum = inner.mass_kg * (G * host_kg * inner.a_m).sqrt();
    // The torque that makes ȧ/a = 1/t, over the torque of a unit k₂/Q.
    let locked = momentum / (2.0 * migration_s) / torque(host_radius_m, 1.0, inner);
    locked.min(MAX_HOST_K2_OVER_Q)
}

/// Equilibrium tidal heat, W, of the pair `inner`–`outer` locked in a
/// resonance round a host of radius `host_radius_m` responding with
/// `host_k2_over_q` (Meyer & Wisdom 2007,
/// doi:10.1016/j.icarus.2007.03.001): the host's torques `Tᵢ`
/// ([`torque`]) feed the orbits energy `Σ nᵢTᵢ`; locked together, the
/// pair migrates as one at the mean motion `n̄ = Σ nᵢLᵢ / Σ Lᵢ` weighted
/// by the angular momenta `Lᵢ = mᵢ nᵢ aᵢ²`, whose orbits take `n̄ ΣTᵢ`;
/// the rest, `Σ nᵢTᵢ − n̄ ΣTᵢ`, is dissipated by the eccentricity the
/// resonance forces. At least 0: when the outer moon is pushed out
/// faster than the inner one, the pair would part, and nothing is
/// pumped.
pub fn pair_heat_w(
    host_radius_m: f64,
    host_k2_over_q: f64,
    inner: &Orbiter,
    outer: &Orbiter,
) -> f64 {
    let pair = [inner, outer];
    let torques = pair.map(|m| torque(host_radius_m, host_k2_over_q, m));
    let pumped: f64 = pair
        .iter()
        .zip(&torques)
        .map(|(m, t)| mean_motion(m) * t)
        .sum();
    let momentum: f64 = pair.iter().map(|m| angular_momentum(m)).sum();
    let mean_n = pair
        .iter()
        .map(|m| mean_motion(m) * angular_momentum(m))
        .sum::<f64>()
        / momentum;
    (pumped - mean_n * torques.iter().sum::<f64>()).max(0.0)
}

/// The equilibrium heat, W, each moon of `chain` (moons locked together
/// in resonance, nearest first) gets from the tides of a host of radius
/// `host_radius_m` responding with `host_k2_over_q`: each adjacent pair
/// ([`pair_heat_w`]) heats its inner moon, the outermost moon none.
///
/// The pair, not the whole chain, so that a moon's heat depends on its
/// own orbit and its outer neighbour's, not on the drawn moons further
/// out; the inner moon, as Meyer & Wisdom 2007
/// (doi:10.1016/j.icarus.2007.03.001) put all of the Enceladus–Dione
/// pair's heat in Enceladus: the inner moon, closer to the host, carries
/// the larger forced eccentricity and tide (Io over Europa, Enceladus
/// over Dione), and the share does not hinge on drawn eccentricities.
/// `host_kg` is unused by the pair heat and kept for symmetry with
/// [`host_k2_over_q`].
pub fn equilibrium_heat_w(
    host_kg: f64,
    host_radius_m: f64,
    host_k2_over_q: f64,
    chain: &[Orbiter],
) -> Vec<f64> {
    let _ = host_kg;
    (0..chain.len())
        .map(|i| match chain.get(i + 1) {
            Some(outer) => pair_heat_w(host_radius_m, host_k2_over_q, &chain[i], outer),
            None => 0.0,
        })
        .collect()
}

/// A moon's tidal heat today, W, from the heat of its own tide at its
/// drawn eccentricity (`own_w`, [`super::heat::tidal_power_w`]) and the equilibrium
/// heat of its resonance (`resonant_w`, [`resonant_heat`]): the larger.
/// The eccentricity is drawn, not evolved: a moon dissipating more than
/// the equilibrium is caught on its way down, as Io dissipates faster
/// than its resonance resupplies (Lainey et al. 2009,
/// doi:10.1038/nature08108); one dissipating less would have its
/// eccentricity pumped up until it reaches the equilibrium.
pub fn todays_heat_w(own_w: f64, resonant_w: Option<f64>) -> f64 {
    resonant_w.map_or(own_w, |r| own_w.max(r))
}

/// The resonance a moon is locked in, as its heat and its surface see
/// it ([`resonant_heat`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resonance {
    /// Equilibrium heat of its pair, W ([`pair_heat_w`]): 0 for the
    /// outermost moon of its chain, whose pair heats its inner moon, and
    /// for a moon whose pair pumps nothing.
    pub heat_w: f64,
    /// Its host's tidal response `k₂/Q`, locked on the host's innermost
    /// round moon ([`host_k2_over_q`]), that the heat follows from.
    pub host_k2_over_q: f64,
    /// Whether the chain captured it: every moon of a chain but its
    /// innermost, which its inner neighbour's outward migration met
    /// ([`capture`]). Its passage into the resonance pumped its
    /// eccentricity for a while (Showman & Malhotra 1997,
    /// doi:10.1006/icar.1996.5669), the tidal episode of
    /// [`super::surface_age::tidal_episode`].
    pub captured: bool,
}

/// The resonant heat, W, of each of `moons` (a host's moons nearest
/// first) around a host of `host_kg` and radius `host_radius_m`, `age_gyr`
/// old: for each of its [`chains`], the equilibrium heat of each pair
/// ([`equilibrium_heat_w`]) under the host's locked dissipation
/// ([`host_k2_over_q`] with the host's innermost round moon, whose
/// frequency the host's measured `k₂/Q` is taken at: Io's for Jupiter, Lainey et al.
/// 2009, doi:10.1038/nature08108; Enceladus's and its neighbours' for
/// Saturn, Lainey et al. 2012, doi:10.1088/0004-637X/752/1/14). `None`
/// for a moon in no chain: it keeps the heat of its own tide. The
/// outermost moon of a chain, and a moon whose pair pumps nothing, get a
/// heat of 0, so they too keep their own tide's. A moon's heat today is
/// the larger of the two ([`todays_heat_w`]). Each heat comes with the
/// host's `k₂/Q` it was derived under, and whether the chain captured
/// the moon ([`Resonance::captured`]).
pub fn resonant_heat(
    host_kg: f64,
    host_radius_m: f64,
    age_gyr: f64,
    moons: &[Orbiter],
) -> Vec<Option<Resonance>> {
    let mut heat = vec![None; moons.len()];
    let periods: Vec<f64> = moons.iter().map(|m| m.period_s).collect();
    let Some(innermost) = moons.first() else {
        return heat;
    };
    let k2q = host_k2_over_q(host_kg, host_radius_m, innermost, age_gyr);
    if !k2q.is_finite() {
        return heat;
    }
    for chain in chains(&periods) {
        let members = &moons[chain.clone()];
        let pairs = equilibrium_heat_w(host_kg, host_radius_m, k2q, members);
        for ((k, slot), w) in heat[chain].iter_mut().enumerate().zip(pairs) {
            *slot = Some(Resonance {
                heat_w: w.max(0.0),
                host_k2_over_q: k2q,
                captured: k > 0,
            });
        }
    }
    heat
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::heat::{ICY_K2_OVER_Q, ROCKY_K2_OVER_Q};

    const DAY_S: f64 = 86_400.0;
    const SATURN_KG: f64 = 5.683e26;
    const SATURN_R_M: f64 = 6.0268e7;
    const JUPITER_KG: f64 = 1.898e27;
    const JUPITER_R_M: f64 = 7.149e7;

    fn enceladus() -> Orbiter {
        Orbiter {
            mass_kg: 1.08e20,
            a_m: 2.3802e8,
            period_s: 1.370 * DAY_S,
            eccentricity: 0.0047,
            radius_m: 2.521e5,
            k2_over_q: ICY_K2_OVER_Q,
        }
    }

    fn dione() -> Orbiter {
        Orbiter {
            mass_kg: 1.095e21,
            a_m: 3.774e8,
            period_s: 2.737 * DAY_S,
            eccentricity: 0.0022,
            radius_m: 5.61e5,
            k2_over_q: ICY_K2_OVER_Q,
        }
    }

    fn io() -> Orbiter {
        Orbiter {
            mass_kg: 8.93e22,
            a_m: 4.217e8,
            period_s: 1.769 * DAY_S,
            eccentricity: 0.0041,
            radius_m: 1.8216e6,
            k2_over_q: ROCKY_K2_OVER_Q,
        }
    }

    fn europa() -> Orbiter {
        Orbiter {
            mass_kg: 4.80e22,
            a_m: 6.709e8,
            period_s: 3.551 * DAY_S,
            eccentricity: 0.009,
            radius_m: 1.5608e6,
            k2_over_q: ICY_K2_OVER_Q,
        }
    }

    fn ganymede() -> Orbiter {
        Orbiter {
            mass_kg: 1.48e23,
            a_m: 1.0704e9,
            period_s: 7.155 * DAY_S,
            eccentricity: 0.0013,
            radius_m: 2.6341e6,
            k2_over_q: ICY_K2_OVER_Q,
        }
    }

    #[test]
    fn enceladus_and_dione_heat_as_observed() {
        // Saturn's k2/Q measured by Lainey et al. 2012.
        let heat = equilibrium_heat_w(SATURN_KG, SATURN_R_M, 2.3e-4, &[enceladus(), dione()]);
        assert!(
            (10e9..20e9).contains(&heat[0]),
            "Enceladus {} W (Dione {} W)",
            heat[0],
            heat[1]
        );
    }

    #[test]
    fn locking_gives_saturn_and_jupiter_their_measured_dissipation() {
        let saturn = host_k2_over_q(SATURN_KG, SATURN_R_M, &enceladus(), 4.57);
        assert!((1.5e-4..3.5e-4).contains(&saturn), "Saturn {saturn}");
        let jupiter = host_k2_over_q(JUPITER_KG, JUPITER_R_M, &io(), 4.57);
        assert!((0.6e-5..1.6e-5).contains(&jupiter), "Jupiter {jupiter}");
    }

    #[test]
    fn the_laplace_chain_heats_io_most() {
        // Today's heat: each moon's own tide or its pair's equilibrium.
        let moons = [io(), europa(), ganymede()];
        let resonant = resonant_heat(JUPITER_KG, JUPITER_R_M, 4.57, &moons);
        let heat: Vec<f64> = moons
            .iter()
            .zip(&resonant)
            .map(|(m, r)| {
                let own = crate::planet::heat::tidal_power_w(
                    JUPITER_KG,
                    m.radius_m,
                    m.a_m,
                    m.eccentricity,
                    m.k2_over_q,
                );
                todays_heat_w(own, r.map(|r| r.heat_w))
            })
            .collect();
        let total: f64 = heat.iter().sum();
        assert!((2e13..2e14).contains(&total), "{heat:?}");
        assert!(heat[0] > 0.8 * total, "{heat:?}");
    }

    #[test]
    fn chains_need_a_two_to_one() {
        assert_eq!(chains(&[1.0, 2.0, 4.0, 16.0]), vec![0..3]);
        assert_eq!(chains(&[1.0, 4.0]), Vec::<std::ops::Range<usize>>::new());
        // Two runs, and a lone moon between them.
        assert_eq!(chains(&[1.0, 2.01, 8.0, 30.0, 59.9]), vec![0..2, 3..5]);
        // Near 2 is not at it: Dione and Rhea, and a pair 10 % wide.
        assert_eq!(chains(&[1.0, 1.67]), Vec::<std::ops::Range<usize>>::new());
        assert_eq!(chains(&[1.0, 2.2]), Vec::<std::ops::Range<usize>>::new());
    }

    /// `moon` moved out to a period `ratio` times `inner`'s.
    fn at_ratio(inner: &Orbiter, moon: Orbiter, ratio: f64) -> Orbiter {
        Orbiter {
            a_m: inner.a_m * ratio.powf(2.0 / 3.0),
            period_s: inner.period_s * ratio,
            ..moon
        }
    }

    #[test]
    fn a_convergent_pair_formed_wide_of_two_is_captured_at_it() {
        // Io pushes out faster than Europa: formed 8 % wide, the pair
        // closes on the 2:1 and locks there.
        let mut moons = [io(), at_ratio(&io(), europa(), 2.16)];
        let locked = capture(
            JUPITER_KG,
            JUPITER_R_M,
            4.57,
            &mut moons,
            &[Placement::Drawn; 2],
        );
        assert_eq!(locked, vec![true]);
        assert!((moons[1].period_s / moons[0].period_s - 2.0).abs() < 1e-12);
        assert!((moons[1].a_m / moons[0].a_m - COMMENSURATE_A_RATIO).abs() < 1e-12);
        assert_eq!(chains(&moons.map(|m| m.period_s)), vec![0..2]);
    }

    #[test]
    fn a_pair_formed_inside_two_is_never_captured() {
        // Dione and Rhea: converging, but already inside the 2:1.
        for ratio in [1.67, 1.9] {
            let mut moons = [io(), at_ratio(&io(), europa(), ratio)];
            let locked = capture(
                JUPITER_KG,
                JUPITER_R_M,
                4.57,
                &mut moons,
                &[Placement::Drawn; 2],
            );
            assert_eq!(locked, vec![false], "{ratio}");
            assert!((moons[1].period_s / moons[0].period_s - ratio).abs() < 1e-12);
        }
    }

    #[test]
    fn a_divergent_pair_is_not_captured_and_leaves_the_band() {
        // A light inner moon under a heavy outer one: they part.
        let heavy = Orbiter {
            mass_kg: 1e23,
            ..dione()
        };
        let mut moons = [enceladus(), at_ratio(&enceladus(), heavy, 2.005)];
        let locked = capture(
            SATURN_KG,
            SATURN_R_M,
            4.57,
            &mut moons,
            &[Placement::Drawn; 2],
        );
        assert_eq!(locked, vec![false]);
        let ratio = moons[1].period_s / moons[0].period_s;
        assert!(ratio >= RESONANCE_BAND.1, "{ratio}");
        assert!(ratio < 2.04, "{ratio}");
    }

    #[test]
    fn a_far_pair_migrates_too_slowly_to_close_a_wide_gap() {
        // Two Ganymedes far out, 8 % wide: the host's tides hardly move
        // them.
        let inner = Orbiter {
            a_m: 3.0 * ganymede().a_m,
            period_s: 3f64.powf(1.5) * ganymede().period_s,
            ..ganymede()
        };
        let lighter = Orbiter {
            mass_kg: 0.5 * ganymede().mass_kg,
            ..ganymede()
        };
        let mut moons = [io(), inner, at_ratio(&inner, lighter, 2.16)];
        let locked = capture(
            JUPITER_KG,
            JUPITER_R_M,
            4.57,
            &mut moons,
            &[Placement::Partner, Placement::Partner, Placement::Drawn],
        );
        assert!(!locked[1], "{locked:?}");
    }

    #[test]
    fn io_drives_europa_and_ganymede_into_the_laplace_chain() {
        // Each formed 5 % wide of its 2:1: Io catches Europa, and the
        // pair Ganymede (Yoder 1979).
        let europa = at_ratio(&io(), europa(), 2.1);
        let ganymede = at_ratio(&europa, ganymede(), 2.1);
        let mut moons = [io(), europa, ganymede];
        let locked = capture(
            JUPITER_KG,
            JUPITER_R_M,
            4.57,
            &mut moons,
            &[Placement::Drawn; 3],
        );
        assert_eq!(locked, vec![true, true]);
        assert_eq!(chains(&moons.map(|m| m.period_s)), vec![0..3]);
    }

    #[test]
    fn a_placed_moon_stays_and_is_locked_only_at_the_commensurability() {
        let wide = at_ratio(&io(), europa(), 2.1);
        let mut moons = [io(), wide];
        assert_eq!(
            capture(
                JUPITER_KG,
                JUPITER_R_M,
                4.57,
                &mut moons,
                &[Placement::Partner; 2]
            ),
            vec![false]
        );
        assert_eq!(moons[1], wide);
        let mut moons = [io(), at_ratio(&io(), europa(), 2.0)];
        assert_eq!(
            capture(
                JUPITER_KG,
                JUPITER_R_M,
                4.57,
                &mut moons,
                &[Placement::Partner; 2]
            ),
            vec![true]
        );
    }

    #[test]
    fn an_asked_moon_locks_only_with_the_partners_it_names() {
        // Ganymede asked with Europa inside it and none outside: a drawn
        // moon formed 8 % wide of its 2:1, that Io catches once Ganymede
        // is drawn (see the Laplace test), is left wide of it.
        let asked = Placement::Asked {
            inward: true,
            outward: false,
        };
        let inner = at_ratio(&io(), europa(), 2.0);
        let asked_moon = at_ratio(&inner, ganymede(), 2.0);
        let outer = at_ratio(&asked_moon, ganymede(), 2.16);
        let mut moons = [io(), inner, asked_moon, outer];
        let locked = capture(
            JUPITER_KG,
            JUPITER_R_M,
            4.57,
            &mut moons,
            &[
                Placement::Drawn,
                Placement::Partner,
                asked,
                Placement::Drawn,
            ],
        );
        assert_eq!(locked, vec![true, true, false]);
        assert_eq!(moons[3], outer);
        // Drawn there, the same moon would have been captured.
        let mut drawn = [io(), inner, asked_moon, outer];
        let locked = capture(
            JUPITER_KG,
            JUPITER_R_M,
            4.57,
            &mut drawn,
            &[
                Placement::Drawn,
                Placement::Partner,
                Placement::Partner,
                Placement::Drawn,
            ],
        );
        assert_eq!(locked, vec![true, true, true]);
        // Callisto asked with no partner, in the band of a drawn moon
        // inside it: moved just out of it, the drawn moon kept.
        let inner = at_ratio(&io(), europa(), 2.0);
        let callisto = at_ratio(&inner, ganymede(), 2.005);
        let mut moons = [io(), inner, callisto];
        let alone = Placement::Asked {
            inward: false,
            outward: false,
        };
        let locked = capture(
            JUPITER_KG,
            JUPITER_R_M,
            4.57,
            &mut moons,
            &[Placement::Drawn, Placement::Drawn, alone],
        );
        assert_eq!(locked, vec![true, false]);
        assert_eq!(moons[1], inner);
        let ratio = moons[2].period_s / moons[1].period_s;
        assert!((RESONANCE_BAND.1..2.04).contains(&ratio), "{ratio}");
        assert_eq!(chains(&moons.map(|m| m.period_s)), vec![0..2]);
    }

    #[test]
    fn equal_migration_gives_no_heat() {
        // ȧ/a = 2T/L with T ∝ m²/a⁶ and L ∝ m a^½: equal for two moons
        // when m ∝ a^6.5.
        let inner = Orbiter {
            mass_kg: 1e20,
            ..enceladus()
        };
        let a2 = inner.a_m * 2f64.powf(2.0 / 3.0);
        let outer = Orbiter {
            mass_kg: inner.mass_kg * (a2 / inner.a_m).powf(6.5),
            a_m: a2,
            period_s: inner.period_s * 2.0,
            ..dione()
        };
        let k2q = 2.3e-4;
        let heat: f64 = equilibrium_heat_w(SATURN_KG, SATURN_R_M, k2q, &[inner, outer])
            .iter()
            .sum();
        let n_t: f64 = [inner, outer]
            .iter()
            .map(|o| mean_motion(o) * torque(SATURN_R_M, k2q, o))
            .sum();
        assert!(heat < 1e-6 * n_t, "{heat} W against {n_t}");
    }

    #[test]
    fn a_chain_without_net_heat_keeps_own_tides() {
        // A light inner moon under a heavy outer one: the pair migrates
        // at the outer moon's pace, faster than the inner one is pushed.
        let heavy = Orbiter {
            mass_kg: 1e23,
            ..dione()
        };
        let total: f64 = equilibrium_heat_w(SATURN_KG, SATURN_R_M, 2.3e-4, &[enceladus(), heavy])
            .iter()
            .sum();
        assert_eq!(total, 0.0);
        let heat = resonant_heat(SATURN_KG, SATURN_R_M, 4.57, &[enceladus(), heavy]);
        assert!(
            heat.iter().all(|r| r.is_some_and(|r| r.heat_w == 0.0)),
            "{heat:?}"
        );
    }

    #[test]
    fn a_light_distant_inner_moon_hits_the_dissipation_cap() {
        // A 100 km moonlet at 40 Jupiter radii would ask a k2/Q above 1.
        let moonlet = Orbiter {
            mass_kg: 1e19,
            a_m: 40.0 * JUPITER_R_M,
            ..io()
        };
        assert_eq!(
            host_k2_over_q(JUPITER_KG, JUPITER_R_M, &moonlet, 4.57),
            MAX_HOST_K2_OVER_Q
        );
        // Io stays under it.
        assert!(host_k2_over_q(JUPITER_KG, JUPITER_R_M, &io(), 4.57) < MAX_HOST_K2_OVER_Q);
    }

    #[test]
    fn only_chain_members_get_a_resonant_heat() {
        let heat = resonant_heat(SATURN_KG, SATURN_R_M, 4.57, &[enceladus(), dione()]);
        // The pair heats its inner moon; Dione keeps its own tide, but
        // the chain captured it, not Enceladus.
        let (inner, outer) = (heat[0].unwrap(), heat[1].unwrap());
        assert!(inner.heat_w > 0.0 && !inner.captured, "{heat:?}");
        assert!(outer.heat_w == 0.0 && outer.captured, "{heat:?}");
        assert_eq!(
            resonant_heat(SATURN_KG, SATURN_R_M, 4.57, &[enceladus()]),
            vec![None]
        );
    }
}
