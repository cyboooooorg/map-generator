//! Builds science's table of stellar spectra on the shortwave's bands,
//! `crates/science/data/stellar-spectra.tsv` (see README.md): each
//! downloaded spectrum integrated over the 240 bands of
//! `science::radiation::starlight` and normalised to 1.
//!
//! ```text
//! stellar-spectra [--raw DIR] [--out FILE]
//! ```
//!
//! `--raw` is the directory `fetch.sh` fills (default
//! `tools/stellar-spectra/raw`), `--out` the table written (default
//! `crates/science/data/stellar-spectra.tsv`).

mod fits;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Definition: number of bands, the value of
/// `science::radiation::starlight::BANDS` (the science crate checks the
/// table against it).
const BANDS: usize = 240;

/// Definition: shortest wavelength of the bands, m
/// (`starlight::FROM_M`).
const FROM_M: f64 = 0.2e-6;

/// Definition: longest wavelength of the bands, m (`starlight::TO_M`).
const TO_M: f64 = 5.0e-6;

/// Definition: metres per ångström, a unit conversion.
const M_PER_ANGSTROM: f64 = 1e-10;

/// Definition: the ATLAS9 grid's surface gravity column for the hot
/// stars, log g = 4.0 (`g40`): the main-sequence log g of 2.6 to 8 M☉
/// stars at mid-life, 4.00 to 4.05 in the stellar tracks of
/// `crates/science/data/stellar-tracks.tsv`, rounded to the grid's
/// 0.5 dex step.
const ATLAS_GRAVITY: &str = "g40";

/// Definition: the hottest PHOENIX row, K; ATLAS9 takes over above it.
const PHOENIX_TOP_K: f64 = 12_000.0;

/// One spectrum binned on the bands.
struct Row {
    /// [M/H], dex.
    metallicity: f64,
    /// Effective temperature, K.
    teff: f64,
    /// log g, cgs.
    gravity: f64,
    /// The source file's name.
    source: String,
    /// Each band's share of the 0.2-5 µm light, bluest first.
    shares: Vec<f64>,
}

/// The band holding `lambda_m`, or `None` outside 0.2-5 µm. The bands
/// are even in `ln lambda`, as `starlight::band_edge_m` makes them.
fn band_of(lambda_m: f64) -> Option<usize> {
    if !(FROM_M..TO_M).contains(&lambda_m) {
        return None;
    }
    let step = (TO_M / FROM_M).ln() / BANDS as f64;
    // Truncation toward zero is the floor for a non-negative ratio.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a band index from a non-negative ratio below BANDS"
    )]
    let band = ((lambda_m / FROM_M).ln() / step) as usize;
    Some(band.min(BANDS - 1))
}

/// Each band's share of a spectrum sampled at `lambda_m` (increasing) with
/// flux density `flux` per unit wavelength: each sample weighs its flux
/// times its cell, half the distance to each neighbour. An error when the
/// samples do not span 0.2-5 µm.
fn bin(lambda_m: &[f64], flux: &[f64]) -> Result<Vec<f64>, String> {
    let n = lambda_m.len();
    if n != flux.len() || n < 2 {
        return Err(format!("{n} wavelengths for {} fluxes", flux.len()));
    }
    if lambda_m[0] > FROM_M || lambda_m[n - 1] < TO_M {
        return Err(format!(
            "the spectrum spans {:e} to {:e} m, not 0.2-5 µm",
            lambda_m[0],
            lambda_m[n - 1]
        ));
    }
    let mut shares = vec![0.0; BANDS];
    for i in 0..n {
        let Some(band) = band_of(lambda_m[i]) else {
            continue;
        };
        let low = lambda_m[i.saturating_sub(1)];
        let high = lambda_m[(i + 1).min(n - 1)];
        shares[band] += flux[i] * 0.5 * (high - low);
    }
    let total: f64 = shares.iter().sum();
    if !(total > 0.0 && total.is_finite()) || shares.iter().any(|s| *s < 0.0) {
        return Err(format!("a spectrum without light in 0.2-5 µm: {total}"));
    }
    Ok(shares.into_iter().map(|s| s / total).collect())
}

