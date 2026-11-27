//! Ozone: the column an oxygen air holds under its star
//! ([`ozone_column`], calibration 1 of `docs/science/ozone.md`), the
//! cross-sections with which it takes the star's ultraviolet and visible
//! light ([`CROSS_SECTIONS`]), and how the column is laid out in the
//! infrared column, whose table holds its lines ([`OzoneProfile`],
//! calibration 5 of `docs/science/ozone.md`; issue #704).
//!
//! The cross-sections are Serdyuchenko et al.'s (2014, *Atmos. Meas.
//! Tech.* 7, 625, doi:10.5194/amt-7-625-2014), measured from 213 to
//! 1100 nm, reduced per band of the star's light to the k-tables'
//! g-points (`crates/science/data/ozone.tsv`, built by `tools/opacity-table
//! --ozone`, provenance in `crates/science/data/OPACITY.md`): a band's
//! mean transmission through a column `N` is `Σ w_g exp(−D σ_g N)`. The
//! shortwave puts the column above the air as a pure absorber at
//! [`OZONE_LAYER_K`] (`super::shortwave::Paths::through`, decided with the
//! user on 2026-11-25, `docs/science/ozone.md`, "Where it departs").

use std::sync::LazyLock;

use super::ktable::G;
use super::starlight::{BANDS, check_layout};
use crate::constants::{AVOGADRO, LOSCHMIDT, PASCALS_PER_BAR, SOLAR_EFFECTIVE_TEMPERATURE};
use crate::units::{Bar, DobsonUnit, Kelvin};

#[cfg(test)]
#[path = "ozone_tests.rs"]
mod tests;

/// The table of cross-sections (see the module's documentation).
const TABLE: &str = include_str!("../../data/ozone.tsv");

/// Calibration: the temperature of the ozone layer's cross-sections,
/// 223 K, one temperature for every layer and host, decided with the user
/// on 2026-11-25. It rests on the ozone-weighted temperature of the AFGL
/// profiles, 225 to 232 K (Anderson et al. 1986; ozone lies above 80 to
/// 90 % of the air's mass), and 223 K is the column of Serdyuchenko et
/// al. 2014's file nearest that range's cold end (the file steps by
/// 10 K). Its weight is small: 233 K absorbs 0.33 % more (a single pass,
/// measured on the table's Sun), and study 34's probe found that 243 K
/// moves Earth's Bond albedo by 0.0002 or less. Searched and not found:
/// an ozone-layer temperature for a planet of another host. See
/// `docs/science/ozone.md`, "Calibrations". The table also holds 233 K,
/// which a test compares.
pub(crate) const OZONE_LAYER_K: f64 = 223.0;

/// Definition: the thickness of a column of one Dobson unit brought to
/// 273.15 K and 101.325 kPa, 10 µm (Cooke et al. 2022, *R. Soc. Open
/// Sci.* 9, 211165, §3.1; NASA Ozone Watch, "What is a Dobson Unit?").
const DOBSON_UNIT_M: f64 = 1e-5;

/// Definition: square centimetres per square metre.
const CM2_PER_M2: f64 = 1e4;

/// Source: Earth's ozone column, 300 Dobson units: "roughly 300 Dobson
/// Units" (NASA Ozone Watch, "What is a Dobson Unit?", read 2026-11-25),
/// the 300 ± 30 DU input of Kim & Ramanathan 2008's global run (*J.
/// Geophys. Res.* 113, D02203, doi:10.1029/2007JD008434, Table 2). The
/// column of an oxygen air under the Sun ([`ozone_column`]).
pub const EARTH_OZONE_COLUMN: DobsonUnit = DobsonUnit(300.0);

