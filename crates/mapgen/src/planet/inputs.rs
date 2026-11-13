//! What a planet is made of, drawn from where and around what it formed
//! (spec "Inputs"): its iron core share from the star's metallicity, its
//! water from its distance to the ice line, its hydrogen and helium
//! envelope from its mass (core accretion), its volatiles from its rock
//! and ice, and its radiogenic abundance ([`draw_inputs`]).
//!
//! No type name enters here: the inputs follow from the formation
//! [`Context`] and the mass only.
use rand::RngExt;

use super::{Exposure, GIANT_MIN_MASS_EARTH, PlanetInputs, Volatiles};
use crate::giants::normal;
use crate::star::StarInfo;

/// Where and around what a planet formed, and what its star gives it now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Context {
    /// Mass of the star, solar masses.
    pub star_mass: f64,
    /// Metallicity of the star, `[Fe/H]` in dex (0 the Sun's).
    pub metallicity: f64,
    /// Semi-major axis of the planet's orbit, AU.
    pub orbit_au: f64,
    /// Eccentricity of the orbit (0 circular).
    pub eccentricity: f64,
    /// Distance of the system's ice line, AU.
    pub ice_line_au: f64,
    /// Light received over the year, Earth's = 1.
    pub insolation: f64,
    /// Equilibrium temperature, K.
    pub equilibrium_k: f64,
    /// Age of the system, Gyr.
    pub age_gyr: f64,
    /// Effective temperature of the star, K.
    pub teff_k: f64,
}

impl Context {
    /// The context of the planet `star` describes (its orbit, light and
    /// age), around a star of `metallicity` (`[Fe/H]`, dex) whose ice line
    /// lies at `ice_line_au`.
    pub fn of(star: &StarInfo, metallicity: f32, ice_line_au: f32) -> Self {
        Self {
            star_mass: f64::from(star.mass),
            metallicity: f64::from(metallicity),
            orbit_au: f64::from(star.orbit_au),
            eccentricity: f64::from(star.eccentricity),
            ice_line_au: f64::from(ice_line_au),
            insolation: f64::from(star.insolation),
            equilibrium_k: f64::from(star.equilibrium_k),
            age_gyr: f64::from(star.age_gyr),
            teff_k: f64::from(star.teff_k),
        }
    }

    /// What the star gives the planet, which sizes its envelope.
    pub fn exposure(&self) -> Exposure {
        Exposure {
            insolation: self.insolation,
            equilibrium_k: self.equilibrium_k,
            age_gyr: self.age_gyr,
        }
    }

    /// Orbital period, Earth days: Kepler's third law, `P = 365.25 d ·
    /// sqrt(a³ / M)` with `a` in AU and `M` in solar masses.
    pub fn period_days(&self) -> f64 {
        365.25 * (self.orbit_au.powi(3) / self.star_mass).sqrt()
    }
}

/// Mean iron core share of the refractory mass around a star of solar
/// metallicity. Calibration, no source: Earth's 0.325 taken as the mean
/// of all planets.
const CMF_MEAN: f64 = 0.325;

/// Growth of the mean core share per dex of `[Fe/H]`: planets of
/// iron-rich stars are iron-rich (Adibekyan et al. 2021, Science 374,
/// 330, doi:10.1126/science.abg8794, find the planets' iron fraction
/// rising with the star's, more steeply than one to one). Calibration, no
/// source, for its value: 0.15 per dex keeps the core share of the
/// metallicities drawn (about ±0.3 dex) round Earth's, between Mars
/// (about 0.2) and Venus (about 0.3).
const CMF_PER_DEX: f64 = 0.15;

/// Spread of the core share at a given metallicity. Calibration, no
/// source: it spans Mars to Venus and Earth.
const CMF_SPREAD: f64 = 0.05;

/// Bounds of the core share of a planet not stripped by giant impacts.
/// Calibration, no source.
const CMF_RANGE: (f64, f64) = (0.15, 0.55);

