//! The methane a living, oxygenated air keeps: the steady state of a
//! biosphere's flux and the air's lifetime against it, study 37
//! (`docs/science/life-gases.md`, Wiki `37-life-gases-audit`, issue
//! #719), decided with the user on 2026-11-26.
//!
//! The burden of a gas is its flux times its lifetime, `B = Φ τ`; per
//! unit of surface a burden of mass `m` weighs `m g`, and the chain's
//! partial pressures are weights (a gas's mass share times the surface
//! pressure), so the partial pressure is `p = Φ_A τ g`, with `Φ_A` the
//! flux per square metre. A `Modern` biosphere (the planet engine's
//! stage, from 4 Gyr, which `life::oxygenate` gives [`modern_methane`])
//! takes Earth's pre-industrial natural flux per square metre, and the
//! air the pre-industrial lifetime times the host factor of calibration
//! 1 ([`methane_host_factor`]): a cooler host sends less of the near
//! ultraviolet that makes the OH radical, so the same flux keeps more
//! methane. No methane feedback (the lifetime does not grow with the
//! methane, #738); N₂O comes with N₂O as a gas (#737; its lines are in
//! the infrared column's table).

use crate::constants::{
    DAY, EARTH_EQUATORIAL_RADIUS, JULIAN_YEAR_DAYS, SOLAR_EFFECTIVE_TEMPERATURE,
};
use crate::units::{Bar, EarthFlux, Kelvin, MetrePerSecondSquared, Pascal, Year};

#[cfg(test)]
#[path = "life_gases_tests.rs"]
pub(crate) mod tests;

/// Source: Earth's natural emissions of methane in 1750, 202 ± 28 Tg of
/// CH₄ per year (Prather, Holmes & Hsu 2012, *Geophys. Res. Lett.* 39,
/// L09803, doi:10.1029/2012GL051440, Table 1): the flux of a `Modern`
/// biosphere ([`modern_methane`]), spread over Earth's surface
/// ([`earth_area_m2`]). Applying Earth's flux per square metre to every
/// `Modern` planet is a departure (#738), not a definition.
pub const EARTH_METHANE_EMISSIONS_TG_PER_YEAR: f64 = 202.0;

/// Source: the lifetime of methane in Earth's air of 1750, 9.5 ± 1.3
/// years (Prather et al. 2012, Table 1): the lifetime around the Sun,
/// which [`methane_host_factor`] carries to other hosts.
pub const EARTH_METHANE_LIFETIME: Year = Year(9.5);

/// Definition: kilograms in a teragram.
const KG_PER_TG: f64 = 1e9;

/// Definition: Earth's surface, m², `4 π R²` of
/// [`EARTH_EQUATORIAL_RADIUS`], 5.112e14 m², over which study 37 spreads
/// Earth's flux.
pub fn earth_area_m2() -> f64 {
    4.0 * std::f64::consts::PI * EARTH_EQUATORIAL_RADIUS.0 * EARTH_EQUATORIAL_RADIUS.0
}

/// Calibration: the surface methane, ppm, of an Earth-like planet at
/// Earth's modern fluxes around the F, G and K hosts of Rugheimer &
/// Kaltenegger 2018 (*ApJ* 854, 19, doi:10.3847/1538-4357/aaa47a,
/// arXiv:1712.10027, Table 2, modern epoch), at the hosts' effective
/// temperatures, K: F0V, F7V, G8V (τ Ceti), K2V, K7V. Each over the same
/// model's Sun ([`RK2018_SUN_CH4_PPM`]) is the host factor of
/// calibration 1 of `docs/science/life-gases.md`: a ratio of abundances
/// at one flux stands for a ratio of lifetimes. Searched and not found:
/// a law of methane's lifetime against a host's ultraviolet, or a grid
/// outside the Kasting family of codes.
const RK2018_CH4_PPM: [(f64, f64); 5] = [
    (7000.0, 3.58),
    (6250.0, 2.46),
    (5500.0, 420.0),
    (5000.0, 453.0),
    (4250.0, 125.0),
];

/// Source: the surface methane of the same planet around the Sun in
/// Rugheimer & Kaltenegger 2018's model, 1.73 ppm (Table 2), over which
/// [`RK2018_CH4_PPM`] are ratios. The Sun is placed at
/// [`SOLAR_EFFECTIVE_TEMPERATURE`].
const RK2018_SUN_CH4_PPM: f64 = 1.73;

/// Calibration: the surface methane, ppm, of an Earth-like planet at
/// Earth's fluxes around the active M dwarfs of Rugheimer et al. 2015a
/// (*ApJ* 809, 57, doi:10.1088/0004-637X/809/1/57, arXiv:1506.07202,
/// Table 3; temperatures, K, from its Table 1): M1, M3 and M8 (the M8's
/// 1000 ppm fixed by the authors, "the last stable run": a lower bound,
/// Earth's whole flux having no steady state there). Over the same
/// model's Sun ([`RUGHEIMER_2015A_SUN_CH4_PPM`]), the M dwarfs' host
/// factor of calibration 1, decided with the user on 2026-11-26 in
/// place of Rugheimer & Kaltenegger 2018's own M column, whose M3V's
/// 145 ppm zigzags against its neighbours.
const RUGHEIMER_2015A_CH4_PPM: [(f64, f64); 3] =
    [(3600.0, 370.0), (3200.0, 570.0), (2400.0, 1000.0)];

