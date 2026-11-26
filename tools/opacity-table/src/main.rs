//! Builds science's correlated-k opacity table from HITRAN line lists (see
//! README.md): for each absorber set, pressure and temperature of the
//! grid, the line-by-line absorption of every spectral bin, sorted and
//! reduced to g-points.
//!
//! ```text
//! opacity-table [--shortwave | --continua | --ozone] [--raw DIR] [--out FILE] [--bin-width CM]
//!               [--top CM] [--temperatures K,K,…]
//!               [--pressure-step DECADES] [--threads N]
//! ```
//!
//! `--shortwave` builds the shortwave table instead (`shortwave.bin`):
//! the bands of the albedo's stellar spectrum from 0.7 to 5 µm
//! ([`shortwave_edges`]), with MT_CKD's water continuum binned in.
//!
//! `--continua` writes, instead of a table, the three text files the
//! spectrum reads: `mt-ckd-water.tsv` (MT_CKD's water continuum at every
//! 50 cm⁻¹), `co2-cia.tsv` (HITRAN's CO₂–CO₂ collision-induced
//! absorption on 20 cm⁻¹ cells) and `pairs-cia.tsv` (HITRAN's N₂–N₂,
//! O₂–O₂, O₂–N₂, N₂–CH₄ and CH₄–CH₄ pairs on the same cells), into
//! `crates/science/data/` (or the directory of `--out`).
//!
//! `--ozone` writes, instead of a table, `ozone.tsv`: ozone's
//! cross-sections of Serdyuchenko et al. (2014) at 223 and 233 K reduced
//! to g-points on every band of the albedo's spectrum that they cover
//! ([`ozone`]).

mod cia;
mod continuum;
mod hitran;
mod kdist;
mod lbl;
mod netcdf;
mod ozone;
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
/// their continua take over (water's in science, CO₂'s in the table, see
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
    /// standard atomic weights, as science's).
    molar_kg: f64,
    /// What the lines lose under the cutoff (MT_CKD's convention for the
    /// gas, see [`lbl::Pedestal`]).
    pedestal: Pedestal,
}

struct Options {
    /// Whether to build the shortwave table ([`shortwave_edges`]).
    shortwave: bool,
    /// Whether to write the continuum text files instead of a table.
    continua: bool,
    /// Whether to write the ozone cross-sections instead of a table.
    ozone: bool,
    raw: PathBuf,
    out: PathBuf,
    /// Whether `--out` was given.
    out_given: bool,
    bin_width_cm: f64,
    top_cm: f64,
    temperatures_k: Vec<f64>,
    pressure_step: f64,
    threads: usize,
}

