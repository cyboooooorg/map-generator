//! The light of the star, by wavelength: the bands the shortwave table is
//! built on, each with its Rayleigh scale and its share of the light.
//!
//! The shares come from stellar model spectra binned on the bands
//! (`crates/science/data/stellar-spectra.tsv`, built by
//! `tools/stellar-spectra`, provenance in `crates/science/data/SPECTRA.md`):
//! PHOENIX-ACES-AGSS-COND-2011 from 2300 to 12 000 K (Husser et al. 2013,
//! doi:10.1051/0004-6361/201219058) and ATLAS9 above, to 27 000 K
//! (Castelli & Kurucz 2003, arXiv:astro-ph/0405087), at [M/H] −1, −0.5, 0
//! and +0.5, interpolated linearly in both. A real star is not a black
//! body: its own atmosphere takes light out of the blue and ultraviolet,
//! where Rayleigh scattering acts (study 35, Wiki
//! `35-stellar-spectra-audit`, issue #655; the legacy and the first port
//! weighted the bands with a black body).
//!
//! A redder star's light scatters less, Rayleigh's `lambda^-4` weighing the
//! blue, and more of it falls in the near infrared, where CO2 and water
//! absorb it: the two reasons the outer edge of an M dwarf's habitable
//! zone lies at a lower flux (Kopparapu et al. 2013, section 4.1,
//! doi:10.1088/0004-637X/765/2/131, read on arXiv:1301.6674v2).

use std::sync::LazyLock;

use crate::constants::SOLAR_EFFECTIVE_TEMPERATURE;

/// Definition: number of bands of the star's light, a numerical choice
/// (bands even in `ln lambda`), see the radiative reference page in
/// `docs/science/`, shortwave section.
pub(crate) const BANDS: usize = 240;

/// Definition: shortest wavelength of the bands, m, a numerical choice of
/// the band model's range.
pub(crate) const FROM_M: f64 = 0.2e-6;

/// Definition: longest wavelength of the bands, m, a numerical choice of
/// the band model's range.
pub(crate) const TO_M: f64 = 5.0e-6;

/// Definition: wavelength at which the Rayleigh scale is 1, m (550 nm, the
/// reference of the Rayleigh cross sections).
const REFERENCE_M: f64 = 0.55e-6;

/// Definition: the Sun's metallicity, dex: [M/H] is the logarithm of a
/// star's metal abundance relative to the Sun's.
pub(crate) const SOLAR_METALLICITY: f64 = 0.0;

/// The Rayleigh scale of band `band` (bluest first): `(550 nm / λ)⁴` at
/// its centre in `ln λ`, the ratio of its Rayleigh depth to the depth at
/// 550 nm that the shortwave and the column past the longwave table
/// (`column::beyond`) both take.
pub(crate) fn rayleigh_scale(band: usize) -> f64 {
    let lambda = (band_edge_m(band) * band_edge_m(band + 1)).sqrt();
    (REFERENCE_M / lambda).powi(4)
}

/// The table of spectra (see the module's documentation).
const TABLE: &str = include_str!("../../data/stellar-spectra.tsv");

/// Wavelength, m, of the `e`-th edge of the bands (0 to [`BANDS`]), from
/// 0.2 um: even in `ln lambda`. The shortwave table and the table of
/// spectra are built on the same edges (`tools/opacity-table
/// --shortwave`, `tools/stellar-spectra`).
pub(crate) fn band_edge_m(e: usize) -> f64 {
    let step = (TO_M / FROM_M).ln() / BANDS as f64;
    FROM_M * (step * e as f64).exp()
}

/// The spectra of the table: the band shares of each metallicity and
/// temperature, on one grid (every metallicity has the same
/// temperatures).
#[derive(Debug)]
pub(crate) struct Spectra {
    /// [M/H], dex, increasing.
    pub(crate) metallicities: Vec<f64>,
    /// Effective temperatures, K, increasing.
    pub(crate) temperatures: Vec<f64>,
    /// The shares of each spectrum, `[metallicity][temperature]`.
    pub(crate) shares: Vec<Vec<[f64; BANDS]>>,
}

impl Spectra {
    /// Parses the table. It is part of the build: a malformed line is a
    /// bug of the build, and panics with its line number.
    fn parse(text: &str) -> Spectra {
        let mut rows: Vec<(f64, f64, [f64; BANDS])> = Vec::new();
        let mut layout = false;
        for (n, line) in text.lines().enumerate() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            if fields[0] == "bands" {
                check_layout(&fields, n);
                layout = true;
                continue;
            }
            assert_eq!(
                fields.len(),
                4 + BANDS,
                "stellar-spectra.tsv:{}: {} fields",
                n + 1,
                fields.len()
            );
            let number = |s: &str| -> f64 {
                s.parse()
                    .unwrap_or_else(|e| panic!("stellar-spectra.tsv:{}: {s}: {e}", n + 1))
            };
            let mut shares = [0.0; BANDS];
            for (share, field) in shares.iter_mut().zip(&fields[4..]) {
                *share = number(field);
            }
            rows.push((number(fields[0]), number(fields[1]), shares));
        }
        assert!(layout, "stellar-spectra.tsv: no bands line");
        let mut metallicities: Vec<f64> = rows.iter().map(|r| r.0).collect();
        metallicities.sort_by(f64::total_cmp);
        metallicities.dedup();
        let mut temperatures: Vec<f64> = rows.iter().map(|r| r.1).collect();
        temperatures.sort_by(f64::total_cmp);
        temperatures.dedup();
        let shares = metallicities
            .iter()
            .map(|m| {
                temperatures
                    .iter()
                    .map(|t| {
                        rows.iter()
                            .find(|r| r.0 == *m && r.1 == *t)
                            .unwrap_or_else(|| panic!("stellar-spectra.tsv: no {t} K at {m}"))
                            .2
                    })
                    .collect()
            })
            .collect();
        Spectra {
            metallicities,
            temperatures,
            shares,
        }
    }
}

