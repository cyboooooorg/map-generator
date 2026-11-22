//! The table's grid and its binary format (read by the science crate's
//! `planet/ktable.rs`; documented in `crates/science/data/OPACITY.md`).
//!
//! Little-endian throughout:
//!
//! | Field | Type |
//! |---|---|
//! | magic `MGOPACK2` | 8 bytes |
//! | bins, g-points, pressures, temperatures, sets | 5 × u32 |
//! | bin width, cm⁻¹ | f64 |
//! | g-interval edges | (g-points + 1) × f64 |
//! | `ln p` of the pressures, bar | pressures × f64 |
//! | temperatures, K | temperatures × f64 |
//! | `ln k` of code 1, and the step per code | 2 × f64 |
//! | codes | sets × pressures × temperatures × bins × g-points × u16 |
//! | CO₂ continuum: `T_eff`, K | f64 |
//! | CO₂ continuum per bin: coefficient, cm² per molecule per cm⁻¹, and exponent | bins × 2 × f64 |
//!
//! A code `q` ≥ 1 is `ln k = ln_k0 + (q − 1) step` with `k` in m²/kg; code
//! 0 is a coefficient under `exp(ln_k0)`, taken as zero.
//!
//! The shortwave table (`shortwave.bin`, [`SHORTWAVE_MAGIC`]) has bins of
//! uneven width, the bands of the albedo's stellar spectrum: its bin
//! width field is replaced by the `bins + 1` edges (cm⁻¹, ascending), and
//! MT_CKD's water continuum follows the CO₂ continuum, per bin its self
//! coefficient at 296 K, the self exponent and the foreign coefficient at
//! 296 K (cm² per molecule per cm⁻¹), `bins × 3 × f64`.

use crate::kdist::{BOUNDS, G};

/// File signature.
pub const MAGIC: &[u8; 8] = b"MGOPACK2";

/// File signature of the shortwave table.
pub const SHORTWAVE_MAGIC: &[u8; 8] = b"MGSWCK01";

/// `ln k` of code 1: k = 9.4e-14 m²/kg. Venus's 92 bar of CO₂ weigh
/// 1e6 kg/m², so a coefficient under it adds under 1e-7 to any depth.
pub const LN_K0: f64 = -30.0;

/// `ln k` of the top code: k = 6.6e7 m²/kg, above the Doppler core of
/// CO₂'s strongest lines at 100 K.
pub const LN_K_TOP: f64 = 18.0;

/// `ln k` per code: 7.3e-4, a relative step of 0.07 % in `k`.
pub const LN_K_STEP: f64 = (LN_K_TOP - LN_K0) / 65_534.0;

/// The absorber sets, in file order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Set {
    /// CO₂'s lines broadened by air.
    Co2,
    /// Water's lines broadened by air (the foreign set).
    WaterForeign,
    /// Water's lines broadened by water vapour (the self set).
    WaterSelf,
    /// CO₂'s lines broadened by CO₂ (HITRAN's `γ_self`): the bulk gas of a
    /// thick CO₂ air.
    Co2Self,
}

pub const SETS: [Set; 4] = [Set::Co2, Set::WaterForeign, Set::WaterSelf, Set::Co2Self];

/// Pressures of the grid, bar: every `step` decades from 1e-5 to 100
/// (the table's: every half decade).
pub fn pressures(step: f64) -> Vec<f64> {
    let n = (7.0 / step).round() as usize;
    (0..=n)
        .map(|i| 10f64.powf(-5.0 + 7.0 * i as f64 / n as f64))
        .collect()
}

/// Decades between two pressures of the grid.
pub const PRESSURE_STEP: f64 = 0.5;

/// Temperatures of the grid, K: 25 K apart where Earth-like, Martian and
/// moist columns lie, wider in the hot steam and Venus range. Sixteen, so
/// that the four sets fit in 4 MB (450 and 700 K were dropped with the
/// CO₂ self set; the deep layers there are opaque in the lines).
pub const TEMPERATURES: [f64; 16] = [
    100.0, 125.0, 150.0, 175.0, 200.0, 225.0, 250.0, 275.0, 300.0, 325.0, 350.0, 400.0, 500.0,
    600.0, 800.0, 900.0,
];

/// Temperatures of the shortwave table's grid, K: thirteen, so that its
/// 147 bins fit in 2 MB. The star's near infrared is absorbed low in the
/// column, where the air is 150 to 350 K on the temperate and cold
/// bodies the albedo matters for; the wider steps above reach the
/// steam columns (a runaway's albedo blends towards the moist one,
/// `climate::runaway_albedo`). Colder levels take the 150 K
/// coefficients (the table clamps).
pub const SHORTWAVE_TEMPERATURES: [f64; 13] = [
    150.0, 175.0, 200.0, 225.0, 250.0, 275.0, 300.0, 325.0, 350.0, 400.0, 500.0, 650.0, 900.0,
];

