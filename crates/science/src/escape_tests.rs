use super::*;
use crate::constants::{EARTH_MASS, EARTH_MEAN_RADIUS, GRAVITATIONAL_CONSTANT};
use crate::xuv::EARTH_XUV_FLUENCE;

/// Source: Earth's escape velocity, 11.186 km/s, NASA NSSDCA Earth Fact
/// Sheet (D. R. Williams, last updated 15 November 2024),
/// <https://nssdc.gsfc.nasa.gov/planetary/factsheet/earthfact.html>,
/// row "Escape velocity (km/s)".
const FACT_SHEET_ESCAPE_VELOCITY: MetrePerSecond = MetrePerSecond(11_186.0);

/// Definition: relative tolerance of a law checked against its own
/// formula evaluated by hand (round-off only).
const ROUND_OFF: f64 = 1e-12;

/// Definition: relative tolerance of the escape velocity against the fact
/// sheet, whose Earth radius (6371.000 km, volumetric) differs from
/// [`EARTH_MEAN_RADIUS`] by 8 m.
const FACT_SHEET_TOLERANCE: f64 = 0.005;

/// Definition: escape velocities, m/s, at which the shoreline is checked,
/// from an airless dwarf to a super-Earth.
const VELOCITIES: [f64; 4] = [2_000.0, 5_000.0, 11_186.0, 20_000.0];

fn relative(value: f64, expected: f64) -> f64 {
    (value / expected - 1.0).abs()
}

#[test]
fn the_loss_is_owen_and_wus_energy_limit() {
    let loss = energy_limited_loss(EARTH_MEAN_RADIUS, EARTH_MASS, EARTH_XUV_FLUENCE);
    // Owen & Wu 2017, eq. 19 integrated over the fluence, eta = 0.1.
    let by_hand = 0.1 * std::f64::consts::PI * EARTH_MEAN_RADIUS.0.powi(3) * EARTH_XUV_FLUENCE.0
        / (GRAVITATIONAL_CONSTANT * EARTH_MASS.0);
    assert!(
        relative(loss.0, by_hand) < ROUND_OFF,
        "{loss:?} against {by_hand}"
    );
}

#[test]
fn the_loss_scales_as_r_cubed_over_m() {
    let base = energy_limited_loss(EARTH_MEAN_RADIUS, EARTH_MASS, EARTH_XUV_FLUENCE).0;
    let wider = energy_limited_loss(EARTH_MEAN_RADIUS * 2.0, EARTH_MASS, EARTH_XUV_FLUENCE).0;
    let heavier = energy_limited_loss(EARTH_MEAN_RADIUS, EARTH_MASS * 4.0, EARTH_XUV_FLUENCE).0;
    assert!(
        relative(wider, base * 8.0) < ROUND_OFF,
        "{wider} against {base}"
    );
    assert!(
        relative(heavier, base / 4.0) < ROUND_OFF,
        "{heavier} against {base}"
    );
}

#[test]
fn no_fluence_no_loss() {
    let none = JoulePerSquareMetre(0.0);
    assert_eq!(
        energy_limited_loss(EARTH_MEAN_RADIUS, EARTH_MASS, none),
        Kilogram(0.0)
    );
    for v in VELOCITIES {
        for magnetised in [true, false] {
            assert_eq!(
                shoreline_kept(MetrePerSecond(v), none, magnetised).get(),
                1.0,
                "{v} {magnetised}"
            );
        }
    }
}

#[test]
fn earths_escape_velocity_is_11_2_km_s() {
    let v = escape_velocity(EARTH_MASS, EARTH_MEAN_RADIUS);
    assert!(
        relative(v.0, FACT_SHEET_ESCAPE_VELOCITY.0) < FACT_SHEET_TOLERANCE,
        "{v:?}"
    );
}

#[test]
fn the_shoreline_passes_half_at_its_line() {
    let kept = shoreline_kept(SHORELINE_VELOCITY, EARTH_XUV_FLUENCE, true).get();
    assert!(relative(kept, 0.5) < ROUND_OFF, "{kept}");
}

#[test]
fn a_missing_dynamo_doubles_the_fluence() {
    for v in VELOCITIES {
        for phi in [0.1, 1.0, 10.0, 1e3] {
            let fluence = EARTH_XUV_FLUENCE * phi;
            let bare = shoreline_kept(MetrePerSecond(v), fluence, false).get();
            let doubled = shoreline_kept(MetrePerSecond(v), fluence * 2.0, true).get();
            assert!(
                (bare - doubled).abs() <= ROUND_OFF * doubled.max(f64::MIN_POSITIVE),
                "{v} {phi}: {bare} against {doubled}"
            );
        }
    }
}

