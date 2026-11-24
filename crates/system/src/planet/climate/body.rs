//! The body the climate settles: its inputs, its numerics, and what it
//! derives at a surface temperature (its albedo, its emissivity, its
//! frost), with the caches of its columns and of its light's paths.
//! Ported from `legacy-final:crates/mapgen/src/planet/climate.rs`
//! (`Body`, l.1605-1785, and `derived_albedo_over`, l.2102-2128).

use super::carbon::maximum_greenhouse;
use super::{ClimateInputs, Numerics};
use science::balance::PLATEAU_SURFACES;
use science::balance::{earth_moist_plateau, moist_plateau, runaway_albedo, steam_share};
use science::outgassing::column_pressure;
use science::radiation::albedo::{ShortwaveCache, Underneath, shortwave_budget};
use science::radiation::clouds::cloud_cover;
use science::radiation::column::{ColumnCache, Profile};
use science::radiation::gases::PartialPressures;
use science::radiation::veils::sulphuric_deck;
use science::surface::{condense, cover, frost, frost_emissivity, ground_albedo};
use science::units::{Bar, Fraction, Kelvin, Kilogram, WattPerSquareMetre};

/// A body whose climate is being solved (legacy `Body`): its inputs, the
/// numerics of the solve, the CO₂ its frozen branch holds (`None`: the
/// thermostat's, or the air's), what it searched once (its maximum
/// greenhouse and its moist plateau), and the caches of its columns and
/// of its light's paths, owned by the solve.
#[derive(Debug)]
pub(crate) struct ClimateBody<'a> {
    pub inputs: &'a ClimateInputs,
    pub numerics: Numerics,
    /// The CO₂ of the frozen branch, bar; `None` elsewhere.
    #[cfg_attr(
        test,
        expect(dead_code, reason = "read by the frozen branch, next commit")
    )]
    pub co2: Option<Bar>,
    /// The CO₂ of the maximum greenhouse, once found
    /// ([`ClimateBody::greenhouse_co2`]).
    pub greenhouse: Option<Bar>,
    /// The moist plateau, once found ([`ClimateBody::plateau`]).
    #[cfg_attr(
        test,
        expect(dead_code, reason = "read by the runaway's albedo, next commit")
    )]
    pub plateau: Option<WattPerSquareMetre>,
    pub columns: ColumnCache,
    pub shortwave: ShortwaveCache,
}

impl<'a> ClimateBody<'a> {
    /// The body of `inputs` solved with `numerics`, nothing found yet.
    pub(crate) fn new(inputs: &'a ClimateInputs, numerics: Numerics) -> Self {
        ClimateBody {
            inputs,
            numerics,
            co2: None,
            greenhouse: None,
            plateau: None,
            columns: ColumnCache::default(),
            shortwave: ShortwaveCache::default(),
        }
    }

    /// The pressure its water would weigh as vapour
    /// ([`column_pressure`] of the water kept on its gravity and radius;
    /// legacy `water_bar`).
    pub(crate) fn water_column(&mut self) -> Bar {
        let i = self.inputs;
        column_pressure(i.water, i.gravity, i.radius)
    }

    /// Whether it keeps water out of a runaway.
    pub(crate) fn wet(&mut self) -> bool {
        !self.inputs.runaway && self.inputs.water.0 > 0.0
    }

    /// The light its surface takes under the Bond `albedo`:
    /// `S₀/4 · Σ flux · (1 − albedo)`, `S₀` the unit of the flux.
    pub(crate) fn starlight(&mut self, albedo: Fraction) -> WattPerSquareMetre {
        let flux: f64 = self.inputs.sources.iter().map(|s| s.flux.0).sum();
        let top = WattPerSquareMetre::from(science::units::EarthFlux(flux));
        WattPerSquareMetre(top.0 / 4.0 * (1.0 - albedo.get()))
    }

    /// The weight of each star in its light, `fluxᵢ / Σ flux`; equal
    /// weights when no star sends any.
    fn weights(&self) -> Vec<f64> {
        let sources = &self.inputs.sources;
        let total: f64 = sources.iter().map(|s| s.flux.0.max(0.0)).sum();
        sources
            .iter()
            .map(|s| {
                if total > 0.0 {
                    s.flux.0.max(0.0) / total
                } else {
                    1.0 / sources.len() as f64
                }
            })
            .collect()
    }

    /// The frost on its ground at `t` under `air`, gas by gas
    /// ([`frost`] of the condensate [`condense`] leaves of its inventory
    /// at `t`, with the CO₂ the cold trap froze and its SO₂ frost).
    pub(crate) fn frost(&mut self, t: Kelvin, air: &PartialPressures) -> PartialPressures {
        let i = self.inputs;
        let condensate = condense(&i.air, t).condensate;
        frost(&condensate, i.frozen_co2, i.so2_frost, t, air)
    }

