//! The atmosphere of a rocky or icy body: what its interior outgasses,
//! what escape to space leaves of it and what its cold freezes out (part 2
//! of the physical planet model, issue #423).
//!
//! [`outgassed`] turns the volatile inventory into partial pressures by
//! regime, [`atmosphere`] keeps the share the cosmic shoreline of Zahnle &
//! Catling 2017 allows, [`methane_photolysis`] keeps the methane the
//! outgassing adds faster than the star's ultraviolet destroys it,
//! [`cold_trap`] freezes out what the equilibrium temperature cannot hold
//! as gas, and [`photochemistry`] turns ammonia into nitrogen and
//! oxidises warm methane. The carbonate thermostat (climate) and oxygen
//! (life) come after; the climate condenses methane and nitrogen at its
//! surface temperature ([`condense`]).
use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use super::climate::SurfaceLiquid;
use super::{Core, Interior, PlanetInputs, Water};
use crate::exomoon::{EARTH_KG, EARTH_M, G};
use crate::geology::Regime;

/// Pascals in a bar.
const PA_PER_BAR: f64 = 1.0e5;

/// Mass of SO₂ per mass of sulphur: 64 for 32. Stoichiometry.
const SO2_PER_S: f64 = 2.0;

/// Share of each volatile a regime brings up from the interior over the
/// body's life. Calibration, no source, within the picture of Kite,
/// Manga & Gaidos 2009 (doi:10.1088/0004-637X/700/2/1732: plates and
/// lids outgas through melting, lids less of it): plates, an episodic lid
/// (Venus's resurfacing) and heat pipes (Io) bring up nearly all of it; a
/// stagnant lid, which melts only its upper mantle, a fifth (Mars, Titan).
fn outgassed_share(regime: Regime) -> f64 {
    match regime {
        Regime::Plates | Regime::Episodic | Regime::HeatPipe => 0.95,
        Regime::Stagnant => 0.2,
    }
}

/// Pressure, bar, at the floor of a water layer at which half of the
/// volatiles its body outgasses reach the air (see
/// [`released_under_ocean`]): 3 GPa. Calibration, no source: Titan
/// (about 1.0 GPa under its 45 % of ice in the model), Ganymede (1.1 GPa)
/// and Callisto keep 98 % or more of their air, while a 1 M⊕ planet of
/// 30 % water (about 17 GPa) lets about 1e-3 of it out, which leaves it
/// the few bar of CO₂ before the thermostat that Kite & Ford 2018 find on
/// waterworlds (see [`released_under_ocean`]). A known departure: Kite &
/// Ford (Fig. 5; checked) find that a stagnant-lid planet under a 3 GPa
/// ocean does not melt at all, so 3 GPa is their full shutoff, not a half
/// point. A shutoff there leaves the 30 %-water planet no air at all
/// (frozen at 257 K at 1 AU instead of an ocean world): its air would
/// have to come from the volatiles dissolved in its ocean, which the
/// model does not carry.
const SEAFLOOR_HALF_BAR: f64 = 3.0e4;

/// Steepness of the release across [`SEAFLOOR_HALF_BAR`]: the share is
/// `1 / (1 + (P / P_half)^k)`. Calibration, no source, with
/// [`SEAFLOOR_HALF_BAR`] on the same bodies: 4.1 is the least that
/// keeps Ganymede at 98 % (4 left it 97.9 %).
const SEAFLOOR_STEEPNESS: f64 = 4.1;

/// Share of the volatiles a body outgasses that reaches the air under a
/// water layer whose floor is at `seafloor_bar`:
/// `1 / (1 + (P / P_half)^k)` ([`SEAFLOOR_HALF_BAR`],
/// [`SEAFLOOR_STEEPNESS`]).
///
/// Under a deep water layer the weight of the water on the seafloor
/// curtails the decompression melting that outgasses the mantle (Kite &
/// Ford 2018, ApJ 864, 75, doi:10.3847/1538-4357/aad6e0, for waterworlds
/// of 10 to 1000 times Earth's water: their pCO₂ often lands between 0.2
/// and 20 bar; checked), and high-pressure ice at its base seals the
/// rock below (Noack et al. 2016, Icarus 277, 215,
/// doi:10.1016/j.icarus.2016.05.009; the paper was not read, its
/// companion abstract gives no single pressure). The ice's own volatiles
/// stay dissolved in the ocean or locked in clathrates. One share for
/// both, a labelled calibration on the pressure at the floor: a runaway's
/// steam weighs on the rock as its ocean did, so the share holds in a
/// runaway too.
fn released_under_ocean(seafloor_bar: f64) -> f64 {
    1.0 / (1.0 + (seafloor_bar.max(0.0) / SEAFLOOR_HALF_BAR).powf(SEAFLOOR_STEEPNESS))
}

/// Share of the carbon, sulphur and water plates outgas that subduction
/// carries back into the mantle. Calibration, no source, set equal to
/// the nitrogen's before that was raised (no target constrains it once
/// the thermostat takes Earth's CO₂). Venus, without subduction, keeps
/// all it outgasses.
const RECYCLED_UNDER_PLATES: f64 = 0.63;

/// Share of the nitrogen (N₂ and NH₃) plates outgas that subduction
/// carries back into the mantle: ammonium bound in clays and sediments
/// rides the slabs down (Goldblatt et al. 2009, doi:10.1038/ngeo692,
/// estimate that the crust holds about 0.5 and the mantle more than 1.4
/// times the air's nitrogen; checked in their abstract, the full text not
/// read: the stores below hold at least twice the air's, the order of
/// the four to one this share returns). Calibration, no source: Earth's
/// air keeps about 0.9 bar of N₂ once its NH₃ is photolysed into it.
const NITROGEN_RECYCLED_UNDER_PLATES: f64 = 0.8;

/// Share of the outgassed sulphur that stays in the air as SO₂: the rest
/// ends in sulphides and sulphates at the surface (on Venus SO₂ is
/// buffered by its reaction with the crust, Fegley & Prinn 1989, Nature
/// 337, 55, doi:10.1038/337055a0: reference checked, not the share).
/// Calibration on Venus's SO₂ of about
/// 150 ppm of 92 bar (Bertaux et al. 2007, Marcq et al. 2018).
const SULPHUR_IN_AIR: f64 = 5.0e-5;

/// Free oxygen, bar, from which an air washes its SO2 out as it does over
/// liquid water: oxygen and the radicals it makes oxidise SO2 to sulphate.
/// A hundredth of a bar, about a twentieth of Earth's: the oxidation is
/// fast long before the air is Earth's. Calibration, no source.
const OXIDISING_O2_BAR: f64 = 0.01;

/// Share of its SO2 an air keeps over liquid water or with free oxygen:
/// the lifetime of SO2 there, about a day and a half (oxidation in cloud
/// water and by OH, Seinfeld & Pandis 2016, Atmospheric Chemistry and
/// Physics, ch. 6: 1 to 2 days in the lower troposphere), over its
/// lifetime in a dry, oxygen-free air, where only the crust takes it: about
/// 1 Myr (Venus's air would lose its SO2 to the crust in a few Myr
/// without resupply, Fegley & Prinn 1989, Nature 337, 55; value from
/// memory, not checked). 1.5 days over 1 Myr is `4e-9`.
const WET_SO2_SHARE: f64 = 4.0e-9;

/// Escape velocity, km/s, at which the cosmic shoreline crosses Earth's
/// XUV fluence. Zahnle & Catling 2017 (ApJ 843, 122,
/// doi:10.3847/1538-4357/aa7846) find the bodies with and without air
/// split by `I_XUV ∝ v_esc⁴`, with Mars close to the line (checked). Their
/// line is drawn by eye on their Fig. 1 and 2, with no normalisation in
/// the text. Calibration, no source, on the Solar System bodies, for
/// 6.5 km/s: Mars keeps a trace, Titan a bar, Venus and Earth their air.
const SHORELINE_V_KM_S: f64 = 6.5;

/// Steepness of the retention across the shoreline: the share kept is
/// `1 / (1 + (I/I_line)^k)`. Calibration, no source, on the Solar System
/// bodies (Zahnle & Catling's line has no width): Titan (about 0.1 dex
/// on the side of the air) keeps most, Mars (about 0.4 dex on the other)
/// keeps under 1 %.
const SHORELINE_STEEPNESS: f64 = 6.0;

/// Factor on the XUV fluence of a body without a dynamo: the stellar wind
/// reaches its upper atmosphere and picks up its ions. Calibration, no
/// source (ties to #344), kept small enough that Venus, with no dynamo,
/// keeps its air.
const UNMAGNETISED_ESCAPE: f64 = 2.0;

/// Partial pressures at the surface, bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Gases {
    pub n2: f64,
    pub co2: f64,
    pub h2o: f64,
    pub ch4: f64,
    pub nh3: f64,
    pub so2: f64,
    pub o2: f64,
}

impl Gases {
    /// Surface pressure, bar: the sum of the partial pressures.
    pub fn total(&self) -> f64 {
        self.n2 + self.co2 + self.h2o + self.ch4 + self.nh3 + self.so2 + self.o2
    }

    /// Every partial pressure times `share`.
    fn scaled(self, share: f64) -> Self {
        Self {
            n2: self.n2 * share,
            co2: self.co2 * share,
            h2o: self.h2o * share,
            ch4: self.ch4 * share,
            nh3: self.nh3 * share,
            so2: self.so2 * share,
            o2: self.o2 * share,
        }
    }
}

