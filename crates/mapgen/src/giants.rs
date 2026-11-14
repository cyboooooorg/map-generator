//! The giant planets of a world's system, drawn from its seed (backlog
//! item 14, step A; `docs/analysis/12-gas-giants.md`).
//!
//! **How many.** A Poisson number of gas giants (0.1 to 13 Jupiter masses)
//! and of ice giants (10 to 50 Earth masses). A Sun-like star of the Sun's
//! metallicity has 0.25 gas giants and 0.3 ice giants on average (Fulton
//! et al. 2021 count about 0.23 giants per star between 2 and 32 AU); gas
//! giants grow a hundredfold per dex of metallicity squared
//! (`10^(2 [Fe/H])`, Fischer & Valenti 2005) and with the star's mass
//! (Johnson et al. 2010), ice giants more weakly. About 1 % of Sun-like
//! stars have a hot Jupiter (Wright et al. 2012), five times fewer when the
//! world orbits close to its star: hot Jupiters are lonely (Steffen et al.
//! 2012).
//!
//! **Where.** Gas giants peak beyond the ice line, `2.7 √L` AU (Hayashi
//! 1981; Fulton et al. 2021): their distances are log-normal around twice
//! it, ice giants around five times it. Masses follow `dN/d ln M ∝ M^−0.31`
//! (Cumming et al. 2008) for gas giants, log-uniform for ice giants;
//! eccentricities Kipping's law, like the world's, none for hot Jupiters.
//!
//! **Spacing.** Neighbouring orbits keep [`hill_spacing`] mutual Hill radii
//! apart, apoapsis to periapsis: 10 for Earth-mass planets, fewer for
//! heavier ones (Chambers et al. 1996; Pu & Wu 2015), never under `2√3`
//! (Gladman 1993). A giant too close to the world or to another giant is
//! redrawn a few times, then dropped. At most [`MAX_GIANTS`].
//!
//! **Effects on the world.** An eccentric giant forces an eccentricity on
//! the world (Laplace–Lagrange secular theory; the request raises a drawn
//! eccentricity to it), and the giant outside the world's orbit sets how
//! many impactors reach it (Horner & Jones 2008; see [`impact_factor`]).
use crate::giant_clouds::Clouds;
use crate::noise::{stream, stream_seed};
use crate::star::{
    DEFAULT_AGE_GYR, StarValues, draw_eccentricity, equilibrium_k, mean_temperature_factor, values,
    year_days,
};
use crate::world::WorldParams;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

/// Most giants a system keeps.
pub const MAX_GIANTS: usize = 4;
/// Earth masses in a Jupiter mass, and in a solar mass.
pub const JUPITER_EARTH_MASSES: f32 = 317.8;
const SUN_EARTH_MASSES: f64 = 332_946.0;
/// Mean number of gas and ice giants around a Sun-like star of the Sun's
/// metallicity.
const GAS_GIANTS: f64 = 0.25;
const ICE_GIANTS: f64 = 0.3;
/// Share of Sun-like stars of the Sun's metallicity with a hot Jupiter.
pub const HOT_JUPITERS: f64 = 0.01;
/// Gas giant masses, Jupiter masses: from 0.1 (about a third of Saturn) to
/// the deuterium-burning limit, and the slope of `dN/d ln M`.
const GAS_MASS: (f64, f64) = (0.1, 13.0);
const GAS_MASS_SLOPE: f64 = -0.31;
/// Ice giant masses, Earth masses.
const ICE_MASS: (f64, f64) = (10.0, 50.0);
/// Distances, as `log10` of a multiple of the ice line: mean and spread.
const GAS_ORBIT: (f64, f64) = (0.3, 0.4);
const ICE_ORBIT: (f64, f64) = (0.7, 0.3);
/// Distances kept, AU.
const ORBIT_RANGE: (f64, f64) = (0.02, 60.0);
/// Hot Jupiter periods, days.
pub const HOT_PERIOD_DAYS: (f64, f64) = (1.0, 10.0);
/// Gap kept between two Earth-mass neighbours, in mutual Hill radii: about
/// what systems of a few Earth masses need to last a billion years (Pu &
/// Wu 2015; Obertas et al. 2017).
const EARTH_HILL_SPACING: f64 = 10.0;

/// Least gap kept between two neighbouring orbits, apoapsis to periapsis,
/// in mutual Hill radii, for a pair whose mean mass share of the star is
/// `mass_share` (`(m₁ + m₂) / 2M`).
///
/// [`EARTH_HILL_SPACING`] for Earth-mass planets. Heavier planets need
/// fewer Hill radii: Chambers et al. (1996) found the stable spacings of
/// different masses agree once scaled by `μ^¼` rather than by the Hill
/// radius's `μ^⅓`, so the spacing falls as `μ^(−1/12)`: about 6.4 for a
/// Jupiter and a Saturn, which keeps the Solar System's own pair (6.5
/// apart). Never below `2√3`, the limit under which even two planets alone
/// meet (Gladman 1993).
pub fn hill_spacing(mass_share: f64) -> f64 {
    const EARTH_SHARE: f64 = 1.0 / 332_946.0;
    (EARTH_HILL_SPACING * (mass_share / EARTH_SHARE).powf(-1.0 / 12.0)).max(2.0 * 3f64.sqrt())
}
/// Tries to place a giant before it is dropped.
const TRIES: usize = 6;
/// Metallicity of the stars nearby, dex: mean, spread and limits
/// (Casagrande et al. 2011).
pub const METALLICITY_MEAN: f32 = -0.1;
pub const METALLICITY_SPREAD: f32 = 0.2;
pub const METALLICITY: (f32, f32) = (-1.0, 0.5);
/// Largest forced eccentricity applied to a world.
const MAX_FORCED: f64 = 0.5;

/// Whether a world has the giant planets its seed draws, or none (giants
/// not modelled: the impact rate stays the Solar System's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum GiantsMode {
    #[default]
    Auto,
    None,
}