    /// What lies under its air at `t` under `air`: the ground's albedo
    /// ([`ground_albedo`] of its [`cover`]: its water when it is wet,
    /// none otherwise, its frost, its obliquity, whether it is locked),
    /// the water clouds ([`cloud_cover`]) and the sulphuric deck
    /// ([`sulphuric_deck`]).
    pub(crate) fn underneath(&mut self, t: Kelvin, air: &PartialPressures) -> Underneath {
        let water = if self.wet() {
            self.inputs.water
        } else {
            Kilogram(0.0)
        };
        let frost = self.frost(t, air);
        let i = self.inputs;
        let covered = cover(t, water, i.radius, &frost, i.obliquity, i.locked);
        Underneath {
            ground: ground_albedo(&covered, Bar(air.total())),
            clouds: cloud_cover(t, Bar(air.h2o)),
            deck: sulphuric_deck(t, Bar(air.so2)),
        }
    }

    /// Its Bond albedo at `t` under `air`: the [`shortwave_budget`] at
    /// each star's temperature over the same [`Underneath`]
    /// ([`ClimateBody::underneath`]), weighted by the stars' flux (#592).
    pub(crate) fn derived_albedo(&mut self, t: Kelvin, air: &PartialPressures) -> Fraction {
        let under = self.underneath(t, air);
        let weights = self.weights();
        let i = self.inputs;
        let mean: f64 = i
            .sources
            .iter()
            .zip(weights)
            .map(|(s, w)| {
                let budget = shortwave_budget(
                    air,
                    t,
                    i.gravity,
                    s.effective_temperature,
                    under,
                    &mut self.shortwave,
                );
                w * budget.bond_albedo.get()
            })
            .sum();
        Fraction::new(mean.clamp(0.0, 1.0)).expect("a mean of fractions is a fraction")
    }

    /// The emissivity of its surface at `t` under `air`
    /// ([`frost_emissivity`] of its frost there).
    pub(crate) fn emissivity_at(&mut self, t: Kelvin, air: &PartialPressures) -> Fraction {
        frost_emissivity(&self.frost(t, air))
    }

    /// The moist plateau its runaway albedo binds at its stars' limit
    /// (legacy `Body::plateau`, l.1672): for a body in a water runaway
    /// that keeps water, its own steam column's ([`moist_plateau`] of its
    /// dry air with its water, on its gravity and the saturated column,
    /// started from Earth's) where it exceeds Earth's air's
    /// ([`earth_moist_plateau`]); Earth's air's otherwise. Found once.
    #[cfg_attr(
        test,
        expect(dead_code, reason = "read by the fixed point, next commit")
    )]
    pub(crate) fn plateau(&mut self) -> WattPerSquareMetre {
        if let Some(p) = self.plateau {
            return p;
        }
        let earth = earth_moist_plateau();
        let i = self.inputs;
        let p = if i.runaway && i.water.0 > 0.0 {
            let dry = PartialPressures { h2o: 0.0, ..i.air };
            let water = self.water_column();
            let own = moist_plateau(
                &dry,
                water,
                i.gravity,
                Profile::SATURATED,
                &PLATEAU_SURFACES,
                earth,
                &mut self.columns,
            );
            WattPerSquareMetre(own.0.max(earth.0))
        } else {
            earth
        };
        self.plateau = Some(p);
        p
    }

    /// The Bond albedo it absorbs its light under, given its `derived`
    /// one (legacy `runaway_albedo_under`): for a body in a water runaway
    /// that keeps water, the blend from `derived` to the runaway albedo
    /// `A_rg` ([`runaway_albedo`] of each star on its [`plateau`],
    /// weighted by the stars' flux) by the [`steam_share`] of its air's
    /// vapour; `derived` otherwise.
    ///
    /// [`plateau`]: ClimateBody::plateau
    #[cfg_attr(
        test,
        expect(dead_code, reason = "read by the fixed point, next commit")
    )]
    pub(crate) fn runaway_albedo(&mut self, derived: Fraction) -> Fraction {
        let i = self.inputs;
        let share = steam_share(Bar(i.air.h2o)).get();
        if !(i.runaway && i.water.0 > 0.0) || share <= 0.0 {
            return derived;
        }
        let plateau = self.plateau();
        let weights = self.weights();
        let moist: f64 = self
            .inputs
            .sources
            .iter()
            .zip(weights)
            .map(|(s, w)| w * runaway_albedo(s.effective_temperature, plateau).get())
            .sum();
        let a = derived.get() + (moist - derived.get()) * share;
        Fraction::new(a.clamp(0.0, 1.0)).expect("a blend of fractions is a fraction")
    }

    /// The CO₂ of its maximum greenhouse ([`maximum_greenhouse`]),
    /// searched once on its inventory air and kept (#593 lever 2), or at
    /// each call when the numerics say `greenhouse_once` is off.
    pub(crate) fn greenhouse_co2(&mut self) -> Bar {
        if self.numerics.greenhouse_once
            && let Some(co2) = self.greenhouse
        {
            return co2;
        }
        let co2 = maximum_greenhouse(self);
        self.greenhouse = Some(co2);
        co2
    }
}