/// Escape velocity at the surface, km/s: `v = sqrt(2 G M / R)`.
pub fn escape_velocity_km_s(interior: &Interior, mass_earth: f64) -> f64 {
    let radius = interior.radius_earth * EARTH_M;
    (2.0 * G * mass_earth * EARTH_KG / radius).sqrt() / 1.0e3
}

/// Bar per kilogram of gas spread over a body's surface:
/// `P = m g / (4π R²)` with `g = G M / R²`, in bar for `m` = 1 kg.
pub(crate) fn bar_per_kg(mass_earth: f64, interior: &Interior) -> f64 {
    let mass = mass_earth * EARTH_KG;
    let radius = interior.radius_earth * EARTH_M;
    let gravity = G * mass / (radius * radius);
    gravity / (4.0 * PI * radius * radius) / PA_PER_BAR
}

/// The gases a body's interior has brought to its surface, bar.
///
/// Each volatile of `inputs` (kg per kg of planet) times the mass, times
/// the share its `regime` outgasses ([`outgassed_share`], less what
/// subduction returns under plates: [`NITROGEN_RECYCLED_UNDER_PLATES`] of
/// the N₂ and NH₃, [`RECYCLED_UNDER_PLATES`] of the rest), gives a mass of gas `m` that weighs
/// `P = m g / (4π R²)` on the surface, with `g = G M / R²`, times the
/// share [`released_under_ocean`] lets through the body's water (its
/// weight, `water.kept_kg`, on the seafloor). Sulphur comes
/// up as SO₂, of which [`SULPHUR_IN_AIR`] stays in the air. The water is
/// in the air as vapour only in a runaway greenhouse, and then all that
/// the runaway has left (`water.kept_kg`): it is the ocean itself, not a
/// volatile of the mantle, so neither the regime's share nor the seafloor
/// holds it back; otherwise it is in the oceans and the climate sets its
/// vapour. No oxygen: life makes it.
pub fn outgassed(
    inputs: &PlanetInputs,
    water: &Water,
    regime: Regime,
    interior: &Interior,
) -> Gases {
    let mass = inputs.mass_earth * EARTH_KG;
    let bar_per_kg = bar_per_kg(inputs.mass_earth, interior);
    let seafloor_bar = water.kept_kg * bar_per_kg;
    let share = outgassed_share(regime) * bar_per_kg;
    let released = share * released_under_ocean(seafloor_bar);
    let (kept, nitrogen_kept) = if regime == Regime::Plates {
        (
            1.0 - RECYCLED_UNDER_PLATES,
            1.0 - NITROGEN_RECYCLED_UNDER_PLATES,
        )
    } else {
        (1.0, 1.0)
    };
    let v = &inputs.volatiles;
    let steam = if water.runaway { water.kept_kg } else { 0.0 };
    let nitrogen = released * nitrogen_kept;
    let rest = released * kept;
    Gases {
        n2: v.n2 * mass * nitrogen,
        co2: v.co2 * mass * rest,
        h2o: steam * bar_per_kg,
        ch4: v.ch4 * mass * rest,
        nh3: v.nh3 * mass * nitrogen,
        so2: v.s * mass * SO2_PER_S * SULPHUR_IN_AIR * rest,
        o2: 0.0,
    }
}

/// `gases` after the washing of SO2 out of a wet or oxidising air: with
/// liquid water at the surface or `o2` of [`OXIDISING_O2_BAR`] or more,
/// the SO2 falls to [`WET_SO2_SHARE`] of what the volcanoes gave; a dry,
/// oxygen-poor air keeps it in proportion to its pressure below
/// [`DRY_COLUMN_BAR`] (monotonic: the thinner the air, the less it keeps). Sulphuric acid clouds are not made here: the
/// sky follows the SO2 left, whatever the liquid.
pub fn wash_out(gases: Gases, liquid: SurfaceLiquid) -> Gases {
    if liquid == SurfaceLiquid::Water || gases.o2 >= OXIDISING_O2_BAR {
        Gases {
            so2: gases.so2 * WET_SO2_SHARE,
            ..gases
        }
    } else {
        Gases {
            so2: gases.so2 * (gases.total() / DRY_COLUMN_BAR).min(1.0),
            ..gases
        }
    }
}

/// Column of air, bar, from which a dry, oxygen-free air keeps all its
/// SO2 for the dry lifetime of [`WET_SO2_SHARE`]. The dry sink is
/// photochemical: UV-made oxidants (O, OH from photolysed CO2 and H2O)
/// destroy SO2 at a rate per unit area that does not grow with the
/// column, so the lifetime scales with the column mass that shields and
/// dilutes it. Venus's 92 bar keeps SO2 for about 1 Myr, Mars's 0.006 bar
/// for about a century or less (Krasnopolsky 2012, Icarus 217, 144,
/// doi:10.1016/j.icarus.2011.10.019: Mars SO2 upper limit; the lifetime of
/// about 2 yr and Fegley & Prinn 1989, Nature 337, 55, for Venus: from
/// memory, check). Calibrated on Venus and Mars together: 10 bar leaves
/// Venus at its 150 ppm and Mars's SO2 far under its 0.3 ppb limit.
const DRY_COLUMN_BAR: f64 = 10.0;

/// The share of its air a body keeps against escape to space: the cosmic
/// shoreline of Zahnle & Catling 2017 (doi:10.3847/1538-4357/aa7846).
///
/// The shoreline is `I_line = (v_esc / 6.5 km/s)⁴` in Earth's XUV
/// fluence ([`SHORELINE_V_KM_S`]); a body without a dynamo meets
/// [`UNMAGNETISED_ESCAPE`] times its fluence. The share kept falls
/// smoothly across the line, `1 / (1 + (I / I_line)^k)` with
/// `k =` [`SHORELINE_STEEPNESS`]: half on the line, nearly all well
/// below it, nearly nothing well above. Every gas is kept in the same
/// share: the shoreline is a bulk rule, not a species-by-species escape.
pub fn atmosphere(
    outgassed: Gases,
    interior: &Interior,
    mass_earth: f64,
    core: &Core,
    fluence_rel: f64,
) -> Gases {
    let v = escape_velocity_km_s(interior, mass_earth);
    let line = (v / SHORELINE_V_KM_S).powi(4);
    let shield = if core.dynamo {
        1.0
    } else {
        UNMAGNETISED_ESCAPE
    };
    let exposure = fluence_rel.max(0.0) * shield / line;
    let kept = 1.0 / (1.0 + exposure.powf(SHORELINE_STEEPNESS));
    outgassed.scaled(kept)
}

/// Pressure, bar, at which a body's air that non-thermal loss has
/// exhausted is stored ([`non_thermal_loss`]): an exosphere, whose
/// molecules fly from the surface without meeting each other. Its column
/// is at most about one exobase column, `1/σ` with the hard-sphere
/// cross-section `σ ≈ 4e-19 m²` of N₂, O₂ and CO₂ (kinetic diameters of
/// 3.3 to 3.6 Å; from memory, not checked): `2.5e18 m⁻²`, which weighs
/// `N m g` = 2 to 3e-12 bar of CO₂ under a Galilean moon's 1.2 to
/// 1.4 m/s². The measured exospheres sit there: Callisto's CO₂ at
/// 7.5e-12 bar, a column of 8e14 cm⁻² at about 150 K (Carlson 1999,
/// doi:10.1126/science.283.5403.820; its abstract, checked through a
/// bibliographic record), and the O₂ of Europa and Ganymede at about
/// 1e-12 bar (Hall et al. 1995 and 1998, from memory, not checked). The
/// exosphere's pressure is an order of magnitude, not a chemistry (the
/// radiolytic O₂ of an icy surface is not modelled): 3e-12 bar, far under
/// [`crate::atmosphere::AIRLESS_BAR`].
pub const EXOSPHERE_BAR: f64 = 3.0e-12;

/// A giant whose magnetosphere a moon orbits in, as [`non_thermal_loss`]
/// reads it: the corotating plasma that sputters the moon's surface and
/// air grows with the giant's mass and falls steeply with the moon's
/// distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HostPlasma {
    /// Mass of the giant, Earth masses.
    pub host_mass_earth: f64,
    /// Semi-major axis of the moon's orbit, in the giant's radii.
    pub semi_major_axis_radii: f64,
}

/// What [`non_thermal_loss`] reads of a body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LossContext {
    /// Surface gravity, m/s².
    pub gravity: f64,
    /// Radius, m.
    pub radius_m: f64,
    /// Distance to the star, AU (of the planet, for a moon).
    pub orbit_au: f64,
    /// Age of the system, Gyr: how long the loss has worked.
    pub age_gyr: f64,
    /// The giant whose magnetosphere the body orbits in, `None` for a
    /// planet and for a moon of a body that is no giant.
    pub host: Option<HostPlasma>,
}

/// Photo-ionisation rate of a molecule of the air at 1 AU from the Sun,
/// per second: N₂ about 3.5e-7 and CO₂ about 6e-7 under the quiet Sun
/// (Huebner, Keady & Lyon 1992, Astrophys. Space Sci. 195, 1; from
/// memory, not checked), one order for every gas. Scaled by `1/d²`.
const PHOTO_IONISATION_AT_1_AU_S: f64 = 5.0e-7;

/// Column of an exobase, molecules per m²: `1/σ` with the hard-sphere
/// cross-section of [`EXOSPHERE_BAR`], 4e-19 m². Above it, a molecule
/// leaves without a collision; the photo-ions made there are what the
/// giant's corotating field sweeps away, whatever the column under it.
const EXOBASE_COLUMN_M2: f64 = 2.5e18;

