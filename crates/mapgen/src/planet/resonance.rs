//! Moons in mean-motion resonance and the heat their host's tides pump
//! into them (spec "Climate pass lot 3", part A, tidal heating).
//!
//! The regular moons of a giant form chains, each period about twice the
//! last ([`chains`]), as Io, Europa and Ganymede, or Enceladus and Dione.
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

/// Neighbouring period ratio, outer over inner, of two moons in a 2:1
/// resonance (open interval): 2 within the ±5 % jitter in semi-major
/// axis of the chain `crate::moons` draws (its `CHAIN_STEP`, whose
/// periods double).
pub const RESONANCE_BAND: (f64, f64) = (1.8, 2.2);

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

/// The chains among moons of periods `periods_s`, sorted by distance from
/// their host: the runs of consecutive moons whose neighbouring period
/// ratio lies in [`RESONANCE_BAND`], as index ranges into `periods_s`. A
/// moon without such a neighbour is in none (runs of one are left out).
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

/// The resonant heat, W, of each of `moons` (a host's moons nearest
/// first) around a host of `host_kg` and radius `host_radius_m`, `age_gyr`
/// old: for each of its [`chains`], the equilibrium heat of each pair
/// ([`equilibrium_heat_w`]) under the host's locked dissipation
/// ([`host_k2_over_q`] with the host's innermost round moon, whose
/// frequency the host's measured `k₂/Q` is taken at: Io's for Jupiter, Lainey et al.
/// 2009, doi:10.1038/nature08108; Enceladus's and its neighbours' for
/// Saturn, Lainey et al. 2012, doi:10.1088/0004-637X/752/1/14). `None`
/// for a moon in no chain, the outermost of a chain, and a moon whose
/// pair pumps nothing: it keeps the heat of its own tide. A moon's heat
/// today is the larger of the two ([`todays_heat_w`]).
pub fn resonant_heat(
    host_kg: f64,
    host_radius_m: f64,
    age_gyr: f64,
    moons: &[Orbiter],
) -> Vec<Option<f64>> {
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
        for (slot, w) in heat[chain].iter_mut().zip(pairs) {
            *slot = (w > 0.0).then_some(w);
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
                todays_heat_w(own, *r)
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
        assert_eq!(chains(&[1.0, 2.1, 8.0, 30.0, 59.0]), vec![0..2, 3..5]);
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
        assert_eq!(
            resonant_heat(SATURN_KG, SATURN_R_M, 4.57, &[enceladus(), heavy]),
            vec![None, None]
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
        // The pair heats its inner moon; Dione keeps its own tide.
        assert!(heat[0].is_some() && heat[1].is_none(), "{heat:?}");
        assert_eq!(
            resonant_heat(SATURN_KG, SATURN_R_M, 4.57, &[enceladus()]),
            vec![None]
        );
    }
}
