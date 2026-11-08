//! The host star of a planet: what a spectral type means physically, its
//! colour, its habitable zone, and what an orbit around it implies (see
//! `docs/analysis/10-stars.md`).
//!
//! A spectral type is a class and a subtype, `G2V` for the Sun (`V`: a
//! main-sequence dwarf, the only luminosity class modelled). Internally a
//! type is a number, `class × 10 + subtype` with O = 0 … M = 6 (`G2` is
//! 42), so types interpolate: the physical values come from the mean dwarf
//! sequence of Pecaut & Mamajek (2013), version 2022.04.16 of Mamajek's
//! table.
//!
//! An orbit is placed by its position in the habitable zone of Kopparapu
//! et al. (2013, 2014): 0 at the inner edge (runaway greenhouse), 1 at the
//! outer edge (maximum greenhouse), below 0 closer in, above 1 farther
//! out; the flux varies geometrically with the position, so it extends
//! smoothly beyond the edges.
use crate::preset::ParamDefault;
use crate::request::{ECCENTRICITY, HZ_POSITION, ORBIT_AU};
use rand::RngExt;
use serde::{Deserialize, Serialize};

/// Spectral classes, hottest first.
pub const CLASSES: [char; 7] = ['O', 'B', 'A', 'F', 'G', 'K', 'M'];

/// The Sun's spectral type (`G2V`).
pub const SUN: f32 = 42.0;

/// Effective temperature of the Sun, K (IAU 2015 nominal value).
pub const SUN_TEFF_K: f32 = 5772.0;

/// One row of the mean dwarf sequence.
struct Row {
    /// `class × 10 + subtype`.
    code: f32,
    teff_k: f32,
    /// log10 of the luminosity in L☉.
    log_l: f32,
    /// Radius, R☉.
    radius: f32,
    /// Mass, M☉.
    mass: f32,
}

const fn row(code: f32, teff_k: f32, log_l: f32, radius: f32, mass: f32) -> Row {
    Row {
        code,
        teff_k,
        log_l,
        radius,
        mass,
    }
}

/// Pecaut & Mamajek (2013, ApJS 208, 9), Mamajek's table version
/// 2022.04.16 (<https://www.pas.rochester.edu/~emamajek/EEM_dwarf_UBVIJHK_colors_Teff.txt>):
/// spectral type, Teff, log L, R and M of main-sequence dwarfs, O3V to
/// M9.5V. The Sun's row (G2V) reads 5 770 K, log L = 0.01.
const SEQUENCE: &[Row] = &[
    row(3.0, 44900.0, 5.82, 13.43, 59.0),
    row(4.0, 42900.0, 5.65, 12.13, 48.0),
    row(5.0, 41400.0, 5.54, 11.45, 43.0),
    row(5.5, 40500.0, 5.44, 10.71, 38.0),
    row(6.0, 39500.0, 5.36, 10.27, 35.0),
    row(6.5, 38300.0, 5.27, 9.82, 31.0),
    row(7.0, 37100.0, 5.18, 9.42, 28.0),
    row(7.5, 36100.0, 5.09, 8.95, 26.0),
    row(8.0, 35100.0, 4.99, 8.47, 23.6),
    row(8.5, 34300.0, 4.91, 8.06, 21.9),
    row(9.0, 33300.0, 4.82, 7.72, 20.2),
    row(9.5, 31900.0, 4.72, 7.50, 18.7),
    row(10.0, 31400.0, 4.65, 7.16, 17.7),
    row(10.5, 29000.0, 4.43, 6.48, 14.8),
    row(11.0, 26000.0, 4.13, 5.71, 11.8),
    row(11.5, 24500.0, 3.91, 5.02, 9.9),
    row(12.0, 20600.0, 3.43, 4.06, 7.3),
    row(12.5, 18500.0, 3.20, 3.89, 6.1),
    row(13.0, 17000.0, 2.99, 3.61, 5.4),
    row(14.0, 16400.0, 2.89, 3.46, 5.1),
    row(15.0, 15700.0, 2.77, 3.36, 4.7),
    row(16.0, 14500.0, 2.57, 3.27, 4.3),
    row(17.0, 14000.0, 2.48, 2.94, 3.92),
    row(18.0, 12300.0, 2.19, 2.86, 3.38),
    row(19.0, 10700.0, 1.86, 2.49, 2.75),
    row(19.5, 10400.0, 1.80, 2.45, 2.68),
    row(20.0, 9700.0, 1.58, 2.193, 2.18),
    row(21.0, 9300.0, 1.49, 2.136, 2.05),
    row(22.0, 8800.0, 1.38, 2.117, 1.98),
    row(23.0, 8600.0, 1.23, 1.861, 1.86),
    row(24.0, 8250.0, 1.13, 1.794, 1.93),
    row(25.0, 8100.0, 1.09, 1.785, 1.88),
    row(26.0, 7910.0, 1.05, 1.775, 1.83),
    row(27.0, 7760.0, 1.00, 1.750, 1.77),
    row(28.0, 7590.0, 0.96, 1.747, 1.81),
    row(29.0, 7400.0, 0.92, 1.747, 1.75),
    row(30.0, 7220.0, 0.86, 1.728, 1.61),
    row(31.0, 7020.0, 0.79, 1.679, 1.50),
    row(32.0, 6820.0, 0.71, 1.622, 1.46),
    row(33.0, 6750.0, 0.67, 1.578, 1.44),
    row(34.0, 6670.0, 0.62, 1.533, 1.38),
    row(35.0, 6550.0, 0.56, 1.473, 1.33),
    row(36.0, 6350.0, 0.43, 1.359, 1.25),
    row(37.0, 6280.0, 0.39, 1.324, 1.21),
    row(38.0, 6180.0, 0.29, 1.221, 1.18),
    row(39.0, 6050.0, 0.22, 1.167, 1.13),
    row(39.5, 5990.0, 0.18, 1.142, 1.08),
    row(40.0, 5930.0, 0.13, 1.100, 1.06),
    row(41.0, 5860.0, 0.08, 1.060, 1.03),
    row(42.0, 5770.0, 0.01, 1.012, 1.00),
    row(43.0, 5720.0, -0.01, 1.002, 0.99),
    row(44.0, 5680.0, -0.04, 0.991, 0.985),
    row(45.0, 5660.0, -0.05, 0.977, 0.98),
    row(46.0, 5600.0, -0.10, 0.949, 0.97),
    row(47.0, 5550.0, -0.13, 0.927, 0.95),
    row(48.0, 5480.0, -0.17, 0.914, 0.94),
    row(49.0, 5380.0, -0.26, 0.853, 0.90),
    row(50.0, 5270.0, -0.34, 0.813, 0.88),
    row(51.0, 5170.0, -0.39, 0.797, 0.86),
    row(52.0, 5100.0, -0.43, 0.783, 0.82),
    row(53.0, 4830.0, -0.55, 0.755, 0.78),
    row(54.0, 4600.0, -0.69, 0.713, 0.73),
    row(55.0, 4440.0, -0.76, 0.701, 0.70),
    row(56.0, 4300.0, -0.86, 0.669, 0.69),
    row(57.0, 4100.0, -1.00, 0.630, 0.64),
    row(58.0, 3990.0, -1.06, 0.615, 0.62),
    row(59.0, 3930.0, -1.10, 0.608, 0.59),
    row(60.0, 3850.0, -1.16, 0.588, 0.57),
    row(60.5, 3770.0, -1.27, 0.544, 0.54),
    row(61.0, 3660.0, -1.39, 0.501, 0.50),
    row(61.5, 3620.0, -1.44, 0.482, 0.47),
    row(62.0, 3560.0, -1.54, 0.446, 0.44),
    row(62.5, 3470.0, -1.64, 0.421, 0.40),
    row(63.0, 3430.0, -1.79, 0.361, 0.37),
    row(63.5, 3270.0, -2.03, 0.300, 0.27),
    row(64.0, 3210.0, -2.14, 0.274, 0.23),
    row(64.5, 3110.0, -2.40, 0.217, 0.184),
    row(65.0, 3060.0, -2.52, 0.196, 0.162),
    row(65.5, 2930.0, -2.79, 0.156, 0.123),
    row(66.0, 2810.0, -2.98, 0.137, 0.102),
    row(66.5, 2740.0, -3.10, 0.126, 0.093),
    row(67.0, 2680.0, -3.19, 0.120, 0.090),
    row(67.5, 2630.0, -3.24, 0.116, 0.088),
    row(68.0, 2570.0, -3.28, 0.114, 0.085),
    row(68.5, 2420.0, -3.47, 0.104, 0.080),
    row(69.0, 2380.0, -3.52, 0.102, 0.079),
    row(69.5, 2350.0, -3.57, 0.101, 0.078),
];