/// Molecules the giant's plasma removes from a moon's surface and air at
/// Callisto's place (Jupiter's mass, 26.3 Jupiter radii), per m² and per
/// second, before the share fast enough to escape
/// ([`escaping_share`]). Calibration on Callisto's CO₂: the column the
/// cosmic shoreline leaves it (about 0.05 bar, 5.5e28 m⁻², more on the
/// archetype's richest draws) must be gone within its age, since Carlson
/// 1999 (see [`EXOSPHERE_BAR`]) sees only an exosphere, which
/// photo-ionisation and Jupiter's field sweep away in a few years and
/// which needs a steady supply. The value is about fifty times the water
/// ice Jupiter's ions sputter there: they bring Callisto about 2e8 keV
/// cm⁻² s⁻¹ (Cooper et al. 2001, Icarus 149, 133,
/// doi:10.1006/icar.2000.6498, through a secondary table, not checked),
/// at about 0.1 molecule of ice per keV (Europa's 2e27 H₂O s⁻¹ over
/// its 6e10 keV cm⁻² s⁻¹; Johnson et al. 2004, "Radiation effects on the
/// surfaces of the Galilean satellites", in *Jupiter: The Planet,
/// Satellites and Magnetosphere*, ch. 20; from memory, not checked):
/// 2e11 m⁻² s⁻¹, fifty times less. A frost of CO₂ or N₂, bound less
/// than water ice, sputters several times more per ion; the rest of the
/// factor is the calibration's uncertainty.
const PLASMA_LOSS_AT_CALLISTO: f64 = 1.0e13;

/// Exponent of the moon's distance (giant radii) in the plasma's loss:
/// the energy flux of Jupiter's energetic ions falls from Europa
/// (9.4 R_J, about 6e10 keV cm⁻² s⁻¹) to Ganymede (15.0, 5e9) and
/// Callisto (26.3, 2e8) about as `r^-5.5` (Cooper et al. 2001, see
/// [`PLASMA_LOSS_AT_CALLISTO`]; a fit through the three).
const PLASMA_DISTANCE_EXPONENT: f64 = 5.5;

/// Exponent of the giant's mass in the plasma's loss. Calibration, no
/// source: a heavier giant has a stronger field and a denser plasma
/// (Jupiter's dipole is about 20 times Saturn's for 3.3 times its mass).
/// The cube keeps Titan (Saturn, 20.3 radii) losing at most 7 % of its N₂
/// over its age and Triton (Neptune, 12.4 to 15 radii) at most 4 %, while
/// Callisto loses all its CO₂ (`archetype_census`, seeds 1 to 40). With 2,
/// Triton, whose giant is 18.5 times lighter than Jupiter, would lose up
/// to about 80 % of its N₂ and its frost (and Titan up to 23 %): too
/// close to the line for a body that keeps its air.
const PLASMA_MASS_EXPONENT: f64 = 3.0;

/// The most molecules per m² and per second a giant's plasma removes
/// from a moon: Io's, the largest known, about one tonne per second
/// over its 4.2e7 km² (Thomas et al. 2004, "The Io neutral clouds and
/// plasma torus", in *Jupiter: The Planet, Satellites and
/// Magnetosphere*, ch. 23; from memory, not checked), 2.2e14 SO₂ m⁻²
/// s⁻¹. The power law of [`plasma_flux`] would give Io about 170 times that,
/// and a moon a few radii from a giant of ten Jupiter masses millions of
/// times: the plasma a giant's moons load is limited by what they
/// supply and what its rotation can carry, which the law does not see.
const PLASMA_LOSS_MAX: f64 = 2.2e14;

/// Jupiter's mass, Earth masses, and Callisto's orbit, Jupiter radii: the
/// place of [`PLASMA_LOSS_AT_CALLISTO`].
const JUPITER_MASS_EARTH: f64 = crate::giants::JUPITER_EARTH_MASSES as f64;
const CALLISTO_RADII: f64 = 26.3;

/// Binding energy of water ice, eV: the energy scale of the molecules
/// a plasma ion sputters from an icy surface (Johnson 1990, *Energetic
/// Charged-Particle Interactions with Atmospheres and Surfaces*,
/// Springer; from memory, not checked).
const ICE_BINDING_EV: f64 = 0.45;

/// Joules per electronvolt.
const EV_J: f64 = 1.602_176_634e-19;

/// Atomic mass unit, kg.
const AMU_KG: f64 = 1.660_54e-27;

/// Molecular masses of the gases of [`Gases`], u, in its field order.
const MOLECULAR_U: [f64; 7] = [28.014, 44.01, 18.015, 16.04, 17.031, 64.07, 32.0];

impl Gases {
    /// The partial pressures, in [`MOLECULAR_U`]'s order.
    fn to_array(self) -> [f64; 7] {
        [
            self.n2, self.co2, self.h2o, self.ch4, self.nh3, self.so2, self.o2,
        ]
    }

    fn from_array(a: [f64; 7]) -> Self {
        let [n2, co2, h2o, ch4, nh3, so2, o2] = a;
        Self {
            n2,
            co2,
            h2o,
            ch4,
            nh3,
            so2,
            o2,
        }
    }
}

/// Share of the molecules a plasma sputters that leave faster than the
/// escape velocity: the sputtered energies follow `E / (E + U)³`
/// (Sigmund–Thompson, `U` the [`ICE_BINDING_EV`]), of which
/// `U (2x + U) / (x + U)²` lie above the escape energy `x = m v² / 2`.
/// About 0.4 for CO₂ on Callisto, 0.5 for N₂ on Titan, 0.05 for N₂ on a
/// moon of the Earth's mass: a large moon holds what its plasma knocks
/// up.
fn escaping_share(molecule_kg: f64, body: &LossContext) -> f64 {
    let x = 0.5 * molecule_kg * 2.0 * body.gravity * body.radius_m / EV_J;
    let u = ICE_BINDING_EV;
    u * (2.0 * x + u) / ((x + u) * (x + u))
}

/// Molecules per m² and per second a moon's air loses at the top to its
/// giant's magnetosphere, before the share that escapes: the plasma's
/// sputtering ([`PLASMA_LOSS_AT_CALLISTO`] scaled by the giant's mass and
/// the moon's distance in its radii), at most [`PLASMA_LOSS_MAX`].
fn plasma_flux(host: &HostPlasma) -> f64 {
    let mass = host.host_mass_earth / JUPITER_MASS_EARTH;
    let distance = host.semi_major_axis_radii.max(1.0) / CALLISTO_RADII;
    let flux = PLASMA_LOSS_AT_CALLISTO
        * mass.powf(PLASMA_MASS_EXPONENT)
        * distance.powf(-PLASMA_DISTANCE_EXPONENT);
    flux.min(PLASMA_LOSS_MAX)
}

/// The gases a body keeps against non-thermal loss over its age, and
/// what it loses: `(kept, lost)`.
///
/// The air (`gases`, what the cosmic shoreline of [`atmosphere`] left,
/// its frosts to come included) is what the body supplied over its age.
/// Inside a giant's magnetosphere each of its molecules at the top meets
/// a loss per m² that does not depend on how much air there is,
/// `L = (J N_exo + f P) t`:
///
/// - photo-ionisation at the body's distance, `J =`
///   [`PHOTO_IONISATION_AT_1_AU_S`]` / d²`, of the exobase column
///   `N_exo` ([`EXOBASE_COLUMN_M2`]), whose ions the giant's corotating
///   field sweeps away (Carlson 1999's loss for Callisto's CO₂);
/// - the giant's plasma sputtering ([`plasma_flux`]), times the share of
///   the sputtered molecules that escape ([`escaping_share`], which
///   depends on the molecule's mass).
///
/// The air loses its gases in proportion to their share of its column
/// `Σ p / (m g)`, so the mean of their `L` weighted by those shares is
/// the air's loss over the age `t`. An air whose column is less than
/// that is exhausted: it had no air that lasts, only the exosphere its
/// supply feeds, an order of magnitude ([`EXOSPHERE_BAR`]) that its
/// gases share in proportion to their supply, each keeping at most what
/// it had. An air that outlasts its loss keeps all of it: the loss is
/// then a share of a large column, and the bulk escape is the
/// shoreline's.
///
/// A moon of a giant also meets the shoreline without its own field
/// (`planet::shield`): the giant's plasma reaching
/// its upper air lets the star's XUV and wind strip it as an unmagnetised
/// body's, a bulk escape scaled from the star's light by a fixed factor
/// of 2 that does not see the plasma's own energy, its distance or its
/// giant's mass. This step adds that energy, the sputtering and the
/// sweeping of photo-ions. The two overlap in cause (the plasma) but not
/// in process; any overlap is absorbed by the calibration of
/// [`PLASMA_LOSS_AT_CALLISTO`], made on the air the unshielded shoreline
/// leaves, and the thick airs it must keep (Titan, Triton) do not change.
///
/// Outside a giant's magnetosphere nothing is lost here: the stellar
/// wind's pick-up of photo-ions is already in the shoreline, which Zahnle
/// & Catling 2017 fitted on the planets (Mars, Mercury) that undergo it;
/// counting it again would strip Mars. Titan's 1.5 bar and Triton's N₂
/// and its frost lose a few percent of their column over the age;
/// Europa, Ganymede, Callisto and Io lose their CO₂ and N₂.
pub fn non_thermal_loss(gases: Gases, body: &LossContext) -> (Gases, Gases) {
    let Some(host) = &body.host else {
        return (gases, Gases::default());
    };
    let seconds = body.age_gyr.max(0.0) * GYR_S;
    let photo = PHOTO_IONISATION_AT_1_AU_S / (body.orbit_au * body.orbit_au) * EXOBASE_COLUMN_M2;
    let plasma = plasma_flux(host);
    let partial = gases.to_array();
    let (mut column, mut loss) = (0.0, 0.0);
    for (&bar, &u) in partial.iter().zip(&MOLECULAR_U) {
        let molecule = u * AMU_KG;
        let n = bar.max(0.0) * PA_PER_BAR / (molecule * body.gravity);
        column += n;
        loss += n * (photo + plasma * escaping_share(molecule, body)) * seconds;
    }
    // `loss` is the share-weighted loss times the column.
    if column <= 0.0 || column * column >= loss {
        return (gases, Gases::default());
    }
    let supply: f64 = partial.iter().map(|p| p.max(0.0)).sum();
    let kept = partial.map(|p| p.min(EXOSPHERE_BAR * p.max(0.0) / supply));
    let lost = std::array::from_fn(|i| partial[i] - kept[i]);
    (Gases::from_array(kept), Gases::from_array(lost))
}

