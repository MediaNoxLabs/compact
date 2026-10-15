// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Real neutral crypto/leaf owners, no syntax or native fallback in the consumer.
//! This is a deliberately limited test protocol, not production body admission.
use super::*;
use crate::pure_semantics::{
    carrier::CheckSink,
    children::Children,
    crypto::{CheckedCrypto, fold_crypto},
    leaf::{LeafSource, fold_leaf},
    sink::{CheckedLeaf, CryptoSink, LeafSink, Typed},
};
struct NoSyntax {
    carrier: CheckSink,
}
impl LeafSink for NoSyntax {
    type Value = ();
    type Binding = ();
    type Carriers = CheckSink;
    fn carriers(&mut self) -> &mut CheckSink {
        &mut self.carrier
    }
    fn leaf(&mut self, _: CheckedLeaf<'_, (), ()>) {}
}
impl CryptoSink for NoSyntax {
    fn crypto(&mut self, _: CheckedCrypto<()>) {}
}
struct TestChildren<'a> {
    bindings: HashMap<&'a str, (&'a Type, ())>,
}
impl Children<NoSyntax> for TestChildren<'_> {
    fn visit(&mut self, source: &Expr, sink: &mut NoSyntax) -> Result<Typed<()>, RenderError> {
        if let Some(source) = source::descriptor(source) {
            return fold_crypto(source, self, sink);
        }
        let source = match source {
            Expr::Unit => LeafSource::Unit,
            Expr::Default { ty } => LeafSource::Default(ty),
            Expr::Boolean { value } => LeafSource::Boolean(*value),
            Expr::FieldLiteral { value } => LeafSource::FieldLiteral(value),
            Expr::BytesLiteral { bytes } => LeafSource::BytesLiteral(bytes),
            Expr::UnsignedLiteral { value, max } => LeafSource::UnsignedLiteral { value, max },
            Expr::Parameter { name } => LeafSource::Parameter(name),
            Expr::EnumVariant { ty, variant } => LeafSource::EnumVariant { ty, variant },
            Expr::KernelSelf { .. } => LeafSource::Effectful,
            _ => unreachable!("crypto/leaf-only test protocol"),
        };
        // fold_leaf performs the original selected-Type scan, not carrier
        // validation of private Parameter payload annotations.
        fold_leaf(source, &self.bindings, sink)
    }
}
pub(super) fn compare(
    source: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
) -> Result<Type, RenderError> {
    let current = observe(source, parameters, &HashMap::new());
    assert_eq!(
        current,
        old_oracle::observe(source, parameters, &HashMap::new()),
        "{source:?}"
    );
    // Old crypto retains helper entrypoints; separate exact helper-oracle tests
    // below independently prevent a changed helper from masking checker drift.
    let bindings = parameters
        .iter()
        .map(|(name, (ty, _))| (*name, (*ty, ())))
        .collect();
    let checked = fold_crypto(
        source::descriptor(source).unwrap(),
        &mut TestChildren { bindings },
        &mut NoSyntax { carrier: CheckSink },
    )
    .map(|typed| typed.ty);
    let native = current.map(|(_, _, ty)| ty);
    assert_eq!(checked, native, "generic consumer {source:?}");
    native
}