/// Earliest and latest spectral types of [`SEQUENCE`] (O3 and M9.5).
pub const TYPE_RANGE: (f32, f32) = (3.0, 69.5);

/// Physical values of a main-sequence star.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarValues {
    /// Effective temperature, K.
    pub teff_k: f32,
    /// Bolometric luminosity, L☉.
    pub luminosity: f32,
    /// Radius, R☉.
    pub radius: f32,
    /// Mass, M☉.
    pub mass: f32,
}

/// The values of a spectral type (`class × 10 + subtype`), interpolated
/// in the mean dwarf sequence (linearly in log for every value), clamped
/// to O3–M9.5.
pub fn values(code: f32) -> StarValues {
    let code = code.clamp(TYPE_RANGE.0, TYPE_RANGE.1);
    let i = SEQUENCE
        .windows(2)
        .position(|w| code <= w[1].code)
        .unwrap_or(SEQUENCE.len() - 2);
    let (a, b) = (&SEQUENCE[i], &SEQUENCE[i + 1]);
    let f = ((code - a.code) / (b.code - a.code)).clamp(0.0, 1.0);
    // On a row, its own values, unrounded by the interpolation.
    for row in [a, b] {
        if code == row.code {
            return StarValues {
                teff_k: row.teff_k,
                luminosity: 10f32.powf(row.log_l),
                radius: row.radius,
                mass: row.mass,
            };
        }
    }
    let log_mix = |x: f32, y: f32| (x.ln() + (y.ln() - x.ln()) * f).exp();
    StarValues {
        teff_k: log_mix(a.teff_k, b.teff_k),
        luminosity: 10f32.powf(a.log_l + (b.log_l - a.log_l) * f),
        radius: log_mix(a.radius, b.radius),
        mass: log_mix(a.mass, b.mass),
    }
}

/// Reads a spectral type: a class letter, an optional subtype (0–9.5,
/// default 5) and an optional luminosity class `V` (`G2V`, `M4.5`, `K`).
/// Case-insensitive.
pub fn parse_type(text: &str) -> Result<f32, String> {
    let text = text.trim();
    let mut chars = text.chars();
    let class = chars
        .next()
        .map(|c| c.to_ascii_uppercase())
        .and_then(|c| CLASSES.iter().position(|&k| k == c))
        .ok_or_else(|| format!("spectral type {text:?}: starts with one of O B A F G K M"))?;
    let rest = chars.as_str();
    let rest = rest.strip_suffix(['V', 'v']).unwrap_or(rest);
    let subtype = if rest.is_empty() {
        5.0
    } else {
        rest.parse::<f32>()
            .ok()
            .filter(|s| (0.0..10.0).contains(s))
            .ok_or_else(|| {
                format!("spectral type {text:?}: the subtype is a number from 0 to 9.5")
            })?
    };
    let code = class as f32 * 10.0 + subtype;
    if (TYPE_RANGE.0..=TYPE_RANGE.1).contains(&code) {
        Ok(code)
    } else {
        Err(format!(
            "spectral type {text:?}: main-sequence dwarfs go from O3 to M9.5"
        ))
    }
}