fn options() -> Result<Options, String> {
    let shortwave = std::env::args().any(|a| a == "--shortwave");
    let continua = std::env::args().any(|a| a == "--continua");
    let ozone = std::env::args().any(|a| a == "--ozone");
    let mut o = Options {
        shortwave,
        continua,
        ozone,
        raw: PathBuf::from("tools/opacity-table/raw"),
        out: PathBuf::from(if shortwave {
            "crates/science/data/shortwave.bin"
        } else if ozone {
            "crates/science/data/ozone.tsv"
        } else {
            "crates/science/data/opacity.bin"
        }),
        out_given: false,
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
    let mut args = std::env::args()
        .skip(1)
        .filter(|a| a != "--shortwave" && a != "--continua" && a != "--ozone");
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(format!("{flag} needs a value"))?;
        let number = |v: &str| v.parse::<f64>().map_err(|e| format!("{v}: {e}"));
        match flag.as_str() {
            "--raw" => o.raw = value.into(),
            "--out" => {
                o.out = value.into();
                o.out_given = true;
            }
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
/// and its number of bands, even in `ln λ` (`science`'s
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

/// The directory of the continuum files: the default output directory,
/// or the one `--out` names (its file name is then ignored).
const CONTINUA_DIR: &str = "crates/science/data";

/// Temperatures, K, of the columns of `co2-cia.tsv`: a project choice
/// that spans the roto-translational band's 200-800 K sets (and the
/// dimer bands' 192-500 K ones). The science crate holds the same list
/// (`spectrum::CO2_CIA_K`); both check it against the file's header,
/// which [`cia_header`] writes from this one.
const CIA_TEMPERATURES_K: [f64; 4] = [200.0, 300.0, 450.0, 800.0];

/// The first line of `co2-cia.tsv`: its columns, the temperatures named.
fn cia_header() -> String {
    CIA_TEMPERATURES_K
        .iter()
        .fold(String::from("# cell_start_cm"), |h, t| {
            h + &format!(" k_{t}K")
        })
}

/// Temperatures, K, of the columns of `pairs-cia.tsv`: every 10 K from
/// 70 to 400 K, the grid of HITRAN's N₂–N₂ roto-translational blocks
/// (Chistikov et al. 2019) and of its N₂–CH₄ file (Finenko et al. 2022),
/// onto which the other files' blocks are interpolated
/// ([`cia::Cia::interpolated`]). The science crate holds the same list
/// (`spectrum::PAIRS_CIA_K`); both check it against the file's header,
/// which [`pairs_header`] writes from this one.
const PAIR_TEMPERATURES_K: [f64; 34] = {
    let mut out = [0.0; 34];
    let mut i = 0;
    while i < 34 {
        out[i] = 70.0 + 10.0 * i as f64;
        i += 1;
    }
    out
};

/// The pairs of `pairs-cia.tsv`, in its order: the name its rows carry
/// and HITRAN's file (`fetch.sh`; sources in `crates/science/data/OPACITY.md`).
const PAIR_FILES: [(&str, &str); 5] = [
    ("N2-N2", "N2-N2_2021.cia"),
    ("O2-O2", "O2-O2_2024.cia"),
    ("O2-N2", "O2-N2_2024.cia"),
    ("N2-CH4", "N2-CH4_2024.cia"),
    ("CH4-CH4", "CH4-CH4_2011.cia"),
];

/// The first line of `pairs-cia.tsv`: its columns, the temperatures
/// named.
fn pairs_header() -> String {
    PAIR_TEMPERATURES_K
        .iter()
        .fold(String::from("# pair cell_start_cm"), |h, t| {
            h + &format!(" k_{t}K")
        })
}

/// Spacing, cm⁻¹, of the water continuum's nodes in `mt-ckd-water.tsv`,
/// and the last of them.
const WATER_NODES_CM: (f64, f64) = (50.0, 5000.0);

/// Writes `mt-ckd-water.tsv`, `co2-cia.tsv` and `pairs-cia.tsv`
/// (`--continua`).
fn write_continua(o: &Options) -> Result<(), String> {
    let dir = if o.out_given {
        o.out.parent().map(Path::to_path_buf).unwrap_or_default()
    } else {
        PathBuf::from(CONTINUA_DIR)
    };
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let read = |name: &str| {
        let path = o.raw.join(name);
        std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
    };
    let nc = read("absco-ref_wv-mt-ckd.nc")?;
    let water = netcdf::NetCdf::parse(&nc)
        .and_then(|nc| continuum::WaterContinuum::from_netcdf(&nc))
        .ok_or("absco-ref_wv-mt-ckd.nc: no water continuum")?;
    let text = String::from_utf8(read("CO2-CO2_2024.cia")?).map_err(|e| e.to_string())?;
    let cia = cia::Cia::parse(&text).ok_or("CO2-CO2_2024.cia: unreadable")?;

    let mut tsv = String::from(
        "# wavenumber_cm self_296K exponent foreign_296K\n\
         # MT_CKD 4.3 water continuum (absco-ref_wv-mt-ckd.nc) at every 50 cm-1: wavenumber in cm-1;\n\
         # self coefficient at 296 K and foreign coefficient at 296 K in cm2 per molecule per cm-1\n\
         # (1013 mbar); self temperature exponent, dimensionless. Written by tools/opacity-table --continua.\n",
    );
    let nodes = water.nodes(WATER_NODES_CM.0, WATER_NODES_CM.1);
    for (nu, own, exponent, foreign) in &nodes {
        tsv += &format!("{nu:.1}\t{own:.3e}\t{exponent:.3}\t{foreign:.3e}\n");
    }
    write_file(&dir.join("mt-ckd-water.tsv"), &tsv)?;

    let mut tsv = cia_header() + "\n";
    tsv += "# HITRAN CO2-CO2_2024.cia binned on 20 cm-1 cells (trapezoid mean, nearest set per region):\n\
         # cell start in cm-1; binary absorption coefficient in cm5 per molecule2 at 200, 300, 450 and 800 K.\n\
         # Written by tools/opacity-table --continua.\n";
    let cells = cia.binned(20.0, &CIA_TEMPERATURES_K);
    for (start, k) in &cells {
        tsv += &format!(
            "{start:.1}\t{:.3e}\t{:.3e}\t{:.3e}\t{:.3e}\n",
            k[0], k[1], k[2], k[3]
        );
    }
    write_file(&dir.join("co2-cia.tsv"), &tsv)?;
    eprintln!("{} water nodes, {} CIA cells", nodes.len(), cells.len());

    let mut tsv = pairs_header() + "\n";
    tsv += "# HITRAN N2-N2_2021, O2-O2_2024, O2-N2_2024, N2-CH4_2024 and CH4-CH4_2011 (alternate) .cia\n\
         # binned on 20 cm-1 cells to 5000 cm-1 (trapezoid mean; blocks interpolated linearly in temperature\n\
         # within each band, held at its ends): pair; cell start in cm-1; binary absorption coefficient\n\
         # in cm5 per molecule2 at every 10 K from 70 to 400 K. Written by tools/opacity-table --continua.\n";
    for (name, file) in PAIR_FILES {
        let text = String::from_utf8(read(file)?).map_err(|e| e.to_string())?;
        let cia = cia::Cia::parse(&text).ok_or(format!("{file}: unreadable"))?;
        let cells = cia.interpolated(20.0, 5000.0, &PAIR_TEMPERATURES_K);
        for (start, k) in &cells {
            tsv += &format!("{name}\t{start:.1}");
            for v in k {
                tsv += &format!("\t{v:.4e}");
            }
            tsv += "\n";
        }
        eprintln!("{name}: {} cells", cells.len());
    }
    write_file(&dir.join("pairs-cia.tsv"), &tsv)?;
    Ok(())
}

/// Writes `ozone.tsv` (`--ozone`) from `serdyuchenkogorshelev5digits.dat`.
fn write_ozone(o: &Options) -> Result<(), String> {
    let path = o.raw.join("serdyuchenkogorshelev5digits.dat");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let rows = ozone::parse(&text).ok_or(format!("{}: unreadable", path.display()))?;
    let tsv = ozone::table(&rows, SPECTRUM_UM).ok_or("no ozone band")?;
    if let Some(dir) = o.out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    write_file(&o.out, &tsv)
}

fn write_file(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    eprintln!("wrote {} ({} bytes)", path.display(), text.len());
    Ok(())
}

fn run() -> Result<(), String> {
    let o = options()?;
    if o.continua {
        return write_continua(&o);
    }
    if o.ozone {
        return write_ozone(&o);
    }
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
    #[test]
    fn the_committed_cia_table_is_at_the_tools_temperatures() {
        let committed = include_str!("../../../crates/science/data/co2-cia.tsv");
        assert_eq!(committed.lines().next(), Some(super::cia_header().as_str()));
        let committed = include_str!("../../../crates/science/data/pairs-cia.tsv");
        assert_eq!(
            committed.lines().next(),
            Some(super::pairs_header().as_str())
        );
    }

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