/// Source: Tang, Fortney & Murray-Clay 2024 (arXiv:2410.08577), Table 2,
/// the largest H/He mass fraction a planet keeps after the boil-off, read
/// on the paper's rendered page: `(core mass M⊕, flux F⊕, fraction)` for
/// every numeric cell. The per cent cells are divided by 100; the starred
/// cells (2 × 10⁻⁶, 6 × 10⁻⁶, 4 × 10⁻⁶) carry no per cent sign.
const TANG_TABLE_2: [(f64, f64, f64); 32] = [
    (1.5, 1.0, 1.18e-2),
    (2.4, 1.0, 3.38e-2),
    (3.6, 1.0, 8.62e-2),
    (1.5, 3.0, 0.49e-2),
    (2.4, 3.0, 1.79e-2),
    (3.6, 3.0, 4.93e-2),
    (5.5, 3.0, 14.64e-2),
    (1.5, 10.0, 0.14e-2),
    (2.4, 10.0, 0.77e-2),
    (3.6, 10.0, 2.56e-2),
    (5.5, 10.0, 8.85e-2),
    (1.5, 30.0, 0.02e-2),
    (2.4, 30.0, 0.25e-2),
    (3.6, 30.0, 1.17e-2),
    (5.5, 30.0, 4.44e-2),
    (8.5, 30.0, 16.75e-2),
    (1.5, 100.0, 2e-6),
    (2.4, 100.0, 0.03e-2),
    (3.6, 100.0, 0.30e-2),
    (5.5, 100.0, 1.67e-2),
    (8.5, 100.0, 7.21e-2),
    (13.0, 100.0, 23.49e-2),
    (2.4, 300.0, 6e-6),
    (3.6, 300.0, 0.04e-2),
    (5.5, 300.0, 0.53e-2),
    (8.5, 300.0, 3.14e-2),
    (13.0, 300.0, 11.13e-2),
    (3.6, 1000.0, 4e-6),
    (5.5, 1000.0, 0.04e-2),
    (8.5, 1000.0, 0.73e-2),
    (13.0, 1000.0, 4.04e-2),
    (20.0, 1000.0, 13.50e-2),
];

/// Source: Tang et al. 2024, Table 2, the cells marked "−", "zero or very
/// slight mass loss": `(core mass M⊕, flux F⊕)`.
const TANG_NO_LOSS: [(f64, f64); 14] = [
    (5.5, 1.0),
    (8.5, 1.0),
    (13.0, 1.0),
    (20.0, 1.0),
    (8.5, 3.0),
    (13.0, 3.0),
    (20.0, 3.0),
    (8.5, 10.0),
    (13.0, 10.0),
    (20.0, 10.0),
    (13.0, 30.0),
    (20.0, 30.0),
    (20.0, 100.0),
    (20.0, 300.0),
];

/// Source: Tang et al. 2024, Table 2, the cells marked "0", a planet that
/// keeps nothing: `(core mass M⊕, flux F⊕)`. With [`TANG_TABLE_2`] and
/// [`TANG_NO_LOSS`], the 49 cells of the grid.
const TANG_NOTHING_LEFT: [(f64, f64); 3] = [(1.5, 300.0), (1.5, 1000.0), (2.4, 1000.0)];

/// Source: Berger, Schlieder, Huber & Barclay 2023 (arXiv:2302.00009),
/// abstract: the radius valley moves with the insolation as `S^α`,
/// `α = 0.069 (+0.019/−0.023)`; the bounds of its 1σ interval.
const BERGER_ALPHA: (f64, f64) = (0.069 - 0.023, 0.069 + 0.019);

/// Source: Van Eylen et al. 2018 (MNRAS, doi:10.1093/mnras/sty1783),
/// conclusions: the valley at `log R = m log P + a`,
/// `m = −0.09 (+0.02/−0.04)`; the bounds of its interval.
const VAN_EYLEN_SLOPE: (f64, f64) = (-0.09 - 0.04, -0.09 + 0.02);

/// Source: Petigura et al. 2022 (AJ, doi:10.3847/1538-3881/ac51e3),
/// abstract: the valley's slope with period `m = −0.10 ± 0.03`; the
/// bounds of its interval.
const PETIGURA_SLOPE: (f64, f64) = (-0.10 - 0.03, -0.10 + 0.03);

/// Source: the radius of a rocky core, `R ∝ M^(1/4)`: Lopez & Fortney
/// 2014 (ApJ 792, 1), eq. 1, as `crate::envelope` reads it; Gupta &
/// Schlichting 2019 § 2.1, after Valencia et al. 2006.
const CORE_RADIUS_EXPONENT: f64 = 0.25;

/// Definition: round one star the flux falls as `a⁻²` and Kepler's third
/// law gives `a ∝ P^(2/3)`, so `F ∝ P^(−4/3)`.
const FLUX_PER_PERIOD: f64 = -4.0 / 3.0;