/// The code of a coefficient `k`, m²/kg.
pub fn encode(k: f64) -> u16 {
    if k.is_nan() || k <= 0.0 || k.ln() < LN_K0 {
        return 0;
    }
    (((k.ln() - LN_K0) / LN_K_STEP).round() as i64 + 1).clamp(1, 65_535) as u16
}

/// The coefficient, m²/kg, of a code (as mapgen decodes it).
#[cfg(test)]
pub fn decode(q: u16) -> f64 {
    if q == 0 {
        0.0
    } else {
        (LN_K0 + (q - 1) as f64 * LN_K_STEP).exp()
    }
}

/// The shape of a table.
#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub bins: usize,
    pub bin_width_cm: f64,
    /// Edges of the bins, cm⁻¹, ascending, for a table of uneven bins
    /// (the shortwave table); `None` for even bins of `bin_width_cm`
    /// from 0.
    pub edges_cm: Option<Vec<f64>>,
    pub pressures_bar: Vec<f64>,
    pub temperatures_k: Vec<f64>,
    /// MT_CKD's CO₂ continuum of each bin: coefficient (cm² per molecule
    /// per cm⁻¹, at [`crate::continuum::T_EFF_K`]) and exponent.
    pub co2_continuum: Vec<(f64, f64)>,
    /// MT_CKD's water continuum of each bin of the shortwave table (see
    /// [`crate::continuum::WaterContinuum::bin`]); empty for the
    /// longwave table, whose continuum `mapgen` holds.
    pub water_continuum: Vec<(f64, f64, f64)>,
}

impl Header {
    /// Edges of bin `b`, cm⁻¹.
    pub fn bin(&self, b: usize) -> (f64, f64) {
        match &self.edges_cm {
            Some(edges) => (edges[b], edges[b + 1]),
            None => (
                b as f64 * self.bin_width_cm,
                (b + 1) as f64 * self.bin_width_cm,
            ),
        }
    }

    /// Index of the codes of `(set, pressure, temperature)`, whose
    /// `bins × G` codes follow in bin-major order.
    pub fn offset(&self, set: usize, p: usize, t: usize) -> usize {
        ((set * self.pressures_bar.len() + p) * self.temperatures_k.len() + t) * self.bins * G
    }

    /// Number of codes.
    pub fn codes(&self) -> usize {
        self.offset(SETS.len(), 0, 0)
    }
}