/// Closest orbits, as a share of the ice-line distance, where giant
/// impacts strip mantles, and how often they do. Calibration, no source:
/// Mercury, and the super-Mercuries Adibekyan et al. (2021) find a
/// distinct population (5 of their 22 rocky planets, from press reports
/// of the paper, not checked there; a sample biased towards them; a few
/// percent of small planets overall).
const STRIP_MAX_X: f64 = 0.3;
const STRIP_CHANCE: f64 = 0.05;

/// Core share of a stripped planet. Calibration, no source: brackets
/// Mercury's, about 0.7.
const STRIPPED_CMF: (f64, f64) = (0.55, 0.75);

/// Ice share of the heavy part (all but hydrogen and helium) of a body
/// formed beyond the ice line: from rock-ice moons (Ganymede, Titan about
/// 0.4 to 0.5) down to Ceres-like bodies (about 0.25). Calibration, no
/// source.
const WET: (f64, f64) = (0.2, 0.5);

/// `log10` of the water share of a body formed inside the ice line,
/// 1e-4 to 2e-3. Calibration, no source: Earth holds 2.3e-4 (Marty 2012,
/// Earth Planet. Sci. Lett. 313, 56, gives 1000 to 3000 ppm in the
/// silicate Earth, up to 2e-3 of the planet).
const DRY_LOG: (f64, f64) = (-4.0, -2.7);

/// Chance that a body inside the ice line formed beyond it and migrated
/// in, keeping its ices. Calibration, no source.
const MIGRATED_CHANCE: f64 = 0.1;

/// Below this mass, Earth masses, a body finishes forming after the gas
/// disc has gone and keeps no hydrogen. Calibration, no source: Mars,
/// whose growth ended within the disc's few Myr, holds none.
const GAS_MIN_MASS: f64 = 0.5;

/// Mass, Earth masses, from which every core forms within the gas disc;
/// between [`GAS_MIN_MASS`] and it, the chance rises linearly.
/// Calibration, no source.
const GAS_SURE_MASS: f64 = 5.0;

/// Envelope share of a 5 M⊕ core and its spread, dex: super-Earths that
/// assemble as the disc clears hold 1 to 10 % of their mass in gas (Lee &
/// Chiang 2016, doi:10.3847/0004-637X/817/2/90). The share grows in
/// proportion to the mass. Calibration, no source, for the slope and the
/// spread.
const ENVELOPE_AT_5: f64 = 0.02;
const ENVELOPE_SPREAD_DEX: f64 = 0.5;

/// Largest envelope share of a planet under [`GIANT_MIN_MASS_EARTH`].
/// Calibration, no source: the super-puffs of Lee & Chiang (2016), about
/// 20 % and more.
const ENVELOPE_MAX_SMALL: f64 = 0.3;

/// Envelope share of the ice giants, 10 to 30 M⊕. Calibration, no
/// source: Uranus and Neptune hold about 10 to 20 % of their mass in
/// hydrogen and helium.
const ICE_GIANT_ENVELOPE: (f64, f64) = (0.05, 0.3);

/// Mass from which a core runs away into a gas giant, Earth masses
/// (Pollack et al. 1996, doi:10.1006/icar.1996.0190: runaway accretion
/// once the envelope weighs as much as the core). Calibration, no source,
/// for 30 rather than the crossover of about 20.
const GAS_GIANT_MASS: f64 = 30.0;

/// Heavy-element mass of a gas giant, Earth masses, at most a tenth of
/// the planet. Calibration, no source: Saturn's 95 M⊕ keep about 15 to
/// 20 in heavy elements, at the top of this range; Jupiter's 318 M⊕ keep
/// more than the 20 M⊕ this allows it.
const GIANT_CORE: (f64, f64) = (5.0, 20.0);
const GIANT_CORE_MAX_SHARE: f64 = 0.1;

/// Carbon and nitrogen per kilogram of rock and iron: Earth and Venus
/// hold about 1e-4 of carbon (Marty 2012, Earth Planet. Sci. Lett. 313,
/// 56, gives up to 500 ppm in the silicate Earth) and Earth about 2e-6 of
/// nitrogen (about 2 ppm: not checked in Marty 2012, whose abstract says
/// nitrogen is depleted by about an order of magnitude against water).
/// Sulphur at 1e-4: calibration, no source.
const ROCK_CO2: f64 = 1e-4;
const ROCK_N2: f64 = 2e-6;
const ROCK_S: f64 = 1e-4;