/// `(teff, log g, [M/H])` from a PHOENIX file name,
/// `lte05800-4.50-0.0.PHOENIX-…` or `lte05800-4.50+0.5.PHOENIX-…`.
fn phoenix_parameters(name: &str) -> Result<(f64, f64, f64), String> {
    let stem = name
        .strip_prefix("lte")
        .and_then(|s| s.split(".PHOENIX").next())
        .ok_or_else(|| format!("{name}: not a PHOENIX spectrum"))?;
    let bad = || format!("{name}: unreadable parameters");
    let teff: f64 = stem.get(..5).ok_or_else(bad)?.parse().map_err(|_| bad())?;
    let gravity: f64 = stem
        .get(6..10)
        .ok_or_else(bad)?
        .parse()
        .map_err(|_| bad())?;
    let metallicity: f64 = stem.get(10..).ok_or_else(bad)?.parse().map_err(|_| bad())?;
    // `-0.0` is solar: adding zero turns it into `0`.
    Ok((teff, gravity, metallicity + 0.0))
}

/// `(teff, [M/H])` from an ATLAS9 file name, `ckm05_13000.fits` or
/// `ckp05_13000.fits`.
fn atlas_parameters(name: &str) -> Result<(f64, f64), String> {
    let bad = || format!("{name}: not an ATLAS9 spectrum");
    let stem = name.strip_suffix(".fits").ok_or_else(bad)?;
    let (z, t) = stem
        .strip_prefix("ck")
        .ok_or_else(bad)?
        .split_once('_')
        .ok_or_else(bad)?;
    let tenths: f64 = z.get(1..).ok_or_else(bad)?.parse().map_err(|_| bad())?;
    let sign = match z.chars().next() {
        Some('m') => -1.0,
        Some('p') => 1.0,
        _ => return Err(bad()),
    };
    Ok((t.parse().map_err(|_| bad())?, sign * tenths / 10.0 + 0.0))
}

/// The files of `dir` whose names start with `prefix`, sorted.
fn files(dir: &Path, prefix: &str) -> Result<Vec<PathBuf>, String> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(prefix) && n.ends_with(".fits"))
        })
        .collect();
    out.sort();
    Ok(out)
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_owned()
}

/// The PHOENIX rows: every `lte*.fits` of `dir`, on the wavelengths of
/// `WAVE_PHOENIX-ACES-AGSS-COND-2011.fits` (Å, vacuum).
fn phoenix(dir: &Path) -> Result<Vec<Row>, String> {
    let wave_bytes = std::fs::read(dir.join("WAVE_PHOENIX-ACES-AGSS-COND-2011.fits"))
        .map_err(|e| format!("the PHOENIX wavelengths: {e}"))?;
    let wave_units = fits::units(&wave_bytes)?;
    let lambda_m: Vec<f64> = fits::image(&wave_units[0])?
        .into_iter()
        .map(|a| a * M_PER_ANGSTROM)
        .collect();
    let mut rows = Vec::new();
    for path in files(dir, "lte")? {
        let name = name_of(&path);
        let (teff, gravity, metallicity) = phoenix_parameters(&name)?;
        if teff > PHOENIX_TOP_K {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("{name}: {e}"))?;
        let units = fits::units(&bytes)?;
        let flux = fits::image(&units[0])?;
        let shares = bin(&lambda_m, &flux).map_err(|e| format!("{name}: {e}"))?;
        rows.push(Row {
            metallicity,
            teff,
            gravity,
            source: name,
            shares,
        });
    }
    Ok(rows)
}

/// The ATLAS9 rows: every `ck*.fits` of `dir` above [`PHOENIX_TOP_K`], its
/// [`ATLAS_GRAVITY`] column on its `WAVELENGTH` column (Å).
fn atlas(dir: &Path) -> Result<Vec<Row>, String> {
    let mut rows = Vec::new();
    for path in files(dir, "ck")? {
        let name = name_of(&path);
        let (teff, metallicity) = atlas_parameters(&name)?;
        if teff <= PHOENIX_TOP_K {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("{name}: {e}"))?;
        let units = fits::units(&bytes)?;
        let table = units.get(1).ok_or_else(|| format!("{name}: no table"))?;
        let lambda_m: Vec<f64> = fits::column(table, "WAVELENGTH")?
            .into_iter()
            .map(|a| a * M_PER_ANGSTROM)
            .collect();
        let flux = fits::column(table, ATLAS_GRAVITY)?;
        let shares = bin(&lambda_m, &flux).map_err(|e| format!("{name}: {e}"))?;
        rows.push(Row {
            metallicity,
            teff,
            gravity: 4.0,
            source: name,
            shares,
        });
    }
    Ok(rows)
}