/// The name of a spectral type, to a tenth of a subtype: `G2V`, `M4.5V`.
pub fn type_name(code: f32) -> String {
    let tenth = (code.clamp(TYPE_RANGE.0, TYPE_RANGE.1) * 10.0).round() / 10.0;
    let class = ((tenth / 10.0).floor() as usize).min(CLASSES.len() - 1);
    let subtype = ((tenth - class as f32 * 10.0) * 10.0).round() / 10.0;
    if subtype.fract() == 0.0 {
        format!("{}{}V", CLASSES[class], subtype as u32)
    } else {
        format!("{}{subtype:.1}V", CLASSES[class])
    }
}

/// Share of main-sequence stars of each class in the solar neighbourhood,
/// O to M (Mamajek's number densities; RECONS): about three quarters are
/// M dwarfs.
const CLASS_SHARES: [f64; 7] = [3e-7, 0.0013, 0.006, 0.03, 0.076, 0.121, 0.7657];

/// A spectral type drawn with the frequencies of the solar neighbourhood,
/// to a half subtype. Within a class the subtype is uniform, except for O
/// and M, which start at O3 and stop at M9.5.
pub fn draw_type(rng: &mut impl RngExt) -> f32 {
    let total: f64 = CLASS_SHARES.iter().sum();
    let mut pick = rng.random::<f64>() * total;
    let mut class = CLASS_SHARES.len() - 1;
    for (i, share) in CLASS_SHARES.iter().enumerate() {
        if pick < *share {
            class = i;
            break;
        }
        pick -= share;
    }
    let first = if class == 0 { 6 } else { 0 };
    let half = rng.random_range(first..20) as f32 / 2.0;
    class as f32 * 10.0 + half
}

/// Flux at the edges of the habitable zone, relative to Earth's (S⊕),
/// for an effective temperature: Kopparapu et al. 2014, runaway greenhouse
/// (inner) and maximum greenhouse (outer), for a 1 M⊕ planet. The fit
/// holds for 2 600–7 200 K; outside, the temperature is clamped.
pub fn habitable_fluxes(teff_k: f32) -> (f32, f32) {
    let t = (teff_k.clamp(2600.0, 7200.0) - 5780.0) as f64;
    let fit = |s: f64, a: f64, b: f64, c: f64, d: f64| {
        (s + a * t + b * t * t + c * t.powi(3) + d * t.powi(4)) as f32
    };
    (
        fit(1.107, 1.332e-4, 1.580e-8, -8.308e-12, -1.931e-15),
        fit(0.356, 6.171e-5, 1.698e-9, -3.198e-12, -5.575e-16),
    )
}

/// Flux at a position in the habitable zone (0 inner edge, 1 outer edge),
/// relative to Earth's.
pub fn flux_at(teff_k: f32, position: f32) -> f32 {
    let (inner, outer) = habitable_fluxes(teff_k);
    inner * (outer / inner).powf(position)
}

/// Position in the habitable zone of a flux (inverse of [`flux_at`]).
pub fn position_of(teff_k: f32, flux: f32) -> f32 {
    let (inner, outer) = habitable_fluxes(teff_k);
    (flux / inner).ln() / (outer / inner).ln()
}

/// Distance in AU at which a star of `luminosity` (L☉) gives `flux` (S⊕).
pub fn distance_au(luminosity: f32, flux: f32) -> f32 {
    (luminosity / flux).sqrt()
}

/// Equilibrium temperature of a planet with Earth's Bond albedo (0.3)
/// under a flux (S⊕), K: 255 K for Earth.
pub fn equilibrium_k(flux: f32) -> f32 {
    255.0 * flux.powf(0.25)
}

/// Orbital period in Earth days (Kepler's third law).
pub fn year_days(orbit_au: f32, mass: f32) -> f32 {
    365.25 * (orbit_au.powi(3) / mass).sqrt()
}

/// Apparent diameter of the star from the planet, degrees (the Sun from
/// Earth: 0.533°).
pub fn angular_diameter_deg(radius: f32, orbit_au: f32) -> f32 {
    // 2 R☉ / 1 AU in degrees.
    0.5332 * radius / orbit_au
}

/// Main-sequence lifetimes, (mass M☉, Gyr): Table 22.1 of Fraknoi et al.
/// (2022, OpenStax *Astronomy 2e*), O5 to M0, and the lightest star, which
/// stirs all its hydrogen into its core (Adams & Laughlin 1997).
const LIFETIMES: [(f32, f32); 8] = [
    (0.08, 11_000.0),
    (0.4, 200.0),
    (0.8, 14.0),
    (1.1, 9.0),
    (1.7, 2.7),
    (3.3, 0.5),
    (16.0, 0.01),
    (40.0, 0.001),
];

/// Main-sequence lifetime of a star of `mass` (M☉), Gyr: interpolated in
/// [`LIFETIMES`], power law to power law, by mass (the table's types are
/// heavier than [`SEQUENCE`]'s: its M0 weighs 0.4 M☉, an M0V here 0.57).
/// 10.3 Gyr for the Sun, 51 for an M0V, 1.9 trillion years for an M5V,
/// 1.4 Gyr for an A0V. The simpler `10 Gyr × M / L` assumes every star
/// burns the same share of its hydrogen: it gave an M5V 0.5 trillion
/// years, although a fully convective star burns all of it, and an A0V
/// 0.6 Gyr.
pub fn lifetime_gyr(mass: f32) -> f32 {
    let m = mass.clamp(LIFETIMES[0].0, LIFETIMES[LIFETIMES.len() - 1].0);
    let k = LIFETIMES
        .windows(2)
        .position(|w| m <= w[1].0)
        .unwrap_or(LIFETIMES.len() - 2);
    let ((m0, t0), (m1, t1)) = (LIFETIMES[k], LIFETIMES[k + 1]);
    let at = (m / m0).ln() / (m1 / m0).ln();
    (t0.ln() + at * (t1 / t0).ln()).exp()
}

