//! `--ozone`: ozone's shortwave cross-sections on the albedo's bands, as
//! a sub-band distribution of each band (`crates/science/data/ozone.tsv`,
//! documented in `crates/science/data/OPACITY.md`, "Ozone
//! cross-sections").
//!
//! The input is the IUP Bremen file of Serdyuchenko et al. (2014,
//! *Atmos. Meas. Tech.* 7, 625, doi:10.5194/amt-7-625-2014),
//! `serdyuchenkogorshelev5digits.dat`: 213.33 to 1100 nm on a 0.01 nm
//! grid of vacuum wavelengths, cross-sections in cm² per molecule at 293
//! to 193 K in steps of 10 K. Each sample goes to the band of the
//! albedo's spectrum that holds its wavelength; a band's samples are
//! sorted and reduced to the k-tables' g-points ([`crate::kdist`]), so
//! that the science crate takes a band's mean transmission
//! `Σ w_g exp(−D σ_g N)` at any column `N`.

use crate::kdist::{BOUNDS, G, reduce};

/// Number of header lines of the IUP file (its own last header line says
/// "NUMBER OF HEADER LINES: 45").
const HEADER_LINES: usize = 45;

/// Temperatures of the file's columns 2 to 12, K (its header).
const FILE_TEMPERATURES_K: [f64; 11] = [
    293.0, 283.0, 273.0, 263.0, 253.0, 243.0, 233.0, 223.0, 213.0, 203.0, 193.0,
];

/// SHA-256 of `serdyuchenkogorshelev5digits.dat` as downloaded on
/// 2026-11-25 (and again on 2026-11-26); `fetch.sh` checks it.
pub const SHA256: &str = "4dfbf021b746512c192df5f0d43c54cee6ea3b4365bb490bcf6ed347f0ce7092";

/// Temperatures the table keeps, K: 223 K, the one the science crate
/// takes for the layer, and 233 K, which its tests compare it with (the
/// ozone-weighted temperature of the AFGL profiles is 225 to 232 K,
/// `docs/science/ozone.md`).
pub const TEMPERATURES_K: [f64; 2] = [223.0, 233.0];

/// One row of the file: its vacuum wavelength, nm, and its cross-sections
/// at [`FILE_TEMPERATURES_K`], cm² per molecule.
pub struct Row {
    pub wavelength_nm: f64,
    pub sigma: [f64; 11],
}

/// The rows of the IUP file `text`, past its header; `None` if a data
/// line does not hold twelve numbers.
pub fn parse(text: &str) -> Option<Vec<Row>> {
    text.lines()
        .skip(HEADER_LINES)
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let mut numbers = line.split_whitespace().map(|x| x.parse::<f64>().ok());
            let wavelength_nm = numbers.next()??;
            let mut sigma = [0.0; 11];
            for s in &mut sigma {
                *s = numbers.next()??;
            }
            numbers.next().is_none().then_some(Row {
                wavelength_nm,
                sigma,
            })
        })
        .collect()
}

/// The column of `t_k` in [`FILE_TEMPERATURES_K`].
fn column(t_k: f64) -> Option<usize> {
    FILE_TEMPERATURES_K.iter().position(|&t| t == t_k)
}

/// The band of the albedo's spectrum (`(from_um, to_um, bands)`, even in
/// `ln λ`, bluest first) that holds `wavelength_nm`; `None` outside it.
fn band(wavelength_nm: f64, (from_um, to_um, bands): (f64, f64, usize)) -> Option<usize> {
    let x = (wavelength_nm * 1e-3 / from_um).ln() / (to_um / from_um).ln() * bands as f64;
    (x >= 0.0 && x < bands as f64).then(|| x.floor() as usize)
}

/// The g-points of each band, bluest first, at `t_k`: the bands from the
/// first that holds a sample to the last; a band before the first sample
/// takes the first band's g-points (the file starts at 213.33 nm, inside
/// the fifth band). A negative cross-section (the file's noise around
/// zero, where the absorption is under its detection) counts as zero.
pub fn bands(rows: &[Row], t_k: f64, spectrum: (f64, f64, usize)) -> Option<Vec<[f64; G]>> {
    let c = column(t_k)?;
    let mut samples: Vec<Vec<f64>> = vec![Vec::new(); spectrum.2];
    for row in rows {
        if let Some(b) = band(row.wavelength_nm, spectrum) {
            samples[b].push(row.sigma[c].max(0.0));
        }
    }
    let first = samples.iter().position(|s| !s.is_empty())?;
    let last = samples.iter().rposition(|s| !s.is_empty())?;
    let mut out: Vec<[f64; G]> = samples[first..=last]
        .iter_mut()
        .map(|s| reduce(s))
        .collect();
    let held = out[0];
    out.splice(0..0, std::iter::repeat_n(held, first));
    Some(out)
}

