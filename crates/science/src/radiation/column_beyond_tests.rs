//! Tests of the column past the longwave table ([`beyond`]): how it
//! splits the black body, its scattering-only visible, and a hot steam
//! column against Boukrouche et al. 2021.

use super::super::{Profile, Scratch, radiate, with_column};
use super::*;
use crate::constants::{STANDARD_GRAVITY, STEFAN_BOLTZMANN};
use crate::radiation::gases::PartialPressures;
use crate::radiation::spectrum::reference::planck_between;
use crate::radiation::spectrum::{BANDS, TAIL};

/// Source: the black body's share at wavelengths under `λ`, `F(0→λT)`,
/// at `λT` = 2000, 3000, 4000 and 6000 µm·K (Incropera, DeWitt, Bergman
/// & Lavine 2007, *Fundamentals of Heat and Mass Transfer*, 6th ed.,
/// Table 12.1): with `λ` = 2 µm, the share past 5000 cm⁻¹ of a black body
/// at 1000, 1500, 2000 and 3000 K.
const SHARES_PAST_2_UM: [(f64, f64); 4] = [
    (1000.0, 0.066_728),
    (1500.0, 0.273_232),
    (2000.0, 0.480_877),
    (3000.0, 0.737_818),
];

/// Definition: the tolerance on [`SHARES_PAST_2_UM`], 5e-5: the table
/// strays from Planck's law with CODATA 2018's second radiation constant
/// (`constants`) by up to 2.9e-5 (at 6000 µm·K, where the series of
/// Planck's integral gives 0.737 789; derived), and [`TRANSPARENT_AIR`]
/// scatters back up to [`TOTAL_TOLERANCE`].
const SHARE_TOLERANCE: f64 = 5e-5;

/// Definition: how far from `σT⁴` the whole column of
/// [`TRANSPARENT_AIR`] may send out, a share, 1e-5: its 1 mbar of
/// nitrogen, a Rayleigh depth of 1e-4 at 550 nm, scatters back under
/// 1e-5 of the light past 0.7 µm (8 % of the black body at 3000 K), and
/// absorbs less of the longwave bins.
const TOTAL_TOLERANCE: f64 = 1e-5;

/// Definition: an air that hardly absorbs nor scatters, 1 mbar of
/// nitrogen (no line of the shortwave table, its pairs negligible).
const TRANSPARENT_AIR: PartialPressures = PartialPressures {
    n2: 1e-3,
    co2: 0.0,
    h2o: 0.0,
    ch4: 0.0,
    nh3: 0.0,
    so2: 0.0,
    o2: 0.0,
    ar: 0.0,
};

/// Each band of `gases`'s column over a black ground at `surface_k`,
/// clear, on Earth's gravity, from [`radiate`].
fn bands(gases: &PartialPressures, surface_k: f64) -> [f64; BANDS] {
    let mut scratch = Scratch::new();
    with_column(
        gases,
        surface_k,
        0.0,
        STANDARD_GRAVITY.0,
        Profile::CLEAR,
        |c| radiate(c, &mut scratch),
    )
    .expect("an air")
}

#[test]
fn a_transparent_air_sends_the_black_bodys_share_past_2_um() {
    for (t, share) in SHARES_PAST_2_UM {
        let out = bands(&TRANSPARENT_AIR, t);
        let black = STEFAN_BOLTZMANN * t.powi(4);
        let tail = out[TAIL] / black;
        let total = out.iter().sum::<f64>() / black;
        println!("{t} K: past 2 µm {tail:.7} (Incropera {share}), all {total:.7}");
        assert!((tail - share).abs() < SHARE_TOLERANCE, "{t} K: {tail}");
        // Nothing counted twice nor left out: the whole black body.
        assert!((total - 1.0).abs() < TOTAL_TOLERANCE, "{t} K: {total}");
    }
}

/// Definition: the nitrogen of a column that only scatters past 2 µm,
/// 10 bar (no line of the shortwave table; a Rayleigh depth of about 1
/// at 550 nm).
const SCATTERING_AIR: PartialPressures = PartialPressures {
    n2: 10.0,
    ..TRANSPARENT_AIR
};

/// Definition: the surface of [`SCATTERING_AIR`]'s column, K, at which
/// 8 % of the black body lies past 0.7 µm (derived from Planck's law).
const SCATTERING_SURFACE_K: f64 = 3000.0;