/// Solar constant, W/m² (Kopp & Lean 2011, doi:10.1029/2010GL045777):
/// the insolation of 1.
const SOLAR_CONSTANT_W_M2: f64 = 1361.0;

/// Energy of a Lyman α photon, J: `h c / λ` at 121.567 nm.
const LYMAN_ALPHA_PHOTON_J: f64 = 1.634e-18;

/// Mass of a methane molecule, kg: 16.04 u.
const CH4_MOLECULE_KG: f64 = 16.04 * 1.660_54e-27;

/// Seconds in a gigayear (Julian years).
const GYR_S: f64 = 3.155_76e16;

/// Methane molecules destroyed for each Lyman α photon the star sends a
/// body, averaged over its sphere. Calibration on Titan: Yung, Allen &
/// Pinto 1984 (ApJS 55, 465, doi:10.1086/190963; their abstract,
/// checked) find that Titan "might have lost 5.6 × 10⁴ g cm⁻²" of
/// methane, "the equivalent of 8 bars", over geologic time; read as
/// 4.5 Gyr at today's rate (an assumption: the full text was not read),
/// that is 1.48e10 molecules cm⁻² s⁻¹. Titan gets 1.07e9 Lyman α photons
/// cm⁻² s⁻¹ on average today (3.9e11 at 1 AU from
/// [`super::xuv::LYMAN_ALPHA`], over 9.58² and over 4 for the sphere),
/// so 13.9 molecules per photon. More than one, because Lyman α stands
/// here for all the ultraviolet that destroys methane: the photolysis
/// proper below 145 nm and the dissociation photosensitised by acetylene
/// at longer wavelengths (Yung et al.'s process (a)). The rate it gives
/// destroys Titan's air's methane in a few tens of millions of years, as
/// Tobie, Lunine & Sotin 2006 (Nature 440, 61, doi:10.1038/nature04497;
/// abstract checked) state.
const CH4_PER_LYMAN_ALPHA_PHOTON: f64 = 13.9;

/// Universal gas constant, J/(mol·K).
const R_GAS: f64 = 8.314;

/// Triple point of nitrogen, K: under it, the nitrogen a surface
/// condenses is a frost (NIST Chemistry WebBook, 63.15 K; from memory,
/// not checked).
pub(crate) const NITROGEN_TRIPLE_K: f64 = 63.15;

/// A gas's triple point (K, bar) and its enthalpy of sublimation
/// (J/mol). Values from memory (NIST Chemistry WebBook), not checked:
/// CO₂ 216.6 K and 5.185 bar, 26.1 kJ/mol (which gives its 1 atm
/// sublimation at 194.7 K); NH₃ 195.4 K and 0.0606 bar, 29 kJ/mol; CH₄
/// 90.7 K and 0.117 bar, 9.1 kJ/mol; N₂ 63.15 K and 0.125 bar,
/// 6.3 kJ/mol. SO₂'s is [`SO2_FROST`].
pub struct Frost {
    /// Temperature of the triple point, K.
    pub triple_k: f64,
    /// Pressure of the triple point, bar.
    pub triple_bar: f64,
    /// Enthalpy of sublimation, J/mol.
    pub sublimation_j_mol: f64,
}

const CO2_FROST: Frost = Frost {
    triple_k: 216.6,
    triple_bar: 5.185,
    sublimation_j_mol: 26_100.0,
};
const NH3_FROST: Frost = Frost {
    triple_k: 195.4,
    triple_bar: 0.0606,
    sublimation_j_mol: 29_000.0,
};
const CH4_FROST: Frost = Frost {
    triple_k: 90.7,
    triple_bar: 0.117,
    sublimation_j_mol: 9_100.0,
};
/// SO₂: triple point 197.64 K and 0.0167 bar (Giauque & Stephenson 1938,
/// J. Am. Chem. Soc. 60, 1389, through the NIST Chemistry WebBook;
/// checked there), and the sublimation curve
/// `P = 1.516e8 exp(-4510 K / T)` bar of Wagman 1979 (*Sublimation
/// pressure and enthalpy of SO₂*, Chemical Thermodynamics Data Center,
/// National Bureau of Standards), which Io's atmosphere is read with
/// (quoted by Giles et al. 2024, arXiv:2405.19253; checked there):
/// `4510 K · R` = 37.5 kJ/mol. Anchored at the triple point, the curve
/// gives 1.3e-9 bar at 115 K, 1.4e-9 in Wagman's form. Fray & Schmitt
/// 2009 (Planet. Space Sci. 57, 2053, doi:10.1016/j.pss.2009.09.011)
/// review SO₂'s sublimation pressure too; their table was not read.
pub const SO2_FROST: Frost = Frost {
    triple_k: 197.64,
    triple_bar: 0.0167,
    sublimation_j_mol: 4510.0 * R_GAS,
};
const N2_FROST: Frost = Frost {
    triple_k: NITROGEN_TRIPLE_K,
    triple_bar: 0.125,
    sublimation_j_mol: 6_300.0,
};

impl Frost {
    /// The partial pressure the gas keeps at `temperature_k`: below its
    /// triple point, at most its sublimation pressure (Clausius–Clapeyron
    /// from the triple point); at or above it, all of it.
    fn cap(&self, partial_bar: f64, temperature_k: f64) -> f64 {
        if temperature_k >= self.triple_k {
            return partial_bar;
        }
        let exponent =
            -self.sublimation_j_mol / R_GAS * (1.0 / temperature_k.max(1.0) - 1.0 / self.triple_k);
        partial_bar.min(self.triple_bar * exponent.exp())
    }
}

/// One standard atmosphere, bar.
const ATM_BAR: f64 = 1.013_25;

/// A volatile that condenses on a body's surface at the surface
/// temperature, inside the climate's fixed point ([`condense`]): its
/// [`Frost`] below its triple point, its liquid's vapour pressure above,
/// and none of it condensed past its critical point.
struct Condensable {
    frost: Frost,
    /// Normal boiling point, K, at one atmosphere.
    boils_k: f64,
    /// Critical temperature, K: above it no liquid exists.
    critical_k: f64,
    /// Share of the pure gas's saturation pressure that the air over its
    /// condensate holds (Raoult's law: the gas's mole fraction in the
    /// condensate), 1 for a pure condensate.
    activity: f64,
    /// The gas's partial pressure in a [`Gases`].
    of: fn(&mut Gases) -> &mut f64,
}

impl Condensable {
    /// Saturation vapour pressure of the pure gas at `temperature_k`,
    /// bar: the sublimation curve of its [`Frost`] below its triple point;
    /// above it, Clausius–Clapeyron with the mean latent heat that joins
    /// the triple point to the normal boiling point (both from the NIST
    /// Chemistry WebBook); infinite at and past the critical point, where
    /// the gas does not condense.
    fn saturation_bar(&self, temperature_k: f64) -> f64 {
        let f = &self.frost;
        if temperature_k < f.triple_k {
            return f.cap(f64::INFINITY, temperature_k);
        }
        if temperature_k >= self.critical_k {
            return f64::INFINITY;
        }
        let l_over_r = (ATM_BAR / f.triple_bar).ln() / (1.0 / f.triple_k - 1.0 / self.boils_k);
        f.triple_bar * (l_over_r * (1.0 / f.triple_k - 1.0 / temperature_k)).exp()
    }
}

/// Mole fraction of methane in a Titan sea: about a half, the rest
/// nitrogen and ethane (Glein & Shock 2013, doi:10.1016/j.gca.2013.03.030,
/// and Tan, Kargel & Marion 2013, doi:10.1016/j.icarus.2012.10.032, find
/// 0.5 to 0.7 for its seas; from the plan, not checked). By Raoult's law
/// the air over such a sea holds this share of pure methane's saturation
/// pressure ([`condense`]). The low end, because Titan's air near its
/// surface holds 45 to 50 % of pure methane's saturation (Niemann et al.
/// 2005, doi:10.1038/nature04122; from memory, not checked), and its
/// seas hold that air.
pub(crate) const SEA_METHANE_SHARE: f64 = 0.5;

