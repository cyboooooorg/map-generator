//! Rayleigh scattering of the gases: each gas's cross-section from its
//! refractive index and King factor, and the optical depth of a column.
//!
//! The reference is `docs/science/radiative-transfer.md`, "Rayleigh
//! scattering" and "Per-gas data".
//!
//! `sigma = 24 pi^3 nu^4 ((n^2 - 1)/(n^2 + 2) / N)^2 F_K` (He et al. 2021,
//! Atmos. Chem. Phys. 21, 14927, doi:10.5194/acp-21-14927-2021, eq. 1,
//! after Sneep & Ubachs 2005), with `N` the number density at which `n`
//! was measured: the Lorentz-Lorenz ratio over `N` does not depend on it.
//! This replaces legacy `RAYLEIGH_N2_550`, `CO2_RAYLEIGH_PER_N2` and its
//! polarizability table, none of which is ported.

use std::f64::consts::PI;

use crate::constants::{AVOGADRO, LOSCHMIDT, PASCALS_PER_BAR, SPEED_OF_LIGHT};
use crate::radiation::gases::{
    CH4_MOLAR_MASS, CO2_MOLAR_MASS, H2O_MOLAR_MASS, N2_MOLAR_MASS, NH3_MOLAR_MASS, O2_MOLAR_MASS,
    PartialPressures, SO2_MOLAR_MASS,
};

/// A gas that scatters: the seven of [`PartialPressures`], and argon,
/// which has no field and serves only the dry-air check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Gas {
    N2,
    O2,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "only the dry-air check against Bucholtz 1995 reads it"
        )
    )]
    Ar,
    Co2,
    Ch4,
    H2o,
    Nh3,
    So2,
}

/// Definition: the reference wavelength of the visible, 550 nm, in m.
const WAVELENGTH_550_M: f64 = 550e-9;

/// Source: number density of an ideal gas at 288.15 K and 1013.25 hPa,
/// 2.546 899e19 cm^-3 (He et al. 2021, Table 1 note), at which the
/// refractive indices of N₂, Ar (15 °C, 760 Torr), CO₂ and CH₄ are given.
const DENSITY_288_CM3: f64 = 2.546_899e19;

/// Definition: [`LOSCHMIDT`] in cm^-3, the density of the refractive
/// indices given at 0 °C and 1013.25 hPa (O₂, NH₃, SO₂).
const DENSITY_273_CM3: f64 = LOSCHMIDT * 1e-6;

/// Source: N₂, `10^8 (n - 1) = A + B / (C - s^2)`, `s` in µm^-1, 468-2059
/// nm, 15 °C and 760 Torr (Peck & Khanna 1966, J. Opt. Soc. Am. 56, 1059,
/// doi:10.1364/JOSA.56.001059, abstract): A, B, C.
const N2_INDEX: [f64; 3] = [6497.378, 3_073_864.9, 144.0];

/// Source: N₂ King factor `F_K = a + b nu^2`, nu in cm^-1 (Bates 1984,
/// Planet. Space Sci. 32, 785, read in He et al. 2021, Table 1): a, b.
const N2_KING: [f64; 2] = [1.034, 3.17e-12];

/// Source: O₂, `10^8 (n - 1) = A + B / (C - nu^2)`, nu in cm^-1, 288-546
/// nm, 0 °C and 1013.25 hPa (He et al. 2021, Table 1, after Bates 1984 and
/// Sneep & Ubachs 2005): A, B, C.
const O2_INDEX: [f64; 3] = [20_564.8, 2.480_899e13, 4.09e9];

/// Source: O₂ King factor `F_K = a + b nu^2 + c nu^4`, nu in cm^-1 (He et
/// al. 2021, Table 1, after Bates 1984): a, b, c.
const O2_KING: [f64; 3] = [1.09, 1.385e-11, 1.448e-20];