/// Checks a table's `bands` line (`bands`, their number, their first
/// and last edges, m) against [`BANDS`], [`FROM_M`] and [`TO_M`]: a table
/// built on other bands is a bug of the build, and panics with its line.
pub(crate) fn check_layout(fields: &[&str], n: usize) {
    let parsed: Option<(usize, f64, f64)> = (|| {
        Some((
            fields.get(1)?.parse().ok()?,
            fields.get(2)?.parse().ok()?,
            fields.get(3)?.parse().ok()?,
        ))
    })();
    assert_eq!(
        parsed,
        Some((BANDS, FROM_M, TO_M)),
        "line {}: the table's bands are not the shortwave's",
        n + 1
    );
}

/// The measured Sun's shares on the bands (`crates/science/data/tsis-sun.tsv`,
/// TSIS-1 HSRS, Coddington et al. 2021, binned by `tools/stellar-spectra`),
/// as [`StarBands::bands`] holds them: the tests' reference Sun.
#[cfg(test)]
pub(crate) fn measured_sun() -> Vec<(f64, f64)> {
    let text = include_str!("../../data/tsis-sun.tsv");
    let mut lines = text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.starts_with('#') && !l.trim().is_empty());
    let (n, layout) = lines.next().expect("a bands line");
    check_layout(&layout.split('\t').collect::<Vec<_>>(), n);
    let (_, row) = lines.next().expect("a row of shares");
    let shares: Vec<f64> = row
        .split('\t')
        .map(|x| x.parse().expect("a share"))
        .collect();
    assert_eq!(shares.len(), BANDS);
    let total: f64 = shares.iter().sum();
    shares
        .iter()
        .enumerate()
        .map(|(i, share)| (rayleigh_scale(i), share / total))
        .collect()
}

/// The table, parsed once.
pub(crate) static SPECTRA: LazyLock<Spectra> = LazyLock::new(|| Spectra::parse(TABLE));

/// The index `i` of the grid's interval holding `x` (`grid[i] <= x <=
/// grid[i + 1]`) and `x`'s place in it, 0 to 1; `x` is held within the
/// grid.
fn place(grid: &[f64], x: f64) -> (usize, f64) {
    let last = grid.len() - 1;
    let x = x.clamp(grid[0], grid[last]);
    let i = grid.partition_point(|g| *g <= x).clamp(1, last) - 1;
    (i, (x - grid[i]) / (grid[i + 1] - grid[i]))
}

/// The light of a star: its effective temperature, its metallicity and
/// its bands.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StarBands {
    /// Effective temperature, K, the bands were computed for.
    pub(crate) star_k: f64,
    /// Metallicity [M/H], dex, the bands were computed for.
    pub(crate) metallicity: f64,
    /// Each band's `(550 nm / lambda)^4` and its share of the light (they
    /// sum to 1), bluest first.
    pub(crate) bands: Vec<(f64, f64)>,
}

/// The light of a star of effective temperature `star_k` and metallicity
/// `metallicity` ([M/H], dex), by wavelength: the table's spectra
/// interpolated linearly in temperature and metallicity, then normalised.
/// A temperature or a metallicity outside the table is held at its edge
/// (2300 to 27 000 K, −1 to +0.5 dex: every star the system engine draws
/// lies inside). A temperature that is not positive or not finite (NaN,
/// infinite) is the Sun's, a metallicity that is not finite the Sun's.
pub(crate) fn star_bands(star_k: f64, metallicity: f64) -> StarBands {
    let star_k = if star_k > 0.0 && star_k.is_finite() {
        star_k
    } else {
        SOLAR_EFFECTIVE_TEMPERATURE.0
    };
    let metallicity = if metallicity.is_finite() {
        metallicity
    } else {
        SOLAR_METALLICITY
    };
    let spectra = &*SPECTRA;
    let (t, ft) = place(&spectra.temperatures, star_k);
    let (m, fm) = place(&spectra.metallicities, metallicity);
    let corners = [
        (m, t, (1.0 - fm) * (1.0 - ft)),
        (m, t + 1, (1.0 - fm) * ft),
        (m + 1, t, fm * (1.0 - ft)),
        (m + 1, t + 1, fm * ft),
    ];
    let mut shares = [0.0; BANDS];
    for (mi, ti, w) in corners {
        if w == 0.0 {
            continue;
        }
        for (share, s) in shares.iter_mut().zip(&spectra.shares[mi][ti]) {
            *share += w * s;
        }
    }
    let total: f64 = shares.iter().sum();
    let bands = shares
        .iter()
        .enumerate()
        .map(|(i, share)| (rayleigh_scale(i), share / total))
        .collect();
    StarBands {
        star_k,
        metallicity,
        bands,
    }
}

#[cfg(test)]
#[path = "starlight_tests.rs"]
mod tests;