/// Time for the star's tides to lock an Earth-like planet's spin, Gyr
/// (Gladman et al. 1996, with an initial 12 h day, Q = 100, k₂ = 0.3):
/// about 166 Gyr × a⁶ / M², so Earth is far from it and a habitable planet
/// of an M dwarf locks within a million years.
pub fn locking_gyr(orbit_au: f32, mass: f32) -> f32 {
    166.0 * orbit_au.powi(6) / (mass * mass)
}

/// CIE 1931 2° colour matching functions, the multi-lobe fit of Wyman,
/// Sloan & Shirley (2013), at a wavelength in nm.
fn cie_xyz(nm: f64) -> [f64; 3] {
    let g = |mu: f64, s1: f64, s2: f64| {
        let s = if nm < mu { s1 } else { s2 };
        (-0.5 * ((nm - mu) / s).powi(2)).exp()
    };
    [
        1.056 * g(599.8, 37.9, 31.0) + 0.362 * g(442.0, 16.0, 26.7) - 0.065 * g(501.1, 20.4, 26.2),
        0.821 * g(568.8, 46.9, 40.5) + 0.286 * g(530.9, 16.3, 31.1),
        1.217 * g(437.0, 11.8, 36.0) + 0.681 * g(459.0, 26.0, 13.8),
    ]
}

/// Linear sRGB of a black body (unnormalised).
fn black_body_rgb(teff_k: f32) -> [f64; 3] {
    // Planck's law, constants folded: B ∝ λ⁻⁵ / (exp(hc / λkT) − 1).
    const HC_K: f64 = 1.438_776_9e7; // nm·K
    let t = teff_k as f64;
    let mut xyz = [0.0; 3];
    for nm in (380..=780).step_by(5) {
        let l = nm as f64;
        let b = l.powi(-5) / ((HC_K / (l * t)).exp() - 1.0);
        let c = cie_xyz(l);
        for k in 0..3 {
            xyz[k] += b * c[k];
        }
    }
    let [x, y, z] = xyz;
    [
        3.2406 * x - 1.5372 * y - 0.4986 * z,
        -0.9689 * x + 1.8758 * y + 0.0415 * z,
        0.0557 * x - 0.2040 * y + 1.0570 * z,
    ]
}

/// The colour of a star's light, linear RGB, white-balanced on the Sun
/// (an eye adapted to sunlight sees the Sun white) and scaled so the
/// brightest channel is 1: about `[1, 0.56, 0.2]` for an M5 dwarf
/// (3 060 K), `[1, 0.82, 0.59]` for a K5 (4 440 K), `[0.52, 0.66, 1]` for
/// an A0 (9 700 K).
pub fn light_colour(teff_k: f32) -> [f32; 3] {
    let star = black_body_rgb(teff_k);
    let sun = black_body_rgb(SUN_TEFF_K);
    let rgb = [0, 1, 2].map(|k| (star[k] / sun[k]).max(0.0));
    let max = rgb.iter().cloned().fold(f64::MIN, f64::max);
    rgb.map(|c| (c / max) as f32)
}

/// Age assumed for a star when deciding whether its planet is locked: the
/// Sun's, or the star's whole main-sequence life if shorter.
pub const AGE_GYR: f32 = 4.6;

/// A preset's `[star]` table:
///
/// ```toml
/// [star]
/// type = "G2V"                       # left out: drawn with the frequencies of the solar neighbourhood
/// position = [0.1, 0.9]              # in the habitable zone: 0 inner edge, 1 outer edge
/// # orbit_au = 1.524                 # or a distance, instead of the position
/// ```
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
pub struct StarSpec {
    /// Spectral type (`G2V`); `None` draws one.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Position in the habitable zone, a fixed value or a range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<ParamDefault>,
    /// Semi-major axis in AU, instead of the position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orbit_au: Option<f32>,
    /// Eccentricity of the orbit, a value or a range; left out, drawn like
    /// the planets known (see [`draw_eccentricity`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eccentricity: Option<ParamDefault>,
    /// Metallicity of the star, `[Fe/H]` in dex; left out, drawn like the
    /// stars nearby (see `crate::giants`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metallicity: Option<f32>,
    /// The spin-orbit state the planet is held in, `1:1` (locked), `3:2` or
    /// `2:1`, instead of the one its seed draws from its eccentricity (see
    /// `crate::spin::capture`): a preset named after a planet whose state
    /// is observed pins it (`mercury-like` is `3:2`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resonance: Option<String>,
}

impl StarSpec {
    /// Checks the type parses and the orbit is within its limits.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(kind) = &self.kind {
            parse_type(kind).map_err(|e| format!("star.type: {e}"))?;
        }
        if self.position.is_some() && self.orbit_au.is_some() {
            return Err("star: set position or orbit_au, not both".into());
        }
        if let Some(position) = self.position {
            position
                .check("position", HZ_POSITION.min, HZ_POSITION.max)
                .map_err(|e| e.replacen("defaults.", "star.", 1))?;
        }
        if let Some(e) = self.eccentricity {
            e.check("eccentricity", ECCENTRICITY.min, ECCENTRICITY.max)
                .map_err(|e| e.replacen("defaults.", "star.", 1))?;
        }
        if let Some(r) = &self.resonance
            && crate::spin::parse_resonance(r).is_none()
        {
            return Err(format!(
                "star.resonance: `{r}` is not one of \"1:1\", \"3:2\", \"2:1\""
            ));
        }
        if let Some(m) = self.metallicity
            && !crate::request::METALLICITY.contains(m)
        {
            return Err(format!(
                "star.metallicity: must stay within [{}, {}]",
                crate::request::METALLICITY.min,
                crate::request::METALLICITY.max
            ));
        }
        if let Some(au) = self.orbit_au
            && !ORBIT_AU.contains(au)
        {
            return Err(format!(
                "star.orbit_au: must stay within [{}, {}]",
                ORBIT_AU.min, ORBIT_AU.max
            ));
        }
        Ok(())
    }
}

