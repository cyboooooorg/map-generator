//! MT_CKD's CO₂ continuum, binned on the table's bins, and its water
//! continuum, binned on the shortwave table's ([`WaterContinuum`]).
//!
//! MT_CKD's CO₂ continuum (Mlawer et al. 2012, *Phil. Trans. R. Soc. A*
//! 370, 2520) is the absorption of CO₂'s lines beyond the 25 cm⁻¹ cutoff
//! (and under the pedestal, see [`crate::lbl::Pedestal`]), with the
//! sub-Lorentzian far wings and the line coupling that LBLRTM's lines
//! leave to it. Its coefficients live in LBLRTM's `contnm.f90`, which
//! MT_CKD 4.3 takes as a submodule:
//!
//! - `BLOCK DATA BFCO2`: the coefficient `C(ν)` every 2 cm⁻¹ from −4 to
//!   10 000 cm⁻¹, in units of 1e-20 cm² per molecule per cm⁻¹ (the file's
//!   "(CM**3/MOL)*1.E-20");
//! - `XFACCO2`: a factor on 2000-2998 cm⁻¹, every 2 cm⁻¹ (mt_ckd_2.5);
//! - `tdep_bandhead` of `FRNCO2`: exponents `n` on 2386-2434 cm⁻¹ that
//!   scale it as `(T / 246 K)^n` (mt_ckd_2.5).
//!
//! `CONTNM` takes the optical depth of a layer as `C(ν) × 1e-20 × W_CO₂ ×
//! (p / 1013 mbar)(296 K / T) × R(ν, T)`, `W_CO₂` the CO₂ column
//! (molecules/cm²) and `R = ν tanh(c₂ν / 2T)` the radiation term: the form
//! of MT_CKD's foreign water continuum, broadened by the whole air.
//!
//! A bin keeps the mean of `C × XFACCO2` over the 2 cm⁻¹ points inside it,
//! and the mean exponent weighted by the coefficient, which is exact at
//! 246 K.

/// Temperature, K, at which the band-head exponents scale by 1 (the
/// `t_eff` of `FRNCO2`).
pub const T_EFF_K: f64 = 246.0;

/// The CO₂ continuum of `contnm.f90`, every 2 cm⁻¹: its first wavenumber,
/// and per point the coefficient times `XFACCO2` (cm² per molecule per
/// cm⁻¹) and the band-head exponent.
#[derive(Clone, Debug, PartialEq)]
pub struct Co2Continuum {
    first_cm: f64,
    step_cm: f64,
    points: Vec<(f64, f64)>,
}

/// The numbers of a Fortran `DATA` list, `/…/`, that starts at `from` in
/// `text`.
fn data_list(text: &str, from: usize) -> Option<Vec<f64>> {
    let open = from + text[from..].find('/')?;
    let close = open + 1 + text[open + 1..].find('/')?;
    text[open + 1..close]
        .split(|c: char| c == ',' || c.is_whitespace() || c == '&')
        .filter(|s| !s.is_empty())
        .map(|s| s.replace(['d', 'D'], "e").parse::<f64>().ok())
        .collect()
}

impl Co2Continuum {
    /// Reads the CO₂ continuum from the text of `contnm.f90`.
    pub fn parse(text: &str) -> Option<Self> {
        let block = text.find("BLOCK DATA BFCO2")?;
        let end = block + text[block..].find("end block data BFCO2")?;
        let body = &text[block..end];
        let header = body.find("DATA V1,V2,DV,NPT")?;
        let shape = data_list(body, header)?;
        let [first, _, step, count] = shape[..] else {
            return None;
        };
        let mut values = Vec::new();
        let mut at = header + 1;
        while let Some(i) = body[at..].find("DATA F") {
            let start = at + i;
            values.extend(data_list(body, start)?);
            at = start + 6;
        }
        if values.len() != count as usize {
            return None;
        }
        let factors = data_list(text, text.find("DATA XFACCO2/")?)?;
        let bandhead_at = text.find("data (tdep_bandhead(i),i=1196,1220)/")?;
        let exponents = data_list(
            text,
            bandhead_at + "data (tdep_bandhead(i),i=1196,1220)".len(),
        )?;
        if factors.len() != 500 || exponents.len() != 25 {
            return None;
        }
        let points = values
            .iter()
            .enumerate()
            .map(|(n, c)| {
                // `FRNCO2`'s index I runs from 1 at `first`.
                let i = n + 1;
                let nu = first + step * n as f64;
                let factor = if (2000.0..=2998.0).contains(&nu) {
                    // `JFAC = (VJ − 1998)/2`, from 1.
                    factors[((nu - 1998.0) / 2.0 + 1e-5) as usize - 1]
                } else {
                    1.0
                };
                let exponent = if (1196..=1220).contains(&i) {
                    exponents[i - 1196]
                } else {
                    0.0
                };
                (c * 1e-20 * factor, exponent)
            })
            .collect();
        Some(Self {
            first_cm: first,
            step_cm: step,
            points,
        })
    }

