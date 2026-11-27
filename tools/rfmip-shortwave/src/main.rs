//! Builds `crates/science/data/rfmip-profiles.tsv` from the RFMIP-IRF
//! profiles that `fetch.sh` downloads (see README.md): the field the
//! science crate's shortwave is tested on; and
//! `crates/science/data/rfmip-longwave.tsv`, the field of all 100 sites
//! its infrared is tested on (methane's step, issue #704). From the clear-sky shortwave
//! fluxes of LBLRTM and RTE-RRTMGP it prints the three weighted
//! differences the tests quote (`science::radiation::rfmip_tests`), and
//! writes nothing of them: those files' licence (CC BY-NC-SA 4.0) keeps
//! their derived data out of the repository.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{env, fs, process};

/// Definition: the experiments of RFMIP-IRF the tests take (indices of
/// `expt` in the RFMIP files, named by their `expt_label`): present day,
/// 0.5 and 8 times its CO₂, pre-industrial CO₂, +4 K, and +4 K at
/// constant relative humidity.
const EXPERIMENTS: [usize; 6] = [0, 4, 7, 8, 13, 14];

/// Definition: the differences the tests quote, as (from, to)
/// experiments: pre-industrial to present-day CO₂, 0.5 to 8 times
/// present-day CO₂, +4 K to +4 K at constant relative humidity.
const DIFFERENCES: [(&str, usize, usize); 3] = [
    ("pre-industrial to present-day CO2", 8, 0),
    ("0.5x to 8x CO2", 4, 7),
    ("+4 K to +4 K at constant RH", 13, 14),
];

/// Definition: the experiments whose profiles the table holds: present
/// day (its temperatures and water serve the CO₂ experiments, which
/// change only the CO₂), +4 K and +4 K at constant relative humidity.
const PRESENT_DAY: usize = 0;
const WARMER: usize = 13;
const WARMER_MOISTER: usize = 14;

/// The table's header: what it holds, its licence (the profiles' only,
/// not the code's), its attribution and the changes made.
const HEADER: &str = "\
# RFMIP-IRF clear-sky field: the atmospheric conditions of RFMIP's rad-irf
# experiment, built by tools/rfmip-shortwave (map-generator).
# Attribution: Pincus, R., Forster, P. M., Stevens, B. 2016, The Radiative
# Forcing Model Intercomparison Project (RFMIP), Geosci. Model Dev. 9, 3447,
# doi:10.5194/gmd-9-3447-2016; dataset input4MIPs.CMIP6.RFMIP.UColorado.
# UColorado-RFMIP-1-2 (v20190401, hdl:21.14100/f379c294-f7bb-442d-bca7-64661d60780e),
# University of Colorado.
# Licence of this file (its data only, not the repository's code): the source
# file says its data are licensed under a Creative Commons Attribution \"Share
# Alike\" 4.0 International License, while its link points to CC BY 4.0; the
# stricter reading is taken: CC BY-SA 4.0,
# https://creativecommons.org/licenses/by-sa/4.0/. No warranty, as the source
# states.
# Changes made: the 51 sunlit sites of 100 only; the sunlight at the top made
# from total_solar_irradiance and solar_zenith_angle; weights renormalised over
# all 100 sites; water vapour moved from the layers to the levels (linear in
# ln p); rounded. Details: crates/science/data/RFMIP.md.
# X expt co2_ppmv (carbon_dioxide_GM)
# S site weight insolation_w_m2 surface_albedo (sunlit sites; weights over all)
# L p_pa t_k x_h2o t_k(+4K) x_h2o(+4K) x_h2o(+4K const RH), surface first
";

/// Definition: the experiments the longwave field holds the gases of:
/// present day, pre-industrial methane (`expt_label` "PI CH4") and
/// pre-industrial N₂O ("PI N2O"), whose temperatures, water and ozone are
/// present day's.
const LONGWAVE_EXPERIMENTS: [usize; 3] = [0, 9, 10];

/// The longwave field's header, as [`HEADER`]'s terms.
const LONGWAVE_HEADER: &str = "\
# RFMIP-IRF clear-sky longwave field: the atmospheric conditions of RFMIP's
# rad-irf experiment, built by tools/rfmip-shortwave (map-generator).
# Attribution: Pincus, R., Forster, P. M., Stevens, B. 2016, The Radiative
# Forcing Model Intercomparison Project (RFMIP), Geosci. Model Dev. 9, 3447,
# doi:10.5194/gmd-9-3447-2016; dataset input4MIPs.CMIP6.RFMIP.UColorado.
# UColorado-RFMIP-1-2 (v20190401, hdl:21.14100/f379c294-f7bb-442d-bca7-64661d60780e),
# University of Colorado.
# Licence of this file (its data only, not the repository's code): the source
# file says its data are licensed under a Creative Commons Attribution \"Share
# Alike\" 4.0 International License, while its link points to CC BY 4.0; the
# stricter reading is taken: CC BY-SA 4.0,
# https://creativecommons.org/licenses/by-sa/4.0/. No warranty, as the source
# states.
# Changes made: present day's profiles only (the methane and N2O experiments
# change only that gas); weights renormalised over the 100 sites; water vapour and
# ozone moved from the layers to the levels (linear in ln p); rounded.
# Details: crates/science/data/RFMIP.md.
# X expt co2_ppmv ch4_ppbv n2 o2 n2o_ppbv (the *_GM mole fractions)
# S site weight surface_k surface_emissivity
# L p_pa t_k x_h2o x_o3, surface first
";