/// Source: Ar, `10^7 (n - 1) = A + B / (C - s^2)`, `s` in µm^-1, 468-2059
/// nm, 15 °C and 760 Torr (Peck & Fisher 1964, J. Opt. Soc. Am. 54, 1362,
/// doi:10.1364/JOSA.54.001362, abstract); its King factor is 1, an atom
/// (He et al. 2021, §2.3): A, B, C.
const AR_INDEX: [f64; 3] = [643.2135, 286_060.21, 144.0];

/// Source: molar mass of Ar, 39.95 g/mol, in kg/mol (CIAAW abridged
/// standard atomic weights 2024, after the IUPAC Atomic Weights 2021,
/// ciaaw.org/abridged-atomic-weights.htm, read 2026-11-23).
const AR_MOLAR_MASS: f64 = 39.95e-3;

/// Source: CO₂, `n - 1 = 1.1427e3 sum_i a_i / (b_i^2 - nu^2)`, nu in
/// cm^-1, 15 °C and 1013.25 hPa (He et al. 2021, Table 1, after
/// Bideau-Mehu et al. 1973 and Alms et al. 1975; their measurements agree
/// from 333 to 725 nm): the factor, then each (a_i, b_i).
const CO2_INDEX: (f64, [(f64, f64); 5]) = (
    1.1427e3,
    [
        (5799.25, 128_908.9),
        (120.05, 89_223.8),
        (5.3334, 75_037.5),
        (4.3244, 67_837.7),
        (1.218_145e-5, 2418.136),
    ],
);

/// Source: CO₂ King factor `F_K = a + b nu^2`, nu in cm^-1 (He et al.
/// 2021, Table 1, after Alms et al. 1975): a, b.
const CO2_KING: [f64; 2] = [1.1364, 2.53e-11];

/// Source: CH₄, `10^8 (n - 1) = A + B / (C - nu^2)`, nu in cm^-1, 250-650
/// nm, 15 °C and 1013.25 hPa, King factor 1 (He et al. 2021, Table 1,
/// after Sneep & Ubachs 2005 and Wilmouth & Sayres 2020): A, B, C.
const CH4_INDEX: [f64; 3] = [4869.8, 4.1023e14, 1.133e10];

/// Source: NH₃, `n - 1 = A / (B - f^2)`, f in s^-1, 480-671 nm, ideal gas
/// at 0 °C and 760 mm (Cuthbertson & Cuthbertson 1914, Phil. Trans. R.
/// Soc. A 213, 1, doi:10.1098/rsta.1914.0001, p. 22): A, B.
const NH3_INDEX: [f64; 2] = [2.9658e27, 8135.3e27];

/// Source: SO₂, `n - 1 = A / (B - f^2)`, f in s^-1, 500-670 nm, ideal gas
/// at 0 °C and 760 mm (Cuthbertson & Cuthbertson 1910, Proc. R. Soc. A 83,
/// 171, doi:10.1098/rspa.1910.0003, p. 172): A, B.
const SO2_INDEX: [f64; 2] = [5.728e27, 8929e27];

/// Source: NH₃ static polarizability, mean and anisotropy, bohr^3
/// (Chong 2018, Can. J. Chem. 96, 934, doi:10.1139/cjc-2018-0171, Table 4,
/// CCSD(T)/daug-cc-pVTZ): King factor 1.0032.
const NH3_POLARIZABILITY: [f64; 2] = [14.16, 1.71];

/// Source: SO₂ static polarizability, mean and anisotropy, bohr^3 (Chong
/// 2018, Table 4, CCSD(T)): King factor 1.062, uncertain by the 2.4 % on
/// sigma between Chong's two methods.
const SO2_POLARIZABILITY: [f64; 2] = [26.04, 13.74];

/// Source: H₂O principal polarizabilities at 514.5 nm, Å^3 (Murphy 1977,
/// J. Chem. Phys. 67, 5877, doi:10.1063/1.434794, abstract): King factor
/// 1.0010.
const H2O_POLARIZABILITY: [f64; 3] = [1.468, 1.415, 1.528];