impl std::str::FromStr for GiantsMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "auto" => Ok(Self::Auto),
            "none" => Ok(Self::None),
            other => Err(format!("unknown giants mode {other:?} (auto or none)")),
        }
    }
}

/// What a giant is mostly made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum GiantKind {
    /// Hydrogen and helium (Jupiter, Saturn).
    Gas,
    /// Water, ammonia and methane under a thin envelope (Uranus, Neptune).
    Ice,
}

/// The rings of a giant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum Rings {
    None,
    /// Narrow or dusty, hard to see (Jupiter, Uranus, Neptune).
    Faint,
    /// Broad and icy (Saturn).
    Bright,
}

/// One giant planet of the system, as `world.json` stores it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Giant {
    /// Its letter in the system, by distance from the star (`b`, `c`, …).
    pub name: String,
    pub kind: GiantKind,
    /// Mass and radius, Earth units.
    pub mass_earth: f32,
    pub radius_earth: f32,
    /// Semi-major axis, AU, and eccentricity of its orbit.
    pub orbit_au: f32,
    pub eccentricity: f32,
    /// Orientation of its periapsis and its place on its orbit when the
    /// world is at its own starting point, radians: only a look.
    pub periapsis_angle: f32,
    pub phase: f32,
    /// Orbital period, Earth days.
    pub year_days: f32,
    /// Equilibrium temperature (Earth's albedo) and effective temperature
    /// with its own internal heat, K.
    pub equilibrium_k: f32,
    pub effective_k: f32,
    /// Cloud class of Sudarsky, Burrows & Pinto (2000), 1 to 5, from the
    /// effective temperature and the gravity: ammonia clouds, water clouds,
    /// clear, alkali "roasters", silicate clouds (see [`Clouds::of`]).
    pub class: u8,
    /// A hot giant (`Teq` over 1 000 K): inflated, dark, locked.
    pub hot: bool,
    /// Colour of its disk, sRGB in `[0, 1]`.
    pub colour: [f32; 3],
    pub rings: Rings,
    /// Seed of its own look (its bands and storms, for its own view).
    pub seed: u32,
}

/// The giants of a world's system and what they do to the world, as
/// `world.json` stores them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GiantSystem {
    /// Giants by distance from the star.
    pub giants: Vec<Giant>,
    /// Eccentricity the giants force on the world's orbit (secular theory):
    /// a drawn eccentricity is at least this.
    pub forced_eccentricity: f32,
    /// Impact rate on the world relative to the Solar System's, from the
    /// giant outside its orbit (see [`impact_factor`]).
    pub impact_factor: f32,
}

/// A metallicity drawn like those of the stars nearby, from its own random
/// stream (the other drawn values of a seed do not change).
pub fn draw_metallicity(seed: u32) -> f32 {
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(seed, stream::METALLICITY)));
    (METALLICITY_MEAN + METALLICITY_SPREAD * normal(&mut rng) as f32)
        .clamp(METALLICITY.0, METALLICITY.1)
}

/// A standard normal variate (Box–Muller).
pub(crate) fn normal(rng: &mut impl RngExt) -> f64 {
    let (u1, u2): (f64, f64) = (rng.random::<f64>().max(1e-12), rng.random());
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

/// A Poisson variate of mean `mean` (Knuth; means here are small).
fn poisson(mean: f64, rng: &mut impl RngExt) -> usize {
    let limit = (-mean).exp();
    let (mut k, mut p) = (0, rng.random::<f64>());
    while p > limit && k < 16 {
        k += 1;
        p *= rng.random::<f64>();
    }
    k
}

/// Mass of a world in Earth masses: its own, as [`WorldParams::mass_earth`]
/// weighs it (the rocky mass-radius relation); for the moon of a giant, its
/// giant's (they share the orbit).
fn world_mass_earth(p: &WorldParams) -> f64 {
    match crate::exomoon::HostGiant::draw(p) {
        Some(host) => f64::from(host.giant.mass_earth),
        None => f64::from(p.mass_earth()),
    }
}

/// The heat a hot giant's interior gets from its star's light, relative to
/// the most any giant gets, at an `equilibrium_k` (K, Earth's albedo, as
/// [`equilibrium_k`] gives it): the flux times the share of it turned into
/// heat, the Gaussian in `log F` of Thorngren & Fortney (2018, eq. 34 of the preprint:
/// 2.37 % at most, at `log F` = 0.14 with `F` in Gerg s⁻¹ cm⁻², spread
/// 0.37). The share falls at high fluxes faster than the flux grows: the
/// heat peaks at about 1 700 K, is 0.8 of it at 1 500 K, 0.5 at 2 200 K
/// and 0.04 at 1 000 K.
fn inflation_heat(equilibrium_k: f32) -> f32 {
    const PEAK_LOG_F: f32 = 0.14;
    const SPREAD: f32 = 0.37;
    // Earth's flux, Gerg s⁻¹ cm⁻².
    const EARTH_FLUX: f32 = 1.361e-3;
    let flux = (equilibrium_k / equilibrium_k_of_earth()).powi(4) * EARTH_FLUX;
    let ln_power =
        |x: f32| -(x - PEAK_LOG_F).powi(2) / (2.0 * SPREAD * SPREAD) + x * std::f32::consts::LN_10;
    let peak = PEAK_LOG_F + SPREAD * SPREAD * std::f32::consts::LN_10;
    (ln_power(flux.max(1e-12).log10()) - ln_power(peak)).exp()
}

/// Earth's equilibrium temperature as [`equilibrium_k`] gives it, K.
fn equilibrium_k_of_earth() -> f32 {
    equilibrium_k(1.0)
}

/// What a giant is made of, as shares of its mass (those of
/// [`crate::planet::PlanetInputs`]): it sets the radius of a giant under
/// 30 M⊕ (see [`giant_radius_of`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GiantComposition {
    /// Iron core share of the refractory (iron + rock) mass.
    pub core_mass_fraction: f64,
    /// Share of the mass in water and ices.
    pub water_fraction: f64,
    /// Hydrogen and helium share of the mass.
    pub envelope_fraction: f64,
}

