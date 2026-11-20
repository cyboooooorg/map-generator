//! Every number in `science` is sourced, calibrated or a definition (No
//! invented data, rule 3).

use syn::visit::{self, Visit};
use syn::{Attribute, Expr, ImplItemConst, ItemConst, ItemImpl, ItemMod, ItemStatic, Lit, Type};

/// Doc-comment prefixes that account for a constant: a citation, a fitted
/// value with how it was fitted, or an exact definition or unit conversion.
pub const PREFIXES: [&str; 3] = ["Source:", "Calibration:", "Definition:"];

/// Names (with their module and `impl` path) of the numeric `const` and
/// `static` items of `source` whose doc comment does not start with one of
/// [`PREFIXES`]. An item is numeric when its type is a primitive number or
/// its initializer holds a numeric literal (a typed unit such as
/// `Kelvin(5772.0)` counts). Panics when `source` does not parse: the
/// compiler would reject it first.
pub fn undocumented(source: &str) -> Vec<String> {
    let file = syn::parse_file(source).expect("science source parses");
    let mut visitor = Finder::default();
    visitor.visit_file(&file);
    visitor.found
}

#[derive(Default)]
struct Finder {
    path: Vec<String>,
    found: Vec<String>,
}

impl Finder {
    fn check(&mut self, name: &syn::Ident, attrs: &[Attribute], ty: &Type, expr: Option<&Expr>) {
        let numeric = is_numeric_type(ty) || expr.is_some_and(has_numeric_literal);
        if numeric && !documented(attrs) {
            let mut full = self.path.clone();
            full.push(name.to_string());
            self.found.push(full.join("::"));
        }
    }
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item_const(&mut self, item: &'ast ItemConst) {
        self.check(&item.ident, &item.attrs, &item.ty, Some(&item.expr));
    }

    fn visit_item_static(&mut self, item: &'ast ItemStatic) {
        self.check(&item.ident, &item.attrs, &item.ty, Some(&item.expr));
    }

    fn visit_impl_item_const(&mut self, item: &'ast ImplItemConst) {
        self.check(&item.ident, &item.attrs, &item.ty, Some(&item.expr));
    }

    fn visit_trait_item_const(&mut self, item: &'ast syn::TraitItemConst) {
        let expr = item.default.as_ref().map(|(_, expr)| expr);
        self.check(&item.ident, &item.attrs, &item.ty, expr);
    }

    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        self.path.push(item.ident.to_string());
        visit::visit_item_mod(self, item);
        self.path.pop();
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        let name = match item.self_ty.as_ref() {
            Type::Path(path) => path
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string())
                .unwrap_or_default(),
            _ => "impl".to_owned(),
        };
        self.path.push(name);
        visit::visit_item_impl(self, item);
        self.path.pop();
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.path.push(item.ident.to_string());
        visit::visit_item_trait(self, item);
        self.path.pop();
    }
}

fn documented(attrs: &[Attribute]) -> bool {
    let first = attrs.iter().find_map(|attr| {
        if !attr.path().is_ident("doc") {
            return None;
        }
        match &attr.meta {
            syn::Meta::NameValue(value) => match &value.value {
                Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(text),
                    ..
                }) => Some(text.value()),
                _ => None,
            },
            _ => None,
        }
    });
    first.is_some_and(|line| {
        PREFIXES
            .iter()
            .any(|prefix| line.trim_start().starts_with(prefix))
    })
}

fn is_numeric_type(ty: &Type) -> bool {
    const NUMBERS: [&str; 14] = [
        "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64",
        "u128", "usize",
    ];
    match ty {
        Type::Path(path) => path
            .path
            .get_ident()
            .is_some_and(|ident| NUMBERS.contains(&ident.to_string().as_str())),
        Type::Array(array) => is_numeric_type(&array.elem),
        Type::Reference(reference) => is_numeric_type(&reference.elem),
        Type::Slice(slice) => is_numeric_type(&slice.elem),
        _ => false,
    }
}

fn has_numeric_literal(expr: &Expr) -> bool {
    #[derive(Default)]
    struct Literals(bool);
    impl<'ast> Visit<'ast> for Literals {
        fn visit_lit(&mut self, lit: &'ast Lit) {
            if matches!(lit, Lit::Int(_) | Lit::Float(_)) {
                self.0 = true;
            }
        }
    }
    let mut literals = Literals::default();
    literals.visit_expr(expr);
    literals.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_need_a_source_a_calibration_or_a_definition() {
        let source = r#"
            /// Source: CODATA 2018.
            pub const G: f64 = 6.674_30e-11;
            /// Seconds per day.
            pub const DAY: f64 = 86_400.0;
            /// Definition: seconds per day.
            pub const DAY_S: f64 = 86_400.0;
            /// Calibration: fitted on Earth's 1 bar.
            pub const P: Pascal = Pascal(101_325.0);
            pub(crate) static K: f32 = 0.3;
            mod inner { const X: u32 = 7; }
            struct Foo;
            impl Foo { const Y: f64 = 1.5; }
            trait Law { const Z: f64 = 2.5; }
            const NAME: &str = "x";
            const UNIT: Kelvin = Kelvin::ZERO;
            pub const SOLAR: Kelvin = Kelvin(5772.0);
        "#;
        assert_eq!(
            undocumented(source),
            ["DAY", "K", "inner::X", "Foo::Y", "Law::Z", "SOLAR"]
        );
    }
}