/// Source: IAPWS R9-97 (Release on the Refractive Index of Ordinary Water
/// Substance, 1997), Table 1: a0, a2, a3, a4, a5, a6 of `L` (a1 and a7
/// multiply the reduced density and vanish in a vapour).
const H2O_L: [f64; 6] = [
    0.244_257_733,
    -3.732_349_96e-3,
    2.686_784_72e-4,
    1.589_205_70e-3,
    2.459_342_59e-3,
    0.900_704_920,
];

/// Source: IAPWS R9-97, Table 1: the reduced ultraviolet and infrared
/// resonance wavelengths, lambda_UV and lambda_IR.
const H2O_RESONANCES: [f64; 2] = [0.229_202_0, 5.432_937];

/// Definition: IAPWS R9-97's reference density, temperature and
/// wavelength: 1000 kg/m^3, 273.15 K, 0.589 µm (in m).
const H2O_REFERENCE: [f64; 3] = [1000.0, 273.15, 0.589e-6];

/// Definition: the temperature of the vapour's refractive index, 288.15 K,
/// the 15 °C of the other gases' references (the page's 3.335e-27 cm²).
const H2O_TEMPERATURE_K: f64 = 288.15;

/// Cross-section of one molecule of `gas` at `wavelength_m`, cm².
pub(crate) fn cross_section_cm2(gas: Gas, wavelength_m: f64) -> f64 {
    let nu = 0.01 / wavelength_m;
    let s2 = (1e-6 / wavelength_m).powi(2);
    let f2 = (SPEED_OF_LIGHT / wavelength_m).powi(2);
    let nu2 = nu * nu;
    // The Lorentz-Lorenz ratio per molecule, cm^3, and the King factor.
    let (per_molecule, king) = match gas {
        Gas::N2 => {
            let [a, b, c] = N2_INDEX;
            let [k0, k2] = N2_KING;
            let n1 = (a + b / (c - s2)) * 1e-8;
            (lorentz_lorenz(n1) / DENSITY_288_CM3, k0 + k2 * nu2)
        }
        Gas::O2 => {
            let [a, b, c] = O2_INDEX;
            let [k0, k2, k4] = O2_KING;
            let n1 = (a + b / (c - nu2)) * 1e-8;
            (
                lorentz_lorenz(n1) / DENSITY_273_CM3,
                k0 + k2 * nu2 + k4 * nu2 * nu2,
            )
        }
        Gas::Ar => {
            let [a, b, c] = AR_INDEX;
            let n1 = (a + b / (c - s2)) * 1e-7;
            (lorentz_lorenz(n1) / DENSITY_288_CM3, 1.0)
        }
        Gas::Co2 => {
            let (factor, terms) = CO2_INDEX;
            let [k0, k2] = CO2_KING;
            let n1 = factor * terms.iter().map(|&(a, b)| a / (b * b - nu2)).sum::<f64>();
            (lorentz_lorenz(n1) / DENSITY_288_CM3, k0 + k2 * nu2)
        }
        Gas::Ch4 => {
            let [a, b, c] = CH4_INDEX;
            let n1 = (a + b / (c - nu2)) * 1e-8;
            (lorentz_lorenz(n1) / DENSITY_288_CM3, 1.0)
        }
        Gas::Nh3 => {
            let [a, b] = NH3_INDEX;
            let [mean, anisotropy] = NH3_POLARIZABILITY;
            (
                lorentz_lorenz(a / (b - f2)) / DENSITY_273_CM3,
                king_factor(mean, anisotropy),
            )
        }
        Gas::So2 => {
            let [a, b] = SO2_INDEX;
            let [mean, anisotropy] = SO2_POLARIZABILITY;
            (
                lorentz_lorenz(a / (b - f2)) / DENSITY_273_CM3,
                king_factor(mean, anisotropy),
            )
        }
        Gas::H2o => (water_per_molecule_cm3(wavelength_m), water_king_factor()),
    };
    24.0 * PI.powi(3) * nu2 * nu2 * per_molecule * per_molecule * king
}