/// The star of a world and the planet's orbit, with what they imply, as
/// `world.json` stores them.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct StarInfo {
    /// Spectral type, `G2V`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Effective temperature, K.
    pub teff_k: f32,
    /// Luminosity, mass and radius in solar units.
    pub luminosity: f32,
    pub mass: f32,
    pub radius: f32,
    /// Semi-major axis of the planet's orbit, AU.
    pub orbit_au: f32,
    /// Eccentricity of the orbit (0 circular).
    #[serde(default)]
    pub eccentricity: f32,
    /// Closest and farthest distances to the star, AU.
    #[serde(default)]
    pub periapsis_au: f32,
    #[serde(default)]
    pub apoapsis_au: f32,
    /// Flux the planet receives over its year, relative to Earth's: an
    /// elliptical orbit receives `1 / sqrt(1 − e²)` times more than a
    /// circular one of the same semi-major axis.
    pub insolation: f32,
    /// Flux at the closest and farthest points, relative to Earth's.
    #[serde(default)]
    pub insolation_periapsis: f32,
    #[serde(default)]
    pub insolation_apoapsis: f32,
    /// Position in the habitable zone (0 inner edge, 1 outer edge).
    pub hz_position: f32,
    /// Edges of the habitable zone, AU.
    pub hz_inner_au: f32,
    pub hz_outer_au: f32,
    /// Equilibrium temperature with Earth's albedo, K.
    pub equilibrium_k: f32,
    /// Orbital period, Earth days.
    pub year_days: f32,
    /// Apparent diameter of the star from the planet, degrees.
    pub angular_diameter_deg: f32,
    /// Main-sequence lifetime of the star, Gyr.
    pub lifetime_gyr: f32,
    /// Time for the star's tides to lock the planet's spin, Gyr.
    pub locking_gyr: f32,
    /// Whether the planet is tidally locked, one face to its star: its
    /// locking time is shorter than the star's age ([`AGE_GYR`] or the
    /// star's lifetime) and its eccentricity under 0.06 (see
    /// `crate::spin`).
    pub tidally_locked: bool,
    /// Whether the star's tides hold the planet in the 3:2 spin–orbit
    /// resonance instead, three turns every two orbits, as Mercury: despun
    /// on an orbit of eccentricity 0.06 or more (see `crate::spin`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub spin_orbit_3_2: bool,
    /// Turns per orbit of a planet its star's tides hold in a spin-orbit
    /// resonance: 1.5 (3:2) or 2 (2:1), drawn by the
    /// eccentricity (see `crate::spin::capture`); 0 for a planet in none
    /// (free or locked).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f32>", optional))]
    pub spin_turns: f32,
    /// Colour of the star's light, linear RGB, the Sun white.
    pub light: [f32; 3],
    /// Metallicity, `[Fe/H]` in dex (0 the Sun's).
    #[serde(default)]
    pub metallicity: f32,
    /// Longitude of the orbit's perihelion, degrees from the spring equinox
    /// (see [`crate::world::WorldParams::perihelion_longitude_deg`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub perihelion_longitude_deg: Option<f32>,
}

fn is_zero(x: &f32) -> bool {
    *x == 0.0
}

impl StarInfo {
    /// The star of type `code` with an Earth-like planet (Earth's mass and
    /// radius) on an orbit of semi-major axis `orbit_au` and
    /// `eccentricity`; the draw of its spin-orbit resonance is keyed by
    /// the orbit (see [`Self::for_planet`]).
    pub fn new(code: f32, orbit_au: f32, eccentricity: f32) -> Self {
        let key = orbit_au.to_bits() ^ eccentricity.to_bits().rotate_left(16) ^ code.to_bits();
        Self::for_planet(code, orbit_au, eccentricity, 1.0, 1.0, key)
    }

    /// The star of type `code` with a rocky planet of `mass_earth` and
    /// `radius_earth` (Earth units) on an orbit of semi-major axis
    /// `orbit_au` and `eccentricity`: its tides lock or despin it in a
    /// time that scales with the planet's mass and radius
    /// ([`crate::spin::rocky_locking_gyr`]), and the resonance it is
    /// captured in, if despun on an eccentric orbit, is drawn from `seed`
    /// ([`crate::spin::capture`]).
    pub fn for_planet(
        code: f32,
        orbit_au: f32,
        eccentricity: f32,
        mass_earth: f32,
        radius_earth: f32,
        seed: u32,
    ) -> Self {
        let v = values(code);
        let at = |au: f32| v.luminosity / (au * au);
        let insolation = at(orbit_au) * mean_flux_factor(eccentricity);
        let (periapsis, apoapsis) = (
            orbit_au * (1.0 - eccentricity),
            orbit_au * (1.0 + eccentricity),
        );
        let (inner, outer) = habitable_fluxes(v.teff_k);
        let lifetime = lifetime_gyr(v.mass);
        let locking = crate::spin::rocky_locking_gyr(orbit_au, v.mass, mass_earth, radius_earth);
        let age = AGE_GYR.min(lifetime);
        let turns = if locking < age {
            crate::spin::capture(eccentricity, seed)
        } else {
            0.0
        };
        Self {
            kind: type_name(code),
            teff_k: v.teff_k,
            luminosity: v.luminosity,
            mass: v.mass,
            radius: v.radius,
            orbit_au,
            eccentricity,
            periapsis_au: periapsis,
            apoapsis_au: apoapsis,
            insolation,
            insolation_periapsis: at(periapsis),
            insolation_apoapsis: at(apoapsis),
            hz_position: position_of(v.teff_k, insolation),
            hz_inner_au: distance_au(v.luminosity, inner),
            hz_outer_au: distance_au(v.luminosity, outer),
            equilibrium_k: equilibrium_k(at(orbit_au)) * mean_temperature_factor(eccentricity),
            year_days: year_days(orbit_au, v.mass),
            angular_diameter_deg: angular_diameter_deg(v.radius, orbit_au),
            lifetime_gyr: lifetime,
            locking_gyr: locking,
            tidally_locked: locking < age && turns == 0.0,
            spin_orbit_3_2: turns == crate::spin::RESONANCE_TURNS_PER_ORBIT,
            spin_turns: turns,
            light: light_colour(v.teff_k),
            metallicity: 0.0,
            perihelion_longitude_deg: None,
        }
    }