/// The text view of one OPeNDAP variable: its dimensions and its values,
/// row-major.
struct Variable {
    dims: Vec<usize>,
    values: Vec<f64>,
}

impl Variable {
    fn at(&self, index: &[usize]) -> f64 {
        let mut flat = 0;
        for (i, d) in index.iter().zip(&self.dims) {
            assert!(i < d, "index {index:?} outside {:?}", self.dims);
            flat = flat * d + i;
        }
        self.values[flat]
    }
}

/// Parses an OPeNDAP ASCII response (`.ascii?` view of a THREDDS
/// dataset): after the dashed line, each variable is a header
/// `name[d1][d2]…` followed by rows of comma-separated values, each
/// optionally led by its indices, until a blank line.
fn parse(text: &str) -> Result<BTreeMap<String, Variable>, String> {
    let body = text
        .split_once("\n---")
        .and_then(|(_, rest)| rest.split_once('\n'))
        .ok_or("no data section")?
        .1;
    let mut vars = BTreeMap::new();
    let mut current: Option<(String, Variable)> = None;
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            if let Some((name, var)) = current.take() {
                vars.insert(name, var);
            }
            continue;
        }
        if current.is_none() {
            let (name, dims) = line.split_once('[').ok_or(format!("bad header: {line}"))?;
            let dims = format!("[{dims}")
                .split(['[', ']'])
                .filter(|s| !s.is_empty())
                .map(|s| s.parse::<usize>().map_err(|e| format!("{line}: {e}")))
                .collect::<Result<Vec<_>, _>>()?;
            current = Some((
                name.to_string(),
                Variable {
                    dims,
                    values: Vec::new(),
                },
            ));
            continue;
        }
        let (_, var) = current.as_mut().expect("a variable is open");
        let data = if line.starts_with('[') {
            line.split_once(", ").map_or("", |(_, rest)| rest)
        } else {
            line
        };
        for field in data.split(',') {
            let v: f64 = field.trim().parse().map_err(|e| format!("{field}: {e}"))?;
            var.values.push(v);
        }
    }
    if let Some((name, var)) = current.take() {
        vars.insert(name, var);
    }
    for (name, var) in &vars {
        let n: usize = var.dims.iter().product();
        if n != var.values.len() {
            return Err(format!(
                "{name}: {} values for {:?}",
                var.values.len(),
                var.dims
            ));
        }
    }
    Ok(vars)
}