/// The gases that condense on the surface inside the climate
/// ([`condense`]):
///
/// - methane over a sea of methane, nitrogen and ethane
///   ([`SEA_METHANE_SHARE`]), normal boiling point 111.67 K and critical
///   point 190.56 K;
/// - nitrogen over its own ice (activity 1: a pure frost, which Triton's
///   N₂ air sits in equilibrium with; issue #464), normal boiling point
///   77.36 K and critical point 126.19 K.
///
/// Points from the NIST Chemistry WebBook (from memory, not checked).
const CONDENSING: [Condensable; 2] = [
    Condensable {
        frost: CH4_FROST,
        boils_k: 111.67,
        critical_k: 190.56,
        activity: SEA_METHANE_SHARE,
        of: |g| &mut g.ch4,
    },
    Condensable {
        frost: N2_FROST,
        boils_k: 77.36,
        critical_k: 126.19,
        activity: 1.0,
        of: |g| &mut g.n2,
    },
];

/// Saturation vapour pressure of pure methane at `temperature_k`, bar:
/// over its ice below its triple point (90.7 K, 0.117 bar), over its
/// liquid above, up to its critical point (190.56 K), past which it is
/// infinite: methane does not condense there.
pub(crate) fn methane_saturation_bar(temperature_k: f64) -> f64 {
    CONDENSING[0].saturation_bar(temperature_k)
}

/// The air and the condensate of `gases` (bar) on a surface at
/// `surface_k`: each gas that condenses there ([`CONDENSING`]: methane
/// and nitrogen)
/// keeps in the air at most its activity times its saturation pressure
/// at the surface temperature, and the rest lies on the ground, as a
/// liquid or a frost. Returns `(air, condensate)`; `air + condensate` is
/// `gases`.
///
/// The climate applies it inside its fixed point, at the surface
/// temperature it solves for (see [`super::climate::climate`]): unlike
/// the [`cold_trap`] at the equilibrium temperature, which the climate
/// cannot move, how much of a condensing gas is in the air sets the
/// greenhouse that sets the temperature.
pub(crate) fn condense(gases: Gases, surface_k: f64) -> (Gases, Gases) {
    let mut air = gases;
    let mut condensate = Gases::default();
    for gas in &CONDENSING {
        let kept = (gas.of)(&mut air);
        let most = gas.activity * gas.saturation_bar(surface_k);
        if *kept > most {
            *(gas.of)(&mut condensate) = *kept - most;
            *kept = most;
        }
    }
    (air, condensate)
}

/// `air` with each gas that condenses on the surface ([`condense`]) set
/// to what its `inventory` (bar, in the air and on the ground) leaves in
/// the air at `surface_k`: the air a climate solved with a temperature
/// close to `surface_k`, made to agree with it exactly.
pub(crate) fn condensed_air(air: Gases, inventory: &Gases, surface_k: f64) -> Gases {
    let (kept, _) = condense(*inventory, surface_k);
    let mut air = air;
    for gas in &CONDENSING {
        let mut kept = kept;
        *(gas.of)(&mut air) = *(gas.of)(&mut kept);
    }
    air
}

/// Partial pressure of CO₂, bar, at which it frosts out at
/// `temperature_k` (its sublimation curve below its triple point,
/// [`Frost`]); unbounded above the triple point, where it does not
/// freeze.
pub(super) fn co2_frost_bar(temperature_k: f64) -> f64 {
    CO2_FROST.cap(f64::INFINITY, temperature_k)
}

/// Partial pressure of SO₂, bar, at which it frosts out at
/// `temperature_k` ([`SO2_FROST`]); unbounded above its triple point.
pub(super) fn so2_frost_bar(temperature_k: f64) -> f64 {
    SO2_FROST.cap(f64::INFINITY, temperature_k)
}

/// Heat a kilogram of erupted magma carries out of a heat-pipe body, J:
/// its latent heat of crystallisation, about 4e5 J/kg, and its heat
/// capacity, about 1200 J/(kg·K), over the 1400 K from an eruption at
/// about 1600 K (Io's ultramafic lavas, McEwen et al. 1998, Science 281,
/// 87, doi:10.1126/science.281.5373.87) to its cold surface; values from
/// memory, not checked. In a heat pipe the magma carries the body's heat
/// up (O'Reilly & Davies 1981, Geophys. Res. Lett. 8, 313,
/// doi:10.1029/GL008i004p00313; from memory, not checked), so the heat
/// over this is the magma erupted: about 5e7 kg/s for Io's 1e14 W.
const MAGMA_HEAT_J_KG: f64 = 2.1e6;

/// SO₂ the volcanoes of a body bring up, kg/s: the magma its tidal heat
/// erupts (`heat.tidal_w` over [`MAGMA_HEAT_J_KG`]) times the sulphur of
/// its silicate (`inputs.volatiles.s` over the silicate share, that is
/// [`super::inputs`]'s 250 ppm of rock), all of it degassed as SO₂
/// ([`SO2_PER_S`]) into the vacuum or thin air of the vent. Only the
/// tidal heat: a heat pipe fed by tides (Io) erupts it as fast as it
/// comes, while radiogenic and secular heat leave by conduction in the
/// other regimes; [`super::bare`] reads it for heat pipes only. About
/// 1.7e4 kg/s for the model's Io (8.6e13 W of tides); its torus takes
/// about 300 kg/s of it ([`so2_loss_kg_s`], at most [`PLASMA_LOSS_MAX`]),
/// the rest falls back as frost.
pub fn volcanic_so2_supply(heat: &super::InternalHeat, inputs: &PlanetInputs) -> f64 {
    let silicate = (1.0 - inputs.water_fraction - inputs.envelope_fraction)
        * (1.0 - inputs.core_mass_fraction);
    if silicate <= 0.0 {
        return 0.0;
    }
    let magma = heat.tidal_w.max(0.0) / MAGMA_HEAT_J_KG;
    magma * inputs.volatiles.s.max(0.0) / silicate * SO2_PER_S
}

/// Time a heat-pipe body's SO₂ frost stays on its surface before fresh
/// lava and plume deposits bury it, s: 150 000 years. Calibration on Io,
/// no source: SO₂ frost covers most of Io's surface (Douté et al. 2001,
/// Icarus 149, 107, doi:10.1006/icar.2000.6513; from memory, not
/// checked), and this residence covers at least 80 % of the disc with
/// frost on every draw of the `io-like` archetype, whose sulphur scatters
/// by a factor of 20 (7.9e3 to 1.5e5 kg/s of SO₂ on seeds 1 to 40,
/// 0.017 to 0.33 bar of frost against [`super::albedo`]'s layer of
/// 0.02 bar). A shorter residence leaves its low-sulphur draws half
/// bare, dark and too warm: their dayside frost then breathes more than
/// 1e-8 bar. Not measured: Io's
/// resurfacing, of the order of a centimetre a year (from memory, not
/// checked), would bury a metre-thick deposit in about a century, but
/// the frost sublimes and condenses again each day and so stays on top
/// of what buries it.
const SO2_FROST_RESIDENCE_S: f64 = 150_000.0 * 3.155_76e7;

/// SO₂ frost, bar, a body's volcanoes keep on its ground: what of their
/// `supply_kg_s` ([`volcanic_so2_supply`]) the body's non-thermal loss
/// does not take ([`so2_loss_kg_s`]), laid over
/// [`SO2_FROST_RESIDENCE_S`], `bar_per_kg` the weight of a kilogram on
/// its surface. None when the loss takes all of the supply: the air and
/// the frost leave as fast as they come, an exosphere.
pub fn volcanic_so2_frost_bar(supply_kg_s: f64, body: &LossContext, bar_per_kg: f64) -> f64 {
    let net = supply_kg_s - so2_loss_kg_s(body);
    net.max(0.0) * SO2_FROST_RESIDENCE_S * bar_per_kg
}

/// SO₂ a body loses to [`non_thermal_loss`], kg/s: the photo-ions of its
/// exobase swept away and the share of its giant's plasma sputtering
/// fast enough to escape ([`escaping_share`]), over its whole surface.
/// None outside a giant's magnetosphere.
fn so2_loss_kg_s(body: &LossContext) -> f64 {
    let Some(host) = &body.host else {
        return 0.0;
    };
    let molecule = MOLECULAR_U[5] * AMU_KG;
    let photo = PHOTO_IONISATION_AT_1_AU_S / (body.orbit_au * body.orbit_au) * EXOBASE_COLUMN_M2;
    let flux = photo + plasma_flux(host) * escaping_share(molecule, body);
    flux * 4.0 * PI * body.radius_m * body.radius_m * molecule
}

/// Factor on a body's equilibrium temperature (under its Bond albedo,
/// starlight spread over the whole sphere) that gives the mean
/// temperature of its dayside frost: the starlight spread over the
/// dayside only, `2^(1/4)`. A heat pipe's internal heat leaves through
/// its vents and lava, not through the cold lid the frost lies on.
/// The model's Io, at 93 to 100 K of equilibrium under its Bond albedo
/// of 0.54 to 0.63, has its frost at 110 to 119 K: its equatorial frost
/// is at 106 to 117 K (Giles et al. 2024, arXiv:2405.19253; checked).
const DAYSIDE_FACTOR: f64 = 1.189_207_115;