/// Definition: the fluxes over which study 30 measures the slope of the
/// stripping line, 30 and 1 000 F⊕, the rows of Tang et al.'s Table 2
/// where a core of 2 to 6 M⊕ is stripped (the study's chosen range).
const SLOPE_FLUXES: (f64, f64) = (30.0, 1000.0);

/// Source: HD 56414 b (Giacalone et al. 2022, ApJL,
/// doi:10.3847/2041-8213/ac80f4, Table 1 and § 5): radius 3.71 ±
/// 0.20 R⊕, at 0.229 au from an 8 500 K, 14.39 L☉ A star, 420 ± 140 Myr
/// old, its mass not measured but in 8 to 14 M⊕ (§ 5).
const HD_56414_B: Hd56414b = Hd56414b {
    radius: (3.71, 0.20),
    luminosity: 14.39,
    orbit: 0.229,
    age: 0.42,
    masses: (8.0, 14.0),
};

struct Hd56414b {
    radius: (f64, f64),
    luminosity: f64,
    orbit: f64,
    age: f64,
    masses: (f64, f64),
}

/// Definition: steps of the bisections below, each halving a bracket of
/// a factor 100 in mass (60 reach the f64 resolution).
const BISECTION_STEPS: u32 = 60;

/// Definition: the core masses the bisection of [`stripped_core`]
/// brackets, M⊕, around every stripping line from 1 to 1 000 F⊕.
const BISECTION_CORES: (f64, f64) = (0.1, 30.0);

/// Definition: the core masses of the monotony sweep, M⊕: from a third
/// of the grid's lightest core to twice its heaviest.
const SWEEP_CORES: (f64, f64) = (0.5, 40.0);

/// Definition: the fluxes of the monotony sweep, F⊕: from a tenth of the
/// grid's lowest flux to ten times its highest.
const SWEEP_FLUXES: (f64, f64) = (0.1, 1e4);

/// Definition: points `(core mass M⊕, flux F⊕)` off both axes of Tang et
/// al.'s grid, each lighter and more irradiated than the "0" cell at
/// 1.5 M⊕ and 1 000 F⊕ ([`TANG_NOTHING_LEFT`]), from just off the corner
/// to two decades past it.
const LIGHTER_AND_HOTTER_THAN_BARE: [(f64, f64); 3] = [(0.3, 3e3), (1.0, 1e4), (1.4, 1e5)];

/// Definition: samples per decade of the monotony sweep.
const SAMPLES_PER_DECADE: f64 = 20.0;

/// Definition: the range of ceilings the monotony check reads: from ten
/// times Tang et al.'s starred values (a planet already bare) to under
/// the 14.64 % cell next to the "−" cells, the range where a draw is
/// decided.
const DECIDED: (f64, f64) = (1e-5, 0.15);

fn ceiling(core: f64, flux: f64) -> f64 {
    boil_off_ceiling(EarthMass(core), EarthFlux(flux)).get()
}

/// The largest core the ceiling strips at `flux`: under it the ceiling
/// falls below [`MIN_ENVELOPE`].
fn stripped_core(flux: f64) -> f64 {
    let (mut low, mut high) = BISECTION_CORES;
    for _ in 0..BISECTION_STEPS {
        let mid = (low * high).sqrt();
        if ceiling(mid, flux) < MIN_ENVELOPE {
            low = mid;
        } else {
            high = mid;
        }
    }
    low
}

#[test]
fn the_ceiling_reads_tangs_table_2_at_its_nodes() {
    for (core, flux, expected) in TANG_TABLE_2 {
        let read = ceiling(core, flux);
        assert!(
            relative(read, expected) < ROUND_OFF,
            "{core} M⊕, {flux} F⊕: {read} against {expected}"
        );
    }
}

#[test]
fn a_cell_without_loss_never_cuts_a_draw() {
    // The highest envelope drawn below 20 M⊕ is Lopez & Fortney's 20 %.
    let highest = crate::envelope::HIGHEST_TABULATED.get();
    for (core, flux) in TANG_NO_LOSS {
        let read = ceiling(core, flux);
        assert!(read >= highest, "{core} M⊕, {flux} F⊕: {read}");
    }
}

#[test]
fn a_cell_that_keeps_nothing_is_bare() {
    for (core, flux) in TANG_NOTHING_LEFT {
        let read = ceiling(core, flux);
        assert!(read < MIN_ENVELOPE, "{core} M⊕, {flux} F⊕: {read}");
    }
    // Tang et al.'s abstract: boil-off strips "a highly irradiated (F ≥
    // 100 F⊕) planet that has a low-mass core (M_c ≤ 4 M⊕)": the starred
    // cells.
    for (core, flux) in [(1.5, 100.0), (2.4, 300.0), (3.6, 1000.0)] {
        assert!(ceiling(core, flux) < MIN_ENVELOPE, "{core} {flux}");
    }
}