impl GiantComposition {
    /// Neptune's (and Uranus's), rounded: 65 % water and ices, 15 %
    /// hydrogen and helium, the rest rock with a third of iron (Guillot
    /// 2005, doi:10.1146/annurev.earth.33.092203.122639; Helled et al.
    /// 2011, doi:10.1088/0004-637X/726/1/15). [`giant_radius`] takes it
    /// up to Neptune's mass.
    pub const NEPTUNE: Self = Self {
        core_mass_fraction: 0.30,
        water_fraction: 0.65,
        envelope_fraction: 0.15,
    };

    /// The composition of a planet of the model.
    pub fn of(inputs: &crate::planet::PlanetInputs) -> Self {
        Self {
            core_mass_fraction: inputs.core_mass_fraction,
            water_fraction: inputs.water_fraction,
            envelope_fraction: inputs.envelope_fraction,
        }
    }

    /// The planet model's inputs of a body of `mass_earth` of this
    /// composition (no volatiles, Earth's radiogenic abundance: the
    /// radius does not read them).
    pub(crate) fn inputs(&self, mass_earth: f64) -> crate::planet::PlanetInputs {
        crate::planet::PlanetInputs {
            mass_earth,
            core_mass_fraction: self.core_mass_fraction,
            water_fraction: self.water_fraction,
            envelope_fraction: self.envelope_fraction,
            volatiles: crate::planet::Volatiles::default(),
            radiogenic: 1.0,
            life_draw: 0.0,
        }
    }
}

/// Neptune's mass, Earth masses, and the radius the giant law gives it,
/// Earth radii (3.86 observed, NASA Neptune fact sheet): the first
/// anchor of [`gas_giant_radius`].
pub const NEPTUNE_MASS_EARTH: f64 = 17.15;
pub const NEPTUNE_RADIUS_EARTH: f64 = 3.88;

/// Mass from which a giant takes the law of the gas giants whatever its
/// composition, Earth masses: the planet model's gas giant line (under
/// it, an ice giant has 5 to 30 % gas; from it, 90 % and more).
const GAS_GIANT_LAW_MASS: f64 = crate::planet::inputs::GAS_GIANT_MASS;

/// Radius of a body of `mass_earth` and `composition` under `exposure`,
/// Earth radii: the planet model's solid layers
/// ([`crate::planet::interior::solid_interior`]) under its envelope
/// ([`crate::planet::interior::envelope_thickness`], the fit of Lopez &
/// Fortney 2014 thinned by the heavy elements of a wet planet). Uranus,
/// with Neptune's composition, comes out 3.80 R⊕ (3.98 observed), Neptune
/// 3.88 (3.86).
fn composition_radius(
    mass_earth: f64,
    composition: &GiantComposition,
    exposure: &crate::planet::Exposure,
) -> f64 {
    let inputs = composition.inputs(mass_earth);
    crate::planet::interior::solid_interior(&inputs).radius_earth
        + crate::planet::interior::envelope_thickness(
            mass_earth,
            composition.envelope_fraction,
            composition.water_fraction,
            exposure,
        )
}

/// The law of the gas giants, Earth radii: through Neptune, Saturn and
/// Jupiter (log-linear between them; Chen & Kipping 2017,
/// doi:10.3847/1538-4357/834/1/17), then `M^−0.04`; inflated when hot, by
/// the heat of [`inflation_heat`] (Thorngren & Fortney 2018), most near
/// 1 700 K, the less massive the more, from 30 M⊕. Below Neptune's mass,
/// which only a requested gas giant reaches (see [`giant_radius_of`]),
/// `M^0.59` (Chen & Kipping 2017).
pub(crate) fn gas_giant_radius(mass_earth: f32, equilibrium_k: f32) -> f32 {
    const ANCHORS: [(f32, f32); 3] = [(17.15, 3.88), (95.16, 9.45), (317.8, 11.21)];
    let m = mass_earth;
    let cold = if m <= ANCHORS[0].0 {
        ANCHORS[0].1 * (m / ANCHORS[0].0).powf(0.59)
    } else if m >= ANCHORS[2].0 {
        ANCHORS[2].1 * (m / ANCHORS[2].0).powf(-0.04)
    } else {
        let i = usize::from(m > ANCHORS[1].0);
        let ((m0, r0), (m1, r1)) = (ANCHORS[i], ANCHORS[i + 1]);
        let f = (m / m0).ln() / (m1 / m0).ln();
        (r0.ln() + (r1.ln() - r0.ln()) * f).exp()
    };
    let heat = inflation_heat(equilibrium_k);
    let inflation = (1.0 + 0.7 * heat * (JUPITER_EARTH_MASSES / m.max(30.0)).powf(0.25)).min(1.9);
    cold * if m > 30.0 { inflation } else { 1.0 }
}

/// Radius of a giant whose composition is not known, Earth radii, from its
/// mass (Earth masses) and its equilibrium temperature (K): the giant
/// neighbours of a world, the host giants of moons, the giant worlds
/// without a planet model and the giants placed before their model is
/// drawn.
///
/// Up to Neptune's mass, a giant of Neptune's composition
/// ([`GiantComposition::NEPTUNE`]) at the coldest light of the envelope
/// fit and 4.6 Gyr: 3.42 R⊕ at 5 M⊕, 3.64 at 10, 3.80 at Uranus's mass
/// and 3.88 at Neptune's. (A law of the mass alone, `M^0.59` (Chen &
/// Kipping 2017), fits a population whose gas share grows with its mass,
/// and gave Uranus 3.52 R⊕, 11.6 % small.) Above, the law of the gas
/// giants ([`gas_giant_radius`]), which carries that growing gas share
/// from Neptune to Saturn.
pub fn giant_radius(mass_earth: f32, equilibrium_k: f32) -> f32 {
    if f64::from(mass_earth) <= NEPTUNE_MASS_EARTH {
        let cold = crate::planet::Exposure {
            insolation: 0.1,
            equilibrium_k: f64::from(equilibrium_k),
            age_gyr: f64::from(DEFAULT_AGE_GYR),
        };
        composition_radius(f64::from(mass_earth), &GiantComposition::NEPTUNE, &cold) as f32
    } else {
        gas_giant_radius(mass_earth, equilibrium_k)
    }
}

