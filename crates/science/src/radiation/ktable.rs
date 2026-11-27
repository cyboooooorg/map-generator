//! The correlated-k tables of the gases' lines that
//! `tools/opacity-table` builds from HITRAN: the longwave one,
//! `crates/science/data/opacity.bin`, on the column's even bins, and the
//! shortwave one on the albedo's bands. Both share one
//! format (documented in `crates/science/data/OPACITY.md`; the tool's
//! `table.rs` writes it), one decoding and one interpolation:
//!
//! - for each set (CO₂ broadened by air, water broadened by air, water
//!   broadened by itself, CO₂ broadened by itself; in the longwave table
//!   also methane broadened by air and by itself, ozone broadened by air
//!   and N₂O broadened by air: [`N2O_SET`], the eighth, index 7), each
//!   pressure and
//!   temperature of the grid, each bin and each g-point, the mass
//!   absorption coefficient of the gas, m²/kg, as a 16-bit code of `ln k`;
//! - MT_CKD 4.3's CO₂ continuum binned on the table's bins, and, in the
//!   shortwave table, its water continuum too.
//!
//! Between the grid's nodes `ln k` is interpolated linearly in `ln p` and
//! in `T`; past its edges the temperature is clamped and `k` scales as
//! `p` (Lorentz).

use super::fast_math;
use super::spectrum::Level;

/// Definition: the number of g-points per bin of the table files.
pub(crate) const G: usize = 8;

/// Definition: the file signature of the longwave table, on even bins from
/// 0 cm⁻¹, with [`LONGWAVE_SETS`] sets (`OPACITY.md`, Layout).
pub(crate) const LONGWAVE_MAGIC: &[u8; 8] = b"MGOPACK3";

/// Definition: the file signature of the shortwave table, on bins of uneven
/// width.
pub(crate) const SHORTWAVE_MAGIC: &[u8; 8] = b"MGSWCK01";

/// Definition: the `ln k` given to the code 0 (a coefficient under the table's floor,
/// 9.4e-14 m²/kg): 2e-22 m²/kg (`e⁻⁵⁰`), nothing at any column
/// mass.
const LN_K_ZERO: f32 = -50.0;

/// Definition: index of the sets in [`Table`], the order of the file
/// (`OPACITY.md`, Grid).
pub(crate) const CO2_SET: usize = 0;
/// Definition: index of water broadened by air, see [`CO2_SET`].
pub(crate) const WATER_FOREIGN_SET: usize = 1;
/// Definition: index of water broadened by itself, see [`CO2_SET`].
pub(crate) const WATER_SELF_SET: usize = 2;
/// Definition: index of CO₂ broadened by itself, see [`CO2_SET`].
pub(crate) const CO2_SELF_SET: usize = 3;
/// Definition: index of methane broadened by air, in the longwave table
/// only, see [`CO2_SET`].
pub(crate) const CH4_SET: usize = 4;
/// Definition: index of methane broadened by itself, see [`CH4_SET`].
pub(crate) const CH4_SELF_SET: usize = 5;
/// Definition: index of ozone broadened by air, see [`CH4_SET`].
pub(crate) const O3_SET: usize = 6;
/// Definition: index of N₂O broadened by air, see [`CH4_SET`].
pub(crate) const N2O_SET: usize = 7;
/// Definition: the number of sets of the longwave table, eight.
pub(crate) const LONGWAVE_SETS: usize = 8;
/// Definition: the number of sets of the shortwave table, four: CO₂'s
/// and water's.
const SHORTWAVE_SETS: usize = 4;