/// `gases` with the SO₂ air a body's volcanic frost holds: the vapour of
/// `so2_frost_bar` of frost ([`SO2_FROST`]) at the temperature of its
/// dayside ([`DAYSIDE_FACTOR`] times `equilibrium_k`), at most the frost
/// itself, where that is more than the air holds already. Io's dayside
/// column, 1e16 to 2e17 cm⁻² (about 1 to 10 nbar), is that equilibrium
/// (Giles et al. 2024, arXiv:2405.19253; checked).
///
/// It enters after the climate and the washing out ([`wash_out`]): the
/// frost resupplies it within hours, faster than the plasma takes it
/// ([`non_thermal_loss`], whose loss [`volcanic_so2_frost_bar`] already
/// counts), and a nanobar weighs nothing in the climate.
pub fn frost_so2_air(gases: Gases, so2_frost_bar: f64, equilibrium_k: f64) -> Gases {
    if so2_frost_bar <= 0.0 {
        return gases;
    }
    let vapour = SO2_FROST.cap(so2_frost_bar, equilibrium_k * DAYSIDE_FACTOR);
    Gases {
        so2: gases.so2.max(vapour),
        ..gases
    }
}

/// Methane the star's ultraviolet destroys on a body, bar per Gyr, per
/// unit of the Lyman α share of the star's light
/// ([`super::xuv::LYMAN_ALPHA`]): `Y · S₀ I / 4 / E_γ` photons per m² and
/// second ([`CH4_PER_LYMAN_ALPHA_PHOTON`], the insolation `I` over the
/// sphere), times the mass of a molecule, times `g / 1e5` bar per kg/m².
/// The rate does not depend on how much methane there is: any column
/// worth counting absorbs all the Lyman α (photon-limited).
fn methane_destroyed_bar_per_gyr(insolation: f64, gravity_m_s2: f64) -> f64 {
    let photons = SOLAR_CONSTANT_W_M2 * insolation.max(0.0) / 4.0 / LYMAN_ALPHA_PHOTON_J;
    CH4_PER_LYMAN_ALPHA_PHOTON * photons * CH4_MOLECULE_KG * gravity_m_s2 / PA_PER_BAR * GYR_S
}

/// The methane left, bar, once the star's ultraviolet has worked on what
/// a body outgassed over its life.
///
/// The body outgasses its methane (`gases.ch4`, what escape left of it)
/// at a steady rate over its `context.age_gyr`; the star destroys it at
/// the rate of [`methane_destroyed_bar_per_gyr`], which follows its
/// Lyman α history ([`super::xuv::LYMAN_ALPHA`]: saturated, then falling
/// as `t^-0.72`). While destruction outruns the supply nothing builds up;
/// once the supply overtakes it, at `t*`, methane gathers in the air and
/// on the ground, `∫_{t*}^{age} (S − D(t)) dt`, analytic. Destroyed
/// methane is gone: its hydrogen escapes and its carbon ends in heavier
/// hydrocarbons on the ground, which the model does not carry.
///
/// This is Titan's resupply: Tobie, Lunine & Sotin 2006
/// (doi:10.1038/nature04497) explain its methane by outgassing from
/// its interior, without which photolysis would have removed it. The
/// model spreads the outgassing evenly over the age, where they find it
/// in episodes. Ganymede and Callisto, which get 3.4 times Titan's light
/// per unit area and outgas a tenth of its methane or less, keep none.
///
/// Apply it to the air escape leaves, before [`cold_trap`]: what the cold
/// freezes onto the ground feeds the air back as it is photolysed.
pub fn methane_photolysis(
    gases: Gases,
    context: &super::Context,
    mass_earth: f64,
    interior: &Interior,
) -> Gases {
    use super::xuv::{LYMAN_ALPHA, age_below_ratio, integrated_ratio_gyr};
    let age = context.age_gyr;
    if gases.ch4 <= 0.0 || age <= 0.0 {
        return gases;
    }
    let radius = interior.radius_earth * EARTH_M;
    let gravity = G * mass_earth * EARTH_KG / (radius * radius);
    let per_ratio = methane_destroyed_bar_per_gyr(context.insolation, gravity);
    let supply = gases.ch4 / age;
    // The supply overtakes the destruction once the Lyman α share has
    // fallen to `supply / per_ratio`.
    let onset = age_below_ratio(LYMAN_ALPHA, context.star_mass, supply / per_ratio);
    let ch4 = if onset >= age {
        0.0
    } else {
        let destroyed = per_ratio
            * (integrated_ratio_gyr(LYMAN_ALPHA, context.star_mass, age)
                - integrated_ratio_gyr(LYMAN_ALPHA, context.star_mass, onset));
        (supply * (age - onset) - destroyed).max(0.0)
    };
    Gases { ch4, ..gases }
}

/// The gases left once what the cold freezes out has gone to the ground.
///
/// A gas whose triple point lies above the body's `equilibrium_k` cannot
/// stay liquid there: it frosts onto the surface until its partial
/// pressure falls to its sublimation pressure (Clausius–Clapeyron from
/// the triple point, see [`Frost`]). This freezes Titan's CO₂ and NH₃.
/// Above the triple point the gas stays: whether it rains out as a liquid
/// depends on the surface temperature, which the climate sets. Water is
/// the climate's too (a runaway's steam sits on a planet far hotter than
/// its equilibrium temperature), and so are methane and nitrogen, whose
/// air moves the surface temperature they condense at ([`condense`]):
/// Triton's N₂ is the vapour of its frost at its surface temperature,
/// which the frost's albedo lowers (issue #464).
pub fn cold_trap(gases: Gases, equilibrium_k: f64) -> Gases {
    Gases {
        co2: CO2_FROST.cap(gases.co2, equilibrium_k),
        nh3: NH3_FROST.cap(gases.nh3, equilibrium_k),
        ..gases
    }
}

/// Mass of N₂ per mass of NH₃ photolysed: `2 NH₃ → N₂ + 3 H₂`, 28 for
/// 34. Stoichiometry.
const N2_PER_NH3: f64 = 28.0 / 34.0;

/// Equilibrium temperature, K, under which methane survives in the air.
/// Above it, water vapour reaches the upper air, where its photolysis
/// makes the OH that oxidises methane within a geologically short time;
/// below it, water is frozen out so deep that none does (at 150 K the
/// sublimation pressure of ice is about 1e-11 bar). Calibration, no
/// source: between Titan (about 82 K at the model's albedo, keeps its
/// methane) and an abiotic Earth (255 K) or Mars (207 K), which do not.
const METHANE_SURVIVES_K: f64 = 150.0;

