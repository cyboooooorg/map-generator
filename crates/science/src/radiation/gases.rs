//! The gases of an atmosphere: their partial pressures, their weights,
//! molar masses and heat capacities.

use crate::units::Bar;

/// Partial pressures of the gases at the surface, bar: each gas's share
/// of the moles times the pressure, `p_i = x_i p` (Dalton's law), so
/// that `p_i / p` is its mole fraction. The column and the shortwave
/// budget read them so (the column's mean molar mass is `Σ p_i M_i / p`).
/// The surface's laws (`crate::surface`: `condense`, `liquid`, `frost`,
/// `cold_trapped`) take [`Weights`] and convert inside, and an air kept
/// as the weights of its gases is read through
/// [`Weights::partial_pressures`] (issue #749).
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

    /// The weights of the air whose partial pressures are `self`, the
    /// inverse of [`Weights::partial_pressures`]: each dry gas's share of
    /// the dry air's mass times the dry air's pressure,
    ///
    /// ```text
    /// w_i = p_i M_i / M̄,   M̄ = Σ_j p_j M_j / Σ_j p_j,
    /// ```
    ///
    /// the sums over the dry gases (all but water). A definition, as
    /// [`Weights::partial_pressures`]: the mass of a gas is its moles
    /// times its molar mass. The way a measured air, given by its shares
    /// by volume, becomes an air as the system keeps it. The vapour `h2o`
    /// passes through; a negative or NaN part counts as none.
    pub fn weights(&self) -> Weights {
        let part = |p: f64| if p > 0.0 { p } else { 0.0 };
        let dry = [
            (part(self.n2), N2_MOLAR_MASS),
            (part(self.co2), CO2_MOLAR_MASS),
            (part(self.ch4), CH4_MOLAR_MASS),
            (part(self.nh3), NH3_MOLAR_MASS),
            (part(self.so2), SO2_MOLAR_MASS),
            (part(self.o2), O2_MOLAR_MASS),
            (part(self.ar), AR_MOLAR_MASS),
        ];
        let pressure: f64 = dry.iter().map(|(p, _)| p).sum();
        let mass: f64 = dry.iter().map(|(p, m)| p * m).sum();
        let w = |(p, m): (f64, f64)| {
            if mass > 0.0 {
                p * m / mass * pressure
            } else {
                0.0
            }
        };
        Weights {
            n2: w(dry[0]),
            co2: w(dry[1]),
            h2o: self.h2o,
            ch4: w(dry[2]),
            nh3: w(dry[3]),
            so2: w(dry[4]),
            o2: w(dry[5]),
            ar: w(dry[6]),
        }
    }
}

/// The weights of the gases of an air on a body's surface, bar: each
/// gas's mass times the surface gravity over the surface's area,
/// `w_i = m_i g / (4 π R²)` ([`crate::outgassing::column_pressure`]), how
/// the system keeps an air, its chain and its climate alike
/// (`docs/science/planet-atmospheres.md`, "Weights and partial
/// pressures"). Weights add as masses do, and the dry air's surface
/// pressure is the sum of its gases' weights; but a gas's weight is its
/// partial pressure only when its molar mass is the air's mean, and a
/// partial pressure moves when any other gas comes or goes, where a
/// weight does not. A law that reads a partial pressure reads
/// [`Weights::partial_pressures`] of the air as it is when it reads it.
///
/// The water is kept apart. `h2o` is the vapour's weight, which the
/// column takes as its pressure at the surface, added to the dry air's
/// (`column::outgoing_longwave`: `p = Σ w_dry + e`), so that the surface
/// pressure is still the sum of the weights; it passes through
/// [`Weights::partial_pressures`] unchanged. A wet climate sets it to the
/// saturation pressure at its surface (`surface_vapour`), the vapour
/// weighing what it presses; a runaway's steam is the weight of the water
/// in its air. That is the column's convention, exact for a pure steam
/// air: in a mixed air the vapour's share of the moles would make its
/// partial pressure `M̄ / M_H2O` of its weight (a departure, issue #749,
/// `docs/science/planet-atmospheres.md`, "Weights and partial
/// pressures").
///
/// A negative or NaN part counts as none, as in [`PartialPressures`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Weights {
    /// Nitrogen, bar.
    pub n2: f64,
    /// Carbon dioxide, bar.
    pub co2: f64,
    /// Water vapour, bar: its weight, taken as its pressure at the
    /// surface (see [`Weights`]).
    pub h2o: f64,
    /// Methane, bar.
    pub ch4: f64,
    /// Ammonia, bar.
    pub nh3: f64,
    /// Sulphur dioxide, bar.
    pub so2: f64,
    /// Oxygen, bar.
    pub o2: f64,
    /// Argon, bar.
    pub ar: f64,
}