    /// The spectral type of a stored star, the Sun's if it does not parse.
    pub fn code(&self) -> f32 {
        parse_type(&self.kind).unwrap_or(SUN)
    }
}

/// Flux a planet on an orbit of semi-major axis `orbit_au` and
/// `eccentricity` gets over its year from a star of type `code`, relative to
/// what Earth gets from the Sun of the model (G2V at 1 AU, circular):
/// exactly 1 for it, so worlds around the Sun at 1 AU keep their climate.
pub fn relative_flux(code: f32, orbit_au: f32, eccentricity: f32) -> f32 {
    let sun = values(SUN).luminosity;
    values(code).luminosity / (orbit_au * orbit_au) / sun * mean_flux_factor(eccentricity)
}

/// How much more light an orbit of `eccentricity` gets over a year than a
/// circular one of the same semi-major axis: `1 / sqrt(1 − e²)` (1.0001 for
/// Earth, 1.02 for Mercury, 1.25 at e = 0.6).
pub fn mean_flux_factor(eccentricity: f32) -> f32 {
    1.0 / (1.0 - eccentricity * eccentricity).max(1e-6).sqrt()
}

/// The equilibrium temperature of an orbit of `eccentricity` averaged over
/// its year, relative to a circular orbit's of the same semi-major axis:
/// `1 − e²/16 − 15e⁴/1024` (Méndez & Rivera-Valentín 2017, eq. 16; 0.997
/// at e = 0.2, 0.976 at 0.6). It falls with the eccentricity although the
/// mean flux rises ([`mean_flux_factor`]): the planet spends longer far
/// out, and the temperature goes as the flux's fourth root. The climate
/// keeps the mean flux, which sets the heat a world absorbs over a year.
pub fn mean_temperature_factor(eccentricity: f32) -> f32 {
    let e2 = eccentricity * eccentricity;
    1.0 - e2 / 16.0 - 15.0 * e2 * e2 / 1024.0
}

/// A gamma variate of `shape` (scale 1), Marsaglia & Tsang (2000), boosted
/// for shapes below 1.
fn gamma(shape: f64, rng: &mut impl RngExt) -> f64 {
    if shape < 1.0 {
        let u: f64 = rng.random::<f64>().max(1e-12);
        return gamma(shape + 1.0, rng) * u.powf(1.0 / shape);
    }
    let d = shape - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();
    loop {
        // A standard normal by Box-Muller.
        let (u1, u2): (f64, f64) = (rng.random::<f64>().max(1e-12), rng.random());
        let x = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
        let v = (1.0 + c * x).powi(3);
        if v <= 0.0 {
            continue;
        }
        let u: f64 = rng.random::<f64>().max(1e-12);
        if u.ln() < 0.5 * x * x + d - d * v + d * v.ln() {
            return d * v;
        }
    }
}

/// An orbital eccentricity drawn like those of the planets known: the beta
/// distribution fitted by Kipping (2013) to radial-velocity planets
/// (a = 0.867, b = 3.03; mean 0.22, median 0.17), kept within
/// [`ECCENTRICITY`].
pub fn draw_eccentricity(rng: &mut impl RngExt) -> f32 {
    let (x, y) = (gamma(0.867, rng), gamma(3.03, rng));
    ((x / (x + y)) as f32).min(ECCENTRICITY.max)
}

/// An orbital eccentricity drawn like those of the small planets known
/// (radius under 6 Earth radii, period over 5 days): the beta
/// distributions Van Eylen et al. (2019, table 3,
/// doi:10.3847/1538-3881/aaf22f) fit to Kepler planets with asteroseismic
/// stars, `a = 1.58`, `b = 4.4` (mean 0.26) for the planets of
/// single-transiting systems (other planets may exist without transiting)
/// and `a = 1.52`, `b = 29` (mean 0.05) for those of multi-transiting
/// systems, against Kipping's (2013) 0.22 fitted to mostly giant planets
/// ([`draw_eccentricity`]). Their sample has periods of 5 to about 100
/// days: the app applies the fits at every distance (an extrapolation).
/// Kept within [`ECCENTRICITY`].
pub fn draw_small_eccentricity(in_system: bool, rng: &mut impl RngExt) -> f32 {
    let (a, b) = if in_system { (1.52, 29.0) } else { (1.58, 4.4) };
    let (x, y) = (gamma(a, rng), gamma(b, rng));
    ((x / (x + y)) as f32).min(ECCENTRICITY.max)
}

/// Temperature change at the outer edge of the habitable zone for a
/// regulated planet, °C: the edge is where the strongest CO₂ greenhouse
/// still keeps water liquid, a mean about 15 °C below Earth's.
const REGULATED_OUTER: f32 = -15.0;
/// Temperature change at the inner edge, °C: the runaway greenhouse sets in.
const REGULATED_INNER: f32 = 30.0;

/// How the star sets a planet's temperatures (see [`star_temperatures`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarClimate {
    /// Spectral type and orbit.
    pub star: f32,
    pub orbit_au: f32,
    pub eccentricity: f32,
    /// How strongly a carbonate-silicate cycle regulates the greenhouse,
    /// in `[0, 1]`: 1 as on Earth (plate tectonics and liquid water), less
    /// on a wet stagnant-lid world, 0 without one.
    pub regulation: f32,
    /// Internal heat, K (see `ClimateShift::internal_heat`).
    pub internal_heat_k: f32,
}