/// Radius of a giant of known `composition`, Earth radii, from its mass
/// (Earth masses) under its star's `exposure`.
///
/// Under [`GAS_GIANT_LAW_MASS`] (30 M⊕) and under the gas giants' class
/// line (half the mass in gas, `planet::classify`), the radius its
/// composition gives ([`composition_radius`]), the same as the planet
/// model gives a planet that is no giant: at a fixed composition it barely
/// grows with the mass (a heavier core squeezes its envelope), and more
/// gas or more light make it larger. From either line, the law of the gas
/// giants ([`gas_giant_radius`]), whatever the composition: the model
/// draws no more than 30 % gas under 30 M⊕, so only a requested gas giant
/// (a `jupiter-like` asked at 20 M⊕) takes that law under 30 M⊕, and its
/// radius does not fall as its mass crosses 30 M⊕.
pub fn giant_radius_of(
    mass_earth: f32,
    composition: &GiantComposition,
    exposure: &crate::planet::Exposure,
) -> f32 {
    if f64::from(mass_earth) < GAS_GIANT_LAW_MASS
        && composition.envelope_fraction < crate::planet::classify::GAS_GIANT_MIN_ENVELOPE
    {
        composition_radius(f64::from(mass_earth), composition, exposure) as f32
    } else {
        gas_giant_radius(mass_earth, exposure.equilibrium_k as f32)
    }
}

/// Sudarsky class (1 to 5) of a giant of effective temperature `teff` (K)
/// and surface gravity `gravity` (m/s²): the decks that condense decide
/// classes I to III, the temperature and the gravity IV and V (see
/// [`Clouds::of`]).
pub fn cloud_class(teff: f32, gravity: f32) -> u8 {
    Clouds::of(teff, gravity).class
}

/// Jupiter's internal heat flux, W/m²: Li et al. 2018
/// (doi:10.1038/s41467-018-06107-2), from Cassini.
const JUPITER_INTERNAL_FLUX: f64 = 7.485;
/// Stefan–Boltzmann constant, W/(m² K⁴).
const STEFAN_BOLTZMANN: f64 = 5.670_374e-8;
/// Youngest age the cooling law is applied at, Gyr: the Baraffe et al.
/// (2003) tables start at 0.1 Gyr.
const YOUNGEST_GYR: f32 = 0.1;
/// Exponents of the cooling law: `T ∝ M^0.5 t^−0.29`, fitted to the COND
/// evolution tables of Baraffe et al. (2003, A&A 402, 701,
/// doi:10.1051/0004-6361:20030252, tables 2 to 5) for 1 to 10.5 Jupiter
/// masses and 0.5 to 10 Gyr (least squares in the logs; the fit is within
/// 10 % of every entry, 5 % for 3 to 10 Jupiter masses). The same T_eff at
/// 4.6 Gyr extrapolates to about 100 K for Jupiter.
const COOLING_MASS_EXPONENT: f32 = 0.5;
const COOLING_AGE_EXPONENT: f32 = -0.29;
/// Internal temperature of an ice giant, K: calibration, no source (little
/// internal heat compared with the gas giants').
const ICE_INTERNAL_K: f32 = 40.0;

/// Temperature of the heat a giant of `mass_earth` radiates from its
/// interior at `age_gyr`, K.
///
/// Giants cool as `T ∝ M^0.5 t^−0.29`, fitted to the evolution tables of
/// Baraffe et al. (2003; see [`COOLING_MASS_EXPONENT`]) and normalised to
/// Jupiter's own, 107.2 K at 4.6 Gyr (`7.485 W/m²`, Li et al. 2018,
/// doi:10.1038/s41467-018-06107-2). Against the tables, ten Jupiter masses
/// have about 340 K at 4.6 Gyr (Baraffe: 322 K at 5 Gyr), 530 K at 1 Gyr
/// (491 K) and 1 000 K at 0.1 Gyr (953 K). The old rule `100 K × (M/MJ)^0.4`
/// had no age. (Burrows et al. 2001, eq. 1, `L ∝ t^−1.3 M^2.64`,
/// doi:10.1103/RevModPhys.73.719, normalised to Jupiter overshoots by half
/// at ten Jupiter masses: its exponents belong to brown dwarfs.) Saturn
/// comes out at 59 K, under its real internal heat (an extra source such as
/// helium rain is not modelled).
///
/// An ice giant keeps [`ICE_INTERNAL_K`].
pub fn internal_temperature(kind: GiantKind, mass_earth: f32, age_gyr: f32) -> f32 {
    if kind == GiantKind::Ice {
        return ICE_INTERNAL_K;
    }
    let jupiter = (JUPITER_INTERNAL_FLUX / STEFAN_BOLTZMANN).powf(0.25) as f32;
    let mass = mass_earth / JUPITER_EARTH_MASSES;
    jupiter
        * mass.powf(COOLING_MASS_EXPONENT)
        * (age_gyr.max(YOUNGEST_GYR) / DEFAULT_AGE_GYR).powf(COOLING_AGE_EXPONENT)
}

/// Surface gravity of a giant of `mass_earth` and `radius_earth`, m/s².
pub fn surface_gravity(mass_earth: f32, radius_earth: f32) -> f32 {
    9.806_65 * mass_earth / (radius_earth * radius_earth)
}