/// Volatiles per kilogram of ice. CO2 at 5 %, CH4 and NH3 at 1 %: within
/// the ranges comets show relative to water (Mumma & Charnley 2011,
/// doi:10.1146/annurev-astro-081309-130811: CO2 a few to 30 %, CH4 and
/// NH3 below about 1.5 %). N2 at 0.1 %: calibration, no source (comets
/// hold little).
const ICE_CO2: f64 = 0.05;
const ICE_N2: f64 = 1e-3;
const ICE_CH4: f64 = 0.01;
const ICE_NH3: f64 = 0.01;

/// Spread, dex, of each volatile around its rule. Calibration, no source.
const VOLATILE_SPREAD_DEX: f64 = 0.3;

/// Spread, dex, of the radiogenic abundance at formation around Earth's:
/// stars differ in their thorium, uranium and potassium as the galaxy
/// enriches (Frank, Meyer & Mojzsis 2014, doi:10.1016/j.icarus.2014.08.031).
/// Calibration, no source, for 0.15 dex (a factor of 1.4).
const RADIOGENIC_SPREAD_DEX: f64 = 0.15;

/// Draws a planet's inputs from its formation `context` and its mass
/// (Earth masses). With `x` the orbit over the ice-line distance:
///
/// - core share `N(0.325 + 0.15 [Fe/H], 0.05)` within 0.15 to 0.55; inside
///   `x = 0.3`, 5 % are stripped by giant impacts to 0.55 to 0.75;
/// - ice share of the heavy part `U(0.2, 0.5)` beyond the ice line,
///   `10^U(−4, −2.7)` inside it, except 10 % that migrated in, drawn as
///   beyond it;
/// - envelope by [`envelope_fraction`];
/// - volatiles by [`volatiles`];
/// - radiogenic abundance `10^N(0, 0.15)` of Earth's;
/// - the life draw `U(0, 1)`, read by [`life`](super::life::life).
///
/// The ice share is of the mass that is not hydrogen and helium, so the
/// water share of the whole is that times `1 − envelope`: a wet giant
/// keeps its rock.
///
/// The draws come in a fixed order and count, whatever the branch taken:
/// core share (one normal), stripping (two uniforms), water (two
/// uniforms), migration (one uniform), envelope (three draws), volatiles
/// (five normals), radiogenic abundance (one normal), life (one uniform,
/// always last). A later draw (the
/// mass, in part 3) goes in front of them.
pub fn draw_inputs(context: &Context, mass_earth: f64, rng: &mut impl RngExt) -> PlanetInputs {
    let x = context.orbit_au / context.ice_line_au;

    let cmf = (CMF_MEAN + CMF_PER_DEX * context.metallicity + CMF_SPREAD * normal(rng))
        .clamp(CMF_RANGE.0, CMF_RANGE.1);
    let stripped = rng.random::<f64>() < STRIP_CHANCE;
    let stripped_cmf = rng.random_range(STRIPPED_CMF.0..=STRIPPED_CMF.1);
    let core_mass_fraction = if x < STRIP_MAX_X && stripped {
        stripped_cmf
    } else {
        cmf
    };

    let dry = 10f64.powf(rng.random_range(DRY_LOG.0..=DRY_LOG.1));
    let wet = rng.random_range(WET.0..=WET.1);
    let migrated = rng.random::<f64>() < MIGRATED_CHANCE;
    let ice_share = if x >= 1.0 || migrated { wet } else { dry };

    let envelope_fraction = envelope_fraction(mass_earth, rng);
    let water_fraction = ice_share * (1.0 - envelope_fraction);
    let refractory = 1.0 - water_fraction - envelope_fraction;
    let volatiles = volatiles(refractory, water_fraction, rng);
    let radiogenic = 10f64.powf(RADIOGENIC_SPREAD_DEX * normal(rng));
    let life_draw = rng.random::<f64>();

    PlanetInputs {
        mass_earth,
        core_mass_fraction,
        water_fraction,
        envelope_fraction,
        volatiles,
        radiogenic,
        life_draw,
    }
}

