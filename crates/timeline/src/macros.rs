//! Declarations shared by the state modules: contract enums and the
//! `TracedFields` plumbing of structs.

/// A contract enum: kebab-case in serde, `#[non_exhaustive]`, and a
/// `NodeValueOf` reporting the variant as a code (the same kebab-case
/// name). The names are written out so a rename of a variant is a
/// visible change of the contract.
macro_rules! contract_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $code:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
            serde::Serialize, serde::Deserialize,
        )]
        #[serde(rename_all = "kebab-case")]
        #[non_exhaustive]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            /// The variant as a code, as serialised.
            pub fn code(self) -> &'static str {
                match self {
                    $(Self::$variant => $code),+
                }
            }
        }

        impl crate::NodeValueOf for $name {
            fn node_value(&self) -> crate::NodeValue {
                crate::NodeValue::Code(crate::Code::of(self.code()))
            }
        }
    };
}

/// `TracedFields` for a struct: visits the listed fields in order.
macro_rules! traced_fields {
    ($name:ident { $($field:ident),+ $(,)? }) => {
        impl crate::TracedFields for $name {
            fn for_each_node(&mut self, f: &mut dyn FnMut(&mut crate::NodeId)) {
                $(self.$field.for_each_node(f);)+
            }
        }
    };
}

pub(crate) use {contract_enum, traced_fields};