impl Weights {
    /// The dry gases, each clamped at zero (a NaN part counts as none),
    /// with its molar mass, in the order N₂, CO₂, CH₄, NH₃, SO₂, O₂, Ar.
    fn dry(&self) -> [(f64, f64); 7] {
        let part = |w: f64| if w > 0.0 { w } else { 0.0 };
        [
            (part(self.n2), N2_MOLAR_MASS),
            (part(self.co2), CO2_MOLAR_MASS),
            (part(self.ch4), CH4_MOLAR_MASS),
            (part(self.nh3), NH3_MOLAR_MASS),
            (part(self.so2), SO2_MOLAR_MASS),
            (part(self.o2), O2_MOLAR_MASS),
            (part(self.ar), AR_MOLAR_MASS),
        ]
    }

    /// Surface pressure, bar: the sum of the weights and the vapour, each
    /// clamped at zero; the [`PartialPressures::total`] of
    /// [`Weights::partial_pressures`].
    pub fn total(&self) -> f64 {
        let vapour = if self.h2o > 0.0 { self.h2o } else { 0.0 };
        self.dry().iter().map(|(w, _)| w).sum::<f64>() + vapour
    }

    /// The partial pressures of the air whose gases weigh `self`: each
    /// dry gas's share of the dry air's moles times the dry air's
    /// pressure, the sum of the dry weights,
    ///
    /// ```text
    /// p_i = x_i Σ_j w_j,   x_i = (w_i / M_i) / Σ_j (w_j / M_j),
    /// ```
    ///
    /// the sums over the dry gases (all but water), `M_i` their molar
    /// masses ([`N2_MOLAR_MASS`] and the others). A definition: the moles
    /// of a gas are its mass over its molar mass, and a column's weight
    /// is its pressure. It keeps the dry air's pressure, and a gas of the
    /// air's mean molar mass keeps its weight. The vapour `h2o` passes
    /// through. No dry air gives none of each.
    pub fn partial_pressures(&self) -> PartialPressures {
        let dry = self.dry();
        let pressure: f64 = dry.iter().map(|(w, _)| w).sum();
        let moles: f64 = dry.iter().map(|(w, m)| w / m).sum();
        let p = |(w, m): (f64, f64)| {
            if moles > 0.0 {
                w / m / moles * pressure
            } else {
                0.0
            }
        };
        PartialPressures {
            n2: p(dry[0]),
            co2: p(dry[1]),
            h2o: self.h2o,
            ch4: p(dry[2]),
            nh3: p(dry[3]),
            so2: p(dry[4]),
            o2: p(dry[5]),
            ar: p(dry[6]),
        }
    }

