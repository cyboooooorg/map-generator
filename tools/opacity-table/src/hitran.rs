//! HITRAN line records and partition sums.
//!
//! A line record is HITRAN's fixed-width 160-character `.par` format
//! (Rothman et al. 2005, *JQSRT* 96, 139, Table 1). Line strengths are
//! given at the reference temperature [`T_REF`] for the natural isotopic
//! abundance, per molecule of the gas; [`strength_at`] moves them to
//! another temperature with HITRAN's documented formula (Gordon et al.
//! 2022, *JQSRT* 277, 107949, and the HITRAN definitions page,
//! hitran.org/docs/definitions-and-units).

/// Second radiation constant `h c / k`, cm·K (CODATA 2018).
pub const C2_CM_K: f64 = 1.438_776_877;

/// HITRAN's reference temperature, K.
pub const T_REF: f64 = 296.0;

/// One spectral line, as HITRAN gives it at [`T_REF`] and 1 atm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    /// HITRAN molecule number (1 H₂O, 2 CO₂).
    pub molecule: u8,
    /// HITRAN isotopologue number within the molecule, from 1.
    pub isotopologue: u8,
    /// Vacuum wavenumber, cm⁻¹.
    pub wavenumber: f64,
    /// Intensity at [`T_REF`], cm⁻¹/(molecule cm⁻²).
    pub strength: f64,
    /// Air-broadened Lorentz half width at half maximum, cm⁻¹/atm.
    pub gamma_air: f64,
    /// Self-broadened half width, cm⁻¹/atm.
    pub gamma_self: f64,
    /// Lower-state energy, cm⁻¹.
    pub lower_energy: f64,
    /// Temperature exponent of the air-broadened width.
    pub n_air: f64,
    /// Air pressure shift of the line position, cm⁻¹/atm.
    pub delta_air: f64,
}

/// Parses one 160-character record; `None` for a short or malformed one.
pub fn parse_line(record: &str) -> Option<Line> {
    let record = record.trim_end_matches(['\r', '\n']);
    if record.len() < 67 || !record.is_ascii() {
        return None;
    }
    let field = |a: usize, b: usize| record[a..b].trim();
    let number = |a: usize, b: usize| field(a, b).parse::<f64>().ok();
    let isotopologue = match record.as_bytes()[2] {
        b'0' => 10,
        c @ b'1'..=b'9' => c - b'0',
        c @ b'A'..=b'Z' => c - b'A' + 11,
        _ => return None,
    };
    Some(Line {
        molecule: field(0, 2).parse().ok()?,
        isotopologue,
        wavenumber: number(3, 15)?,
        strength: number(15, 25)?,
        gamma_air: number(35, 40)?,
        gamma_self: number(40, 45)?,
        lower_energy: number(45, 55)?,
        n_air: number(55, 59)?,
        delta_air: number(59, 67)?,
    })
}

/// A total internal partition sum tabulated at whole kelvins (HITRAN's
/// TIPS files `q<id>.txt`: one `T Q(T)` pair per line, from 1 K).
#[derive(Clone, Debug)]
pub struct PartitionSum {
    /// `(T, Q)` pairs in increasing `T`.
    points: Vec<(f64, f64)>,
}

impl PartitionSum {
    /// Reads a TIPS file.
    pub fn parse(text: &str) -> Option<Self> {
        let mut points = Vec::new();
        for row in text.lines() {
            let mut it = row.split_whitespace();
            let (Some(t), Some(q)) = (it.next(), it.next()) else {
                continue;
            };
            points.push((t.parse().ok()?, q.parse().ok()?));
        }
        (points.len() >= 2).then_some(Self { points })
    }

    /// `Q(T)`, linear between the tabulated kelvins.
    pub fn at(&self, t: f64) -> f64 {
        let p = &self.points;
        let i = p.partition_point(|&(x, _)| x <= t).clamp(1, p.len() - 1);
        let ((t0, q0), (t1, q1)) = (p[i - 1], p[i]);
        q0 + (q1 - q0) * (t - t0) / (t1 - t0)
    }
}

/// The intensity of `line` at `t_k`, cm⁻¹/(molecule cm⁻²), from its value
/// at [`T_REF`] (HITRAN definitions page, eq. for `S_ij(T)`):
///
/// `S(T) = S(T_ref) Q(T_ref)/Q(T) · exp(−c₂E″/T)/exp(−c₂E″/T_ref) ·
/// (1 − exp(−c₂ν/T))/(1 − exp(−c₂ν/T_ref))`,
///
/// with `q_ref` = `Q(T_ref)` and `q_t` = `Q(T)` of the line's
/// isotopologue.
pub fn strength_at(line: &Line, t_k: f64, q_ref: f64, q_t: f64) -> f64 {
    let boltzmann = (-C2_CM_K * line.lower_energy * (1.0 / t_k - 1.0 / T_REF)).exp();
    let stimulated = (-(-C2_CM_K * line.wavenumber / t_k).exp_m1())
        / (-(-C2_CM_K * line.wavenumber / T_REF).exp_m1());
    line.strength * (q_ref / q_t) * boltzmann * stimulated
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The strongest H₂O line between 250 and 500 cm⁻¹ in HITRAN2024 (as served on 2026-11-18), the
    /// 5₅₀ ← 4₄₁ rotational line of H₂¹⁶O, as the API serves it.
    const RECORD: &str = " 11  253.968590 1.839E-18 7.171E+00.05420.307  488.10770.51-.001980          0 0 0          0 0 0  5  5  0        4  4  1      764426807260783049    33.0   27.0";

    #[test]
    fn a_record_parses_into_its_fields() {
        let line = parse_line(RECORD).expect("parses");
        assert_eq!((line.molecule, line.isotopologue), (1, 1));
        assert_eq!(line.wavenumber, 253.968590);
        assert_eq!(line.strength, 1.839e-18);
        assert_eq!(line.gamma_air, 0.0542);
        assert_eq!(line.gamma_self, 0.307);
        assert_eq!(line.lower_energy, 488.1077);
        assert_eq!(line.n_air, 0.51);
        assert_eq!(line.delta_air, -0.00198);
    }

    #[test]
    fn a_water_line_strength_moves_to_250_k_as_hitran_documents() {
        // Q of H₂¹⁶O from HITRAN's q1.txt (TIPS, as served on 2026-11-18): 174.581 at 296 K,
        // 135.700 at 250 K. The expected value is the documented formula
        // evaluated independently (Python, math module):
        // 1.839e-18 * 174.581/134.799 * exp(-c2*488.1077*(1/250-1/296))
        // * (1-exp(-c2*253.96859/250))/(1-exp(-c2*253.96859/296)).
        let line = parse_line(RECORD).unwrap();
        let s = strength_at(&line, 250.0, 174.581, 135.7);
        let expected = EXPECTED_250_K;
        assert!(
            (s / expected - 1.0).abs() < 1e-3,
            "S(250 K) = {s:e}, against {expected:e}"
        );
    }

    /// See `a_water_line_strength_moves_to_250_k_as_hitran_documents`.
    const EXPECTED_250_K: f64 = 1.656_506_615_614_855_4e-18;

    #[test]
    fn partition_sums_interpolate_between_kelvins() {
        let q = PartitionSum::parse("   1   1.0\n   2   3.0\n   3   7.0\n").unwrap();
        assert_eq!(q.at(2.0), 3.0);
        assert_eq!(q.at(2.5), 5.0);
    }
}
