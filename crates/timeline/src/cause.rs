//! The cause graph: every derived quantity is produced together with its
//! cause node by the code that computes it, never recomputed to explain
//! it. Codes and parameters only, no human text.

use crate::ContractError;
use science::units::{
    AstronomicalUnit, Bar, Day, Degree, Dex, EarthFlux, EarthMass, EarthRadius, Fraction, Gyr,
    JoulePerSquareMetre, Kelvin, Kilogram, KilogramPerCubicMetre, Metre, MetrePerSecondSquared,
    Pascal, Radian, Second, SolarLuminosity, SolarMass, SolarRadius, Tesla, Watt,
    WattPerSquareMetre, Year,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

/// A dotted lowercase code (`climate.surface-temperature`): segments of
/// `[a-z][a-z0-9-]*` joined by dots.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Code(Cow<'static, str>);

fn is_valid_code(text: &str) -> bool {
    !text.is_empty()
        && text.split('.').all(|segment| {
            let mut chars = segment.chars();
            chars.next().is_some_and(|c| c.is_ascii_lowercase())
                && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
}

impl Code {
    /// A code from a literal.
    ///
    /// # Panics
    ///
    /// When `text` is not a valid code: a literal is a programmer's
    /// choice, so an invalid one is a bug.
    pub fn of(text: &'static str) -> Code {
        assert!(is_valid_code(text), "invalid code literal: {text:?}");
        Code(Cow::Borrowed(text))
    }

    /// A code from runtime text.
    pub fn new(text: String) -> Result<Code, ContractError> {
        if is_valid_code(&text) {
            Ok(Code(Cow::Owned(text)))
        } else {
            Err(ContractError::InvalidCode(text))
        }
    }

    /// The code as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for Code {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Code {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Code::new(text).map_err(serde::de::Error::custom)
    }
}

/// Where a leaf value comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Provenance {
    /// A user or catalogue input.
    Input,
    /// A measured value.
    Observed,
    /// A draw from the seed hierarchy.
    Drawn,
    /// A value fixed by the engine.
    Imposed,
}

/// The value a node reports.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NodeValue {
    /// A number with its unit symbol.
    Number { value: f64, unit: Cow<'static, str> },
    /// A code (the name of a contract enum variant).
    Code(Code),
    /// A yes or no.
    Flag(bool),
}

/// What a traced value reports to the cause graph.
pub trait NodeValueOf {
    /// The value as a node reports it.
    fn node_value(&self) -> NodeValue;
}

macro_rules! number_node_value {
    ($($unit:ident),* $(,)?) => {$(
        impl NodeValueOf for $unit {
            fn node_value(&self) -> NodeValue {
                NodeValue::Number { value: self.0, unit: Cow::Borrowed($unit::SYMBOL) }
            }
        }
    )*};
}

number_node_value!(
    Kelvin,
    Pascal,
    Kilogram,
    Metre,
    Second,
    Watt,
    WattPerSquareMetre,
    JoulePerSquareMetre,
    MetrePerSecondSquared,
    KilogramPerCubicMetre,
    Radian,
    Tesla,
    AstronomicalUnit,
    SolarMass,
    SolarRadius,
    SolarLuminosity,
    EarthMass,
    EarthRadius,
    EarthFlux,
    Bar,
    Day,
    Year,
    Gyr,
    Degree,
    Dex,
);

impl NodeValueOf for Fraction {
    fn node_value(&self) -> NodeValue {
        NodeValue::Number {
            value: self.get(),
            unit: Cow::Borrowed(Fraction::SYMBOL),
        }
    }
}

impl NodeValueOf for bool {
    fn node_value(&self) -> NodeValue {
        NodeValue::Flag(*self)
    }
}

/// The index of a node in its graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub(crate) u32);

impl NodeId {
    /// The index as a number.
    pub fn get(self) -> u32 {
        self.0
    }
}

/// How a node came to be.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Origin {
    /// Computed from other nodes.
    Derived { from: Vec<NodeId> },
    /// A starting value.
    Leaf(Provenance),
}

/// One cause: a code, the value it reports and its origin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub code: Code,
    pub value: NodeValue,
    pub origin: Origin,
}

/// A value with the node that explains it. Only a [`Trace`] creates one
/// (or deserialisation).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Traced<T> {
    pub(crate) value: T,
    pub(crate) node: NodeId,
}

impl<T> Traced<T> {
    /// The value.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// The node that explains it.
    pub fn node(&self) -> NodeId {
        self.node
    }
}

/// The nodes of a finished timeline.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CauseGraph {
    pub nodes: Vec<Node>,
}

impl CauseGraph {
    /// The node of an id.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0 as usize)
    }

    /// Every source id exists and is smaller than its node's own id (the
    /// graph is built in order, so it is acyclic). A source that does not
    /// exist, or that is not older than its node, is an `UnknownNode`.
    pub fn validate(&self) -> Result<(), ContractError> {
        for (index, node) in self.nodes.iter().enumerate() {
            if let Origin::Derived { from } = &node.origin {
                for id in from {
                    if id.0 as usize >= index {
                        return Err(ContractError::UnknownNode { node: id.0 });
                    }
                }
            }
        }
        Ok(())
    }
}