#[test]
fn past_0_7_um_the_air_only_scatters() {
    // Nitrogen absorbs nothing past 2 µm: the near infrared is the
    // ground's, and past 0.7 µm each band of the star's lets it out
    // through a conservative two-stream layer of its Rayleigh depth,
    // `1 / (1 + (D/2) τ)` (Meador & Weaver 1980).
    let t = SCATTERING_SURFACE_K;
    let tau = rayleigh_depth_550(&SCATTERING_AIR, STANDARD_GRAVITY.0);
    let table = &*SHORTWAVE;
    let top = *table.edges_cm.last().expect("edges");
    let mut expected = planck_between(SPECTRUM_TOP_CM, top, t);
    let visible = starlight::BANDS - table.bins;
    for band in 0..visible {
        let (lo, hi) = (
            wavenumber_cm(band_edge_m(band + 1)),
            wavenumber_cm(band_edge_m(band)),
        );
        let through = 1.0 / (1.0 + 0.5 * DIFFUSIVITY * tau * rayleigh_scale(band));
        expected += planck_between(lo, hi, t) * through;
    }
    let past = wavenumber_cm(band_edge_m(0));
    let rest = STEFAN_BOLTZMANN * t.powi(4) - planck_between(0.0, past, t);
    expected += rest / (1.0 + 0.5 * DIFFUSIVITY * tau * rayleigh_scale(0));
    let tail = bands(&SCATTERING_AIR, t)[TAIL];
    let clear = STEFAN_BOLTZMANN * t.powi(4) - planck_between(0.0, SPECTRUM_TOP_CM, t);
    println!("τ_R {tau:.4}: past 2 µm {tail:.2} W/m², expected {expected:.2}, clear {clear:.2}");
    // The parts left out of the sum, under NEGLIGIBLE of σT⁴, are clear.
    let floor = NEGLIGIBLE * STEFAN_BOLTZMANN * t.powi(4);
    assert!((tail - expected).abs() < floor, "{tail} vs {expected}");
    assert!(
        tail < clear - 1e3 * floor,
        "the air scatters: {tail} vs {clear}"
    );
}

/// Source: the surface's light that reaches space through 260 bar of
/// pure steam on 1 g, W/m², at a surface of 2300 and 3000 K: their
/// "surface contribution" `SCFF = I⁺(0) e^{−τ∞}`, 3.8e4 and 3.3e5,
/// all of it past 12 500 cm⁻¹ ("Only the SW band including the visible
/// and UV regions is contributing to these values"), Boukrouche,
/// Lichtenberg & Pierrehumbert 2021, *ApJ* 919, 130,
/// doi:10.3847/1538-4357/ac1345, §4.2 and Appendix A.
const BOUKROUCHE_SURFACE_SENT: [(f64, f64); 2] = [(2300.0, 3.8e4), (3000.0, 3.3e5)];

/// Source: the steam column of [`BOUKROUCHE_SURFACE_SENT`], bar.
const BOUKROUCHE_STEAM_BAR: f64 = 260.0;

/// Calibration: how far under Boukrouche et al.'s surface light the
/// column's radiation past 2 µm may fall, a factor of 10. Their band-grey
/// model averages ExoMol's water opacities over each band and tunes them
/// to SOCRATES (their Table 2) and scatters nothing; the column caps the
/// steam at 70 bar (`STEAM_MAX_BAR`), takes the shortwave table's lines
/// at 900 K and scatters past 0.7 µm. No source sets how close two such
/// models agree; one decade is the bound of a window that is open, which
/// the copied tail of the longwave table (the absorption of its last
/// bin, on CO₂'s and water's bands at 2 µm) fails by about four decades:
/// it sent out 1.5 W/m² at 2300 K and 53 W/m² at 3000 K (study 41).
const BOUKROUCHE_FACTOR: f64 = 10.0;

#[test]
fn a_hot_steam_column_lets_its_surface_out_past_2_um() {
    let steam = PartialPressures {
        h2o: BOUKROUCHE_STEAM_BAR,
        n2: 0.0,
        ..TRANSPARENT_AIR
    };
    for (t, sent) in BOUKROUCHE_SURFACE_SENT {
        let tail = bands(&steam, t)[TAIL];
        println!("{t} K: past 2 µm {tail:.4e} W/m², Boukrouche's surface {sent:.1e}");
        assert!(tail >= sent / BOUKROUCHE_FACTOR, "{t} K: {tail}");
    }
}