#[test]
fn the_ceiling_falls_with_flux_and_rises_with_core_mass() {
    // Tang et al. 2024 § 3.3: lighter and more irradiated cores keep less.
    let step = 10f64.powf(1.0 / SAMPLES_PER_DECADE);
    let decided = |a: f64, b: f64| a.min(b) >= DECIDED.0 && a.max(b) <= DECIDED.1;
    let mut core = SWEEP_CORES.0;
    while core <= SWEEP_CORES.1 {
        let mut flux = SWEEP_FLUXES.0;
        while flux <= SWEEP_FLUXES.1 {
            let here = ceiling(core, flux);
            let hotter = ceiling(core, flux * step);
            let heavier = ceiling(core * step, flux);
            if decided(here, hotter) {
                assert!(hotter <= here, "{core} M⊕, {flux} F⊕: {hotter} > {here}");
            }
            if decided(here, heavier) {
                assert!(heavier >= here, "{core} M⊕, {flux} F⊕: {heavier} < {here}");
            }
            flux *= step;
        }
        core *= step;
    }
}

#[test]
fn a_light_core_far_off_the_grid_stays_bare() {
    // The physics' direction, not a calibrated value: a planet lighter
    // and more irradiated than one that keeps nothing keeps nothing too
    // (Owen & Wu 2016, conclusion 5: mass loss is more severe for lower
    // core masses and stronger irradiation; Tang et al. 2024, § 3.3).
    // Every point is lighter than 1.5 M⊕ and hotter than 1 000 F⊕, where
    // Table 2 reads "0". The points lie off both axes, so the test also
    // guards the corner rule of the off-grid extension (calibration 1):
    // with the bilinear cross term, 0.3 M⊕ at 3 000 F⊕ would read 3.6 %.
    for (core, flux) in LIGHTER_AND_HOTTER_THAN_BARE {
        let read = ceiling(core, flux);
        assert!(read < MIN_ENVELOPE, "{core} M⊕, {flux} F⊕: {read}");
    }
}

#[test]
fn the_stripping_line_has_the_radius_valleys_slope() {
    let (low, high) = SLOPE_FLUXES;
    let (m_low, m_high) = (stripped_core(low), stripped_core(high));
    let alpha = CORE_RADIUS_EXPONENT * (m_high / m_low).ln() / (high / low).ln();
    assert!(
        alpha >= BERGER_ALPHA.0 && alpha <= BERGER_ALPHA.1,
        "R ∝ F^{alpha} against Berger et al.'s {BERGER_ALPHA:?}"
    );
    let in_period = FLUX_PER_PERIOD * alpha;
    for (paper, (lo, hi)) in [
        ("Van Eylen et al. 2018", VAN_EYLEN_SLOPE),
        ("Petigura et al. 2022", PETIGURA_SLOPE),
    ] {
        assert!(
            in_period >= lo && in_period <= hi,
            "R ∝ P^{in_period} against {paper}'s {lo} to {hi}"
        );
    }
}

#[test]
fn hd_56414_b_keeps_a_neptunes_envelope() {
    // The Neptune of an A star at 274 F⊕ exists: for some core in
    // Giacalone et al.'s range, the envelope the ceiling leaves reaches
    // its measured radius (Lopez & Fortney 2014's tables, the core
    // `M^(1/4)`), and no core in it is stripped.
    let p = &HD_56414_B;
    let flux = p.luminosity / (p.orbit * p.orbit);
    let (low, high) = p.masses;
    for core in [low, high] {
        assert!(ceiling(core, flux) >= MIN_ENVELOPE, "{core} M⊕");
    }
    let radius = |core: f64| {
        let kept = boil_off_ceiling(EarthMass(core), EarthFlux(flux));
        let total = EarthMass(core / (1.0 - kept.get()));
        let thickness = crate::envelope::envelope_thickness(
            total,
            kept,
            EarthFlux(flux),
            crate::units::Gyr(p.age),
        )
        .expect("inside the tables");
        core.powf(CORE_RADIUS_EXPONENT) + thickness.0
    };
    // The radius is continuous in the core: some core in the range
    // reaches the measured interval when the lightest core's radius is
    // not above its top and the heaviest's not below its foot. A ceiling
    // far too generous fails the first bound, one too strict the second.
    let (measured, sigma) = p.radius;
    let (smallest, largest) = (radius(low), radius(high));
    assert!(
        smallest <= measured + sigma,
        "{smallest} R⊕ at {low} M⊕ against {measured} ± {sigma}"
    );
    assert!(
        largest >= measured - sigma,
        "{largest} R⊕ at {high} M⊕ against {measured} ± {sigma}"
    );
}