/// Hydrogen and helium share of a planet of `mass_earth` (core accretion,
/// Pollack et al. 1996, doi:10.1006/icar.1996.0190; Lee & Chiang 2016,
/// doi:10.3847/0004-637X/817/2/90):
///
/// - under 0.5 M⊕, none: it formed after the gas disc dispersed;
/// - from 0.5 to 10, it formed in the gas era with a chance
///   `(M − 0.5) / 4.5`, and then holds `10^N(log10(0.02 M / 5), 0.5)`,
///   at most 0.3; otherwise none;
/// - from 10 to 30, `U(0.05, 0.3)` (Uranus and Neptune 0.1 to 0.2);
/// - from 30, a gas giant: `1 − M_core / M` with
///   `M_core = min(U(5, 20), 0.1 M)`.
///
/// It draws a uniform, a normal and a uniform whatever the mass.
fn envelope_fraction(mass_earth: f64, rng: &mut impl RngExt) -> f64 {
    let formed_in_gas = rng.random::<f64>();
    let spread = normal(rng);
    let amount = rng.random::<f64>();
    if mass_earth < GAS_MIN_MASS {
        0.0
    } else if mass_earth < GIANT_MIN_MASS_EARTH {
        let chance = ((mass_earth - GAS_MIN_MASS) / (GAS_SURE_MASS - GAS_MIN_MASS)).clamp(0.0, 1.0);
        if formed_in_gas < chance {
            let median = ENVELOPE_AT_5 * mass_earth / 5.0;
            10f64
                .powf(median.log10() + ENVELOPE_SPREAD_DEX * spread)
                .min(ENVELOPE_MAX_SMALL)
        } else {
            0.0
        }
    } else if mass_earth < GAS_GIANT_MASS {
        ICE_GIANT_ENVELOPE.0 + (ICE_GIANT_ENVELOPE.1 - ICE_GIANT_ENVELOPE.0) * amount
    } else {
        let core = (GIANT_CORE.0 + (GIANT_CORE.1 - GIANT_CORE.0) * amount)
            .min(GIANT_CORE_MAX_SHARE * mass_earth);
        1.0 - core / mass_earth
    }
}

/// Volatiles carried by a `refractory` share of rock and iron and a
/// `water` share of ices, each rule ([`ROCK_CO2`] … [`ICE_NH3`]) scaled by
/// its own log-normal draw `10^N(0, 0.3)`, drawn in the order CO2, N2, S,
/// CH4, NH3:
///
/// ```text
/// co2 = (1e-4 r + 0.05 w) LN    n2 = (2e-6 r + 1e-3 w) LN
/// s   = 1e-4 r LN               ch4 = 0.01 w LN    nh3 = 0.01 w LN
/// ```
fn volatiles(refractory: f64, water: f64, rng: &mut impl RngExt) -> Volatiles {
    let rule = volatile_rule(refractory, water);
    let mut ln = || 10f64.powf(VOLATILE_SPREAD_DEX * normal(rng));
    let co2 = rule.co2 * ln();
    let n2 = rule.n2 * ln();
    let s = rule.s * ln();
    let ch4 = rule.ch4 * ln();
    let nh3 = rule.nh3 * ln();
    Volatiles {
        co2,
        n2,
        ch4,
        nh3,
        s,
    }
}