/// The gases once the star's ultraviolet has worked on them, over times
/// short against a planet's age. Apply it after [`cold_trap`]: only what
/// is still gas is photolysed.
///
/// - NH₃ is photolysed to N₂ and hydrogen, which escapes: the nitrogen is
///   conserved, `2 NH₃ → N₂ + 3 H₂` (Atreya, Donahue & Kuhn 1978,
///   *Science* 201, 611, doi:10.1126/science.201.4356.611, for Titan's N₂
///   from NH₃; reference from memory, not checked).
/// - CH₄ is kept only below [`METHANE_SURVIVES_K`]; above it OH oxidises
///   it and it is set to zero (the CO₂ it makes is not added: a trace
///   against what is outgassed). A labelled calibration on Titan against
///   an abiotic Earth; life's methane is Task 7's. Its photolysis, which
///   acts on the whole stock and not only on the gas, comes before the
///   cold trap ([`methane_photolysis`]).
pub fn photochemistry(gases: Gases, equilibrium_k: f64) -> Gases {
    Gases {
        n2: gases.n2 + gases.nh3 * N2_PER_NH3,
        nh3: 0.0,
        ch4: if equilibrium_k < METHANE_SURVIVES_K {
            gases.ch4
        } else {
            0.0
        },
        ..gases
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atmosphere::AIRLESS_BAR;
    use crate::exomoon::EARTH_KG;
    use crate::planet::core::core;
    use crate::planet::heat::{InternalHeat, radiogenic_w, secular_w};
    use crate::planet::interior::interior;
    use crate::planet::reference::{exposure_at, model_inputs, reference};
    use crate::planet::regime::{regime, water};
    use crate::planet::xuv::xuv_fluence;
    use crate::planet::{Context, PlanetInputs};
    use crate::star::{StarInfo, parse_type};

    /// The Solar System's age, Gyr.
    const NOW: f64 = 4.6;

    /// The Sun's context at `orbit_au`, 4.6 Gyr, with the insolation of
    /// exactly `1/a²` and its equilibrium temperature.
    fn sun(orbit_au: f64) -> Context {
        let star = StarInfo::new(parse_type("G2V").unwrap(), orbit_au as f32, 0.0, NOW as f32);
        let mut c = Context::of(&star, 0.0, 2.7);
        let exposure = exposure_at(orbit_au);
        c.insolation = exposure.insolation;
        c.equilibrium_k = exposure.equilibrium_k;
        c.star_mass = 1.0;
        c
    }

    /// The air of `inputs` at `orbit_au` round the Sun, through the chain
    /// of part 2: water, regime, core, outgassing, escape, methane
    /// photolysis, cold trap, photochemistry.
    fn air(inputs: &PlanetInputs, orbit_au: f64) -> Gases {
        let (c, int, air) = shoreline(inputs, orbit_au);
        let air = methane_photolysis(air, &c, inputs.mass_earth, &int);
        photochemistry(cold_trap(air, c.equilibrium_k), c.equilibrium_k)
    }

    /// The context, the interior and the air the cosmic shoreline leaves
    /// `inputs` at `orbit_au` round the Sun ([`air`]'s chain up to
    /// [`atmosphere`]).
    fn shoreline(inputs: &PlanetInputs, orbit_au: f64) -> (Context, Interior, Gases) {
        let c = sun(orbit_au);
        let int = interior(inputs, &c.exposure());
        let fluence = xuv_fluence(&c);
        let w = water(&c, inputs, &int, fluence);
        let silicate = inputs.mass_earth * (1.0 - inputs.water_fraction) * EARTH_KG;
        let heat = InternalHeat {
            radiogenic_w: radiogenic_w(silicate, inputs.radiogenic, NOW),
            tidal_w: 0.0,
            secular_w: secular_w(silicate, NOW),
        };
        let r = regime(inputs, &heat, &int, &w);
        let k = core(inputs, &int, r, NOW);
        let out = outgassed(inputs, &w, r, &int);
        let air = atmosphere(out, &int, inputs.mass_earth, &k, fluence);
        (c, int, air)
    }

    /// The air of the reference body `name`.
    fn air_of(name: &str) -> Gases {
        let b = reference(name);
        air(&model_inputs(b), b.orbit_au)
    }

    /// The surface air of the reference body `name` after the whole chain.
    fn surface_air(name: &str) -> Gases {
        let b = reference(name);
        let model = crate::planet::planet_model(
            &crate::planet::reference::sun_at(b.orbit_au),
            model_inputs(b),
        );
        model.surface.expect("a surface").gases
    }

    #[test]
    fn earths_air_keeps_no_sulphur_dioxide() {
        let g = surface_air("Earth");
        assert!(g.so2 < 1e-6, "{g:?}");
    }

    #[test]
    fn venus_keeps_its_sulphur_dioxide() {
        // About 150 ppm of 92 bar (Bertaux et al. 2007, Marcq et al. 2018).
        let g = surface_air("Venus");
        assert!((0.008..=0.02).contains(&g.so2), "{g:?}");
    }

    /// The venus-like archetype keeps the SO₂ of its outgassing on every
    /// seed (0.0054 to 0.072 bar on seeds 1-40 before the volcanic frost
    /// of #525): an episodic lid has no SO₂ frost, so its air is the
    /// outgassed SO₂ that stays in a dry air ([`SULPHUR_IN_AIR`],
    /// [`wash_out`]).
    #[test]
    fn venus_keeps_its_so2() {
        use crate::system::{SystemRequest, WholeSystemRequest, draw};
        use rand::SeedableRng;
        use std::collections::BTreeMap;
        for seed in 1..=40 {
            let request = SystemRequest::System(WholeSystemRequest {
                planets: Some(BTreeMap::from([("venus-like".to_string(), 1)])),
                seed: Some(seed),
                ..WholeSystemRequest::default()
            });
            let system = draw(
                &request,
                &mut rand::rngs::StdRng::seed_from_u64(u64::from(seed)),
            )
            .expect("the system draws");
            let body = system
                .bodies
                .iter()
                .find(|b| b.params.archetype.as_deref() == Some("venus-like"))
                .expect("a venus-like body");
            let s = body
                .model
                .as_ref()
                .and_then(|m| m.surface)
                .expect("surface");
            assert_ne!(s.regime, Regime::HeatPipe, "seed {seed}");
            let so2 = s.gases.so2;
            assert!((0.005..=0.08).contains(&so2), "seed {seed}: {s:?}");
        }
    }

    /// A heat pipe's volcanoes keep SO₂ frost only with more supply than
    /// the plasma takes, starting from none at that line; the frost's
    /// vapour at Io's dayside is of Io's nanobar, and no frost leaves the
    /// air as it is.
    #[test]
    fn volcanic_frost_needs_more_supply_than_loss() {
        let io = LossContext {
            gravity: 1.796,
            radius_m: 1.8216e6,
            orbit_au: 5.2,
            age_gyr: 4.57,
            host: Some(HostPlasma {
                host_mass_earth: JUPITER_MASS_EARTH,
                semi_major_axis_radii: 5.9,
            }),
        };
        let loss = so2_loss_kg_s(&io);
        // Io's torus takes about 1 t/s, mostly SO₂ and its fragments.
        assert!((100.0..=1000.0).contains(&loss), "{loss}");
        let bar_per_kg = 4.3e-19;
        assert_eq!(volcanic_so2_frost_bar(0.5 * loss, &io, bar_per_kg), 0.0);
        assert!(volcanic_so2_frost_bar(1.000_001 * loss, &io, bar_per_kg) < 1e-9);
        assert!(volcanic_so2_frost_bar(50.0 * loss, &io, bar_per_kg) > 0.02);
        let none = Gases::default();
        assert_eq!(frost_so2_air(none, 0.0, 95.0), none);
        let so2 = frost_so2_air(none, 0.05, 95.0).so2;
        assert!((1e-10..=1e-8).contains(&so2), "{so2:e}");
    }

    #[test]
    fn mars_keeps_little_sulphur_dioxide() {
        // Upper limit about 0.3 ppb (Encrenaz et al. 2011).
        let g = surface_air("Mars");
        assert!(g.so2 < 1e-6, "{g:?}");
    }

    #[test]
    fn a_wet_or_oxidising_air_loses_its_sulphur_dioxide() {
        let dry = Gases {
            so2: 1.0,
            ..Gases::default()
        };
        assert_eq!(wash_out(dry, SurfaceLiquid::None).so2, 1.0 / DRY_COLUMN_BAR);
        let thick = Gases { co2: 90.0, ..dry };
        assert_eq!(wash_out(thick, SurfaceLiquid::None).so2, 1.0);
        assert_eq!(wash_out(dry, SurfaceLiquid::Water).so2, WET_SO2_SHARE);
        let oxygen = Gases { o2: 0.2, ..dry };
        assert_eq!(wash_out(oxygen, SurfaceLiquid::None).so2, WET_SO2_SHARE);
    }

    #[test]
    fn venus_has_ninety_bars_of_co2() {
        let g = air_of("Venus");
        assert!((60.0..=120.0).contains(&g.total()), "{g:?}");
        assert!(g.co2 / g.total() > 0.9, "{g:?}");
        assert!((1.0..=6.0).contains(&g.n2), "{g:?}");
    }

    #[test]
    fn earth_has_a_bar_of_nitrogen() {
        // Before the carbonate thermostat (Task 6) takes the CO₂ away.
        let g = air_of("Earth");
        assert!((0.5..=1.2).contains(&g.n2), "{g:?}");
    }

    #[test]
    fn mars_keeps_a_trace() {
        let g = air_of("Mars");
        assert!((0.002..=0.05).contains(&g.total()), "{g:?}");
    }

    #[test]
    fn airless_bodies_have_no_pressure() {
        for name in ["Mercury", "Moon"] {
            let g = air_of(name);
            assert!(g.total() < f64::from(AIRLESS_BAR), "{name} {g:?}");
        }
    }

    #[test]
    fn earth_has_no_abiotic_methane_or_ammonia() {
        let g = air_of("Earth");
        assert!(g.ch4 < 1e-3 && g.nh3 < 1e-3, "{g:?}");
    }

    #[test]
    fn titan_keeps_its_methane() {
        let g = air_of("Titan");
        assert!(g.ch4 > 0.01, "{g:?}");
    }

    #[test]
    fn ganymede_and_callisto_lose_their_methane_to_photolysis() {
        // Their outgassing cannot keep up with the Sun's ultraviolet at
        // 5.2 AU, which destroys methane over three times faster than at
        // Titan.
        for name in ["Ganymede", "Callisto"] {
            let g = air_of(name);
            assert!(g.ch4 < 1e-3, "{name} {g:?}");
        }
    }

    #[test]
    fn titan_loses_methane_at_the_rate_of_yung_1984() {
        // 5.6e4 g/cm² over 4.5 Gyr: 1.48e10 molecules cm⁻² s⁻¹ today.
        let c = sun(9.58);
        let lyman_alpha = crate::planet::xuv::ratio(crate::planet::xuv::LYMAN_ALPHA, 1.0, NOW);
        // At 1 m/s², a bar is 1e5 kg/m².
        let kg_m2_s =
            methane_destroyed_bar_per_gyr(c.insolation, 1.0) * lyman_alpha * PA_PER_BAR / GYR_S;
        let rate = kg_m2_s / CH4_MOLECULE_KG * 1e-4;
        assert!((rate / 1.48e10 - 1.0).abs() < 0.01, "{rate:e}");
    }

    /// Methane outgassed, after escape, and what photolysis leaves of it,
    /// bar, for the reference body `name`.
    fn methane_budget(name: &str) -> (f64, f64) {
        let b = reference(name);
        let inputs = model_inputs(b);
        let c = sun(b.orbit_au);
        let int = interior(&inputs, &c.exposure());
        let fluence = xuv_fluence(&c);
        let w = water(&c, &inputs, &int, fluence);
        let silicate = inputs.mass_earth * (1.0 - inputs.water_fraction) * EARTH_KG;
        let heat = InternalHeat {
            radiogenic_w: radiogenic_w(silicate, inputs.radiogenic, NOW),
            tidal_w: 0.0,
            secular_w: secular_w(silicate, NOW),
        };
        let r = regime(&inputs, &heat, &int, &w);
        let k = core(&inputs, &int, r, NOW);
        let air = atmosphere(
            outgassed(&inputs, &w, r, &int),
            &int,
            inputs.mass_earth,
            &k,
            fluence,
        );
        let left = methane_photolysis(air, &c, inputs.mass_earth, &int);
        (air.ch4, left.ch4)
    }

    #[test]
    fn titans_outgassing_outruns_its_photolysis() {
        // Titan keeps a stock of methane on its ground, from which the
        // climate's condensation leaves its air a few hundredths of a bar.
        let (outgassed, left) = methane_budget("Titan");
        assert!(left > 1.0 && left < outgassed, "{outgassed} {left}");
        let air = surface_air("Titan");
        assert!(air.ch4 > 0.01 && air.ch4 < 0.2, "{air:?}");
    }

    #[test]
    fn photolysis_matches_a_stepwise_budget() {
        // The analytic stock against a step-by-step budget: a steady
        // supply, a destruction that follows the Lyman α history, and a
        // stock that never goes below zero.
        use crate::planet::xuv::{LYMAN_ALPHA, ratio};
        let b = reference("Titan");
        let inputs = model_inputs(b);
        for (orbit_au, star_mass, age, ch4) in [
            (9.58, 1.0, 4.6, 19.0),
            (9.58, 1.0, 4.6, 3.0),
            (5.2, 1.0, 4.6, 0.5),
            (1.0, 0.3, 8.0, 300.0),
            (9.58, 1.0, 0.05, 19.0),
        ] {
            let mut c = sun(orbit_au);
            c.star_mass = star_mass;
            c.age_gyr = age;
            let int = interior(&inputs, &c.exposure());
            let gases = Gases {
                ch4,
                ..Gases::default()
            };
            let exact = methane_photolysis(gases, &c, inputs.mass_earth, &int).ch4;
            let radius = int.radius_earth * EARTH_M;
            let gravity = G * inputs.mass_earth * EARTH_KG / (radius * radius);
            let per_ratio = methane_destroyed_bar_per_gyr(c.insolation, gravity);
            let steps = 200_000;
            let dt = age / f64::from(steps);
            let mut stock = 0.0_f64;
            for i in 0..steps {
                let t = (f64::from(i) + 0.5) * dt;
                stock = (stock + (ch4 / age - per_ratio * ratio(LYMAN_ALPHA, star_mass, t)) * dt)
                    .max(0.0);
            }
            assert!(
                (exact - stock).abs() <= 1e-3 * ch4,
                "{orbit_au} AU, {star_mass} M☉, {age} Gyr, {ch4} bar: {exact} {stock}"
            );
        }
    }

    #[test]
    fn photolysis_conserves_nitrogen() {
        let g = Gases {
            n2: 1.0,
            nh3: 34.0,
            ..Gases::default()
        };
        let p = photochemistry(g, 300.0);
        assert_eq!(p.nh3, 0.0);
        assert!((p.n2 - 29.0).abs() < 1e-9, "{p:?}");
    }

    /// Venus, the Earth and Mars lose nothing to [`non_thermal_loss`]:
    /// a planet's loss to its star is the shoreline's, so their air goes
    /// on as it was. The Earth's air would outlast even Jupiter's plasma
    /// at Callisto's place: its gravity holds what the plasma knocks up
    /// ([`escaping_share`]).
    #[test]
    fn thick_airs_are_untouched() {
        for name in ["Venus", "Earth", "Mars"] {
            let b = reference(name);
            let inputs = model_inputs(b);
            let (c, int, air) = shoreline(&inputs, b.orbit_au);
            let body = crate::planet::loss_context(&c, &inputs, &int);
            assert_eq!(body.host, None, "{name}");
            assert_eq!(
                non_thermal_loss(air, &body),
                (air, Gases::default()),
                "{name}"
            );
        }
        let earth = model_inputs(reference("Earth"));
        let (c, int, air) = shoreline(&earth, 1.0);
        let body = LossContext {
            orbit_au: 5.2,
            host: Some(HostPlasma {
                host_mass_earth: JUPITER_MASS_EARTH,
                semi_major_axis_radii: CALLISTO_RADII,
            }),
            ..crate::planet::loss_context(&c, &earth, &int)
        };
        assert_eq!(non_thermal_loss(air, &body), (air, Gases::default()));
    }

    /// A gas that cannot last is reduced to an exosphere of at most
    /// [`EXOSPHERE_BAR`], shared by the exhausted gases in proportion to
    /// their supply, never more than a gas had; what it lost is the
    /// rest.
    #[test]
    fn an_exhausted_air_is_an_exosphere() {
        let callisto = LossContext {
            gravity: 1.24,
            radius_m: 2.41e6,
            orbit_au: 5.2,
            age_gyr: 4.57,
            host: Some(HostPlasma {
                host_mass_earth: JUPITER_MASS_EARTH,
                semi_major_axis_radii: CALLISTO_RADII,
            }),
        };
        let air = Gases {
            n2: 1e-6,
            co2: 0.05,
            so2: 1e-15,
            ..Gases::default()
        };
        let (kept, lost) = non_thermal_loss(air, &callisto);
        assert!((kept.total() - EXOSPHERE_BAR).abs() < 1e-15, "{kept:?}");
        assert!(kept.co2 > kept.n2 && kept.n2 > 0.0, "{kept:?}");
        assert!(kept.so2 > 0.0 && kept.so2 < air.so2, "{kept:?}");
        // An air already thinner than an exosphere stays as it is.
        let trace = Gases {
            co2: 1e-13,
            ..Gases::default()
        };
        assert_eq!(
            non_thermal_loss(trace, &callisto),
            (trace, Gases::default())
        );
        let sum = |g: Gases| g.to_array().iter().sum::<f64>();
        assert!((sum(kept) + sum(lost) - sum(air)).abs() < 1e-15);
        // Outside a giant's magnetosphere, nothing goes.
        let planet = LossContext {
            host: None,
            ..callisto
        };
        assert_eq!(non_thermal_loss(air, &planet), (air, Gases::default()));
    }

    #[test]
    fn titan_keeps_a_thick_nitrogen_atmosphere() {
        // After the climate: most of its methane condenses on the ground.
        let g = surface_air("Titan");
        assert!((0.5..=5.0).contains(&g.total()), "{g:?}");
        assert!(g.n2 / g.total() > 0.5, "{g:?}");
    }

    #[test]
    fn escape_velocity_of_earth_is_eleven_km_s() {
        let e = reference("Earth");
        let i = model_inputs(e);
        let v = escape_velocity_km_s(&interior(&i, &exposure_at(1.0)), i.mass_earth);
        assert!((11.0..=11.4).contains(&v), "{v}");
    }

    #[test]
    fn a_dynamo_shields_the_air() {
        let e = reference("Mars");
        let i = model_inputs(e);
        let int = interior(&i, &exposure_at(e.orbit_au));
        let out = outgassed(
            &i,
            &Water {
                runaway: false,
                boiled: false,
                kept_kg: 0.0,
                oceans: 0.0,
            },
            Regime::Stagnant,
            &int,
        );
        let bare = Core {
            state: crate::planet::CoreState::Liquid,
            dynamo: false,
            field_ut: 0.0,
        };
        let shielded = Core {
            dynamo: true,
            field_ut: 30.0,
            ..bare
        };
        let fluence = 1.0 / (e.orbit_au * e.orbit_au);
        let a = atmosphere(out, &int, i.mass_earth, &bare, fluence).total();
        let b = atmosphere(out, &int, i.mass_earth, &shielded, fluence).total();
        assert!(out.total() > 0.0);
        assert!(b > a && a > 0.0, "{a} {b}");
        assert!(b <= out.total());
    }

    /// Pressure, bar, at the floor of the water of `inputs` at
    /// `orbit_au`, all its water kept.
    fn seafloor_bar(inputs: &PlanetInputs, orbit_au: f64) -> f64 {
        let int = interior(inputs, &exposure_at(orbit_au));
        inputs.water_fraction * inputs.mass_earth * EARTH_KG * bar_per_kg(inputs.mass_earth, &int)
    }

    #[test]
    fn a_deep_ocean_holds_back_what_the_mantle_outgasses() {
        // The icy moons keep nearly all their air under their ice.
        for name in ["Titan", "Ganymede", "Callisto"] {
            let b = reference(name);
            let p = seafloor_bar(&model_inputs(b), b.orbit_au);
            let share = released_under_ocean(p);
            assert!(share >= 0.98, "{name}: {p} bar, {share}");
        }
        // Earth's ocean changes nothing.
        let earth = model_inputs(reference("Earth"));
        let p = seafloor_bar(&earth, 1.0);
        assert!(released_under_ocean(p) > 0.999_99, "{p} bar");
        // An Earth of 30 % water lets about a thousandth through.
        let deep = PlanetInputs {
            water_fraction: 0.3,
            ..earth
        };
        let p = seafloor_bar(&deep, 1.0);
        let share = released_under_ocean(p);
        assert!((5e-4..=2e-3).contains(&share), "{p} bar, {share}");
        // No water, no hold; less as the floor deepens.
        assert_eq!(released_under_ocean(0.0), 1.0);
        assert!(released_under_ocean(2.0e4) < released_under_ocean(1.0e4));
    }

    #[test]
    fn a_runaway_keeps_its_water_as_vapour() {
        let e = reference("Earth");
        let i = model_inputs(e);
        let int = interior(&i, &exposure_at(1.0));
        let steam = |runaway: bool, kept_kg: f64| {
            let w = Water {
                runaway,
                boiled: false,
                kept_kg,
                oceans: kept_kg / crate::planet::OCEAN_KG,
            };
            outgassed(&i, &w, Regime::Plates, &int).h2o
        };
        assert!(steam(true, 1.4e21) > 1.0);
        assert_eq!(steam(true, 0.0), 0.0);
        assert_eq!(steam(false, 1.4e21), 0.0);
    }
}
