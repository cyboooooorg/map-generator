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
use crate::request::{HZ_POSITION, ORBIT_AU};
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

/// Main-sequence lifetime, Gyr: `10 Gyr × M / L`.
pub fn lifetime_gyr(mass: f32, luminosity: f32) -> f32 {
    10.0 * mass / luminosity
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
    /// Flux the planet receives, relative to Earth's.
    pub insolation: f32,
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
    /// Whether the planet is tidally locked: its locking time is shorter
    /// than the star's age ([`AGE_GYR`] or the star's lifetime).
    pub tidally_locked: bool,
    /// Colour of the star's light, linear RGB, the Sun white.
    pub light: [f32; 3],
}

impl StarInfo {
    /// The star of type `code` with a planet at `orbit_au`.
    pub fn new(code: f32, orbit_au: f32) -> Self {
        let v = values(code);
        let insolation = v.luminosity / (orbit_au * orbit_au);
        let (inner, outer) = habitable_fluxes(v.teff_k);
        let lifetime = lifetime_gyr(v.mass, v.luminosity);
        let locking = locking_gyr(orbit_au, v.mass);
        Self {
            kind: type_name(code),
            teff_k: v.teff_k,
            luminosity: v.luminosity,
            mass: v.mass,
            radius: v.radius,
            orbit_au,
            insolation,
            hz_position: position_of(v.teff_k, insolation),
            hz_inner_au: distance_au(v.luminosity, inner),
            hz_outer_au: distance_au(v.luminosity, outer),
            equilibrium_k: equilibrium_k(insolation),
            year_days: year_days(orbit_au, v.mass),
            angular_diameter_deg: angular_diameter_deg(v.radius, orbit_au),
            lifetime_gyr: lifetime,
            locking_gyr: locking,
            tidally_locked: locking < AGE_GYR.min(lifetime),
            light: light_colour(v.teff_k),
        }
    }

    /// The spectral type of a stored star, the Sun's if it does not parse.
    pub fn code(&self) -> f32 {
        parse_type(&self.kind).unwrap_or(SUN)
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
        let earth = StarInfo::new(SUN, 1.0);
        assert_eq!(earth.kind, "G2V");
        assert!((earth.insolation - 1.0).abs() < 0.03);
        assert!(!earth.tidally_locked);
        assert!(StarInfo::new(parse_type("M5V").unwrap(), orbit_at(65.0, 0.5)).tidally_locked);
        let back = StarInfo::new(earth.code(), earth.orbit_au);
        assert_eq!(back, earth);
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