/// The text of `ozone.tsv` from the IUP file's `rows` (the file of
/// [`SHA256`]): comment lines, a `bands` line (the spectrum's bands and outer
/// edges, m), a `g` line (the g-intervals' edges), then one row per band
/// and temperature, `temperature_k band sigma_1 … sigma_8`, cm² per
/// molecule.
pub fn table(rows: &[Row], spectrum: (f64, f64, usize)) -> Option<String> {
    let mut out = String::from(
        "# Ozone's shortwave cross-sections on the albedo's bands, as g-points\n\
         # Source: Serdyuchenko, A., Gorshelev, V., Weber, M., Chehade, W., Burrows, J. P. 2014,\n\
         # Atmos. Meas. Tech. 7, 625, doi:10.5194/amt-7-625-2014; data file\n\
         # serdyuchenkogorshelev5digits.dat (IUP Bremen, molecular spectroscopy lab; created 05.03.2012),\n",
    );
    out += &format!("# SHA-256 {SHA256}.\n");
    out += "# Terms: none stated on the data page or in the file; the paper is CC BY 3.0.\n\
            # Derived numbers only (sorted per band, g-interval means), cited.\n\
            # Each 0.01 nm sample goes to the band holding its wavelength; negative\n\
            # samples count as zero; the bands before 213.33 nm take the first band's\n\
            # g-points. Built by tools/opacity-table --ozone (crates/science/data/OPACITY.md).\n\
            # Columns: temperature_k band sigma_1 ... sigma_8 (cm2 per molecule), band 0 the bluest.\n";
    let (from_um, to_um, n) = spectrum;
    out += &format!("bands\t{n}\t{from_um}e-6\t{to_um}e-6\n");
    out += "g";
    for b in BOUNDS {
        out += &format!("\t{b}");
    }
    out += "\n";
    for t in TEMPERATURES_K {
        for (b, g) in bands(rows, t, spectrum)?.iter().enumerate() {
            out += &format!("{t}\t{b}");
            for s in g {
                out += &format!("\t{s:.5e}");
            }
            out += "\n";
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The albedo's spectrum (`main::SPECTRUM_UM`).
    const SPECTRUM: (f64, f64, usize) = (0.2, 5.0, 240);

    fn row(wavelength_nm: f64, sigma_223: f64) -> Row {
        let mut sigma = [0.0; 11];
        sigma[column(223.0).unwrap()] = sigma_223;
        Row {
            wavelength_nm,
            sigma,
        }
    }

    #[test]
    fn a_band_of_one_value_keeps_it_and_the_first_band_is_held() {
        // 255 nm lies in band 18 (200 nm × 25^(18/240) = 254.8 nm).
        assert_eq!(band(255.0, SPECTRUM), Some(18));
        assert_eq!(band(199.0, SPECTRUM), None);
        let rows: Vec<Row> = (0..200)
            .map(|i| row(255.0 + 0.01 * f64::from(i), 1e-17))
            .collect();
        let b = bands(&rows, 223.0, SPECTRUM).unwrap();
        assert_eq!(b.len(), 19);
        for g in &b {
            for s in g {
                assert!((s - 1e-17).abs() < 1e-30, "{s}");
            }
        }
    }

    #[test]
    fn negative_noise_counts_as_zero() {
        let rows = [row(600.0, -1e-24), row(600.01, 3e-24)];
        let b = bands(&rows, 223.0, SPECTRUM).unwrap();
        let last = b.last().unwrap();
        assert!(last.iter().all(|s| *s >= 0.0));
        // The mean over the g-intervals is the mean of the samples, zero
        // for the negative one.
        let mean: f64 = last
            .iter()
            .zip(BOUNDS.windows(2))
            .map(|(s, w)| s * (w[1] - w[0]))
            .sum();
        assert!((mean - 1.5e-24).abs() < 1e-36, "{mean}");
    }

    #[test]
    fn the_committed_table_is_on_the_tools_bands_and_g_points() {
        let committed = include_str!("../../../crates/science/data/ozone.tsv");
        let mut lines = committed.lines().filter(|l| !l.starts_with('#'));
        assert_eq!(lines.next(), Some("bands\t240\t0.2e-6\t5e-6"));
        let g: Vec<f64> = lines
            .next()
            .unwrap()
            .split('\t')
            .skip(1)
            .map(|x| x.parse().unwrap())
            .collect();
        assert_eq!(g, BOUNDS);
        assert!(committed.contains(SHA256));
    }

    #[test]
    fn the_file_format_is_read() {
        let mut text = String::new();
        for _ in 0..HEADER_LINES {
            text += "header\r\n";
        }
        text += "213.330 1E-19 2E-19 3E-19 4E-19 5E-19 6E-19 7E-19 8E-19 9E-19 1E-18 2E-18\r\n";
        let rows = parse(&text).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].wavelength_nm, 213.33);
        assert_eq!(rows[0].sigma[column(223.0).unwrap()], 8e-19);
        assert_eq!(rows[0].sigma[column(233.0).unwrap()], 7e-19);
        assert!(
            parse("x\n".repeat(HEADER_LINES).as_str())
                .unwrap()
                .is_empty()
        );
        let short = format!("{}213.33 1E-19\n", "x\n".repeat(HEADER_LINES));
        assert!(parse(&short).is_none());
    }
}