/// Source: the surface methane the fluxes of Rugheimer et al. 2015a give
/// around their Sun, 1.6 ppm (§2.3), over which
/// [`RUGHEIMER_2015A_CH4_PPM`] are ratios.
const RUGHEIMER_2015A_SUN_CH4_PPM: f64 = 1.6;

/// The host factor of methane's lifetime around a host of `star`
/// (calibration 1 of `docs/science/life-gases.md`): the ratio of a
/// photochemical model's surface methane around that host to the same
/// model's around the Sun, at the same flux, from
/// [`RUGHEIMER_2015A_CH4_PPM`] (M dwarfs), the Sun (1, at
/// [`SOLAR_EFFECTIVE_TEMPERATURE`]) and [`RK2018_CH4_PPM`] (F to K);
/// its logarithm linear in the temperature between the grid's points,
/// held past 2400 and 7000 K. It jumps 243-fold between the Sun and
/// τ Ceti (5772 to 5500 K), a step one star's measured ultraviolet sets
/// (the calibration's faults are listed on the page).
pub fn methane_host_factor(star: Kelvin) -> f64 {
    let mut points: Vec<(f64, f64)> = RUGHEIMER_2015A_CH4_PPM
        .iter()
        .map(|&(t, ppm)| (t, (ppm / RUGHEIMER_2015A_SUN_CH4_PPM).ln()))
        .chain(
            RK2018_CH4_PPM
                .iter()
                .map(|&(t, ppm)| (t, (ppm / RK2018_SUN_CH4_PPM).ln())),
        )
        .collect();
    points.push((SOLAR_EFFECTIVE_TEMPERATURE.0, 0.0));
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (first, last) = (points[0], points[points.len() - 1]);
    let t = star.0;
    let ln_ratio = if t.is_nan() {
        0.0
    } else if t <= first.0 {
        first.1
    } else if t >= last.0 {
        last.1
    } else {
        let i = points
            .partition_point(|p| p.0 <= t)
            .clamp(1, points.len() - 1);
        let (a, b) = (points[i - 1], points[i]);
        a.1 + (t - a.0) / (b.0 - a.0) * (b.1 - a.1)
    };
    ln_ratio.exp()
}

/// The lifetime of methane in a `Modern` air lit by `stars` (each
/// star's effective temperature and the flux it sends the planet):
/// [`EARTH_METHANE_LIFETIME`] times each host's
/// [`methane_host_factor`], the loss rates adding, each weighted by its
/// star's share of the light, `1/τ = Σ wᵢ/τᵢ` (calibration 2 of
/// `docs/science/life-gases.md`: no source treats two hosts). Stars
/// whose flux is not positive count for nothing; without any, the
/// lifetime is Earth's.
pub fn methane_lifetime(stars: &[(Kelvin, EarthFlux)]) -> Year {
    let total: f64 = stars.iter().map(|s| s.1.0).filter(|f| *f > 0.0).sum();
    if total <= 0.0 {
        return EARTH_METHANE_LIFETIME;
    }
    let rate: f64 = stars
        .iter()
        .filter(|s| s.1.0 > 0.0)
        .map(|&(star, flux)| flux.0 / total / methane_host_factor(star))
        .sum();
    Year(EARTH_METHANE_LIFETIME.0 / rate)
}

/// The methane of a `Modern` biosphere's air, bar by weight (as the
/// chain's partial pressures are), on a planet of surface `gravity` lit
/// by `stars` (each star's effective temperature and flux):
/// `p = Φ_A τ g`, with `Φ_A` Earth's natural emissions of 1750
/// ([`EARTH_METHANE_EMISSIONS_TG_PER_YEAR`]) over Earth's area
/// ([`earth_area_m2`]) and `τ` the lifetime under those stars
/// ([`methane_lifetime`]). Around the Sun on Earth's gravity (9.798 m/s²)
/// it is 3.68e-7 bar, 673 ppb by moles in the reference Earth's air
/// (study 37). None on a body without gravity.
///
/// The climate converts the air's weights to partial pressures, shares
/// of the moles, at each law it calls (`Weights::partial_pressures`,
/// issue #749): this methane reaches the infrared column as 672.8 ppb of
/// the moles in Earth's air. The planet engine's `Modern` stage reads it
/// through `life::oxygenate`.
pub fn modern_methane(stars: &[(Kelvin, EarthFlux)], gravity: MetrePerSecondSquared) -> Bar {
    let g = gravity.0;
    if g.is_nan() || g <= 0.0 {
        return Bar(0.0);
    }
    let seconds_per_year = JULIAN_YEAR_DAYS * DAY.0;
    let flux_kg_m2_s =
        EARTH_METHANE_EMISSIONS_TG_PER_YEAR * KG_PER_TG / (earth_area_m2() * seconds_per_year);
    let lifetime_s = methane_lifetime(stars).0 * seconds_per_year;
    Bar::from(Pascal(flux_kg_m2_s * lifetime_s * g))
}
