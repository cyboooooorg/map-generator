//! The interior of a body: layers with their stored mass, and radial
//! samples of the state of the matter.

use crate::macros::{contract_enum, traced_fields};
use crate::{ContractError, NodeValue, NodeValueOf, Traced};
use science::units::{Fraction, Kelvin, Kilogram, KilogramPerCubicMetre, Metre, Pascal};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

contract_enum! {
    /// What a layer is made of.
    Material {
        Iron => "iron",
        Rock => "rock",
        RockIce => "rock-ice",
        Water => "water",
        Ice => "ice",
        Steam => "steam",
        Envelope => "envelope",
        Plasma => "plasma",
    }
}

contract_enum! {
    /// The state of the matter of a layer.
    MatterState {
        Solid => "solid",
        Liquid => "liquid",
        Gas => "gas",
        Plasma => "plasma",
        PartlyMolten => "partly-molten",
        Porous => "porous",
    }
}

contract_enum! {
    /// How the heat crosses a sample.
    Transport {
        Radiative => "radiative",
        Convective => "convective",
    }
}

/// One shell, from the outer radius of the layer below.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub outer_radius: Traced<Metre>,
    /// Stored: cross-sections and the moment of inertia read it, never
    /// an integral of the samples.
    pub mass: Traced<Kilogram>,
    pub material: Traced<Material>,
    pub state: Traced<MatterState>,
}

traced_fields!(Layer {
    outer_radius,
    mass,
    material,
    state
});

/// The matter at one radius.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RadialSample {
    pub radius: Metre,
    pub density: KilogramPerCubicMetre,
    pub pressure: Pascal,
    pub temperature: Kelvin,
    pub transport: Option<Transport>,
    /// Share of the energy output made by fusion, in a star.
    pub fusion_share: Option<Fraction>,
}

/// Samples by increasing radius.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RadialSamples(pub Vec<RadialSample>);

impl NodeValueOf for RadialSamples {
    /// The sample count.
    fn node_value(&self) -> NodeValue {
        NodeValue::Number {
            value: self.0.len() as f64,
            unit: Cow::Borrowed("1"),
        }
    }
}

/// The layers and the samples of a body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InteriorProfile {
    pub layers: Vec<Layer>,
    pub samples: Traced<RadialSamples>,
}

traced_fields!(InteriorProfile { layers, samples });

impl InteriorProfile {
    /// Layers strictly increasing in outer radius, samples increasing
    /// in radius, every number finite.
    pub fn validate(&self) -> Result<(), ContractError> {
        let layers = self
            .layers
            .windows(2)
            .all(|w| w[0].outer_radius.value().0 < w[1].outer_radius.value().0);
        let samples = self
            .samples
            .value()
            .0
            .windows(2)
            .all(|w| w[0].radius.0 <= w[1].radius.0);
        if !layers || !samples {
            return Err(ContractError::LayersNotOrdered);
        }
        for s in &self.samples.value().0 {
            let numbers = [s.radius.0, s.density.0, s.pressure.0, s.temperature.0];
            if numbers.iter().any(|n| !n.is_finite()) {
                return Err(ContractError::NotFinite {
                    code: "interior.samples".to_owned(),
                });
            }
        }
        Ok(())
    }
}