fn read(raw: &Path, name: &str) -> Result<BTreeMap<String, Variable>, String> {
    let path = raw.join(name);
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The weighted mean over the sites of the sunlight the atmosphere
/// absorbs in experiment `e`, W/m²: what enters at the top less what
/// leaves it, less what the surface absorbs.
fn absorbed(
    rsd: &Variable,
    rsu: &Variable,
    weights: &[f64],
    top: usize,
    surface: usize,
    e: usize,
) -> f64 {
    let total: f64 = weights.iter().sum();
    weights
        .iter()
        .enumerate()
        .map(|(s, w)| {
            let air = rsd.at(&[e, s, top]) - rsu.at(&[e, s, top]) - rsd.at(&[e, s, surface])
                + rsu.at(&[e, s, surface]);
            w / total * air
        })
        .sum()
}

/// `y` at `x` linearly between the points `(xs, ys)` (`xs` increasing),
/// held at the end values outside them.
fn interpolate(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    if x <= xs[0] {
        return ys[0];
    }
    for k in 1..xs.len() {
        if x <= xs[k] {
            let f = (x - xs[k - 1]) / (xs[k] - xs[k - 1]);
            return ys[k - 1] + f * (ys[k] - ys[k - 1]);
        }
    }
    ys[ys.len() - 1]
}

fn run(raw: &Path, out: &Path, longwave_out: &Path, fluxes: bool) -> Result<(), String> {
    let input = read(
        raw,
        "multiple_input4MIPs_radiation_RFMIP_UColorado-RFMIP-1-2_none.txt",
    )?;
    let get = |name: &str| input.get(name).ok_or(format!("input: no {name}"));
    let (weight, zenith, tsi, albedo) = (
        get("profile_weight")?,
        get("solar_zenith_angle")?,
        get("total_solar_irradiance")?,
        get("surface_albedo")?,
    );
    let (p_level, p_layer, t_level, water, co2) = (
        get("pres_level")?,
        get("pres_layer")?,
        get("temp_level")?,
        get("water_vapor")?,
        get("carbon_dioxide_GM")?,
    );
    let sites = weight.values.len();
    let levels = p_level.dims[1];
    let layers = p_layer.dims[1];
    let weights = &weight.values;
    let total: f64 = weights.iter().sum();
    // The top is the level of least pressure, the surface the greatest
    // (the same for every site).
    let column: Vec<f64> = (0..levels).map(|k| p_level.at(&[0, k])).collect();
    let top = (0..levels)
        .min_by(|a, b| column[*a].total_cmp(&column[*b]))
        .ok_or("no level")?;
    let surface = (0..levels)
        .max_by(|a, b| column[*a].total_cmp(&column[*b]))
        .ok_or("no level")?;

    let mut text = String::new();
    text.push_str(HEADER);
    for e in EXPERIMENTS {
        text.push_str(&format!("X\t{e}\t{}\n", co2.values[e]));
    }
    if fluxes {
        let read_model = |m: &str| -> Result<[BTreeMap<String, Variable>; 2], String> {
            Ok([
                read(raw, &format!("rsd_Efx_{m}_rad-irf_r1i1p1f1_gn.txt"))?,
                read(raw, &format!("rsu_Efx_{m}_rad-irf_r1i1p1f1_gn.txt"))?,
            ])
        };
        println!("weighted absorbed sunlight, W/m² (not written to the table):");
        for m in ["LBLRTM-12-8", "RTE-RRTMGP-181204"] {
            let [d, u] = read_model(m)?;
            let (rsd, rsu) = (d.get("rsd").ok_or("no rsd")?, u.get("rsu").ok_or("no rsu")?);
            for (name, from, to) in DIFFERENCES {
                let change = absorbed(rsd, rsu, weights, top, surface, to)
                    - absorbed(rsd, rsu, weights, top, surface, from);
                println!("  {m}, {name}: {change:.4}");
            }
        }
        println!("weighted clear-sky outgoing longwave at the top, W/m²:");
        for m in ["LBLRTM-12-8", "RTE-RRTMGP-181204"] {
            let up = read(raw, &format!("rlu_Efx_{m}_rad-irf_r1i1p1f1_gn.txt"))?;
            let rlu = up.get("rlu").ok_or("no rlu")?;
            // Present day (0) and "PI HCs" (12), present day with the
            // halocarbons at their pre-industrial amounts.
            for (name, e) in [("present day", 0), ("PI HCs", 12)] {
                let olr: f64 = weights
                    .iter()
                    .enumerate()
                    .map(|(s, w)| w * rlu.at(&[e, s, top]))
                    .sum::<f64>()
                    / total;
                println!("  {m}, {name}: {olr:.4}");
            }
        }
    }
    let mut sunlit = 0;
    for (s, w) in weights.iter().enumerate() {
        let mu = zenith.values[s].to_radians().cos();
        if mu <= 0.0 {
            continue;
        }
        sunlit += 1;
        text.push_str(&format!(
            "S\t{s}\t{:.8}\t{:.4}\t{:.6}\n",
            w / total,
            tsi.values[s] * mu,
            albedo.values[s]
        ));
        let ln_layer: Vec<f64> = (0..layers).map(|k| p_layer.at(&[s, k]).ln()).collect();
        let mut order: Vec<usize> = (0..layers).collect();
        order.sort_by(|a, b| ln_layer[*a].total_cmp(&ln_layer[*b]));
        let xs: Vec<f64> = order.iter().map(|k| ln_layer[*k]).collect();
        let water_at = |e: usize, p: f64| -> f64 {
            let ys: Vec<f64> = order.iter().map(|k| water.at(&[e, s, *k])).collect();
            interpolate(&xs, &ys, p.ln())
        };
        let mut rows: Vec<usize> = (0..levels).collect();
        rows.sort_by(|a, b| p_level.at(&[s, *b]).total_cmp(&p_level.at(&[s, *a])));
        for k in rows {
            let p = p_level.at(&[s, k]);
            text.push_str(&format!(
                "L\t{p}\t{}\t{:.6e}\t{}\t{:.6e}\t{:.6e}\n",
                t_level.at(&[PRESENT_DAY, s, k]),
                water_at(PRESENT_DAY, p),
                t_level.at(&[WARMER, s, k]),
                water_at(WARMER, p),
                water_at(WARMER_MOISTER, p),
            ));
        }
    }
    fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{}: {sunlit} sunlit sites of {sites}", out.display());

    let extra = read(
        raw,
        "multiple_input4MIPs_radiation_RFMIP_UColorado-RFMIP-1-2_none-longwave.txt",
    )?;
    let more = |name: &str| extra.get(name).ok_or(format!("longwave input: no {name}"));
    let (ozone, surface_t, emissivity) = (
        more("ozone")?,
        more("surface_temperature")?,
        more("surface_emissivity")?,
    );
    let (methane, nitrogen, oxygen, nitrous) = (
        more("methane_GM")?,
        more("nitrogen_GM")?,
        more("oxygen_GM")?,
        more("nitrous_oxide_GM")?,
    );
    let mut text = String::new();
    text.push_str(LONGWAVE_HEADER);
    for e in LONGWAVE_EXPERIMENTS {
        text.push_str(&format!(
            "X\t{e}\t{}\t{}\t{}\t{}\t{}\n",
            co2.values[e],
            methane.values[e],
            nitrogen.values[e],
            oxygen.values[e],
            nitrous.values[e]
        ));
    }
    for (s, w) in weights.iter().enumerate() {
        text.push_str(&format!(
            "S\t{s}\t{:.8}\t{}\t{}\n",
            w / total,
            surface_t.at(&[PRESENT_DAY, s]),
            emissivity.values[s]
        ));
        let ln_layer: Vec<f64> = (0..layers).map(|k| p_layer.at(&[s, k]).ln()).collect();
        let mut order: Vec<usize> = (0..layers).collect();
        order.sort_by(|a, b| ln_layer[*a].total_cmp(&ln_layer[*b]));
        let xs: Vec<f64> = order.iter().map(|k| ln_layer[*k]).collect();
        let at_level = |field: &Variable, p: f64| -> f64 {
            let ys: Vec<f64> = order
                .iter()
                .map(|k| field.at(&[PRESENT_DAY, s, *k]))
                .collect();
            interpolate(&xs, &ys, p.ln())
        };
        let mut rows: Vec<usize> = (0..levels).collect();
        rows.sort_by(|a, b| p_level.at(&[s, *b]).total_cmp(&p_level.at(&[s, *a])));
        for k in rows {
            let p = p_level.at(&[s, k]);
            text.push_str(&format!(
                "L\t{p}\t{}\t{:.6e}\t{:.6e}\n",
                t_level.at(&[PRESENT_DAY, s, k]),
                at_level(water, p),
                at_level(ozone, p),
            ));
        }
    }
    fs::write(longwave_out, text).map_err(|e| format!("{}: {e}", longwave_out.display()))?;
    println!("{}: {sites} sites", longwave_out.display());
    Ok(())
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut raw = root.join("raw");
    let mut out = root.join("../../crates/science/data/rfmip-profiles.tsv");
    let mut longwave_out = root.join("../../crates/science/data/rfmip-longwave.tsv");
    let mut fluxes = true;
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--raw" => raw = PathBuf::from(args.next().unwrap_or_default()),
            "--out" => out = PathBuf::from(args.next().unwrap_or_default()),
            "--longwave-out" => longwave_out = PathBuf::from(args.next().unwrap_or_default()),
            "--no-fluxes" => fluxes = false,
            _ => {
                eprintln!(
                    "usage: rfmip-shortwave [--raw DIR] [--out FILE] [--longwave-out FILE] [--no-fluxes]"
                );
                process::exit(2);
            }
        }
    }
    if let Err(e) = run(&raw, &out, &longwave_out, fluxes) {
        eprintln!("rfmip-shortwave: {e}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_view_parses_with_its_indices() {
        let text = "Dataset {\n    Float32 a[x = 2][y = 3];\n} d;\n\
                    ---------------------------------------------\n\
                    a[2][3]\n[0], 1.0, 2.0, 3.0\n[1], 4.0, 5.0, 6E-1\n\n\
                    b[2]\n7.5, 8.0\n";
        let vars = parse(text).unwrap();
        assert_eq!(vars["a"].dims, vec![2, 3]);
        assert_eq!(vars["a"].at(&[1, 2]), 0.6);
        assert_eq!(vars["b"].values, vec![7.5, 8.0]);
    }

    #[test]
    fn a_short_variable_is_refused() {
        let text = "x\n---\na[3]\n1.0, 2.0\n";
        assert!(parse(text).is_err());
    }

    #[test]
    fn interpolation_holds_its_ends() {
        let (xs, ys) = ([0.0, 1.0, 2.0], [10.0, 20.0, 40.0]);
        assert_eq!(interpolate(&xs, &ys, -1.0), 10.0);
        assert_eq!(interpolate(&xs, &ys, 1.5), 30.0);
        assert_eq!(interpolate(&xs, &ys, 3.0), 40.0);
    }
}