/// Colour of a giant's disk from its kind and class, varied by `t` in
/// `[0, 1]`: cream to ochre Jovians, cyan to blue cold ice giants, white
/// water clouds, azure clear skies, near-black roasters, and the pale
/// silicate clouds of class V (a Bond albedo near 0.5 against 0.03 for class
/// IV, Sudarsky et al. 2000). Ice giants keep a tinge of the methane that
/// absorbs the red wherever it stays in the gas.
pub fn giant_colour(kind: GiantKind, class: u8, t: f32) -> [f32; 3] {
    let mix = |a: [f32; 3], b: [f32; 3]| [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * t);
    let colour = match (kind, class) {
        (GiantKind::Ice, 1) => mix([0.66, 0.85, 0.88], [0.32, 0.5, 0.88]),
        (_, 1) => mix([0.86, 0.79, 0.66], [0.76, 0.6, 0.44]),
        (_, 2) => mix([0.94, 0.94, 0.91], [0.86, 0.88, 0.9]),
        (_, 3) => mix([0.42, 0.6, 0.86], [0.3, 0.48, 0.82]),
        (_, 4) => mix([0.2, 0.22, 0.3], [0.16, 0.16, 0.2]),
        _ => mix([0.74, 0.7, 0.64], [0.68, 0.64, 0.6]),
    };
    if kind == GiantKind::Ice && (2..=3).contains(&class) {
        crate::giant_clouds::methane_tint(colour)
    } else {
        colour
    }
}

/// Impact rate on a rocky planet relative to the Solar System's, from the
/// mass (Jupiter masses) of the giant just outside its orbit, after the
/// N-body runs of Horner & Jones (2008), moderated: low without a giant,
/// highest for about a fifth of Jupiter's mass, Jupiter's own the
/// calibration, lower for super-Jupiters.
pub fn impact_factor(giant_mass_jupiter: Option<f32>) -> f32 {
    const CURVE: [(f32, f32); 7] = [
        (0.01, 0.8),
        (0.05, 1.2),
        (0.2, 1.8),
        (0.5, 1.4),
        (1.0, 1.0),
        (3.0, 0.8),
        (13.0, 0.7),
    ];
    let Some(m) = giant_mass_jupiter else {
        return 0.7;
    };
    let x = m.clamp(CURVE[0].0, CURVE[6].0).log10();
    let i = CURVE
        .windows(2)
        .position(|w| x <= w[1].0.log10())
        .unwrap_or(5);
    let ((m0, f0), (m1, f1)) = (CURVE[i], CURVE[i + 1]);
    f0 + (f1 - f0) * ((x - m0.log10()) / (m1.log10() - m0.log10())).clamp(0.0, 1.0)
}

/// Eccentricity a giant on orbit `(a_g, e_g)` forces on a planet at `a`,
/// to first order in secular theory (Heppenheimer 1978): `(5/4)(a/a_g)
/// e_g/(1 − e_g²)` inside the giant's orbit, the ratio inverted outside it.
pub fn forced_eccentricity(a: f64, a_g: f64, e_g: f64) -> f64 {
    let ratio = if a < a_g { a / a_g } else { a_g / a };
    1.25 * ratio * e_g / (1.0 - e_g * e_g).max(1e-6)
}

/// An orbit being placed: its body's mass (solar masses) and its closest
/// and farthest distances.
#[derive(Clone, Copy)]
struct Track {
    mass: f64,
    orbit: f64,
    eccentricity: f64,
}

impl Track {
    /// Whether two orbits keep the least gap of [`hill_spacing`] mutual
    /// Hill radii between them.
    fn clear_of(&self, other: &Track, star_mass: f64) -> bool {
        let (inner, outer) = if self.orbit < other.orbit {
            (self, other)
        } else {
            (other, self)
        };
        let hill = ((self.mass + other.mass) / (3.0 * star_mass)).cbrt()
            * 0.5
            * (self.orbit + other.orbit);
        let gap =
            outer.orbit * (1.0 - outer.eccentricity) - inner.orbit * (1.0 + inner.eccentricity);
        gap >= hill_spacing(0.5 * (self.mass + other.mass) / star_mass) * hill
    }
}

