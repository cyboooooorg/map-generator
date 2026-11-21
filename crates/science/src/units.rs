//! Typed units: a quantity is a newtype over `f64`, never a bare number.
//!
//! A field stores the unit the domain thinks in (au for an orbit, solar
//! masses for a star); a conversion goes through `From` and the sourced
//! constants of [`crate::constants`]. Every unit serialises as a bare number.

use crate::constants::{
    ASTRONOMICAL_UNIT, DAY, DEGREES_PER_HALF_TURN, EARTH_EQUATORIAL_RADIUS, EARTH_MASS,
    JULIAN_YEAR_DAYS, PASCALS_PER_BAR, SOLAR_IRRADIANCE, SOLAR_LUMINOSITY, SOLAR_MASS,
    SOLAR_RADIUS, YEARS_PER_GYR,
};
use serde::{Deserialize, Serialize};
use std::ops::{Add, Div, Mul, Sub};

macro_rules! unit {
    ($(#[$doc:meta])* $name:ident, $symbol:expr) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub f64);

        impl $name {
            /// Symbol printed after a value of this unit.
            pub const SYMBOL: &'static str = $symbol;
        }

        impl Add for $name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }

        impl Sub for $name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }

        impl Mul<f64> for $name {
            type Output = Self;
            fn mul(self, rhs: f64) -> Self {
                Self(self.0 * rhs)
            }
        }

        impl Div<f64> for $name {
            type Output = Self;
            fn div(self, rhs: f64) -> Self {
                Self(self.0 / rhs)
            }
        }
    };
}

/// `From` both ways between a unit and a base unit: one `$big` is `$factor`
/// of `$small`.
macro_rules! convert {
    ($big:ident, $small:ident, $factor:expr) => {
        impl From<$big> for $small {
            fn from(value: $big) -> Self {
                Self(value.0 * $factor)
            }
        }

        impl From<$small> for $big {
            fn from(value: $small) -> Self {
                Self(value.0 / $factor)
            }
        }
    };
}

unit!(/// Temperature, kelvin.
    Kelvin, "K");
unit!(/// Pressure, pascal.
    Pascal, "Pa");
unit!(/// Mass, kilogram.
    Kilogram, "kg");
unit!(/// Length, metre.
    Metre, "m");
unit!(/// Time, second.
    Second, "s");
unit!(/// Power, watt.
    Watt, "W");
unit!(/// Irradiance, watt per square metre.
    WattPerSquareMetre, "W/m2");
unit!(/// Acceleration, metre per second squared.
    MetrePerSecondSquared, "m/s2");
unit!(/// Density, kilogram per cubic metre.
    KilogramPerCubicMetre, "kg/m3");
unit!(/// Plane angle, radian.
    Radian, "rad");
unit!(/// Magnetic flux density, tesla.
    Tesla, "T");

unit!(/// Distance, astronomical unit (IAU 2012 exact definition).
    AstronomicalUnit, "au");
unit!(/// Mass in solar masses (nominal solar mass parameter over G).
    SolarMass, "Msun");
unit!(/// Radius in solar radii (IAU 2015 nominal).
    SolarRadius, "Rsun");
unit!(/// Power in solar luminosities (IAU 2015 nominal).
    SolarLuminosity, "Lsun");
unit!(/// Mass in Earth masses (nominal Earth mass parameter over G).
    EarthMass, "Mearth");
unit!(
    /// Radius in Earth radii: the IAU 2015 nominal *equatorial* radius
    /// (6378.1 km). A law fitted against the mean radius (6371 km) must
    /// convert explicitly.
    EarthRadius, "Rearth");
unit!(/// Irradiance in Earth fluxes: one is the nominal solar irradiance at 1 au.
    EarthFlux, "Searth");
unit!(/// Pressure, bar (exactly 100 000 Pa).
    Bar, "bar");
unit!(/// Time, day of exactly 86 400 s.
    Day, "d");
unit!(/// Time, Julian year of exactly 365.25 days.
    Year, "yr");
unit!(/// Time, billion Julian years.
    Gyr, "Gyr");
unit!(/// Plane angle, degree.
    Degree, "deg");
unit!(/// Logarithmic ratio, decimal exponent (dex).
    Dex, "dex");

convert!(AstronomicalUnit, Metre, ASTRONOMICAL_UNIT.0);
convert!(SolarMass, Kilogram, SOLAR_MASS.0);
convert!(EarthMass, Kilogram, EARTH_MASS.0);
convert!(SolarRadius, Metre, SOLAR_RADIUS.0);
convert!(EarthRadius, Metre, EARTH_EQUATORIAL_RADIUS.0);
convert!(SolarLuminosity, Watt, SOLAR_LUMINOSITY.0);
convert!(EarthFlux, WattPerSquareMetre, SOLAR_IRRADIANCE.0);
convert!(Bar, Pascal, PASCALS_PER_BAR);
convert!(Day, Second, DAY.0);
convert!(Year, Second, JULIAN_YEAR_DAYS * DAY.0);
convert!(Gyr, Year, YEARS_PER_GYR);
convert!(Degree, Radian, std::f64::consts::PI / DEGREES_PER_HALF_TURN);

/// A dimensionless share between 0 and 1 inclusive. The field is private:
/// a value outside the range cannot exist, including through serde.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Fraction(f64);