/// A table, decoded.
pub(crate) struct Table {
    /// Number of bins.
    pub(crate) bins: usize,
    /// Number of sets ([`LONGWAVE_SETS`] or the shortwave's four).
    pub(crate) sets: usize,
    /// Width of a bin, cm⁻¹, of a table of even bins from 0 cm⁻¹ (the
    /// longwave); 0 for uneven bins.
    pub(crate) bin_width_cm: f64,
    /// Edges of the bins, cm⁻¹, ascending, of a table of uneven bins (the
    /// shortwave); empty for even bins.
    pub(crate) edges_cm: Vec<f64>,
    /// Weight of each g-point (its interval of `g`); they sum to 1.
    pub(crate) weights: [f64; G],
    /// `ln p` of the pressures of the grid, bar.
    ln_p: Vec<f64>,
    /// Temperatures of the grid, K.
    t_k: Vec<f64>,
    /// `ln k`, `[set][pressure][temperature][bin][g-point]`.
    pub(crate) ln_k: Vec<f32>,
    /// MT_CKD's CO₂ continuum of each bin: its coefficient at
    /// [`Table::co2_continuum_t_eff_k`], cm² per molecule per cm⁻¹, and its
    /// temperature exponent (see `spectrum::co2_continuum_per_molecule`).
    pub(crate) co2_continuum: Vec<(f64, f64)>,
    /// Temperature, K, at which the continuum's exponents scale by 1.
    pub(crate) co2_continuum_t_eff_k: f64,
    /// MT_CKD's water continuum of each bin of the shortwave table: self
    /// coefficient at 296 K, its temperature exponent, foreign coefficient
    /// at 296 K, cm² per molecule per cm⁻¹ at 1013 mbar. Empty for the
    /// longwave table, whose continuum `spectrum::MT_CKD` holds.
    pub(crate) water_continuum: Vec<(f64, f64, f64)>,
}

