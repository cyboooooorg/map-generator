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
//! (life) come after.
use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

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
/// 337, 55; not checked). Calibration, no source: Venus's SO₂ of about
/// 150 ppm (value from memory).
const SULPHUR_IN_AIR: f64 = 1.0e-4;

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

/// A gas's triple point (K, bar) and its enthalpy of sublimation
/// (J/mol). Values from memory (NIST Chemistry WebBook), not checked:
/// CO₂ 216.6 K and 5.185 bar, 26.1 kJ/mol (which gives its 1 atm
/// sublimation at 194.7 K); NH₃ 195.4 K and 0.0606 bar, 29 kJ/mol; CH₄
/// 90.7 K and 0.117 bar, 9.1 kJ/mol; N₂ 63.15 K and 0.125 bar,
/// 6.3 kJ/mol.
struct Frost {
    triple_k: f64,
    triple_bar: f64,
    sublimation_j_mol: f64,
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
const N2_FROST: Frost = Frost {
    triple_k: 63.15,
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
/// the triple point, see [`Frost`]). This freezes Titan's CO₂ and NH₃ and
/// most of its CH₄. Above the triple point the gas stays: whether it
/// rains out as a liquid depends on the surface temperature, which the
/// climate sets. Water is the climate's too (a runaway's steam sits on a
/// planet far hotter than its equilibrium temperature).
pub fn cold_trap(gases: Gases, equilibrium_k: f64) -> Gases {
    Gases {
        co2: CO2_FROST.cap(gases.co2, equilibrium_k),
        nh3: NH3_FROST.cap(gases.nh3, equilibrium_k),
        ch4: CH4_FROST.cap(gases.ch4, equilibrium_k),
        n2: N2_FROST.cap(gases.n2, equilibrium_k),
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
    use crate::planet::heat::{InternalHeat, radiogenic_w};
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
        let c = sun(orbit_au);
        let int = interior(inputs, &c.exposure());
        let fluence = xuv_fluence(&c);
        let w = water(&c, inputs, &int, fluence);
        let silicate = inputs.mass_earth * (1.0 - inputs.water_fraction) * EARTH_KG;
        let heat = InternalHeat {
            radiogenic_w: radiogenic_w(silicate, inputs.radiogenic, NOW),
            tidal_w: 0.0,
        };
        let r = regime(inputs, &heat, &int, &w);
        let k = core(inputs, &int, r, NOW);
        let out = outgassed(inputs, &w, r, &int);
        let air = atmosphere(out, &int, inputs.mass_earth, &k, fluence);
        let air = methane_photolysis(air, &c, inputs.mass_earth, &int);
        photochemistry(cold_trap(air, c.equilibrium_k), c.equilibrium_k)
    }

    /// The air of the reference body `name`.
    fn air_of(name: &str) -> Gases {
        let b = reference(name);
        air(&model_inputs(b), b.orbit_au)
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
        // cold trap leaves its air a few hundredths of a bar.
        let (outgassed, left) = methane_budget("Titan");
        assert!(left > 1.0 && left < outgassed, "{outgassed} {left}");
        assert!(air_of("Titan").ch4 < left);
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

    #[test]
    fn titan_keeps_a_thick_nitrogen_atmosphere() {
        let g = air_of("Titan");
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