impl Fraction {
    /// Symbol of a dimensionless quantity.
    pub const SYMBOL: &'static str = "1";

    /// `Some` when `value` is finite and within 0 to 1 inclusive.
    pub fn new(value: f64) -> Option<Self> {
        (value.is_finite() && (0.0..=1.0).contains(&value)).then_some(Self(value))
    }

    /// The share as a plain number.
    pub fn get(self) -> f64 {
        self.0
    }
}

/// A number outside 0 to 1 offered as a [`Fraction`]. Its `Display` is a
/// code with its parameter, never prose:
/// `fraction-out-of-range(value=1.5)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FractionOutOfRange(pub f64);

impl std::fmt::Display for FractionOutOfRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fraction-out-of-range(value={})", self.0)
    }
}

impl std::error::Error for FractionOutOfRange {}

impl TryFrom<f64> for Fraction {
    type Error = FractionOutOfRange;

    fn try_from(value: f64) -> Result<Self, FractionOutOfRange> {
        Self::new(value).ok_or(FractionOutOfRange(value))
    }
}

impl<'de> Deserialize<'de> for Fraction {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f64::deserialize(deserializer)?;
        Self::try_from(value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Source: semi-major axis of Mars, 1.523 679 au (JPL Solar System
    /// Dynamics, planetary physical parameters); any non-trivial sample
    /// would do, this one is a real orbit.
    const SAMPLE: f64 = 1.523_679;

    /// Definition: relative tolerance of a conversion round trip.
    const TOLERANCE: f64 = 1e-12;

    macro_rules! round_trip {
        ($a:ident, $b:ident) => {{
            let a = $a(SAMPLE);
            let back = $a::from($b::from(a));
            assert!((back.0 / a.0 - 1.0).abs() < TOLERANCE, stringify!($a));
            let b = $b(SAMPLE);
            let back = $b::from($a::from(b));
            assert!((back.0 / b.0 - 1.0).abs() < TOLERANCE, stringify!($b));
        }};
    }

    #[test]
    fn conversions_round_trip() {
        round_trip!(AstronomicalUnit, Metre);
        round_trip!(SolarMass, Kilogram);
        round_trip!(EarthMass, Kilogram);
        round_trip!(SolarRadius, Metre);
        round_trip!(EarthRadius, Metre);
        round_trip!(SolarLuminosity, Watt);
        round_trip!(EarthFlux, WattPerSquareMetre);
        round_trip!(Bar, Pascal);
        round_trip!(Day, Second);
        round_trip!(Year, Second);
        round_trip!(Gyr, Year);
        round_trip!(Degree, Radian);
    }

    #[test]
    fn conversions_use_the_constants() {
        assert_eq!(Metre::from(AstronomicalUnit(1.0)), ASTRONOMICAL_UNIT);
        assert_eq!(Second::from(Day(1.0)), DAY);
        assert_eq!(Kilogram::from(SolarMass(1.0)), SOLAR_MASS);
        assert!((Radian::from(Degree(180.0)).0 / std::f64::consts::PI - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn a_gyr_is_a_billion_julian_years_of_86400_s_days() {
        let s = Second::from(Year::from(Gyr(1.0))).0;
        assert!((s / 3.155_76e16 - 1.0).abs() < 1e-15);
    }

    #[test]
    fn fraction_accepts_its_bounds_and_rejects_the_rest() {
        assert!(Fraction::new(0.0).is_some() && Fraction::new(1.0).is_some());
        assert!(Fraction::new(-1e-9).is_none() && Fraction::new(1.000_001).is_none());
        assert!(Fraction::new(f64::NAN).is_none());
        assert!(serde_json::from_str::<Fraction>("1.5").is_err());
        assert_eq!(
            Fraction::try_from(1.5).unwrap_err().to_string(),
            "fraction-out-of-range(value=1.5)"
        );
        assert_eq!(
            serde_json::from_str::<Fraction>("1.5")
                .unwrap_err()
                .to_string()
                .split(" at line")
                .next()
                .unwrap(),
            "fraction-out-of-range(value=1.5)"
        );
        assert_eq!(
            serde_json::from_str::<Fraction>("0.25").unwrap().get(),
            0.25
        );
    }

    #[test]
    fn units_serialise_as_bare_numbers() {
        assert_eq!(serde_json::to_string(&Kelvin(288.0)).unwrap(), "288.0");
        let k: Kelvin = serde_json::from_str("288.0").unwrap();
        assert_eq!(k, Kelvin(288.0));
    }

    #[test]
    fn arithmetic_stays_within_the_type() {
        assert_eq!(Metre(3.0) + Metre(4.0), Metre(7.0));
        assert_eq!(Metre(3.0) - Metre(1.0), Metre(2.0));
        assert_eq!(Metre(3.0) * 2.0, Metre(6.0));
        assert_eq!(Metre(3.0) / 2.0, Metre(1.5));
        assert!(Metre(1.0) < Metre(2.0));
    }

    #[test]
    fn symbols_are_the_documented_ones() {
        assert_eq!(Kelvin::SYMBOL, "K");
        assert_eq!(WattPerSquareMetre::SYMBOL, "W/m2");
        assert_eq!(Gyr::SYMBOL, "Gyr");
        assert_eq!(Fraction::SYMBOL, "1");
    }
}