    /// The bin from `lo` to `hi` cm⁻¹: the mean coefficient of its points
    /// (cm² per molecule per cm⁻¹), and their exponent weighted by it.
    pub fn bin(&self, lo: f64, hi: f64) -> (f64, f64) {
        let (mut sum, mut weighted, mut n) = (0.0, 0.0, 0);
        for (i, &(c, a)) in self.points.iter().enumerate() {
            let nu = self.first_cm + self.step_cm * i as f64;
            if nu >= lo && nu < hi {
                sum += c;
                weighted += c * a;
                n += 1;
            }
        }
        if n == 0 || sum <= 0.0 {
            return (0.0, 0.0);
        }
        (sum / n as f64, weighted / sum)
    }
}

/// MT_CKD 4.3's water vapour continuum, as `absco-ref_wv-mt-ckd.nc` gives
/// it every 10 cm⁻¹ from 0 to 20 000 cm⁻¹ (Mlawer et al. 2012): the self
/// coefficient at 296 K, its temperature exponent and the foreign
/// coefficient at 296 K, cm² per molecule per cm⁻¹ at 1013 mbar (the
/// file's `self_absco_ref`, `self_texp` and `for_absco_ref`, those
/// `mapgen`'s longwave continuum reads). `mapgen` takes the absorption
/// per molecule as `mt_ckd_h2o_absco` does:
/// `(C_s (296/T)^n e + C_f (p − e)) / p_ref × (296/T) × ν tanh(c₂ν/2T)`.
pub struct WaterContinuum {
    /// `(ν, C_s, n, C_f)` at each point of the file.
    points: Vec<(f64, f64, f64, f64)>,
}

impl WaterContinuum {
    /// Reads the continuum from the variables of the NetCDF file.
    pub fn from_netcdf(nc: &crate::netcdf::NetCdf) -> Option<Self> {
        let nu = nc.variable("wavenumbers")?;
        let own = nc.variable("self_absco_ref")?;
        let exponent = nc.variable("self_texp")?;
        let foreign = nc.variable("for_absco_ref")?;
        if [own.len(), exponent.len(), foreign.len()] != [nu.len(); 3] {
            return None;
        }
        Some(Self {
            points: (0..nu.len())
                .map(|i| (nu[i], own[i], exponent[i], foreign[i]))
                .collect(),
        })
    }

    /// The file's points at each multiple of `step_cm` from 0 to `top_cm`
    /// (point values, not bin means): `(ν, C_s, n, C_f)`. A multiple the
    /// file has no point at is left out.
    pub fn nodes(&self, step_cm: f64, top_cm: f64) -> Vec<(f64, f64, f64, f64)> {
        let count = (top_cm / step_cm).round() as usize;
        (0..=count)
            .filter_map(|i| {
                let nu = i as f64 * step_cm;
                self.points
                    .iter()
                    .find(|p| (p.0 - nu).abs() < 1e-6)
                    .map(|&(_, s, n, f)| (nu, s, n, f))
            })
            .collect()
    }