/// Source: the ozone column of an Earth-like planet at present O₂ around
/// the grid stars of Rugheimer et al. 2013 (*Astrobiology* 13, 251,
/// doi:10.1089/ast.2012.0888, arXiv:1212.2638, Table 2): effective
/// temperature, K, and column, cm⁻², from the K7V to the F0V star.
const RUGHEIMER_2013: [(f64, f64); 12] = [
    (4250.0, 3.5e18),
    (4500.0, 2.6e18),
    (4750.0, 2.6e18),
    (5000.0, 3.3e18),
    (5250.0, 4.1e18),
    (5500.0, 3.2e18),
    (5750.0, 5.1e18),
    (6000.0, 7.3e18),
    (6250.0, 8.3e18),
    (6500.0, 9.6e18),
    (6750.0, 1.1e19),
    (7000.0, 1.2e19),
];

/// Source: the ozone column of the same planet around the Sun in the same
/// model, 5.3e18 cm⁻² (Rugheimer et al. 2013, Table 2, row "SUN"), which
/// gives the ratios of [`RUGHEIMER_2013`]. The table gives the Sun no
/// temperature: [`ozone_column`] places it at the Sun's
/// ([`SOLAR_EFFECTIVE_TEMPERATURE`]).
const RUGHEIMER_SUN_CM2: f64 = 5.3e18;

/// Source: the ozone column of an Earth-like planet around quiescent
/// model M dwarfs (no chromosphere), Segura et al. 2005 (*Astrobiology*
/// 5, 706, doi:10.1089/ast.2005.5.706, arXiv:astro-ph/0510224, Table 2):
/// effective temperature, K, and column, cm⁻².
const SEGURA_2005: [(f64, f64); 3] = [(3100.0, 1.2e18), (3400.0, 2.4e18), (3650.0, 3.2e18)];

/// Source: the ozone column of the same planet around the Sun in Segura
/// et al. 2005's model, 8.4e18 cm⁻² (Table 2), which gives the ratios of
/// [`SEGURA_2005`].
const SEGURA_SUN_CM2: f64 = 8.4e18;

/// The column, molecules per cm², of `column`: one Dobson unit is
/// [`LOSCHMIDT`] times [`DOBSON_UNIT_M`], 2.687e16 molecules per cm².
pub(crate) fn molecules_per_cm2(column: DobsonUnit) -> f64 {
    column.0 * LOSCHMIDT * DOBSON_UNIT_M / CM2_PER_M2
}

/// The ratio of a planet's ozone column at present O₂ around a host of
/// `star_k` to its column around the Sun: Segura et al. 2005's quiescent
/// M dwarfs from 3100 to 3650 K, the Sun at its temperature, Rugheimer et
/// al. 2013's grid from 4250 to 7000 K, each model's column over its own
/// Sun's, linear in temperature between the points and held at the ends.
fn host_ratio(star_k: f64) -> f64 {
    let sun = SOLAR_EFFECTIVE_TEMPERATURE.0;
    let mut points: Vec<(f64, f64)> = SEGURA_2005
        .iter()
        .map(|&(t, c)| (t, c / SEGURA_SUN_CM2))
        .chain(
            RUGHEIMER_2013
                .iter()
                .map(|&(t, c)| (t, c / RUGHEIMER_SUN_CM2)),
        )
        .collect();
    points.push((sun, 1.0));
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (first, last) = (points[0], points[points.len() - 1]);
    if star_k <= first.0 {
        return first.1;
    }
    if star_k >= last.0 {
        return last.1;
    }
    let i = points.partition_point(|p| p.0 <= star_k) - 1;
    let ((t0, r0), (t1, r1)) = (points[i], points[i + 1]);
    r0 + (r1 - r0) * (star_k - t0) / (t1 - t0)
}