/// The table: its comment header, then one row per spectrum, by [M/H]
/// then T_eff.
fn write_table(rows: &[Row]) -> String {
    let mut out = String::new();
    out.push_str(
        "# Stellar spectra on the shortwave's bands (tools/stellar-spectra, see\n\
         # crates/science/data/SPECTRA.md for the sources, terms and method).\n\
         # Source (PHOENIX rows, 2300-12000 K): PHOENIX-ACES-AGSS-COND-2011, Husser et al.\n\
         # 2013, A&A 553, A6, doi:10.1051/0004-6361/201219058, HiRes files; cited, no\n\
         # licence is stated; the spectra themselves are not redistributed.\n\
         # Source (ATLAS9 rows, 13000-27000 K): Castelli & Kurucz 2003, IAU Symp. 210,\n\
         # A20, arXiv:astro-ph/0405087, STScI ck04models, log g 4.0; cited, no licence\n\
         # is stated; the spectra themselves are not redistributed.\n\
         # Bands: 240, even in ln(lambda) from 0.2 to 5 um, bluest first; each share is\n\
         # the band's integral of the flux over the 0.2-5 um integral.\n\
         # columns: metallicity_dex teff_k logg source share_1 ... share_240\n",
    );
    for row in rows {
        let _ = write!(
            out,
            "{}\t{}\t{}\t{}",
            row.metallicity, row.teff, row.gravity, row.source
        );
        for share in &row.shares {
            let _ = write!(out, "\t{share:.4e}");
        }
        out.push('\n');
    }
    out
}

fn run() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut raw = root.join("raw");
    let mut out = root.join("../../crates/science/data/stellar-spectra.tsv");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--raw" => raw = PathBuf::from(args.next().ok_or("--raw takes a directory")?),
            "--out" => out = PathBuf::from(args.next().ok_or("--out takes a file")?),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let mut rows = phoenix(&raw.join("phoenix"))?;
    rows.extend(atlas(&raw.join("ck04"))?);
    rows.sort_by(|a, b| {
        a.metallicity
            .total_cmp(&b.metallicity)
            .then(a.teff.total_cmp(&b.teff))
    });
    std::fs::write(&out, write_table(&rows)).map_err(|e| format!("{}: {e}", out.display()))?;
    eprintln!("{} rows written to {}", rows.len(), out.display());
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("stellar-spectra: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_spectrum_fills_each_band_by_its_width() {
        // Definition: a flat F_lambda puts in each band its width, which
        // grows as lambda in bands even in ln(lambda).
        // Samples even in ln(lambda) from 0.1 to 6 µm, about 1300 per band.
        let lambda: Vec<f64> = (0..=500_000)
            .map(|i| 0.1e-6 * (60.0_f64.ln() * f64::from(i) / 500_000.0).exp())
            .collect();
        let flux = vec![1.0; lambda.len()];
        let shares = bin(&lambda, &flux).unwrap();
        let sum: f64 = shares.iter().sum();
        assert!((sum - 1.0).abs() < 1e-12);
        let ratio = (TO_M / FROM_M).powf(1.0 / BANDS as f64);
        for pair in shares.windows(2) {
            assert!((pair[1] / pair[0] / ratio - 1.0).abs() < 1e-3, "{pair:?}");
        }
    }

    #[test]
    fn a_short_spectrum_is_refused() {
        let lambda = [0.3e-6, 1.0e-6, 6.0e-6];
        assert!(bin(&lambda, &[1.0, 1.0, 1.0]).is_err());
    }

    #[test]
    fn file_names_give_the_parameters() {
        assert_eq!(
            phoenix_parameters("lte05800-4.50-0.0.PHOENIX-ACES-AGSS-COND-2011-HiRes.fits"),
            Ok((5800.0, 4.5, 0.0))
        );
        assert_eq!(
            phoenix_parameters("lte02300-5.00+0.5.PHOENIX-ACES-AGSS-COND-2011-HiRes.fits"),
            Ok((2300.0, 5.0, 0.5))
        );
        assert_eq!(atlas_parameters("ckm05_13000.fits"), Ok((13000.0, -0.5)));
        assert_eq!(atlas_parameters("ckp00_27000.fits"), Ok((27000.0, 0.0)));
    }
}