/// The volatiles of [`volatiles`] without their scatter: what a
/// `refractory` share of rock and iron and a `water` share of ices carry
/// by the rule alone.
pub(crate) fn volatile_rule(refractory: f64, water: f64) -> Volatiles {
    Volatiles {
        co2: ROCK_CO2 * refractory + ICE_CO2 * water,
        n2: ROCK_N2 * refractory + ICE_N2 * water,
        ch4: ICE_CH4 * water,
        nh3: ICE_NH3 * water,
        s: ROCK_S * refractory,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const DRAWS: usize = 10_000;
    const ICE_LINE_AU: f64 = 2.7;

    /// A Sun-like star's planet at `x` times the ice line, around a star
    /// of metallicity `feh`.
    fn context(x: f64, feh: f64) -> Context {
        let orbit_au = x * ICE_LINE_AU;
        let insolation = 1.0 / (orbit_au * orbit_au);
        Context {
            star_mass: 1.0,
            metallicity: feh,
            orbit_au,
            eccentricity: 0.0,
            ice_line_au: ICE_LINE_AU,
            insolation,
            equilibrium_k: 255.0 * insolation.powf(0.25),
            age_gyr: 4.6,
            teff_k: 5772.0,
        }
    }

    fn draws(context: &Context, mass_earth: f64, seed: u64) -> Vec<PlanetInputs> {
        let mut rng = StdRng::seed_from_u64(seed);
        (0..DRAWS)
            .map(|_| draw_inputs(context, mass_earth, &mut rng))
            .collect()
    }

    fn median(mut values: Vec<f64>) -> f64 {
        values.sort_by(f64::total_cmp);
        values[values.len() / 2]
    }

    fn mean(values: impl Iterator<Item = f64>) -> f64 {
        let (sum, n) = values.fold((0.0, 0usize), |(s, n), v| (s + v, n + 1));
        sum / n as f64
    }

    fn share(inputs: &[PlanetInputs], f: impl Fn(&PlanetInputs) -> bool) -> f64 {
        inputs.iter().filter(|i| f(i)).count() as f64 / inputs.len() as f64
    }

    /// The ice share of what is not hydrogen and helium.
    fn heavy_water(i: &PlanetInputs) -> f64 {
        i.water_fraction / (1.0 - i.envelope_fraction)
    }

    #[test]
    fn bodies_beyond_the_ice_line_are_wet() {
        let inputs = draws(&context(2.0, 0.0), 1.0, 1);
        for i in &inputs {
            let w = heavy_water(i);
            assert!((0.2..=0.5).contains(&w), "ice share {w}");
            if i.envelope_fraction == 0.0 {
                assert!((0.2..=0.5).contains(&i.water_fraction));
            }
        }
        let m = mean(inputs.iter().map(heavy_water));
        assert!(m >= 0.3, "mean ice share {m}");
    }

    #[test]
    fn bodies_inside_are_mostly_dry() {
        let inputs = draws(&context(0.4, 0.0), 1.0, 2);
        let med = median(inputs.iter().map(|i| i.water_fraction).collect());
        assert!(med < 2e-3, "median water {med}");
        let wet = share(&inputs, |i| i.water_fraction > 0.1);
        assert!((0.07..0.13).contains(&wet), "share above 0.1: {wet}");
    }

    #[test]
    fn metal_rich_stars_make_bigger_cores() {
        let rich = draws(&context(0.5, 0.3), 1.0, 3);
        let poor = draws(&context(0.5, -0.3), 1.0, 4);
        for i in rich.iter().chain(&poor) {
            assert!((0.1..=0.8).contains(&i.core_mass_fraction));
        }
        let gap = mean(rich.iter().map(|i| i.core_mass_fraction))
            - mean(poor.iter().map(|i| i.core_mass_fraction));
        assert!(gap >= 0.05, "CMF gap {gap}");
    }

    #[test]
    fn close_in_bodies_are_sometimes_stripped_to_mercuries() {
        // Inside 0.3 × the ice line 5 % are stripped (CMF 0.55 to 0.75),
        // beyond it none: the unstripped CMF stops at 0.55.
        let close = draws(&context(0.1, 0.0), 1.0, 5);
        let stripped = share(&close, |i| i.core_mass_fraction > 0.55);
        assert!((0.04..0.06).contains(&stripped), "stripped {stripped}");
        let far = draws(&context(0.5, 0.0), 1.0, 6);
        assert!(far.iter().all(|i| i.core_mass_fraction <= 0.55));
    }

    #[test]
    fn envelopes_follow_the_core_mass() {
        let ctx = context(0.5, 0.0);
        assert!(
            draws(&ctx, 0.3, 7)
                .iter()
                .all(|i| i.envelope_fraction == 0.0)
        );
        let earth = draws(&ctx, 1.0, 8);
        let thin = share(&earth, |i| i.envelope_fraction < 1e-3);
        assert!(thin >= 0.85, "share below 1e-3 at 1 M⊕: {thin}");
        let five = median(
            draws(&ctx, 5.0, 9)
                .iter()
                .map(|i| i.envelope_fraction)
                .collect(),
        );
        assert!((0.005..=0.05).contains(&five), "median at 5 M⊕: {five}");
        for i in draws(&ctx, 20.0, 10) {
            assert!((0.05..=0.3).contains(&i.envelope_fraction));
        }
        for i in draws(&ctx, 300.0, 11) {
            assert!(i.envelope_fraction >= 0.9);
        }
    }

    #[test]
    fn shares_never_exceed_the_whole() {
        // A wet giant: its ices share the heavy part with its rock.
        for mass in [0.3, 1.0, 5.0, 20.0, 30.0, 300.0, 3000.0] {
            for i in draws(&context(2.0, 0.0), mass, 12) {
                let rest = 1.0 - i.water_fraction - i.envelope_fraction;
                assert!(rest > 0.0, "refractory {rest} at {mass} M⊕");
            }
        }
    }

    #[test]
    fn ices_carry_their_volatiles() {
        let mut rng = StdRng::seed_from_u64(13);
        for _ in 0..DRAWS {
            let dry = volatiles(1.0, 0.0, &mut rng);
            assert_eq!((dry.ch4, dry.nh3), (0.0, 0.0));
            assert!(dry.co2 > 0.0 && dry.n2 > 0.0 && dry.s > 0.0);
        }
        let wet = draws(&context(2.0, 0.0), 1.0, 14);
        let dry = draws(&context(0.4, 0.0), 1.0, 15);
        for f in [|v: &Volatiles| v.ch4, |v: &Volatiles| v.nh3] {
            let w = median(wet.iter().map(|i| f(&i.volatiles)).collect());
            let d = median(dry.iter().map(|i| f(&i.volatiles)).collect());
            assert!(w > 50.0 * d, "wet {w}, dry {d}");
        }
        for mass in [1.0, 300.0] {
            for x in [0.4, 2.0] {
                for i in draws(&context(x, 0.0), mass, 16) {
                    assert!(i.volatiles.co2 > 0.0);
                }
            }
        }
    }

    #[test]
    fn radiogenic_abundance_scatters_round_earths() {
        let inputs = draws(&context(1.0, 0.0), 1.0, 17);
        let logs: Vec<f64> = inputs.iter().map(|i| i.radiogenic.log10()).collect();
        let m = mean(logs.iter().copied());
        let sd = mean(logs.iter().map(|l| (l - m).powi(2))).sqrt();
        assert!(m.abs() < 0.01, "mean log {m}");
        assert!((0.14..0.16).contains(&sd), "spread {sd}");
    }

    #[test]
    fn the_same_rng_gives_the_same_inputs() {
        let ctx = context(0.8, 0.1);
        for mass in [0.2, 1.0, 7.0, 20.0, 300.0] {
            assert_eq!(draws(&ctx, mass, 18), draws(&ctx, mass, 18));
        }
    }

    #[test]
    fn the_context_reads_the_star() {
        let star = StarInfo::new(
            crate::star::parse_type("G2V").unwrap(),
            1.0,
            0.0167,
            crate::star::DEFAULT_AGE_GYR,
        );
        let ctx = Context::of(&star, 0.1, 2.7);
        assert_eq!(ctx.star_mass, f64::from(star.mass));
        assert_eq!(ctx.metallicity, f64::from(0.1f32));
        assert_eq!(ctx.ice_line_au, f64::from(2.7f32));
        assert_eq!(ctx.age_gyr, f64::from(star.age_gyr));
        let exposure = ctx.exposure();
        assert_eq!(exposure.insolation, f64::from(star.insolation));
        assert_eq!(exposure.equilibrium_k, f64::from(star.equilibrium_k));
        assert!((ctx.period_days() - f64::from(star.year_days)).abs() < 0.5);
    }
}