    /// The bin from `lo` to `hi` cm⁻¹: the mean self coefficient of its
    /// points, their exponent weighted by it, and the mean foreign
    /// coefficient. A continuum is smooth across a bin, so the mean of
    /// its coefficients is the mean of its thin optical depth.
    pub fn bin(&self, lo: f64, hi: f64) -> (f64, f64, f64) {
        let (mut own, mut weighted, mut foreign, mut n) = (0.0, 0.0, 0.0, 0);
        for &(nu, s, a, f) in &self.points {
            if nu >= lo && nu < hi {
                own += s;
                weighted += s * a;
                foreign += f;
                n += 1;
            }
        }
        if n == 0 {
            return (0.0, 0.0, 0.0);
        }
        let exponent = if own > 0.0 { weighted / own } else { 0.0 };
        (own / n as f64, exponent, foreign / n as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `contnm.f90` cut down to its CO₂ parts, with the layout of the
    /// real file: six points from −4 cm⁻¹ instead of 5003.
    fn sample() -> String {
        let mut text = String::from(
            "   data (tdep_bandhead(i),i=1196,1220)/                              &\n",
        );
        let exps: Vec<String> = (0..25)
            .map(|i| format!("{:.2E}", 0.01 * i as f64))
            .collect();
        text += &format!("   &    {}/\n", exps.join(","));
        text += "BLOCK DATA BFCO2\n   DATA V1,V2,DV,NPT / -4.0, 6.0, 2.0, 6/\n";
        text += "   DATA F0000/                                                       &\n";
        text += "   &  1.000e-12,  2.000e-12/\n";
        text += "   DATA F0001/                                                       &\n";
        text += "   &  3.000e-12,  4.000e-12,  5.000e-12,  6.000e-12/\n";
        text += "end block data BFCO2\n";
        let factors: Vec<String> = (0..500)
            .map(|i| format!("{:.4}", 1.0 + 0.001 * i as f64))
            .collect();
        text += &format!(
            "DATA XFACCO2/                                                     &\n   &  {}/\n",
            factors.join(", ")
        );
        text
    }

    #[test]
    fn the_continuum_reads_and_bins_its_points() {
        let c = Co2Continuum::parse(&sample()).expect("parses");
        assert_eq!(c.points.len(), 6);
        // Points at −4, −2, 0, 2, 4, 6 cm⁻¹; the bin [0, 4) holds 0 and 2.
        let (mean, exponent) = c.bin(0.0, 4.0);
        assert!(
            (mean / (0.5 * (3e-32 + 4e-32)) - 1.0).abs() < 1e-12,
            "{mean}"
        );
        assert_eq!(exponent, 0.0);
        assert_eq!(c.bin(100.0, 120.0), (0.0, 0.0));
    }

    #[test]
    fn the_water_continuum_bins_its_points() {
        let w = WaterContinuum {
            points: vec![
                (0.0, 4e-22, 2.0, 1e-23),
                (10.0, 2e-22, 5.0, 3e-23),
                (20.0, 1e-22, 1.0, 5e-23),
            ],
        };
        let (own, exponent, foreign) = w.bin(0.0, 20.0);
        assert!((own / 3e-22 - 1.0).abs() < 1e-12);
        assert!((exponent - 3.0).abs() < 1e-12);
        assert!((foreign / 2e-23 - 1.0).abs() < 1e-12);
        assert_eq!(w.bin(100.0, 120.0), (0.0, 0.0, 0.0));
    }

    #[test]
    fn water_nodes_are_the_files_points() {
        // Points every 10 cm⁻¹ from −10 to 110; nodes every 50 from 0 to 100.
        let w = WaterContinuum {
            points: (-1..=11)
                .map(|i| (i as f64 * 10.0, i as f64, 2.0 * i as f64, 3.0 * i as f64))
                .collect(),
        };
        assert_eq!(
            w.nodes(50.0, 100.0),
            vec![
                (0.0, 0.0, 0.0, 0.0),
                (50.0, 5.0, 10.0, 15.0),
                (100.0, 10.0, 20.0, 30.0)
            ]
        );
    }

    #[test]
    fn the_real_layout_parses() {
        // The first points of the real file, as served (MT_CKD 4.3's
        // LBLRTM): 8.391e-13 and 8.359e-13 at −4 and −2 cm⁻¹.
        let text = "BLOCK DATA BFCO2\n   DATA V1,V2,DV,NPT / -4.0, 0.0, 2.0, 3/\n   DATA F0000/  &\n   &  8.391e-13,  8.359e-13/\n   DATA F0001/  &\n   &  8.345e-13/\nend block data BFCO2\n   DATA XFACCO2/ &\n".to_string()
            + &vec!["1.0"; 500].join(",")
            + "/\n   data (tdep_bandhead(i),i=1196,1220)/ &\n"
            + &vec!["0.5"; 25].join(",")
            + "/\n";
        let c = Co2Continuum::parse(&text).expect("parses");
        assert_eq!(c.points[0], (8.391e-33, 0.0));
        assert_eq!(c.points[2], (8.345e-33, 0.0));
    }
}
