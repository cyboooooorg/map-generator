//! Builds mapgen's correlated-k opacity table from HITRAN line lists (see
//! README.md): for each absorber set, pressure and temperature of the
//! grid, the line-by-line absorption of every spectral bin, sorted and
//! reduced to g-points.
//!
//! ```text
//! opacity-table [--shortwave] [--raw DIR] [--out FILE] [--bin-width CM]
//!               [--top CM] [--temperatures K,K,…]
//!               [--pressure-step DECADES] [--threads N]
//! ```
//!
//! `--shortwave` builds the shortwave table instead (`shortwave.bin`):
//! the bands of the albedo's stellar spectrum from 0.7 to 5 µm
//! ([`shortwave_edges`]), with MT_CKD's water continuum binned in.

mod continuum;
mod hitran;
mod kdist;
mod lbl;
mod netcdf;
mod table;
mod voigt;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hitran::{Line, PartitionSum, T_REF, parse_line, strength_at};
use kdist::{G, reduce};
use lbl::{Cutoff, Pedestal, Shape, bin_spectrum};
use table::{Header, SETS, Set, encode};

/// Avogadro constant, 1/mol (CODATA 2018, exact).
const AVOGADRO: f64 = 6.022_140_76e23;
/// Boltzmann constant, J/K (CODATA 2018, exact).
const BOLTZMANN: f64 = 1.380_649e-23;
/// Speed of light, m/s.
const LIGHT: f64 = 299_792_458.0;
/// Bar per standard atmosphere (HITRAN's widths are per atm).
const BAR_PER_ATM: f64 = 1.013_25;

/// Distance from a line's centre beyond which it adds nothing, cm⁻¹: 25,
/// MT_CKD's convention (Mlawer et al. 2012), for both gases; beyond it
/// their continua take over (water's in mapgen, CO₂'s in the table, see
/// [`continuum`]).
const CUTOFF_CM: f64 = 25.0;

/// Most samples of one bin (see [`lbl::sample_step`]): 2e6, a step of
/// 1e-5 cm⁻¹ on 20 cm⁻¹ bins. Only water's lines under about 30 cm⁻¹
/// at the grid's lowest pressures and temperatures are narrower.
const MAX_SAMPLES: usize = 2_000_000;

/// An isotopologue the table takes: HITRAN molecule and local number,
/// global number (the partition-sum file), molar mass in kg/mol (HITRAN's
/// isotopologue table, hitran.org/docs/iso-meta).
struct Isotopologue {
    molecule: u8,
    local: u8,
    global: u32,
    molar_kg: f64,
}

/// The main isotopologues of H₂O (161, 181, 171, 162) and CO₂ (626, 636,
/// 628, 627), each over 1e-4 of its gas.
const ISOTOPOLOGUES: [Isotopologue; 8] = [
    Isotopologue {
        molecule: 1,
        local: 1,
        global: 1,
        molar_kg: 18.010_565e-3,
    },
    Isotopologue {
        molecule: 1,
        local: 2,
        global: 2,
        molar_kg: 20.014_811e-3,
    },
    Isotopologue {
        molecule: 1,
        local: 3,
        global: 3,
        molar_kg: 19.014_780e-3,
    },
    Isotopologue {
        molecule: 1,
        local: 4,
        global: 4,
        molar_kg: 19.016_740e-3,
    },
    Isotopologue {
        molecule: 2,
        local: 1,
        global: 7,
        molar_kg: 43.989_830e-3,
    },
    Isotopologue {
        molecule: 2,
        local: 2,
        global: 8,
        molar_kg: 44.993_185e-3,
    },
    Isotopologue {
        molecule: 2,
        local: 3,
        global: 9,
        molar_kg: 45.994_076e-3,
    },
    Isotopologue {
        molecule: 2,
        local: 4,
        global: 10,
        molar_kg: 44.994_045e-3,
    },
];

/// One gas: its lines (sorted by wavenumber), the partition sum of each of
/// its isotopologues, and its mean molar mass.
struct Gas {
    lines: Vec<Line>,
    /// `(local number, molar mass, Q)` of each isotopologue.
    isotopologues: Vec<(u8, f64, PartitionSum)>,
    /// Mean molar mass of the gas at natural abundance, kg/mol (IUPAC
    /// standard atomic weights, as mapgen's).
    molar_kg: f64,
    /// What the lines lose under the cutoff (MT_CKD's convention for the
    /// gas, see [`lbl::Pedestal`]).
    pedestal: Pedestal,
}