impl Table {
    /// Decodes a table file of either kind, by its signature; `None` if it
    /// is not one, or not of [`G`] g-points and its kind's sets.
    pub(crate) fn parse(bytes: &[u8]) -> Option<Table> {
        let mut at = 0usize;
        let mut take = |n: usize| -> Option<&[u8]> {
            let s = bytes.get(at..at + n)?;
            at += n;
            Some(s)
        };
        let shortwave = match take(8)? {
            m if m == LONGWAVE_MAGIC => false,
            m if m == SHORTWAVE_MAGIC => true,
            _ => return None,
        };
        let mut ints = [0usize; 5];
        for i in &mut ints {
            *i = u32::from_le_bytes(take(4)?.try_into().ok()?) as usize;
        }
        let [bins, g, n_p, n_t, sets] = ints;
        let expected = if shortwave {
            SHORTWAVE_SETS
        } else {
            LONGWAVE_SETS
        };
        if g != G || sets != expected || n_p < 2 || n_t < 2 || bins == 0 {
            return None;
        }
        let mut float = || -> Option<f64> { Some(f64::from_le_bytes(take(8)?.try_into().ok()?)) };
        let (bin_width_cm, edges_cm) = if shortwave {
            let edges = (0..=bins).map(|_| float()).collect::<Option<Vec<_>>>()?;
            (0.0, edges)
        } else {
            (float()?, Vec::new())
        };
        let mut bounds = [0.0; G + 1];
        for b in &mut bounds {
            *b = float()?;
        }
        let ln_p = (0..n_p).map(|_| float()).collect::<Option<Vec<_>>>()?;
        let t_k = (0..n_t).map(|_| float()).collect::<Option<Vec<_>>>()?;
        let (k0, step) = (float()?, float()?);
        let count = sets * n_p * n_t * bins * G;
        let codes = take(2 * count)?;
        let ln_k = codes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| match u16::from_le_bytes(*c) {
                0 => LN_K_ZERO,
                q => (k0 + f64::from(q - 1) * step) as f32,
            })
            .collect();
        let mut float = || -> Option<f64> { Some(f64::from_le_bytes(take(8)?.try_into().ok()?)) };
        let co2_continuum_t_eff_k = float()?;
        let co2_continuum = (0..bins)
            .map(|_| Some((float()?, float()?)))
            .collect::<Option<Vec<_>>>()?;
        let water_continuum = if shortwave {
            (0..bins)
                .map(|_| Some((float()?, float()?, float()?)))
                .collect::<Option<Vec<_>>>()?
        } else {
            Vec::new()
        };
        Some(Table {
            bins,
            sets,
            bin_width_cm,
            edges_cm,
            weights: std::array::from_fn(|i| bounds[i + 1] - bounds[i]),
            ln_p,
            t_k,
            ln_k,
            co2_continuum,
            co2_continuum_t_eff_k,
            water_continuum,
        })
    }

    /// Start in [`Table::ln_k`] of a set's coefficients at a node.
    fn offset(&self, set: usize, p: usize, t: usize) -> usize {
        ((set * self.ln_p.len() + p) * self.t_k.len() + t) * self.bins * G
    }

    /// Where `(p_bar, t_k)` falls on the grid: the four corners' offsets
    /// of `set` and their weights, and the factor `p / p_edge` past the
    /// pressure range.
    pub(crate) fn corners(&self, set: usize, p_bar: f64, t_k: f64) -> ([usize; 4], [f64; 4], f64) {
        let lp = p_bar.max(f64::MIN_POSITIVE).ln();
        let last_p = self.ln_p.len() - 1;
        let (ip, fp, scale) = if lp <= self.ln_p[0] {
            (0, 0.0, (lp - self.ln_p[0]).exp())
        } else if lp >= self.ln_p[last_p] {
            (last_p - 1, 1.0, (lp - self.ln_p[last_p]).exp())
        } else {
            let i = self.ln_p.partition_point(|&x| x <= lp).clamp(1, last_p) - 1;
            (
                i,
                (lp - self.ln_p[i]) / (self.ln_p[i + 1] - self.ln_p[i]),
                1.0,
            )
        };
        let last_t = self.t_k.len() - 1;
        let t = t_k.clamp(self.t_k[0], self.t_k[last_t]);
        let it = self.t_k.partition_point(|&x| x <= t).clamp(1, last_t) - 1;
        let ft = (t - self.t_k[it]) / (self.t_k[it + 1] - self.t_k[it]);
        (
            [
                self.offset(set, ip, it),
                self.offset(set, ip + 1, it),
                self.offset(set, ip, it + 1),
                self.offset(set, ip + 1, it + 1),
            ],
            [
                (1.0 - fp) * (1.0 - ft),
                fp * (1.0 - ft),
                (1.0 - fp) * ft,
                fp * ft,
            ],
            scale,
        )
    }

    /// The lines of one gas, of partial pressure `partial_bar` and molar
    /// mass `molar_kg`, at `level` in the first `m` g-points (over the
    /// bins) into `out`, per unit `ln p` (vertical optical depths), in
    /// single precision: its set broadened by the air (`sets.0`) and its
    /// set broadened by itself (`sets.1`), mixed by its share of the
    /// pressure, each interpolated in `ln k` into `by_air` and `by_self`
    /// with the logarithm of its column mass added, then summed. The level
    /// must hold air (a positive pressure, temperature and gravity) and
    /// the gas.
    #[expect(
        clippy::too_many_arguments,
        reason = "the slices are the caller's buffers, reused per level"
    )]
    pub(crate) fn lines(
        &self,
        level: &Level,
        partial_bar: f64,
        molar_kg: f64,
        sets: (usize, usize),
        m: usize,
        by_air: &mut [f32],
        by_self: &mut [f32],
        out: &mut [f32],
    ) {
        let p = level.pressure_bar();
        let t = level.temperature_k;
        let (air_at, wa, scale) = self.corners(sets.0, p, t);
        let (self_at, ws, _) = self.corners(sets.1, p, t);
        let (wa, ws) = (wa.map(|w| w as f32), ws.map(|w| w as f32));
        let mass = scale * level.mass_per_ln_p(partial_bar, molar_kg);
        let share = (partial_bar / p).clamp(0.0, 1.0);
        // `ln 0` is −∞, which `exp` takes to its floor.
        let (air_ln, self_ln) = (
            (mass * (1.0 - share)).ln() as f32,
            (mass * share).ln() as f32,
        );
        let node = |at: usize| -> &[f32] { &self.ln_k[at..at + m] };
        interpolate(&air_at.map(node), &wa, air_ln, by_air);
        interpolate(&self_at.map(node), &ws, self_ln, by_self);
        add_exp(by_air, by_self, out);
    }
}

/// `ln k` of the four nodes `rows` weighted by `w`, plus `add`, into
/// `out`. Each loop of [`Table::lines`] is a function of its own, whose
/// slices the compiler knows apart, so that it takes vector instructions.
#[inline(never)]
fn interpolate(rows: &[&[f32]; 4], w: &[f32; 4], add: f32, out: &mut [f32]) {
    let [r0, r1, r2, r3] = rows;
    for ((((o, a), b), c), d) in out.iter_mut().zip(*r0).zip(*r1).zip(*r2).zip(*r3) {
        *o = w[0] * a + w[1] * b + w[2] * c + w[3] * d + add;
    }
}