    /// The weight, bar, at which a dry gas of molar mass `molar_mass`
    /// (kg/mol) holds the partial pressure `cap` in an air of the other
    /// dry gases `others` (whose own entry for that gas, and whose
    /// vapour, are not read: pass them as none), their weights held: the
    /// positive root of
    ///
    /// ```text
    /// w² + (B − cap) w − cap A M = 0,   A = Σ_j w_j / M_j,   B = Σ_j w_j,
    /// ```
    ///
    /// the sums over the other dry gases. A definition: with `w` of the
    /// gas, its partial pressure is `(w/M) / (A + w/M) · (B + w)`
    /// ([`Weights::partial_pressures`]), which rises with `w`; setting it
    /// to `cap` gives the quadratic (`docs/science/planet-atmospheres.md`,
    /// "Weights and partial pressures"). The root is taken in the form
    /// that does not cancel when `B` exceeds `cap`. It is how much of a
    /// condensing gas an air keeps when the gas's partial pressure may
    /// not pass its saturation `cap`: a weight over it condenses. A pure
    /// gas keeps `cap`. No pressure (`cap` zero, negative or NaN) keeps
    /// none; an infinite `cap` (past the critical point) keeps any.
    pub fn weight_at(others: &Weights, molar_mass: f64, cap: Bar) -> Bar {
        let cap = cap.0;
        if cap.is_nan() || cap <= 0.0 {
            return Bar(0.0);
        }
        if cap.is_infinite() {
            return Bar(f64::INFINITY);
        }
        let dry = others.dry();
        let a: f64 = dry.iter().map(|(w, m)| w / m).sum();
        let b: f64 = dry.iter().map(|(w, _)| w).sum();
        let c = cap * a * molar_mass;
        let d = b - cap;
        let root = (d * d + 4.0 * c).sqrt();
        Bar(if d > 0.0 {
            2.0 * c / (d + root)
        } else {
            (root - d) / 2.0
        })
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

    /// Source: study 37's reference Earth's dry air by weight, bar: 0.746
    /// of N₂, 0.228 of O₂ and 0.01294 of argon, with the `Modern` methane
    /// law's 3.6782e-7 bar around the Sun (`life_gases::modern_methane`),
    /// which study 37 reads as 672.8 ppb by moles
    /// (`docs/science/life-gases.md`, "In the app").
    const STUDY_37_AIR: Weights = Weights {
        n2: 0.746,
        co2: 0.0,
        h2o: 0.0,
        ch4: 3.6782e-7,
        nh3: 0.0,
        so2: 0.0,
        o2: 0.228,
        ar: 0.012_94,
    };

    /// Source: study 37's methane in [`STUDY_37_AIR`], 672.8 ppb by
    /// moles, to its 0.1 ppb.
    const STUDY_37_PPB: (f64, f64) = (672.8, 0.05);

    #[test]
    fn weights_read_as_study_37_reads_its_methane() {
        let air = STUDY_37_AIR.partial_pressures();
        let ppb = air.ch4 / air.total() * 1e9;
        let (expected, within) = STUDY_37_PPB;
        assert!((ppb - expected).abs() <= within, "{ppb} ppb");
    }

    #[test]
    fn weights_keep_the_pressure_and_a_pure_gas() {
        // Definition: the dry air's pressure is the sum of its weights,
        // a lone gas is its own partial pressure, and the vapour passes
        // through.
        let air = Weights {
            h2o: 0.02,
            ..STUDY_37_AIR
        };
        let p = air.partial_pressures();
        let dry = 0.746 + 0.228 + 0.012_94 + 3.6782e-7;
        assert!((p.total() - p.h2o - dry).abs() < 1e-12, "{p:?}");
        assert_eq!(p.h2o, 0.02);
        // A gas lighter than the mean gains, a heavier one loses.
        assert!(p.ch4 > STUDY_37_AIR.ch4 && p.n2 > STUDY_37_AIR.n2);
        assert!(p.o2 < STUDY_37_AIR.o2 && p.ar < STUDY_37_AIR.ar);
        let pure = Weights {
            co2: 92.0,
            ..Weights::default()
        };
        assert_eq!(pure.partial_pressures().co2, 92.0);
    }

    /// Source: Earth's dry air by volume, 78.08 % N₂, 20.95 % O₂ and
    /// 9340 ppm of argon (NASA Earth fact sheet), whose mass is 0.7551 N₂
    /// and 1.288 % argon (`docs/science/planet-atmospheres.md`, "Earth's
    /// N₂ target" and "Earth's argon, measured", at the fact sheet's
    /// 28.97 g/mol), to their last digit.
    const EARTH_BY_VOLUME: PartialPressures = PartialPressures {
        n2: 0.7808,
        co2: 0.0,
        h2o: 0.0,
        ch4: 0.0,
        nh3: 0.0,
        so2: 0.0,
        o2: 0.2095,
        ar: 9340e-6,
    };
    /// Source: see [`EARTH_BY_VOLUME`]: the mass shares of N₂ and argon.
    const EARTH_BY_MASS: [f64; 2] = [0.7551, 0.012_88];

    /// Definition: the tolerance on [`EARTH_BY_MASS`], 1e-3 of the share:
    /// the three gases' mean molar mass is 28.960 g/mol, where the fact
    /// sheet's 28.97 counts the 0.04 % of the air left out (CO₂, neon).
    const MEAN_MOLAR_MASS_GAP: f64 = 1e-3;

    #[test]
    fn a_measured_air_weighs_its_mass_shares() {
        let w = EARTH_BY_VOLUME.weights();
        let total = w.total();
        assert!((total - EARTH_BY_VOLUME.total()).abs() < 1e-15, "{w:?}");
        for (share, expected) in [w.n2 / total, w.ar / total].into_iter().zip(EARTH_BY_MASS) {
            assert!(
                (share / expected - 1.0).abs() <= MEAN_MOLAR_MASS_GAP,
                "{share} against {expected}"
            );
        }
        // And back: the inverse of `partial_pressures`.
        let back = w.partial_pressures();
        for (a, b) in [(back.n2, 0.7808), (back.o2, 0.2095), (back.ar, 9340e-6)] {
            assert!((a / b - 1.0).abs() < 1e-12, "{back:?}");
        }
    }

    /// Definition: the closed form's check worked by hand in
    /// `docs/science/planet-atmospheres.md` ("Weights and partial
    /// pressures"): over a Titan's 1.422 bar of N₂ by weight, a methane
    /// capped at 0.08 bar keeps 0.0469 bar by weight (the quadratic
    /// `w² + 1.342 w − 0.06515 = 0`), where scaling its weight by its
    /// partial pressure's share would leave it 40 % over the cap.
    const TITAN_CHECK: (f64, f64, f64) = (1.422, 0.08, 0.0469);

    #[test]
    fn the_weight_at_a_cap_reads_back_the_cap() {
        let (n2, cap, kept) = TITAN_CHECK;
        let others = Weights {
            n2,
            ..Weights::default()
        };
        let w = Weights::weight_at(&others, CH4_MOLAR_MASS, Bar(cap)).0;
        assert!((w - kept).abs() < 5e-5, "{w}");
        let read = Weights { ch4: w, ..others }.partial_pressures().ch4;
        assert!((read / cap - 1.0).abs() < 1e-12, "{read}");
        // A cap over the others' weight (the other branch of the root),
        // a pure gas, and the ends.
        let thin = Weights {
            n2: 1e-3,
            ar: 1e-4,
            ..Weights::default()
        };
        let w = Weights::weight_at(&thin, CO2_MOLAR_MASS, Bar(0.5)).0;
        let read = Weights { co2: w, ..thin }.partial_pressures().co2;
        assert!((read / 0.5 - 1.0).abs() < 1e-12, "{read}");
        let pure = Weights::weight_at(&Weights::default(), N2_MOLAR_MASS, Bar(1.4e-5)).0;
        assert_eq!(pure, 1.4e-5);
        assert_eq!(Weights::weight_at(&others, CH4_MOLAR_MASS, Bar(0.0)).0, 0.0);
        assert_eq!(
            Weights::weight_at(&others, CH4_MOLAR_MASS, Bar(f64::NAN)).0,
            0.0
        );
        assert!(
            Weights::weight_at(&others, CH4_MOLAR_MASS, Bar(f64::INFINITY))
                .0
                .is_infinite()
        );
    }

    #[test]
    fn weights_ignore_negative_parts_and_no_air_is_none() {
        let air = Weights {
            n2: 1.0,
            ch4: f64::NAN,
            ar: -1e-12,
            ..Weights::default()
        };
        let p = air.partial_pressures();
        assert_eq!((p.n2, p.ch4, p.ar), (1.0, 0.0, 0.0));
        assert_eq!(
            Weights::default().partial_pressures(),
            PartialPressures::default()
        );
    }
}