struct Options {
    /// Whether to build the shortwave table ([`shortwave_edges`]).
    shortwave: bool,
    raw: PathBuf,
    out: PathBuf,
    bin_width_cm: f64,
    top_cm: f64,
    temperatures_k: Vec<f64>,
    pressure_step: f64,
    threads: usize,
}

fn options() -> Result<Options, String> {
    let shortwave = std::env::args().any(|a| a == "--shortwave");
    let mut o = Options {
        shortwave,
        raw: PathBuf::from("tools/opacity-table/raw"),
        out: PathBuf::from(if shortwave {
            "crates/science/data/shortwave.bin"
        } else {
            "crates/science/data/opacity.bin"
        }),
        bin_width_cm: 20.0,
        top_cm: if shortwave {
            *shortwave_edges().last().expect("edges")
        } else {
            5000.0
        },
        temperatures_k: if shortwave {
            table::SHORTWAVE_TEMPERATURES.to_vec()
        } else {
            table::TEMPERATURES.to_vec()
        },
        pressure_step: table::PRESSURE_STEP,
        threads: 4,
    };
    let mut args = std::env::args().skip(1).filter(|a| a != "--shortwave");
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(format!("{flag} needs a value"))?;
        let number = |v: &str| v.parse::<f64>().map_err(|e| format!("{v}: {e}"));
        match flag.as_str() {
            "--raw" => o.raw = value.into(),
            "--out" => o.out = value.into(),
            "--bin-width" => o.bin_width_cm = number(&value)?,
            "--top" => o.top_cm = number(&value)?,
            "--pressure-step" => o.pressure_step = number(&value)?,
            "--threads" => o.threads = value.parse().map_err(|e| format!("{value}: {e}"))?,
            "--temperatures" => {
                o.temperatures_k = value.split(',').map(number).collect::<Result<_, _>>()?
            }
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    Ok(o)
}

/// Shortest and longest wavelengths of the albedo's stellar spectrum, µm,
/// and its number of bands, even in `ln λ` (`mapgen`'s
/// `planet::albedo::spectrum`, which checks the table's edges against
/// its own).
const SPECTRUM_UM: (f64, f64, usize) = (0.2, 5.0, 240);

/// Shortest wavelength, µm, of the shortwave table: the bands of the
/// spectrum from the one that holds 0.7 µm up, where CO₂ and water
/// absorb the star's near infrared (water's weakest band, near 0.72 µm,
/// and everything redder).
const SHORTWAVE_FROM_UM: f64 = 0.7;

/// Edges, cm⁻¹, ascending, of the shortwave table's bins: the bands of
/// the albedo's spectrum ([`SPECTRUM_UM`]) from the one holding
/// [`SHORTWAVE_FROM_UM`] to 5 µm, 147 bands over 2 000-14 388 cm⁻¹.
fn shortwave_edges() -> Vec<f64> {
    let (from, to, n) = SPECTRUM_UM;
    let step = (to / from).ln() / n as f64;
    let first = ((SHORTWAVE_FROM_UM / from).ln() / step).floor() as usize;
    (first..=n)
        .rev()
        .map(|e| 1e4 / (from * (step * e as f64).exp()))
        .collect()
}

/// Reads the lines of `molecule` from every `.par` file of `dir` whose
/// name starts with `prefix`, keeping the isotopologues of
/// [`ISOTOPOLOGUES`] from `bottom_cm` − the cutoff to `top_cm` + the
/// cutoff.
fn read_lines(
    dir: &Path,
    prefix: &str,
    molecule: u8,
    (bottom_cm, top_cm): (f64, f64),
) -> Result<Vec<Line>, String> {
    let mut lines = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.starts_with(prefix) || !name.ends_with(".par") {
            continue;
        }
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        for record in text.lines().filter(|r| !r.trim().is_empty()) {
            let line =
                parse_line(record).ok_or(format!("{}: bad record {record}", path.display()))?;
            let known = ISOTOPOLOGUES
                .iter()
                .any(|i| i.molecule == line.molecule && i.local == line.isotopologue);
            let inside =
                line.wavenumber >= bottom_cm - CUTOFF_CM && line.wavenumber <= top_cm + CUTOFF_CM;
            if line.molecule == molecule && known && inside {
                lines.push(line);
            }
        }
    }
    lines.sort_by(|a, b| a.wavenumber.total_cmp(&b.wavenumber));
    // The download's 250 cm⁻¹ pieces share their edges.
    lines.dedup();
    Ok(lines)
}