impl GiantSystem {
    /// The giants of the world `p`, or `None` when its request turned them
    /// off. The same parameters always give the same giants.
    pub fn draw(p: &WorldParams) -> Option<Self> {
        if p.giants == GiantsMode::None {
            return None;
        }
        let star = values(p.star);
        let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(p.seed, stream::GIANTS)));
        let world = Track {
            mass: world_mass_earth(p) / SUN_EARTH_MASSES,
            orbit: f64::from(p.orbit_au),
            eccentricity: f64::from(p.eccentricity),
        };
        let metals = f64::from(p.metallicity);
        let star_mass = f64::from(star.mass);
        let mass_factor = star_mass.clamp(0.1, 2.5);
        let gas_mean = GAS_GIANTS * 10f64.powf(2.0 * metals).min(10.0) * mass_factor;
        let ice_mean = ICE_GIANTS * 10f64.powf(0.5 * metals) * mass_factor.sqrt();
        let ice_line = 2.7 * f64::from(star.luminosity).sqrt();
        // A rocky world close in leaves little room for a hot Jupiter.
        let lonely = if world.orbit < 0.5 { 0.2 } else { 1.0 };
        let hot_odds =
            (HOT_JUPITERS * 10f64.powf(2.0 * metals).min(10.0) * mass_factor * lonely).min(0.2);

        // Draw every candidate first, so their number does not change the
        // draws of the others.
        let hot = rng.random::<f64>() < hot_odds;
        let gas = poisson(gas_mean, &mut rng);
        let ice = poisson(ice_mean, &mut rng);
        let mut candidates: Vec<(GiantKind, bool)> = Vec::new();
        if hot {
            candidates.push((GiantKind::Gas, true));
        }
        candidates.extend(std::iter::repeat_n((GiantKind::Gas, false), gas));
        candidates.extend(std::iter::repeat_n((GiantKind::Ice, false), ice));

        let mut placed: Vec<(Track, GiantKind, bool, u32)> = Vec::new();
        for (kind, is_hot) in candidates {
            let mass_earth = match kind {
                GiantKind::Gas => {
                    let (lo, hi) = (
                        GAS_MASS.0.powf(GAS_MASS_SLOPE),
                        GAS_MASS.1.powf(GAS_MASS_SLOPE),
                    );
                    (lo + rng.random::<f64>() * (hi - lo)).powf(1.0 / GAS_MASS_SLOPE)
                        * f64::from(JUPITER_EARTH_MASSES)
                }
                GiantKind::Ice => {
                    (ICE_MASS.0.ln() + rng.random::<f64>() * (ICE_MASS.1 / ICE_MASS.0).ln()).exp()
                }
            };
            let seed = rng.random::<u32>();
            for _ in 0..TRIES {
                let (orbit, eccentricity) = if is_hot {
                    let days = (HOT_PERIOD_DAYS.0.ln()
                        + rng.random::<f64>() * (HOT_PERIOD_DAYS.1 / HOT_PERIOD_DAYS.0).ln())
                    .exp();
                    ((star_mass * (days / 365.25).powi(2)).cbrt(), 0.0)
                } else {
                    let (mean, spread) = if kind == GiantKind::Gas {
                        GAS_ORBIT
                    } else {
                        ICE_ORBIT
                    };
                    let orbit = ice_line * 10f64.powf(mean + spread * normal(&mut rng));
                    (orbit, f64::from(draw_eccentricity(&mut rng)))
                };
                if !(ORBIT_RANGE.0..=ORBIT_RANGE.1).contains(&orbit) {
                    continue;
                }
                let track = Track {
                    mass: mass_earth / SUN_EARTH_MASSES,
                    orbit,
                    eccentricity,
                };
                if track.clear_of(&world, star_mass)
                    && placed
                        .iter()
                        .all(|(other, ..)| track.clear_of(other, star_mass))
                {
                    placed.push((track, kind, is_hot, seed));
                    break;
                }
            }
            if placed.len() == MAX_GIANTS {
                break;
            }
        }
        placed.sort_by(|a, b| a.0.orbit.total_cmp(&b.0.orbit));

        let giants: Vec<Giant> = placed
            .iter()
            .enumerate()
            .map(|(i, &(track, kind, _, seed))| {
                // Lettered among the planets, the world's orbit included.
                let rank = i + usize::from(track.orbit > world.orbit);
                describe(&star, p.age_gyr, track, kind, seed, rank)
            })
            .collect();
        let forced = giants
            .iter()
            .map(|g| {
                forced_eccentricity(
                    world.orbit,
                    f64::from(g.orbit_au),
                    f64::from(g.eccentricity),
                )
                .powi(2)
            })
            .sum::<f64>()
            // An empty sum is −0.
            .max(0.0)
            .sqrt()
            .min(MAX_FORCED);
        // The impactors come from beyond the world, stirred by the most
        // massive giant just outside its orbit (1.5 to 20 times as far).
        let outer = giants
            .iter()
            .filter(|g| (1.5..=20.0).contains(&(f64::from(g.orbit_au) / world.orbit)))
            .map(|g| g.mass_earth / JUPITER_EARTH_MASSES)
            .max_by(f32::total_cmp);
        Some(GiantSystem {
            giants,
            forced_eccentricity: forced as f32,
            impact_factor: impact_factor(outer),
        })
    }
}