/// `(n^2 - 1) / (n^2 + 2)` from the refractivity `n - 1`.
fn lorentz_lorenz(refractivity: f64) -> f64 {
    let n2 = (1.0 + refractivity).powi(2);
    (n2 - 1.0) / (n2 + 2.0)
}

/// King factor from a mean polarizability and its anisotropy:
/// `F_K = 1 + 2 eps / 9`, `eps = (gamma / alpha)^2` (Chance 2012, EPS 238
/// notes 7, §9; Bodhaine et al. 1999, eq. 2; the page's "King factor from a
/// depolarization ratio or an anisotropy").
fn king_factor(mean: f64, anisotropy: f64) -> f64 {
    1.0 + 2.0 * (anisotropy / mean).powi(2) / 9.0
}

/// Water's King factor from Murphy's principal polarizabilities: the
/// anisotropy `gamma^2 = ((a-b)^2 + (b-c)^2 + (c-a)^2) / 2` (Chong 2018,
/// eq. 1), so 1.0010.
fn water_king_factor() -> f64 {
    let [a, b, c] = H2O_POLARIZABILITY;
    let mean = (a + b + c) / 3.0;
    let anisotropy = (((a - b).powi(2) + (b - c).powi(2) + (c - a).powi(2)) / 2.0).sqrt();
    king_factor(mean, anisotropy)
}

/// The vapour's Lorentz-Lorenz ratio per molecule, cm^3: in the
/// low-density limit of IAPWS R9-97 eq. (1), `rho_bar / N = m / rho*`, so
/// it is `m L(0, T, lambda) / rho*` with `m` one molecule's mass.
fn water_per_molecule_cm3(wavelength_m: f64) -> f64 {
    let [a0, a2, a3, a4, a5, a6] = H2O_L;
    let [uv, ir] = H2O_RESONANCES;
    let [density, temperature, wavelength] = H2O_REFERENCE;
    let t = H2O_TEMPERATURE_K / temperature;
    let l2 = (wavelength_m / wavelength).powi(2);
    let l = a0 + a2 * t + a3 * l2 * t + a4 / l2 + a5 / (l2 - uv * uv) + a6 / (l2 - ir * ir);
    let molecule_kg = H2O_MOLAR_MASS / AVOGADRO;
    molecule_kg / density * l * 1e6
}

impl Gas {
    /// Molar mass, kg/mol.
    fn molar_mass(self) -> f64 {
        match self {
            Gas::N2 => N2_MOLAR_MASS,
            Gas::O2 => O2_MOLAR_MASS,
            Gas::Ar => AR_MOLAR_MASS,
            Gas::Co2 => CO2_MOLAR_MASS,
            Gas::Ch4 => CH4_MOLAR_MASS,
            Gas::H2o => H2O_MOLAR_MASS,
            Gas::Nh3 => NH3_MOLAR_MASS,
            Gas::So2 => SO2_MOLAR_MASS,
        }
    }
}

/// Rayleigh optical depth at 550 nm of the whole air above the surface,
/// `sum_i sigma_i(550) p_i N_A / (g mu)`, with `mu` the column's mean molar
/// mass (the page's `tau_R = sigma p / (mu m_u g)`). A negative or NaN
/// part counts as none; no air gives 0. All seven gases scatter, water
/// vapour included (legacy left it out).
pub(crate) fn rayleigh_depth_550(gases: &PartialPressures, gravity_m_s2: f64) -> f64 {
    let parts = [
        (Gas::N2, gases.n2),
        (Gas::O2, gases.o2),
        (Gas::Co2, gases.co2),
        (Gas::Ch4, gases.ch4),
        (Gas::H2o, gases.h2o),
        (Gas::Nh3, gases.nh3),
        (Gas::So2, gases.so2),
    ]
    .map(|(gas, bar)| {
        (
            gas,
            if bar > 0.0 {
                bar * PASCALS_PER_BAR
            } else {
                0.0
            },
        )
    });
    column_depth(&parts, gravity_m_s2)
}