fn read_gas(
    o: &Options,
    prefix: &str,
    molecule: u8,
    molar_kg: f64,
    pedestal: Pedestal,
) -> Result<Gas, String> {
    let bottom_cm = if o.shortwave {
        shortwave_edges()[0]
    } else {
        0.0
    };
    let lines = read_lines(
        &o.raw.join("lines"),
        prefix,
        molecule,
        (bottom_cm, o.top_cm),
    )?;
    let mut isotopologues = Vec::new();
    for i in ISOTOPOLOGUES.iter().filter(|i| i.molecule == molecule) {
        let path = o.raw.join("q").join(format!("q{}.txt", i.global));
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let q = PartitionSum::parse(&text).ok_or(format!("{}: unreadable", path.display()))?;
        isotopologues.push((i.local, i.molar_kg, q));
    }
    Ok(Gas {
        lines,
        isotopologues,
        molar_kg,
        pedestal,
    })
}

/// The lines of `gas` at `p_bar` and `t_k`, broadened by air or by the
/// gas itself (HITRAN's `γ_air` or `γ_self`, both with its `n_air`, the
/// only exponent a `.par` record carries), sorted by their shifted
/// centres.
fn shapes(gas: &Gas, broadening: Set, p_bar: f64, t_k: f64) -> Vec<Shape> {
    let p_atm = p_bar / BAR_PER_ATM;
    // Per isotopologue: local number, Doppler 1/e width per cm⁻¹ of
    // wavenumber, Q(T_ref), Q(T).
    let per_isotopologue: Vec<(u8, f64, f64, f64)> = gas
        .isotopologues
        .iter()
        .map(|(local, molar, q)| {
            let doppler = (2.0 * BOLTZMANN * AVOGADRO * t_k / molar).sqrt() / LIGHT;
            (*local, doppler, q.at(T_REF), q.at(t_k))
        })
        .collect();
    let mut out: Vec<Shape> = gas
        .lines
        .iter()
        .filter_map(|line| {
            let &(_, doppler, q_ref, q_t) =
                per_isotopologue.iter().find(|i| i.0 == line.isotopologue)?;
            let gamma = if matches!(broadening, Set::WaterSelf | Set::Co2Self) {
                line.gamma_self
            } else {
                line.gamma_air
            };
            let centre = line.wavenumber + line.delta_air * p_atm;
            Some(Shape {
                centre,
                strength: strength_at(line, t_k, q_ref, q_t),
                doppler_e: doppler * centre.max(1e-3),
                lorentz: (T_REF / t_k).powf(line.n_air) * gamma * p_atm,
            })
        })
        .collect();
    out.sort_by(|a, b| a.centre.total_cmp(&b.centre));
    out
}

/// The codes of one `(set, pressure, temperature)`: every bin's g-point
/// coefficients, m²/kg.
fn job(gas: &Gas, set: Set, header: &Header, p_bar: f64, t_k: f64) -> Vec<u16> {
    let lines = shapes(gas, set, p_bar, t_k);
    let cutoff = Cutoff {
        distance_cm: CUTOFF_CM,
        pedestal: gas.pedestal,
    };
    let to_mass = 1e-4 * AVOGADRO / gas.molar_kg;
    let mut codes = Vec::with_capacity(header.bins * G);
    for b in 0..header.bins {
        let (lo, hi) = header.bin(b);
        let first = lines.partition_point(|l| l.centre < lo - CUTOFF_CM);
        let last = lines.partition_point(|l| l.centre <= hi + CUTOFF_CM);
        let near = &lines[first..last];
        let mut k = if near.is_empty() {
            vec![0.0]
        } else {
            bin_spectrum(near, lo, hi, cutoff, MAX_SAMPLES)
        };
        let g = reduce(&mut k);
        codes.extend(g.iter().map(|k| encode(k * to_mass)));
    }
    codes
}