/// Calibration (calibration 1 of `docs/science/ozone.md`, "Where it
/// departs", decided with the user on 2026-11-25): the ozone column of an
/// air holding `o2` of O₂ under a host of effective temperature `star`.
/// An air with O₂ (the `Modern` air of planet PR 2d-4) holds
/// [`EARTH_OZONE_COLUMN`] times the ratio of the host's column to the
/// Sun's at present O₂: Rugheimer et al. 2013's grid over 4250 to 7000 K,
/// Segura et al. 2005's quiescent M dwarfs over 3100 to 3650 K (45, 89
/// and 119 of their Sun's 313 DU), the Sun at 5772 K, linear in
/// temperature between them and held past the ends. An air without O₂
/// (none, negative or NaN) holds none.
///
/// What was searched and not found: a column law in both the O₂ and the
/// star's ultraviolet; a three-dimensional model run for a host other
/// than the Sun; one one-dimensional grid over all the hosts whose Sun
/// gives the observed 300 DU (Rugheimer et al.'s gives 197 DU, Segura et
/// al.'s 313 DU but over M dwarfs only), hence the ratios. The column does
/// not follow the amount of O₂: Cooke et al. 2022's columns against O₂
/// (66 DU at 1 % of the present level) are not used while the app's
/// oxygen airs are `Modern` ones.
///
/// A temperature that is not positive or not finite (NaN, infinite) is
/// the Sun's, as for its spectrum.
pub fn ozone_column(o2: Bar, star: Kelvin) -> DobsonUnit {
    if o2.0.is_nan() || o2.0 <= 0.0 {
        return DobsonUnit(0.0);
    }
    let star_k = if star.0 > 0.0 && star.0.is_finite() {
        star.0
    } else {
        SOLAR_EFFECTIVE_TEMPERATURE.0
    };
    EARTH_OZONE_COLUMN * host_ratio(star_k)
}

/// Ozone's cross-sections on the bands of the star's light, as g-points.
#[derive(Debug)]
pub(crate) struct CrossSections {
    /// Temperatures of the table, K.
    pub(crate) temperatures: Vec<f64>,
    /// Weight of each g-point (its interval of `g`); they sum to 1.
    pub(crate) weights: [f64; G],
    /// Cross-sections of each band's g-points, cm² per molecule,
    /// `[temperature][band]`, the bluest band first; zero in the bands the
    /// table does not hold (past 1.1 µm).
    pub(crate) bands: Vec<Vec<[f64; G]>>,
}

impl CrossSections {
    /// The table `text`: comment lines, a `bands` line (checked against
    /// the star's bands), a `g` line (the g-intervals' edges), then rows
    /// `temperature_k band sigma_1 … sigma_G`. A malformed table is a bug
    /// of the build and panics with its line.
    fn parse(text: &str) -> CrossSections {
        let mut lines = text
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.starts_with('#') && !l.trim().is_empty());
        let (n, layout) = lines.next().expect("ozone.tsv: a bands line");
        check_layout(&layout.split('\t').collect::<Vec<_>>(), n);
        let (n, g) = lines.next().expect("ozone.tsv: a g line");
        let edges: Vec<f64> = g
            .split('\t')
            .skip(1)
            .map(|x| x.parse().unwrap_or_else(|_| panic!("line {}: {x}", n + 1)))
            .collect();
        assert_eq!(edges.len(), G + 1, "line {}: not {G} g-points", n + 1);
        let weights: [f64; G] = std::array::from_fn(|i| edges[i + 1] - edges[i]);
        let mut out = CrossSections {
            temperatures: Vec::new(),
            weights,
            bands: Vec::new(),
        };
        for (n, line) in lines {
            let fields: Vec<f64> = line
                .split('\t')
                .map(|x| x.parse().unwrap_or_else(|_| panic!("line {}: {x}", n + 1)))
                .collect();
            assert_eq!(fields.len(), G + 2, "line {}: not {G} g-points", n + 1);
            let t = fields[0];
            let band = fields[1] as usize;
            assert!(band < BANDS, "line {}: band {band}", n + 1);
            let at = match out.temperatures.iter().position(|x| *x == t) {
                Some(i) => i,
                None => {
                    out.temperatures.push(t);
                    out.bands.push(vec![[0.0; G]; BANDS]);
                    out.temperatures.len() - 1
                }
            };
            out.bands[at][band] = std::array::from_fn(|g| fields[g + 2]);
        }
        out
    }

    /// The g-points of every band at `t_k`, one of the table's
    /// temperatures (a bug otherwise).
    pub(crate) fn at(&self, t_k: f64) -> &[[f64; G]] {
        let i = self
            .temperatures
            .iter()
            .position(|t| *t == t_k)
            .unwrap_or_else(|| panic!("ozone.tsv holds no {t_k} K"));
        &self.bands[i]
    }
}