/// Applies the star to rows of sea-level temperature (°C, Earth's profile
/// around the Sun):
///
/// - **unregulated** planets scale in kelvin with the flux: `T × S^(1/4)`,
///   which also narrows the equator-to-pole difference around a dim star;
/// - **regulated** planets keep a temperate climate across the habitable
///   zone, as their greenhouse grows when the light weakens: from +30 °C at
///   the inner edge through Earth's climate at Earth's place to −15 °C at
///   the outer edge; beyond the edges they follow the flux like the others.
///   A partial regulation takes that share of the way from the unregulated
///   temperature to the regulated one;
/// - **internal heat** adds as radiated power: `T = (T⁴ + T_int⁴)^(1/4)`.
///
/// The Sun at 1 AU leaves the rows unchanged.
pub fn star_temperatures(rows: &mut [f32], climate: StarClimate) {
    const K: f32 = 273.15;
    let s = relative_flux(climate.star, climate.orbit_au, climate.eccentricity);
    let scale = |rows: &mut [f32], flux: f32| {
        let f = flux.powf(0.25);
        for t in rows.iter_mut() {
            *t = (*t + K) * f - K;
        }
    };
    if climate.regulation > 0.0 {
        let free: Option<Vec<f32>> = (climate.regulation < 1.0).then(|| {
            let mut free = rows.to_vec();
            if s != 1.0 {
                scale(&mut free, s);
            }
            free
        });
        let teff = values(climate.star).teff_k;
        let (inner, outer) = habitable_fluxes(teff);
        // Positions relative to the model Sun's: Earth's place is exactly
        // Earth's climate.
        let flux = s * values(SUN).luminosity;
        let position = position_of(teff, flux);
        let earth = position_of(values(SUN).teff_k, values(SUN).luminosity);
        let (shift, beyond) = if position < 0.0 {
            (REGULATED_INNER, Some(flux / inner))
        } else if position <= earth {
            (REGULATED_INNER * (earth - position) / earth, None)
        } else if position <= 1.0 {
            (REGULATED_OUTER * (position - earth) / (1.0 - earth), None)
        } else {
            (REGULATED_OUTER, Some(flux / outer))
        };
        for t in rows.iter_mut() {
            *t += shift;
        }
        if let Some(beyond) = beyond {
            scale(rows, beyond);
        }
        if let Some(free) = free {
            let w = climate.regulation.min(1.0);
            for (t, f) in rows.iter_mut().zip(free) {
                *t = f + w * (*t - f);
            }
        }
    } else if s != 1.0 {
        scale(rows, s);
    }
    if climate.internal_heat_k > 0.0 {
        let inner4 = climate.internal_heat_k.powi(4);
        for t in rows.iter_mut() {
            *t = ((*t + K).max(0.0).powi(4) + inner4).powf(0.25) - K;
        }
    }
}