/// A state that holds node ids, so a freeze can renumber them.
pub trait TracedFields {
    /// Calls `f` on every node id held.
    fn for_each_node(&mut self, f: &mut dyn FnMut(&mut NodeId));
}

impl<T> TracedFields for Traced<T> {
    fn for_each_node(&mut self, f: &mut dyn FnMut(&mut NodeId)) {
        f(&mut self.node);
    }
}

impl<T: TracedFields> TracedFields for Option<T> {
    fn for_each_node(&mut self, f: &mut dyn FnMut(&mut NodeId)) {
        if let Some(inner) = self {
            inner.for_each_node(f);
        }
    }
}

impl<T: TracedFields> TracedFields for Vec<T> {
    fn for_each_node(&mut self, f: &mut dyn FnMut(&mut NodeId)) {
        for inner in self {
            inner.for_each_node(f);
        }
    }
}

impl<K, V: TracedFields> TracedFields for BTreeMap<K, V> {
    fn for_each_node(&mut self, f: &mut dyn FnMut(&mut NodeId)) {
        for inner in self.values_mut() {
            inner.for_each_node(f);
        }
    }
}

/// The recorder of causes while a timeline is computed.
#[derive(Clone, Debug, Default)]
pub struct Trace {
    nodes: Vec<Node>,
}

impl Trace {
    /// An empty trace.
    pub fn new() -> Trace {
        Trace::default()
    }

    /// The node of an id this trace issued.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0 as usize)
    }

    /// Records a starting value.
    pub fn leaf<T: NodeValueOf>(
        &mut self,
        code: Code,
        value: T,
        provenance: Provenance,
    ) -> Result<Traced<T>, ContractError> {
        self.push(code, value, Origin::Leaf(provenance))
    }

    /// The number of nodes recorded so far.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether nothing was recorded yet.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Records a value computed from the nodes `from`. Refuses an id the
    /// trace has not issued yet (`id >= len`); it cannot tell an id of
    /// another trace that happens to be below its length.
    pub fn record<T: NodeValueOf>(
        &mut self,
        code: Code,
        value: T,
        from: &[NodeId],
    ) -> Result<Traced<T>, ContractError> {
        if let Some(id) = from.iter().find(|id| id.0 as usize >= self.nodes.len()) {
            return Err(ContractError::UnknownNode { node: id.0 });
        }
        self.push(
            code,
            value,
            Origin::Derived {
                from: from.to_vec(),
            },
        )
    }

    fn push<T: NodeValueOf>(
        &mut self,
        code: Code,
        value: T,
        origin: Origin,
    ) -> Result<Traced<T>, ContractError> {
        let node_value = value.node_value();
        if let NodeValue::Number { value: number, .. } = &node_value
            && !number.is_finite()
        {
            return Err(ContractError::NotFinite {
                code: code.as_str().to_owned(),
            });
        }
        let id = u32::try_from(self.nodes.len()).expect("fewer than 2^32 nodes");
        self.nodes.push(Node {
            code,
            value: node_value,
            origin,
        });
        Ok(Traced {
            value,
            node: NodeId(id),
        })
    }

    /// Keeps the nodes `state` holds and their transitive sources, in
    /// increasing id order, renumbered from 0 (so every source stays
    /// smaller than its node), and rewrites the ids in `state`. Refuses
    /// an id past the end of this trace (`id >= len`); an id of another
    /// trace that is below its length cannot be told apart.
    pub fn freeze<S: TracedFields>(&self, state: &mut S) -> Result<CauseGraph, ContractError> {
        let mut held = Vec::new();
        state.for_each_node(&mut |id| held.push(*id));
        let mut keep = BTreeSet::new();
        let mut pending = Vec::new();
        for id in held {
            if self.node(id).is_none() {
                return Err(ContractError::UnknownNode { node: id.0 });
            }
            pending.push(id);
        }
        while let Some(id) = pending.pop() {
            if !keep.insert(id) {
                continue;
            }
            if let Some(Node {
                origin: Origin::Derived { from },
                ..
            }) = self.node(id)
            {
                pending.extend(from.iter().copied());
            }
        }
        let renumbered: BTreeMap<NodeId, NodeId> = keep
            .iter()
            .enumerate()
            .map(|(new, old)| {
                (
                    *old,
                    NodeId(u32::try_from(new).expect("fewer than 2^32 nodes")),
                )
            })
            .collect();
        let mut nodes = Vec::with_capacity(keep.len());
        for old in &keep {
            let mut node = self.nodes[old.0 as usize].clone();
            if let Origin::Derived { from } = &mut node.origin {
                for id in from {
                    *id = renumbered[id];
                }
            }
            nodes.push(node);
        }
        state.for_each_node(&mut |id| *id = renumbered[id]);
        Ok(CauseGraph { nodes })
    }
}

#[cfg(test)]
#[path = "cause_tests.rs"]
mod tests;