/// The table, parsed once.
pub(crate) static CROSS_SECTIONS: LazyLock<CrossSections> =
    LazyLock::new(|| CrossSections::parse(TABLE));

/// Source: the ozone mixing ratio of the U.S. Standard Atmosphere 1976 in
/// the AFGL atmospheric constituent profiles (Anderson, G. P., Clough,
/// S. A., Kneizys, F. X., Chetwynd, J. H., Shettle, E. P. 1986, "AFGL
/// atmospheric constituent profiles (0-120 km)", AFGL-TR-86-0110), read in
/// libRadtran's `afglus.dat` (SHA-256
/// `dab26290ed81c762ed0c607e5dc2d53393c1462a0c3a528bc5e3f5935191cfb5`,
/// study 32's download): pressure, bar, and ozone over air number
/// density, from the surface (0 km) to 80 km, every kilometre to 25 km,
/// then every 2.5 and 5 km (derived here, the file's `o3` over its `air`
/// column, four digits). Its column is 342.9 DU on the standard gravity
/// (343.2 on 9.798 m/s²; derived here: the mixing ratio linear in `ln p`
/// between rows, integrated exactly over the pressure); study 34 gives
/// 346 DU for the whole file.
const AFGL_US_OZONE: [(f64, f64); 42] = [
    (1.013, 2.6623e-8),
    (0.8988, 2.9336e-8),
    (0.795, 3.2396e-8),
    (0.7012, 3.3196e-8),
    (0.6166, 3.3885e-8),
    (0.5405, 3.7704e-8),
    (0.4722, 4.1137e-8),
    (0.4111, 5.0137e-8),
    (0.3565, 5.9705e-8),
    (0.308, 9.1748e-8),
    (0.265, 1.3140e-7),
    (0.227, 2.1505e-7),
    (0.194, 3.0973e-7),
    (0.1658, 3.8490e-7),
    (0.1417, 5.0331e-7),
    (0.1211, 6.5089e-7),
    (0.1035, 8.7077e-7),
    (0.0885, 1.1878e-6),
    (0.07565, 1.5879e-6),
    (0.06467, 2.0314e-6),
    (0.05529, 2.5804e-6),
    (0.04729, 3.0298e-6),
    (0.04047, 3.6500e-6),
    (0.03467, 4.1699e-6),
    (0.02972, 4.6304e-6),
    (0.02549, 5.1215e-6),
    (0.01743, 5.8072e-6),
    (0.01197, 6.5569e-6),
    (8.2573e-3, 7.1437e-6),
    (5.746e-3, 7.8427e-6),
    (4.0412e-3, 8.0310e-6),
    (2.871e-3, 7.3049e-6),
    (2.06e-3, 6.2045e-6),
    (1.491e-3, 5.2532e-6),
    (1.09e-3, 4.1035e-6),
    (7.978e-4, 3.1020e-6),
    (4.25e-4, 1.8011e-6),
    (2.19e-4, 1.1007e-6),
    (1.09e-4, 7.0042e-7),
    (5.22e-5, 3.0023e-7),
    (2.4e-5, 2.5018e-7),
    (1.05e-5, 3.0021e-7),
];

/// The mixing ratio of [`AFGL_US_OZONE`] at `p_bar`: linear in `ln p`
/// between its rows, held at its first row below the surface's 1.013 bar
/// and at its last above 80 km.
fn afgl_mixing_ratio(p_bar: f64) -> f64 {
    let rows = &AFGL_US_OZONE;
    let (first, last) = (rows[0], rows[rows.len() - 1]);
    if p_bar >= first.0 {
        return first.1;
    }
    if p_bar <= last.0 {
        return last.1;
    }
    let i = rows
        .partition_point(|r| r.0 > p_bar)
        .clamp(1, rows.len() - 1);
    let (a, b) = (rows[i - 1], rows[i]);
    let f = (p_bar / a.0).ln() / (b.0 / a.0).ln();
    a.1 + f * (b.1 - a.1)
}