/// The stored description of a placed giant, the `rank`-th planet from the
/// star (see [`crate::naming`]).
fn describe(
    star: &StarValues,
    age_gyr: f32,
    track: Track,
    kind: GiantKind,
    seed: u32,
    rank: usize,
) -> Giant {
    let mut rng = StdRng::seed_from_u64(u64::from(seed));
    let mass_earth = (track.mass * SUN_EARTH_MASSES) as f32;
    let orbit_au = track.orbit as f32;
    let eccentricity = track.eccentricity as f32;
    let flux = star.luminosity / (orbit_au * orbit_au);
    let equilibrium = equilibrium_k(flux) * mean_temperature_factor(eccentricity);
    // Heat left from formation, cooled over the age of the system.
    let internal = internal_temperature(kind, mass_earth, age_gyr);
    let effective = (equilibrium.powi(4) + internal.powi(4)).powf(0.25);
    let radius_earth = giant_radius(mass_earth, equilibrium);
    let class = cloud_class(effective, surface_gravity(mass_earth, radius_earth));
    let hot = equilibrium > 1000.0;
    // The odds of the giant worlds' looks (`[giant] rings`): bright
    // rings only on cold gas giants, faint ones on most giants, rarely on
    // hot ones, whose ice would boil away.
    let [bright, faint] = match (hot, kind) {
        (true, _) => [0.0, 0.05],
        (false, GiantKind::Gas) => [0.25, 0.5],
        (false, GiantKind::Ice) => [0.0, 0.7],
    };
    let rings = match rng.random::<f32>() {
        r if r < bright => Rings::Bright,
        r if r < bright + faint => Rings::Faint,
        _ => Rings::None,
    };
    Giant {
        name: crate::naming::planet_letter(rank),
        kind,
        mass_earth,
        radius_earth,
        orbit_au,
        eccentricity,
        periapsis_angle: rng.random::<f32>() * std::f32::consts::TAU,
        phase: rng.random::<f32>() * std::f32::consts::TAU,
        year_days: year_days(orbit_au, star.mass),
        equilibrium_k: equilibrium,
        effective_k: effective,
        class,
        hot,
        colour: giant_colour(kind, class, rng.random()),
        rings,
        seed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::look::Look;
    use crate::star::lifetime_gyr;

    #[test]
    fn heavier_pairs_need_fewer_hill_radii() {
        let earth = 1.0 / 332_946.0;
        assert!((hill_spacing(earth) - 10.0).abs() < 1e-9);
        // A Jupiter and a Saturn: 6.4, under their own 6.5.
        let pair = 0.5 * (317.8 + 95.2) * earth;
        assert!(
            (hill_spacing(pair) - 6.4).abs() < 0.05,
            "{}",
            hill_spacing(pair)
        );
        // Never under Gladman's two-planet limit, even for a star's mass.
        assert!(hill_spacing(1.0) >= 2.0 * 3f64.sqrt());
        assert!((hill_spacing(1e3) - 2.0 * 3f64.sqrt()).abs() < 1e-9);
    }

    fn params(seed: u32, star: f32, orbit_au: f32, metallicity: f32) -> WorldParams {
        WorldParams {
            age_gyr: crate::star::DEFAULT_AGE_GYR,
            width: 64,
            height: 32,
            seed,
            class: "rocky".into(),
            archetype: None,
            model: None,
            look: Look::load("terran"),
            sea_level: 0.0,
            volcanic_intensity: 0.0,
            circumference_km: 40_075.0,
            axial_tilt_deg: 23.44,
            erosion: 0.0,
            pressure_bar: 1.0,
            surface_age_ga: 4.0,
            star,
            orbit_au,
            eccentricity: 0.02,
            drift_ma: 0.0,
            moons: crate::moons::MoonsMode::None,
            shape: None,
            metallicity,
            giants: GiantsMode::Auto,
            giant_mass_earth: None,
            host: crate::exomoon::HostMode::None,
            given_rings: None,
            impact_factor: None,
            locked_to_star: None,
            ridge: None,
            perihelion_longitude_deg: None,
            resonance_to_star: None,
            moon: false,
            day_hours: None,
            liquid_poleward_deg: None,
            asked_moons: Vec::new(),
            moon_sizes: Vec::new(),
            gravity: None,
        }
    }

    #[test]
    fn a_rocky_world_weighs_with_its_mass_radius_relation() {
        // Twice Earth's size: 3.2 g × 2² ≈ 12.8 Earth masses, not 2³ = 8.
        let mut p = params(7, 1.0, 1.0, 0.0);
        p.circumference_km = 2.0 * 40_075.0;
        let mass = world_mass_earth(&p);
        assert!(
            (mass - f64::from(p.mass_earth())).abs() < 1e-6,
            "{mass} against {}",
            p.mass_earth()
        );
        assert!(mass > 10.0, "{mass}");
    }

    #[test]
    fn the_radius_goes_through_the_solar_giants() {
        for (mass, radius) in [(17.15, 3.88), (95.16, 9.45), (317.8, 11.21)] {
            assert!((giant_radius(mass, 100.0) - radius).abs() < 0.01, "{mass}");
        }
        // Barely smaller at ten Jupiter masses; inflated when hot.
        assert!((giant_radius(3178.0, 100.0) / 11.21 - 0.912).abs() < 0.01);
        assert!(giant_radius(317.8, 1800.0) > 1.4 * 11.21);
        assert!(giant_radius(317.8, 1800.0) < 1.9 * 11.21);
        // The heat peaks near 1 700 K and falls on both sides (Thorngren &
        // Fortney 2018).
        assert!((inflation_heat(1725.0) - 1.0).abs() < 0.01);
        assert!((inflation_heat(1500.0) - 0.8).abs() < 0.05);
        assert!((inflation_heat(2200.0) - 0.5).abs() < 0.05);
        assert!(inflation_heat(1000.0) < 0.05);
        assert!(giant_radius(317.8, 2600.0) < giant_radius(317.8, 1800.0));
    }

    #[test]
    fn an_ice_giant_swells_with_its_gas_and_its_light() {
        let lit = |insolation: f64| crate::planet::Exposure {
            insolation,
            equilibrium_k: 278.0 * insolation.powf(0.25),
            age_gyr: 4.6,
        };
        let at = |m: f32, water: f64, envelope: f64, insolation: f64| {
            let composition = GiantComposition {
                core_mass_fraction: 0.3,
                water_fraction: water,
                envelope_fraction: envelope,
            };
            giant_radius_of(m, &composition, &lit(insolation))
        };
        // The default is the reference bodies' Uranus and Neptune.
        for name in ["Uranus", "Neptune"] {
            let body = crate::planet::reference::reference(name);
            let composition = GiantComposition {
                core_mass_fraction: body.core_mass_fraction,
                water_fraction: body.water_fraction,
                envelope_fraction: body.envelope_fraction,
            };
            assert_eq!(composition, GiantComposition::NEPTUNE, "{name}");
        }
        // Uranus with Neptune's composition (3.98 R⊕ observed; the mass
        // law alone gave 3.52), and Neptune.
        let uranus = giant_radius(14.54, 60.0);
        assert!((uranus - 3.80).abs() < 0.01, "{uranus}");
        assert!((giant_radius(17.15, 60.0) - 3.88).abs() < 1e-4);
        assert_eq!(giant_radius(14.54, 60.0), at(14.54, 0.65, 0.15, 0.01));
        // More gas, more light: larger, up to the gas giants' line. More
        // ice at the same gas share changes it by less than 15 %: the solid
        // layers grow, the envelope thins with its heavy elements.
        for m in [5.0, 10.0, 14.54, 17.15, 22.0, 29.9] {
            assert!(at(m, 0.6, 0.2, 1.0) > at(m, 0.6, 0.1, 1.0), "{m}");
            assert!(at(m, 0.0, 0.1, 100.0) > at(m, 0.0, 0.1, 1.0), "{m}");
            let (dry, wet) = (at(m, 0.0, 0.1, 1.0), at(m, 0.6, 0.1, 1.0));
            assert!((wet / dry - 1.0).abs() < 0.15, "{m}: {dry} {wet}");
        }
        // From it, the gas giants' law whatever the composition, as it
        // was.
        for m in [30.0, 31.8, 60.0, 95.16, 317.8, 3000.0] {
            for (water, envelope, insolation) in [(0.0, 0.06, 100.0), (0.6, 0.9, 0.01)] {
                let eq = 278.0 * f64::powf(insolation, 0.25);
                assert_eq!(
                    at(m, water, envelope, insolation),
                    gas_giant_radius(m, eq as f32),
                    "{m}"
                );
            }
            assert_eq!(giant_radius(m, 900.0), gas_giant_radius(m, 900.0));
        }
        // A giant of unknown composition grows with its mass up to
        // Jupiter's (then shrinks as `M^−0.04`).
        let mut last = 0.0;
        for i in 0..=209 {
            let m = 5.0 * 1.02f32.powi(i);
            let r = giant_radius(m, 60.0);
            assert!(r > last, "{m}: {r} after {last}");
            last = r;
        }
    }

    #[test]
    fn the_solar_giants_get_their_classes_and_the_curve_its_peak() {
        assert_eq!(cloud_class(124.0, 24.8), 1);
        assert_eq!(cloud_class(1200.0, 24.8), 4);
        assert_eq!(impact_factor(Some(1.0)), 1.0);
        assert!((impact_factor(Some(0.2)) - 1.8).abs() < 1e-5);
        assert!(impact_factor(None) < 1.0);
        // Jupiter (e = 0.049) forces about 0.012 on Earth.
        assert!((forced_eccentricity(1.0, 5.2, 0.049) - 0.0118).abs() < 0.001);
    }

    #[test]
    fn a_giant_cools_with_age_and_grows_hotter_with_mass() {
        // Jupiter's own: 7.485 W/m² of internal heat is 107 K.
        let jupiter = internal_temperature(GiantKind::Gas, JUPITER_EARTH_MASSES, DEFAULT_AGE_GYR);
        assert!((jupiter - 107.2).abs() < 0.3, "{jupiter}");
        // Saturn: the cooling law alone gives 59 K (it really has more).
        let saturn = internal_temperature(GiantKind::Gas, 95.16, DEFAULT_AGE_GYR);
        assert!((54.0..64.0).contains(&saturn), "{saturn}");
        // Baraffe et al. (2003) for ten Jupiter masses: 491 K at 1 Gyr,
        // 322 K at 5 Gyr, 953 K at 0.1 Gyr; within 10 %.
        let heavy = |age| internal_temperature(GiantKind::Gas, 10.0 * JUPITER_EARTH_MASSES, age);
        for (age, table) in [(1.0, 491.0), (5.0, 322.0), (0.1, 953.0)] {
            let got = heavy(age);
            assert!(
                (got / table - 1.0).abs() < 0.12,
                "{age} Gyr: {got} vs {table}"
            );
        }
        // And one Jupiter mass at 1 Gyr: 160 K.
        let one = internal_temperature(GiantKind::Gas, JUPITER_EARTH_MASSES, 1.0);
        assert!((one / 160.0 - 1.0).abs() < 0.2, "{one}");
    }

    #[test]
    fn an_old_massive_giant_far_from_its_star_keeps_its_own_heat() {
        // Ten Jupiter masses at 40 AU: it radiates its own 340 K, with no
        // sunlight to speak of, and sits near the edge of the water clouds.
        let star = values(42.0);
        let internal = internal_temperature(
            GiantKind::Gas,
            3178.0,
            DEFAULT_AGE_GYR.min(lifetime_gyr(star.mass)),
        );
        let teq = equilibrium_k(star.luminosity / 1600.0);
        let teff = (teq.powi(4) + internal.powi(4)).powf(0.25);
        assert!((320.0..380.0).contains(&teff), "{teff}");
        assert!((2..=3).contains(&cloud_class(teff, 60.0)));
    }

    #[test]
    fn sun_like_stars_have_giants_at_the_observed_rates() {
        let n = 4000;
        let systems: Vec<_> = (0..n)
            .map(|s| GiantSystem::draw(&params(s, 42.0, 1.0, 0.0)).unwrap())
            .collect();
        let gas: Vec<&Giant> = systems
            .iter()
            .flat_map(|s| &s.giants)
            .filter(|g| g.kind == GiantKind::Gas)
            .collect();
        let per_star = gas.len() as f64 / f64::from(n);
        assert!((0.17..0.3).contains(&per_star), "{per_star}");
        // Most beyond 1 AU; a few hot Jupiters.
        let far = gas.iter().filter(|g| g.orbit_au > 1.0).count() as f64 / gas.len() as f64;
        assert!(far > 0.75, "{far}");
        let hot = gas.iter().filter(|g| g.hot).count() as f64 / f64::from(n);
        assert!((0.003..0.02).contains(&hot), "{hot}");
        // Every giant clear of the world and of the others.
        for system in &systems {
            assert!(system.giants.len() <= MAX_GIANTS);
            for g in &system.giants {
                let gap = if g.orbit_au > 1.0 {
                    g.orbit_au * (1.0 - g.eccentricity) - 1.02
                } else {
                    0.98 - g.orbit_au * (1.0 + g.eccentricity)
                };
                assert!(gap > 0.0, "{g:?}");
            }
            assert!(
                system
                    .giants
                    .windows(2)
                    .all(|w| w[0].orbit_au < w[1].orbit_au)
            );
        }
    }

    #[test]
    fn metal_rich_and_massive_stars_have_more_giants() {
        let count = |star: f32, metals: f32| -> usize {
            (0..3000)
                .map(|s| {
                    GiantSystem::draw(&params(s, star, 1.0, metals))
                        .unwrap()
                        .giants
                        .iter()
                        .filter(|g| g.kind == GiantKind::Gas)
                        .count()
                })
                .sum()
        };
        let solar = count(42.0, 0.0);
        assert!(count(42.0, 0.3) > 2 * solar);
        assert!(count(64.0, 0.0) < solar / 2);
    }

    #[test]
    fn the_same_world_gets_the_same_giants_and_none_when_off() {
        let p = params(11, 42.0, 1.0, 0.1);
        assert_eq!(GiantSystem::draw(&p), GiantSystem::draw(&p));
        let off = WorldParams {
            giants: GiantsMode::None,
            ..p
        };
        assert_eq!(GiantSystem::draw(&off), None);
        // Metallicities near the Sun's, within the limits.
        let drawn: Vec<f32> = (0..2000).map(draw_metallicity).collect();
        let mean = drawn.iter().sum::<f32>() / drawn.len() as f32;
        assert!((mean - METALLICITY_MEAN).abs() < 0.02);
        assert!(
            drawn
                .iter()
                .all(|m| (METALLICITY.0..=METALLICITY.1).contains(m))
        );
    }
}
