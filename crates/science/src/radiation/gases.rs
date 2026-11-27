//! The gases of an atmosphere: their partial pressures, molar masses and
//! heat capacities.

/// Partial pressures of the gases at the surface, bar.
///
/// A negative or NaN part, as a caller's rounding leaves, counts as none
/// ([`PartialPressures::total`]), as the legacy `max(0.0)` does.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PartialPressures {
    /// Nitrogen, bar.
    pub n2: f64,
    /// Carbon dioxide, bar.
    pub co2: f64,
    /// Water vapour, bar.
    pub h2o: f64,
    /// Methane, bar.
    pub ch4: f64,
    /// Ammonia, bar.
    pub nh3: f64,
    /// Sulphur dioxide, bar.
    pub so2: f64,
    /// Oxygen, bar.
    pub o2: f64,
    /// Argon, bar: it absorbs nothing the column counts, weighs, scatters
    /// and broadens the lines as air does (a departure,
    /// `docs/science/radiative-transfer.md`), and has a monatomic gas's
    /// heat capacity.
    pub ar: f64,
}

impl PartialPressures {
    /// Surface pressure, bar: the sum of the partial pressures, each
    /// clamped at zero (a NaN part counts as zero).
    pub fn total(&self) -> f64 {
        [
            self.n2, self.co2, self.h2o, self.ch4, self.nh3, self.so2, self.o2, self.ar,
        ]
        .into_iter()
        .map(|p| if p > 0.0 { p } else { 0.0 })
        .sum()
    }
}

/// Source: molar mass of N2, 2 x 14.007 g/mol, in kg/mol (IUPAC 2021
/// standard atomic weights, abridged: Prohaska et al. 2022, Pure Appl.
/// Chem. 94, 573, doi:10.1515/pac-2019-0603; legacy `spectrum.rs`).
pub const N2_MOLAR_MASS: f64 = 28.014e-3;
/// Source: molar mass of CO2, 12.011 + 2 x 15.999 g/mol, in kg/mol (IUPAC
/// 2021 standard atomic weights, abridged, doi:10.1515/pac-2019-0603).
pub const CO2_MOLAR_MASS: f64 = 44.009e-3;
/// Source: molar mass of H2O, 2 x 1.008 + 15.999 g/mol, in kg/mol (IUPAC
/// 2021 standard atomic weights, abridged, doi:10.1515/pac-2019-0603).
pub const H2O_MOLAR_MASS: f64 = 18.015e-3;
/// Source: molar mass of CH4, 12.011 + 4 x 1.008 g/mol, in kg/mol (IUPAC
/// 2021 standard atomic weights, abridged, doi:10.1515/pac-2019-0603).
pub const CH4_MOLAR_MASS: f64 = 16.043e-3;
/// Source: molar mass of NH3, 14.007 + 3 x 1.008 g/mol, in kg/mol (IUPAC
/// 2021 standard atomic weights, abridged, doi:10.1515/pac-2019-0603).
pub const NH3_MOLAR_MASS: f64 = 17.031e-3;
/// Source: molar mass of SO2, 32.06 + 2 x 15.999 g/mol, in kg/mol (IUPAC
/// 2021 standard atomic weights, abridged, doi:10.1515/pac-2019-0603).
pub const SO2_MOLAR_MASS: f64 = 64.058e-3;
/// Source: molar mass of O2, 2 x 15.999 g/mol, in kg/mol (IUPAC 2021
/// standard atomic weights, abridged, doi:10.1515/pac-2019-0603).
pub const O2_MOLAR_MASS: f64 = 31.998e-3;
/// Source: molar mass of O3, 3 x 15.999 g/mol, in kg/mol (IUPAC 2021
/// standard atomic weights, abridged, doi:10.1515/pac-2019-0603).
pub const O3_MOLAR_MASS: f64 = 47.997e-3;
/// Source: molar mass of N2O, 2 x 14.007 + 15.999 g/mol, in kg/mol (IUPAC
/// 2021 standard atomic weights, abridged, doi:10.1515/pac-2019-0603).
pub const N2O_MOLAR_MASS: f64 = 44.013e-3;
/// Source: molar mass of Ar, 39.95 g/mol, in kg/mol (CIAAW abridged
/// standard atomic weights 2024, after the IUPAC Atomic Weights 2021,
/// ciaaw.org/abridged-atomic-weights.htm, read 2026-11-23). The air's
/// argon is 99.6 % argon-40 (39.962 g/mol): the 0.03 % between them is
/// left out.
pub const AR_MOLAR_MASS: f64 = 39.95e-3;

/// Source: molar heat capacity of N2 gas at 298.15 K, 29.12 J/(mol K)
/// (Shomate equation, NIST WebBook, Chase 1998, NIST-JANAF Thermochemical
/// Tables, 4th ed.; 29.124 evaluated, audit of the legacy column).
pub const N2_HEAT_CAPACITY: f64 = 29.12;
/// Source: molar heat capacity of CO2 gas at 298.15 K, 37.13 J/(mol K)
/// (Shomate equation, NIST WebBook, Chase 1998).
pub const CO2_HEAT_CAPACITY: f64 = 37.13;
/// Source: molar heat capacity of CH4 gas at 298.15 K, 35.69 J/(mol K)
/// (NIST WebBook, Chase 1998, as stated there).
pub const CH4_HEAT_CAPACITY: f64 = 35.69;
/// Source: molar heat capacity of NH3 gas at 298.15 K, 35.65 J/(mol K)
/// (Shomate equation, NIST WebBook, Chase 1998; the legacy 35.06 was a
/// transcription error, audit of the legacy column).
pub const NH3_HEAT_CAPACITY: f64 = 35.65;
/// Source: molar heat capacity of SO2 gas at 298.15 K, 39.87 J/(mol K)
/// (Shomate equation, NIST WebBook, Chase 1998).
pub const SO2_HEAT_CAPACITY: f64 = 39.87;
/// Source: molar heat capacity of O2 gas at 298.15 K, 29.38 J/(mol K)
/// (Shomate equation, NIST WebBook, Chase 1998).
pub const O2_HEAT_CAPACITY: f64 = 29.38;
/// Source: molar heat capacity of Ar gas, 20.786 J/(mol K) from 298 to
/// 6000 K (Shomate equation, NIST WebBook, Chase 1998, whose other terms
/// are under 1e-6 of it): 5/2 R, a monatomic gas, which stores no heat in
/// rotation.
pub const AR_HEAT_CAPACITY: f64 = 20.786;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_pressures_ignore_negative_parts() {
        let gases = PartialPressures {
            co2: 1.0,
            h2o: -1e-12,
            ch4: f64::NAN,
            ..PartialPressures::default()
        };
        assert_eq!(gases.total(), 1.0);
    }
}