/// `e^a + e^b`, into `out`.
#[inline(never)]
fn add_exp(a: &[f32], b: &[f32], out: &mut [f32]) {
    for ((o, a), b) in out.iter_mut().zip(a).zip(b) {
        *o = fast_math::exp(*a) + fast_math::exp(*b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Source: `crates/science/data/OPACITY.md` (250 bins of 20 cm⁻¹ from 0
    /// to 5000 cm⁻¹).
    const BINS: usize = 250;

    /// Source: `crates/science/data/OPACITY.md`, the bin width of the
    /// longwave table, cm⁻¹.
    const BIN_WIDTH_CM: f64 = 20.0;

    /// Definition: the g-intervals partition [0, 1], so their weights sum
    /// to 1; the tolerance is `f64` rounding over eight terms.
    const WEIGHT_SUM_TOLERANCE: f64 = 1e-12;

    /// Source: `crates/science/data/OPACITY.md`, Grid: 15 pressures, 16
    /// temperatures and 8 sets.
    const NODES: usize = 15 * 16 * 8;

    /// Source: `OPACITY.md` (Layout), the temperature of the CO₂
    /// continuum's band-head exponents, K.
    const CONTINUUM_T_EFF_K: f64 = 246.0;

    /// Source: `OPACITY.md`, Grid: the pressures go to 100 bar.
    const TOP_BAR: f64 = 100.0;

    /// Definition: the tolerance of a factor derived from a logarithm.
    const SCALE_TOLERANCE: f64 = 1e-9;

    /// Definition: the sets in the order of the file.
    const SETS_IN_ORDER: [usize; LONGWAVE_SETS] = [
        CO2_SET,
        WATER_FOREIGN_SET,
        WATER_SELF_SET,
        CO2_SELF_SET,
        CH4_SET,
        CH4_SELF_SET,
        O3_SET,
        N2O_SET,
    ];

    fn longwave() -> Table {
        Table::parse(include_bytes!("../../data/opacity.bin")).expect("a longwave table")
    }

    #[test]
    fn the_table_parses() {
        let table = longwave();
        assert_eq!(table.bins, BINS);
        assert_eq!(table.sets, LONGWAVE_SETS);
        assert_eq!(table.bin_width_cm, BIN_WIDTH_CM);
        assert!(table.edges_cm.is_empty());
        assert!(table.water_continuum.is_empty());
        assert_eq!(table.co2_continuum.len(), BINS);
        assert_eq!(table.co2_continuum_t_eff_k, CONTINUUM_T_EFF_K);
        assert_eq!(table.ln_k.len(), NODES * BINS * G);
        let sum: f64 = table.weights.iter().sum();
        assert!(
            (sum - 1.0).abs() < WEIGHT_SUM_TOLERANCE,
            "weights sum {sum}"
        );
    }

    #[test]
    fn the_table_refuses_what_is_not_one() {
        let bytes = include_bytes!("../../data/opacity.bin");
        assert!(
            Table::parse(&bytes[..bytes.len() - 1]).is_none(),
            "truncated"
        );
        assert!(Table::parse(b"NOTATABLE").is_none(), "wrong signature");
        assert!(Table::parse(SHORTWAVE_MAGIC).is_none(), "signature only");
    }

    #[test]
    fn the_corners_weigh_one_and_scale_past_the_grid() {
        let table = longwave();
        for set in SETS_IN_ORDER {
            let (at, w, scale) = table.corners(set, 1.0, 288.0);
            assert!((w.iter().sum::<f64>() - 1.0).abs() < SCALE_TOLERANCE);
            assert!((scale - 1.0).abs() < SCALE_TOLERANCE);
            assert!(at.iter().all(|&a| a + table.bins * G <= table.ln_k.len()));
            // Ten times the top of the grid: k scales as p, so by 10.
            let (_, w, scale) = table.corners(set, 10.0 * TOP_BAR, 288.0);
            assert!((w.iter().sum::<f64>() - 1.0).abs() < SCALE_TOLERANCE);
            assert!((scale - 10.0).abs() < SCALE_TOLERANCE, "scale {scale}");
        }
        let firsts = SETS_IN_ORDER.map(|s| table.corners(s, 1.0, 288.0).0[0]);
        assert!(firsts.windows(2).all(|p| p[0] < p[1]), "sets in file order");
    }
}