/// The orbit of a planet at `position` in the habitable zone of a star of
/// type `code`, AU.
pub fn orbit_at(code: f32, position: f32) -> f32 {
    let v = values(code);
    distance_au(v.luminosity, flux_at(v.teff_k, position))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn the_sun_has_the_suns_values() {
        let sun = values(SUN);
        assert!((sun.teff_k - 5770.0).abs() < 1.0);
        assert!((sun.luminosity - 1.0).abs() < 0.03);
        assert!((sun.mass - 1.0).abs() < 1e-3);
        assert_eq!(type_name(SUN), "G2V");
    }

    #[test]
    fn types_parse_and_print() {
        assert_eq!(parse_type("G2V").unwrap(), 42.0);
        assert_eq!(parse_type("m4.5").unwrap(), 64.5);
        assert_eq!(parse_type("K").unwrap(), 55.0);
        assert_eq!(type_name(64.5), "M4.5V");
        assert_eq!(type_name(42.3), "G2.3V");
        assert!(parse_type("X2").is_err());
        assert!(parse_type("G12").is_err());
        assert!(parse_type("O1").is_err());
    }

    #[test]
    fn interpolation_is_monotonic() {
        let mut last = values(TYPE_RANGE.0);
        let mut code = TYPE_RANGE.0 + 0.25;
        while code <= TYPE_RANGE.1 {
            let v = values(code);
            assert!(
                v.teff_k <= last.teff_k && v.luminosity <= last.luminosity * 1.001,
                "{code}"
            );
            last = v;
            code += 0.25;
        }
    }

    #[test]
    fn the_habitable_zone_matches_kopparapu() {
        // The Sun: 0.95 to 1.68 AU.
        let (inner, outer) = habitable_fluxes(5780.0);
        assert!((distance_au(1.0, inner) - 0.95).abs() < 0.01);
        assert!((distance_au(1.0, outer) - 1.68).abs() < 0.01);
        // Positions round-trip, and Earth sits inside the zone.
        let earth = position_of(5772.0, 1.0);
        assert!((0.0..0.3).contains(&earth), "{earth}");
        assert!((flux_at(5772.0, earth) - 1.0).abs() < 1e-4);
        // A cool star's zone sits at lower fluxes.
        assert!(habitable_fluxes(3000.0).0 < inner);
    }

    #[test]
    fn orbits_give_the_known_numbers() {
        assert!((year_days(1.0, 1.0) - 365.25).abs() < 1e-3);
        assert!((angular_diameter_deg(1.0, 1.0) - 0.533).abs() < 1e-3);
        assert!((equilibrium_k(1.0) - 255.0).abs() < 1e-3);
        assert!(locking_gyr(1.0, 1.0) > 100.0);
        assert!(locking_gyr(0.05, 0.12) < 1e-3);
    }

    #[test]
    fn colours_go_from_red_to_blue() {
        let sun = light_colour(SUN_TEFF_K);
        assert!(sun.iter().all(|c| (c - 1.0).abs() < 1e-4), "{sun:?}");
        let red = light_colour(3000.0);
        assert!(red[0] == 1.0 && red[2] < 0.5 * red[0], "{red:?}");
        let blue = light_colour(10000.0);
        assert!(blue[2] == 1.0 && blue[0] < blue[2], "{blue:?}");
    }

    #[test]
    fn earth_reads_as_earth() {
        let earth = StarInfo::new(SUN, 1.0, 0.0);
        assert_eq!(earth.kind, "G2V");
        assert!((earth.insolation - 1.0).abs() < 0.03);
        assert!(!earth.tidally_locked);
        assert!(StarInfo::new(parse_type("M5V").unwrap(), orbit_at(65.0, 0.5), 0.0).tidally_locked);
        let back = StarInfo::new(earth.code(), earth.orbit_au, earth.eccentricity);
        assert_eq!(back, earth);
    }

    #[test]
    fn the_sun_at_one_au_keeps_the_climate() {
        let rows = vec![-18.0, 12.0, 27.0];
        for regulation in [0.0, 0.5, 1.0] {
            let mut same = rows.clone();
            star_temperatures(
                &mut same,
                StarClimate {
                    star: SUN,
                    orbit_au: 1.0,
                    eccentricity: 0.0,
                    regulation,
                    internal_heat_k: 0.0,
                },
            );
            for (a, b) in same.iter().zip(&rows) {
                assert!((a - b).abs() < 1e-3, "{same:?}");
            }
        }
    }

    #[test]
    fn the_star_warms_and_cools_planets() {
        let at = |orbit_au: f32, regulation: f32, internal_heat_k: f32| {
            let mut rows = vec![14.0];
            star_temperatures(
                &mut rows,
                StarClimate {
                    star: SUN,
                    orbit_au,
                    eccentricity: 0.0,
                    regulation,
                    internal_heat_k,
                },
            );
            rows[0]
        };
        // Mars, unregulated: 0.81 × 287 K.
        assert!(
            (at(1.524, 0.0, 0.0) - -40.5).abs() < 1.0,
            "{}",
            at(1.524, 0.0, 0.0)
        );
        // Venus before its greenhouse.
        assert!(
            (at(0.723, 0.0, 0.0) - 64.5).abs() < 1.0,
            "{}",
            at(0.723, 0.0, 0.0)
        );
        // A regulated planet stays temperate across the zone: −15 °C at most
        // at its outer edge, then cools.
        let outer_au = orbit_at(SUN, 1.0);
        assert!((at(outer_au, 1.0, 0.0) - -1.0).abs() < 0.1);
        assert!(at(outer_au * 1.5, 1.0, 0.0) < -30.0);
        // A weaker cycle (a wet stagnant lid) goes part of the way.
        let (free, half, full) = (at(1.3, 0.0, 0.0), at(1.3, 0.5, 0.0), at(1.3, 1.0, 0.0));
        assert!(free < half && half < full, "{free} {half} {full}");
        assert!((half - 0.5 * (free + full)).abs() < 1e-3);
        // Internal heat keeps a far world warm.
        assert!(at(30.0, 0.0, 0.0) < -200.0);
        assert!(at(30.0, 0.0, 290.0) > 10.0);
    }

    #[test]
    fn lifetimes_follow_the_table() {
        assert!((lifetime_gyr(1.0) - 10.3).abs() < 0.1);
        assert!((lifetime_gyr(0.4) - 200.0).abs() < 1e-2);
        assert!((lifetime_gyr(0.08) - 11_000.0).abs() < 1.0);
        assert!((lifetime_gyr(0.01) - 11_000.0).abs() < 1.0);
        assert!((lifetime_gyr(60.0) - 0.001).abs() < 1e-6);
        let masses: Vec<f32> = (0..200).map(|k| 0.08 * 1.03f32.powi(k)).collect();
        assert!(
            masses
                .windows(2)
                .all(|m| lifetime_gyr(m[1]) < lifetime_gyr(m[0]))
        );
    }

    #[test]
    fn eccentric_orbits_get_more_light_and_draw_like_known_planets() {
        let mercury = StarInfo::new(SUN, 0.387, 0.2056);
        assert!((mercury.periapsis_au - 0.307).abs() < 1e-3);
        assert!((mercury.apoapsis_au - 0.467).abs() < 1e-3);
        assert!(mercury.insolation_periapsis > 2.0 * mercury.insolation_apoapsis);
        assert!((mean_flux_factor(0.6) - 1.25).abs() < 1e-4);
        // The mean temperature falls instead (Méndez & Rivera-Valentín 2017).
        assert!((mean_temperature_factor(0.6) - 0.9756).abs() < 1e-3);
        let (circle, eccentric) = (StarInfo::new(SUN, 1.0, 0.0), StarInfo::new(SUN, 1.0, 0.6));
        assert!(eccentric.insolation > circle.insolation);
        assert!(eccentric.equilibrium_k < circle.equilibrium_k);
        let mut rng = rand::rngs::StdRng::seed_from_u64(5);
        let draws: Vec<f32> = (0..20_000).map(|_| draw_eccentricity(&mut rng)).collect();
        let mean = draws.iter().sum::<f32>() / draws.len() as f32;
        assert!((0.19..0.25).contains(&mean), "mean eccentricity {mean}");
        assert!(draws.iter().all(|e| (0.0..=ECCENTRICITY.max).contains(e)));
    }

    #[test]
    fn small_planets_are_less_eccentric_than_giants() {
        let mean = |f: &dyn Fn(&mut rand::rngs::StdRng) -> f32| {
            let mut rng = rand::rngs::StdRng::seed_from_u64(5);
            (0..20_000).map(|_| f(&mut rng)).sum::<f32>() / 20_000.0
        };
        // Van Eylen et al. 2019, table 3: Beta(1.58, 4.4) has mean 0.26,
        // Beta(1.52, 29) mean 0.05; Kipping's giants 0.22.
        let alone = mean(&|r| draw_small_eccentricity(false, r));
        let together = mean(&|r| draw_small_eccentricity(true, r));
        let giants = mean(&|r| draw_eccentricity(r));
        assert!((0.24..0.28).contains(&alone), "{alone}");
        assert!((0.04..0.06).contains(&together), "{together}");
        assert!(together < giants && giants < alone + 0.05, "{giants}");
    }

    #[test]
    fn draws_follow_the_neighbourhood() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        let draws: Vec<f32> = (0..20_000).map(|_| draw_type(&mut rng)).collect();
        let m = draws.iter().filter(|&&c| c >= 60.0).count() as f32 / draws.len() as f32;
        assert!((0.72..0.81).contains(&m), "M share {m}");
        assert!(
            draws
                .iter()
                .all(|&c| (TYPE_RANGE.0..=TYPE_RANGE.1).contains(&c))
        );
    }
}