fn main() {
    if let Err(e) = run() {
        eprintln!("opacity-table: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let o = options()?;
    let path = o.raw.join("contnm.f90");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let continuum = continuum::Co2Continuum::parse(&text)
        .ok_or(format!("{}: no CO2 continuum", path.display()))?;
    let edges_cm = o.shortwave.then(shortwave_edges);
    let bins = match &edges_cm {
        Some(edges) => edges.len() - 1,
        None => (o.top_cm / o.bin_width_cm).round() as usize,
    };
    let mut header = Header {
        bins,
        bin_width_cm: if o.shortwave { 0.0 } else { o.bin_width_cm },
        edges_cm,
        pressures_bar: table::pressures(o.pressure_step),
        temperatures_k: o.temperatures_k.clone(),
        co2_continuum: Vec::new(),
        water_continuum: Vec::new(),
    };
    header.co2_continuum = (0..bins)
        .map(|b| {
            let (lo, hi) = header.bin(b);
            continuum.bin(lo, hi)
        })
        .collect();
    if o.shortwave {
        let path = o.raw.join("absco-ref_wv-mt-ckd.nc");
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let water = netcdf::NetCdf::parse(&bytes)
            .and_then(|nc| continuum::WaterContinuum::from_netcdf(&nc))
            .ok_or(format!("{}: no water continuum", path.display()))?;
        header.water_continuum = (0..bins)
            .map(|b| {
                let (lo, hi) = header.bin(b);
                water.bin(lo, hi)
            })
            .collect();
    }
    let water = read_gas(&o, "h2o_", 1, 18.015e-3, Pedestal::Flat)?;
    let co2 = read_gas(&o, "co2_", 2, 44.009e-3, Pedestal::Quadratic)?;
    eprintln!(
        "{} H2O lines, {} CO2 lines; {} bins over {:?} cm-1, {} pressures, {} temperatures",
        water.lines.len(),
        co2.lines.len(),
        header.bins,
        (header.bin(0).0, header.bin(header.bins - 1).1),
        header.pressures_bar.len(),
        header.temperatures_k.len()
    );
    let (n_p, n_t) = (header.pressures_bar.len(), header.temperatures_k.len());
    let jobs: Vec<(usize, usize, usize)> = (0..SETS.len())
        .flat_map(|s| (0..n_p).flat_map(move |p| (0..n_t).map(move |t| (s, p, t))))
        .collect();
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let start = std::time::Instant::now();
    let results: Vec<Vec<(usize, Vec<u16>)>> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..o.threads.max(1))
            .map(|_| {
                scope.spawn(|| {
                    let mut mine = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&(s, p, t)) = jobs.get(i) else {
                            break;
                        };
                        let set = SETS[s];
                        let gas = if matches!(set, Set::Co2 | Set::Co2Self) {
                            &co2
                        } else {
                            &water
                        };
                        let (p_bar, t_k) = (header.pressures_bar[p], header.temperatures_k[t]);
                        let codes = job(gas, set, &header, p_bar, t_k);
                        mine.push((header.offset(s, p, t), codes));
                        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                        eprintln!(
                            "[{n}/{}] {set:?} {p_bar:.0e} bar {t_k} K ({:.0} s)",
                            jobs.len(),
                            start.elapsed().as_secs_f64()
                        );
                    }
                    mine
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().expect("worker"))
            .collect()
    });
    let mut codes = vec![0u16; header.codes()];
    for (offset, c) in results.into_iter().flatten() {
        codes[offset..offset + c.len()].copy_from_slice(&c);
    }
    let bytes = table::write(&header, &codes);
    if let Some(dir) = o.out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&o.out, &bytes).map_err(|e| format!("{}: {e}", o.out.display()))?;
    eprintln!("wrote {} ({} bytes)", o.out.display(), bytes.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shortwave_bins_are_the_spectrum_bands_from_0_7_um() {
        let edges = shortwave_edges();
        assert_eq!(edges.len(), 148);
        assert!((edges[0] - 2000.0).abs() < 1e-9, "{}", edges[0]);
        let last = edges[147];
        assert!((14_300.0..14_400.0).contains(&last), "{last}");
        assert!(edges.windows(2).all(|w| w[1] > w[0]));
        // Even in ln λ: each band spans ln(25)/240.
        for w in edges.windows(2) {
            assert!(((w[1] / w[0]).ln() - 25f64.ln() / 240.0).abs() < 1e-12);
        }
    }
}