/// Depth at 550 nm of a column of `(gas, partial pressure in Pa)`: each
/// gas's molecules per cm², `p_i N_A / (g mu)` with `mu` the mean molar
/// mass, times its cross-section.
fn column_depth(parts: &[(Gas, f64)], gravity_m_s2: f64) -> f64 {
    let pressure: f64 = parts.iter().map(|&(_, p)| p).sum();
    if pressure <= 0.0 {
        return 0.0;
    }
    let mean_molar_mass = parts
        .iter()
        .map(|&(gas, p)| p * gas.molar_mass())
        .sum::<f64>()
        / pressure;
    // Molecules per m² for each pascal, then per cm².
    let per_pascal_cm2 = AVOGADRO / (gravity_m_s2 * mean_molar_mass) * 1e-4;
    parts
        .iter()
        .map(|&(gas, p)| cross_section_cm2(gas, WAVELENGTH_550_M) * p * per_pascal_cm2)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{PASCALS_PER_BAR, STANDARD_GRAVITY};
    use crate::radiation::test_points::{EARTH_AIR, TITAN_GRAVITY, TITAN_SURFACE_AIR};

    /// Definition: one unit of the page's cross-sections, 10^-27 cm².
    const UNIT_CM2: f64 = 1e-27;

    fn sigma(gas: Gas, nm: f64) -> f64 {
        cross_section_cm2(gas, nm * 1e-9) / UNIT_CM2
    }

    fn close(value: f64, expected: f64, tolerance: f64, what: &str) {
        assert!(
            (value / expected - 1.0).abs() < tolerance,
            "{what}: {value} against {expected} (tolerance {tolerance})"
        );
    }

    /// He et al. 2021 (Atmos. Chem. Phys. 21, 14927,
    /// doi:10.5194/acp-21-14927-2021), Table 2: the cross-sections they
    /// compute from these refractive indices and the ones their cavity
    /// measures, in 10^-27 cm², as the reference page lists them.
    #[test]
    fn each_gas_scatters_as_its_published_cross_section() {
        // Source: He et al. 2021, Table 2: (gas, nm, computed, measured).
        // The tolerances are a little wider than the gaps the page records:
        // 0.15-0.20 % against the computed values for CO₂ and CH₄ (not
        // traced, exact for O₂), so 0.25 %; against the measured ones,
        // +2.0 % and -1.3 % for O₂ (2.5 %), up to 0.5 % for CO₂ and CH₄
        // (0.6 %).
        let rows: [(Gas, f64, f64, f64, f64); 6] = [
            (Gas::O2, 532.0, 4.642, 4.55, 0.025),
            (Gas::Co2, 532.0, 13.32, 13.3, 0.006),
            (Gas::Ch4, 532.0, 11.34, 11.3, 0.006),
            (Gas::O2, 660.0, 1.924, 1.95, 0.025),
            (Gas::Co2, 660.0, 5.516, 5.52, 0.006),
            (Gas::Ch4, 660.0, 4.693, 4.68, 0.006),
        ];
        for (gas, nm, computed, measured, measured_tolerance) in rows {
            let value = sigma(gas, nm);
            close(
                value,
                computed,
                0.0025,
                &format!("{gas:?} at {nm} nm, He computed"),
            );
            close(
                value,
                measured,
                measured_tolerance,
                &format!("{gas:?} at {nm} nm, He measured"),
            );
        }
        // Source: Cuthbertson & Cuthbertson 1914 (Phil. Trans. R. Soc. A
        // 213, 1, doi:10.1098/rsta.1914.0001, Table VIII), steam measured
        // at 480-671 nm: 3.198e-27 cm² at 550 nm, 4.1 % under IAPWS as the
        // page records; 4.5 % is a little wider than that gap.
        /// Source: Cuthbertson & Cuthbertson 1914, Table VIII, steam,
        /// sigma(550) in 10^-27 cm², as the page derives it.
        const STEAM: f64 = 3.198;
        close(sigma(Gas::H2o, 550.0), STEAM, 0.045, "H₂O against steam");
    }

    /// The page's own sigma(550) for each row, in 10^-27 cm², to its four
    /// printed digits (half a last digit is at most 0.043 %, of CO₂'s
    /// 11.64): the eight rows computed independently by the study's script.
    #[test]
    fn each_gas_scatters_as_the_reference_page_derives() {
        // Source: `docs/science/radiative-transfer.md`, "Per-gas data",
        // column sigma(550) (He et al. 2021; Peck & Khanna 1966; Peck &
        // Fisher 1964; IAPWS R9-97; Murphy 1977; Cuthbertson & Cuthbertson
        // 1910, 1914; Chong 2018).
        let rows: [(Gas, f64); 8] = [
            (Gas::N2, 4.633),
            (Gas::O2, 4.049),
            (Gas::Ar, 3.991),
            (Gas::Co2, 11.64),
            (Gas::Ch4, 9.905),
            (Gas::H2o, 3.335),
            (Gas::Nh3, 7.191),
            (Gas::So2, 23.41),
        ];
        for (gas, page) in rows {
            close(sigma(gas, 550.0), page, 5e-4, &format!("{gas:?} at 550 nm"));
        }
    }

    /// CO₂ scatters 2.51 times what N₂ does per molecule at 550 nm (the
    /// page, from He et al. 2021); Kopparapu et al. 2013 (ApJ 765, 131,
    /// §2.2.1) write "2.5 times better than air".
    #[test]
    fn co2_scatters_two_and_a_half_times_nitrogen() {
        /// Source: the page's "Orders of magnitude", CO₂/N₂ = 2.51 (He et
        /// al. 2021), checked to its last printed digit.
        const RATIO: f64 = 2.51;
        let value = sigma(Gas::Co2, 550.0) / sigma(Gas::N2, 550.0);
        assert!((value - RATIO).abs() < 0.01, "CO₂/N₂ {value}");
    }

    /// O₂ scatters 0.874 times what N₂ does per molecule at 550 nm (He et
    /// al. 2021, Table 1 rows, whose O₂ value the page reproduces exactly
    /// in Table 2), not the audit's 0.973.
    #[test]
    fn oxygen_scatters_a_little_less_than_nitrogen() {
        /// Source: the page's "Orders of magnitude", O₂/N₂ = 0.874 (He et
        /// al. 2021), checked to its last printed digit.
        const RATIO: f64 = 0.874;
        let value = sigma(Gas::O2, 550.0) / sigma(Gas::N2, 550.0);
        assert!((value - RATIO).abs() < 0.001, "O₂/N₂ {value}");
    }

    /// Bucholtz 1995 (Appl. Opt. 34, 2765, doi:10.1364/AO.34.002765):
    /// standard air scatters 4.509e-27 cm² at 550 nm (Table 2), and the
    /// U.S. Standard Atmosphere has a depth of 0.0972 at 550 nm (Table 4).
    /// The page's dry air gives 4.508 (-0.02 %) and 0.0968 (-0.4 %): the
    /// tolerances, 0.05 % and 0.6 %, are a little wider than those gaps.
    #[test]
    fn earths_air_is_bucholtzs() {
        // Source: dry air, NASA Earth fact sheet as the page reads it:
        // 78.08 % N₂, 20.95 % O₂, 9340 ppm Ar, 420 ppm CO₂.
        let air: [(Gas, f64); 4] = [
            (Gas::N2, 0.7808),
            (Gas::O2, 0.2095),
            (Gas::Ar, 9340e-6),
            (Gas::Co2, 420e-6),
        ];
        let shares: f64 = air.iter().map(|(_, x)| x).sum();
        let mean_sigma: f64 = air
            .iter()
            .map(|&(gas, x)| x * sigma(gas, 550.0))
            .sum::<f64>()
            / shares;
        /// Source: Bucholtz 1995, Table 2, standard air at 550 nm, 10^-27 cm².
        const BUCHOLTZ_SIGMA: f64 = 4.509;
        close(mean_sigma, BUCHOLTZ_SIGMA, 5e-4, "dry air sigma(550)");

        /// Definition: standard sea-level pressure, 1013.25 hPa, bar.
        const SURFACE_BAR: f64 = 1.013_25;
        let column: Vec<(Gas, f64)> = air
            .iter()
            .map(|&(gas, x)| (gas, x / shares * SURFACE_BAR * PASCALS_PER_BAR))
            .collect();
        let depth = column_depth(&column, STANDARD_GRAVITY.0);
        /// Source: Bucholtz 1995, Table 4, U.S. Standard Atmosphere 1962,
        /// optical depth at 550 nm.
        const BUCHOLTZ_DEPTH: f64 = 0.0972;
        close(depth, BUCHOLTZ_DEPTH, 6e-3, "dry air depth at 550 nm");
    }

    /// The depth counts the molecules of the column, `p / (g mu)`: double
    /// gravity halves it; Titan's thick, light air at its weak gravity
    /// scatters as many times Earth's as its column outweighs Earth's,
    /// times what its composition scatters per unit mass against Earth's
    /// (legacy `rayleigh_counts_the_column_mass`, alb 1020).
    #[test]
    fn rayleigh_counts_the_column_mass() {
        let earth = rayleigh_depth_550(&EARTH_AIR, STANDARD_GRAVITY.0);
        let heavier = rayleigh_depth_550(&EARTH_AIR, 2.0 * STANDARD_GRAVITY.0);
        assert!((heavier / earth - 0.5).abs() < 1e-12, "{heavier} / {earth}");

        let titan = rayleigh_depth_550(&TITAN_SURFACE_AIR, TITAN_GRAVITY.0);
        let columns = (TITAN_SURFACE_AIR.total() / TITAN_GRAVITY.0)
            / (EARTH_AIR.total() / STANDARD_GRAVITY.0);
        // Source: the page's per-molecule ratios to N₂ at 550 nm (O₂
        // 0.874, CH₄ 2.14, CO₂ 2.51) and the IUPAC molar masses: what
        // each air scatters per unit mass, relative to N₂.
        let per_mass = |p: &PartialPressures| {
            let scatter = p.n2 + 0.874 * p.o2 + 2.14 * p.ch4 + 2.51 * p.co2;
            let mass = p.n2 * N2_MOLAR_MASS
                + p.o2 * O2_MOLAR_MASS
                + p.ch4 * CH4_MOLAR_MASS
                + p.co2 * CO2_MOLAR_MASS;
            scatter / mass
        };
        let composition = per_mass(&TITAN_SURFACE_AIR) / per_mass(&EARTH_AIR);
        // The page's ratios are rounded to three digits (CH₄'s 2.14 to
        // 0.12 %, weighted by its 5.65 % share): 0.1 % covers them.
        close(
            titan / earth,
            columns * composition,
            1e-3,
            "Titan over Earth",
        );
    }

    #[test]
    fn missing_or_broken_parts_scatter_nothing() {
        let g = STANDARD_GRAVITY.0;
        assert_eq!(rayleigh_depth_550(&PartialPressures::default(), g), 0.0);
        let broken = PartialPressures {
            n2: -1.0,
            co2: f64::NAN,
            ..PartialPressures::default()
        };
        assert_eq!(rayleigh_depth_550(&broken, g), 0.0);
        let with_broken = PartialPressures {
            so2: -0.3,
            h2o: f64::NAN,
            ..EARTH_AIR
        };
        assert_eq!(
            rayleigh_depth_550(&with_broken, g),
            rayleigh_depth_550(&EARTH_AIR, g)
        );
    }

    #[test]
    fn water_vapour_is_in_the_sum() {
        let dry = rayleigh_depth_550(&EARTH_AIR, STANDARD_GRAVITY.0);
        let moist = PartialPressures {
            h2o: 0.017,
            ..EARTH_AIR
        };
        assert!(rayleigh_depth_550(&moist, STANDARD_GRAVITY.0) > dry);
    }
}