/// The file of `header` and its `codes`.
pub fn write(header: &Header, codes: &[u16]) -> Vec<u8> {
    assert_eq!(codes.len(), header.codes());
    assert_eq!(header.co2_continuum.len(), header.bins);
    let mut out = Vec::with_capacity(256 + 2 * codes.len() + 48 * header.bins);
    let shortwave = header.edges_cm.is_some();
    if shortwave {
        assert_eq!(header.water_continuum.len(), header.bins);
    }
    out.extend_from_slice(if shortwave { SHORTWAVE_MAGIC } else { MAGIC });
    for n in [
        header.bins,
        G,
        header.pressures_bar.len(),
        header.temperatures_k.len(),
        SETS.len(),
    ] {
        out.extend_from_slice(&(n as u32).to_le_bytes());
    }
    let mut float = |x: f64| out.extend_from_slice(&x.to_le_bytes());
    match &header.edges_cm {
        Some(edges) => {
            assert_eq!(edges.len(), header.bins + 1);
            edges.iter().for_each(|&e| float(e));
        }
        None => float(header.bin_width_cm),
    }
    BOUNDS.iter().for_each(|&b| float(b));
    header.pressures_bar.iter().for_each(|&p| float(p.ln()));
    header.temperatures_k.iter().for_each(|&t| float(t));
    float(LN_K0);
    float(LN_K_STEP);
    for q in codes {
        out.extend_from_slice(&q.to_le_bytes());
    }
    out.extend_from_slice(&crate::continuum::T_EFF_K.to_le_bytes());
    for (c, a) in &header.co2_continuum {
        out.extend_from_slice(&c.to_le_bytes());
        out.extend_from_slice(&a.to_le_bytes());
    }
    if shortwave {
        for (s, n, f) in &header.water_continuum {
            for x in [s, n, f] {
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
    }
    out
}

/// Reads a file of [`write`]: its header and codes.
#[cfg(test)]
pub fn read(bytes: &[u8]) -> Option<(Header, Vec<u16>)> {
    let mut at = 0usize;
    let mut take = |n: usize| -> Option<&[u8]> {
        let s = bytes.get(at..at + n)?;
        at += n;
        Some(s)
    };
    let shortwave = match take(8)? {
        m if m == MAGIC => false,
        m if m == SHORTWAVE_MAGIC => true,
        _ => return None,
    };
    let mut ints = [0usize; 5];
    for i in &mut ints {
        *i = u32::from_le_bytes(take(4)?.try_into().ok()?) as usize;
    }
    let [bins, g, n_p, n_t, sets] = ints;
    if g != G || sets != SETS.len() {
        return None;
    }
    let mut float = || -> Option<f64> { Some(f64::from_le_bytes(take(8)?.try_into().ok()?)) };
    let (bin_width_cm, edges_cm) = if shortwave {
        let edges = (0..=bins).map(|_| float()).collect::<Option<Vec<_>>>()?;
        (0.0, Some(edges))
    } else {
        (float()?, None)
    };
    for _ in 0..=G {
        float()?;
    }
    let pressures_bar = (0..n_p)
        .map(|_| float().map(f64::exp))
        .collect::<Option<Vec<_>>>()?;
    let temperatures_k = (0..n_t).map(|_| float()).collect::<Option<Vec<_>>>()?;
    let (k0, step) = (float()?, float()?);
    if k0 != LN_K0 || step != LN_K_STEP {
        return None;
    }
    let mut header = Header {
        bins,
        bin_width_cm,
        edges_cm,
        pressures_bar,
        temperatures_k,
        co2_continuum: Vec::new(),
        water_continuum: Vec::new(),
    };
    let rest = take(2 * header.codes())?;
    let codes = rest
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    let mut float = || -> Option<f64> { Some(f64::from_le_bytes(take(8)?.try_into().ok()?)) };
    if float()? != crate::continuum::T_EFF_K {
        return None;
    }
    header.co2_continuum = (0..bins)
        .map(|_| Some((float()?, float()?)))
        .collect::<Option<Vec<_>>>()?;
    if shortwave {
        header.water_continuum = (0..bins)
            .map(|_| Some((float()?, float()?, float()?)))
            .collect::<Option<Vec<_>>>()?;
    }
    Some((header, codes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_keep_a_coefficient_to_a_tenth_of_a_percent() {
        for k in [1e-13, 3.7e-5, 1.0, 220.0, 5e7] {
            let back = decode(encode(k));
            assert!((back / k - 1.0).abs() < 4e-4, "{k} → {back}");
        }
        assert_eq!(encode(0.0), 0);
        assert_eq!(encode(1e-20), 0);
        assert_eq!(decode(0), 0.0);
    }

    #[test]
    fn a_table_reads_back() {
        let header = Header {
            bins: 3,
            bin_width_cm: 20.0,
            edges_cm: None,
            pressures_bar: vec![1e-2, 1.0],
            temperatures_k: vec![200.0, 300.0],
            co2_continuum: vec![(1e-25, 0.0), (2e-26, 0.5), (0.0, 0.0)],
            water_continuum: Vec::new(),
        };
        let codes: Vec<u16> = (0..header.codes()).map(|i| i as u16 * 7).collect();
        let (h, c) = read(&write(&header, &codes)).expect("reads");
        assert_eq!(c, codes);
        assert_eq!((h.bins, h.bin_width_cm), (3, 20.0));
        assert_eq!(h.temperatures_k, header.temperatures_k);
        assert_eq!(h.co2_continuum, header.co2_continuum);
        for (a, b) in h.pressures_bar.iter().zip(&header.pressures_bar) {
            assert!((a / b - 1.0).abs() < 1e-15);
        }
        assert_eq!(&write(&header, &codes)[..8], MAGIC);
    }

    #[test]
    fn a_shortwave_table_reads_back_with_its_edges() {
        let header = Header {
            bins: 2,
            bin_width_cm: 0.0,
            edges_cm: Some(vec![2000.0, 2027.0, 2054.5]),
            pressures_bar: vec![1e-2, 1.0],
            temperatures_k: vec![200.0, 300.0],
            co2_continuum: vec![(1e-25, 0.0), (0.0, 0.0)],
            water_continuum: vec![(1e-24, 4.0, 1e-26), (2e-24, 3.5, 0.0)],
        };
        let codes: Vec<u16> = (0..header.codes()).map(|i| i as u16 * 3).collect();
        let bytes = write(&header, &codes);
        assert_eq!(&bytes[..8], SHORTWAVE_MAGIC);
        let (h, c) = read(&bytes).expect("reads");
        assert_eq!(c, codes);
        assert_eq!(h.edges_cm, header.edges_cm);
        assert_eq!(h.bin(1), (2027.0, 2054.5));
        assert_eq!(h.water_continuum, header.water_continuum);
        assert_eq!(h.co2_continuum, header.co2_continuum);
    }
}