/// `∫₀^p x(p') dp'` of [`afgl_mixing_ratio`], bar: exact on each piece
/// where `x` is linear in `ln p` (`∫ (A + B ln p) dp = A p + B (p ln p −
/// p)`), the mixing ratio held above 80 km and below the surface.
fn afgl_integral(p_bar: f64) -> f64 {
    let rows = &AFGL_US_OZONE;
    let last = rows[rows.len() - 1];
    let mut sum = last.1 * p_bar.min(last.0);
    // `∫` of `x = a.1 + (b.1 − a.1) ln(p/a.0)/ln(b.0/a.0)` from `lo` to
    // `hi` on the piece between rows `a` and `b`.
    let piece = |a: (f64, f64), b: (f64, f64), lo: f64, hi: f64| {
        let slope = (b.1 - a.1) / (b.0 / a.0).ln();
        let primitive = |p: f64| (a.1 - slope * a.0.ln()) * p + slope * (p * p.ln() - p);
        primitive(hi) - primitive(lo)
    };
    for w in rows.windows(2).rev() {
        let (a, b) = (w[0], w[1]);
        if p_bar <= b.0 {
            break;
        }
        sum += piece(a, b, b.0, p_bar.min(a.0));
    }
    let first = rows[0];
    if p_bar > first.0 {
        sum += first.1 * (p_bar - first.0);
    }
    sum
}

/// How the infrared column lays out an air's ozone column: the mixing
/// ratio of [`AFGL_US_OZONE`] against the pressure, scaled so that the
/// air from its surface to space holds the column.
///
/// Calibration: the vertical shape, calibration 5 of
/// `docs/science/ozone.md` (study 34's third proposal, decided for PR 2
/// of the plan with the user on 2026-11-25): Earth's shape in absolute
/// pressure for every air and host, so that the layer stays above the
/// tropopause's 0.1 bar of a thick air. Kozakis et al. 2022 (§4.1,
/// Fig. 4) show the layer moving with the O₂ and the host; no law for its
/// height was found. The layer takes the column's own temperatures, its
/// stratosphere at the skin temperature (214 K for Earth, colder than the
/// 225 to 232 K of the real layer: a departure, "No ozone heating").
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OzoneProfile {
    /// Factor on [`AFGL_US_OZONE`]'s mixing ratio.
    scale: f64,
}

impl OzoneProfile {
    /// Definition: no ozone.
    pub(crate) const NONE: OzoneProfile = OzoneProfile { scale: 0.0 };

    /// The layout of `column` in an air of `surface_bar` at its surface,
    /// of mean molar mass `molar_kg` (kg/mol, the dry air's, ozone lying
    /// above the vapour), on a body of surface gravity `gravity_m_s2`:
    /// the air above a level of `p` holds `x(p) p N_A / (M̄ g)` molecules
    /// of ozone per unit `ln p`, which integrate to the column from the
    /// surface up. None without a column, air or gravity.
    pub(crate) fn of(
        column: DobsonUnit,
        surface_bar: f64,
        molar_kg: f64,
        gravity_m_s2: f64,
    ) -> OzoneProfile {
        let per_m2 = molecules_per_cm2(column) * CM2_PER_M2;
        let shape =
            afgl_integral(surface_bar) * PASCALS_PER_BAR * AVOGADRO / (molar_kg * gravity_m_s2);
        let scale = per_m2 / shape;
        if scale.is_finite() && scale > 0.0 {
            OzoneProfile { scale }
        } else {
            OzoneProfile::NONE
        }
    }

    /// Whether the air holds ozone.
    pub(crate) fn holds(self) -> bool {
        self.scale > 0.0
    }

    /// The ozone at a level of `p_bar`, bar: its mixing ratio times the
    /// pressure.
    pub(crate) fn bar_at(self, p_bar: f64) -> f64 {
        if self.scale > 0.0 {
            self.scale * afgl_mixing_ratio(p_bar) * p_bar
        } else {
            0.0
        }
    }
}
