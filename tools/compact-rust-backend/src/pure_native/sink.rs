// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Complete native consumer. Existing family formatters consume checked facts.
//! Dual views are selected after checking, with no source/type decisions here.
use super::{fragment::NativeFragment, leaf::NativeLeafSink, value};
use crate::{
    pure_semantics::{
        aggregate::CheckedAggregate,
        arithmetic::ArithmeticFacts,
        control::CheckedControl,
        conversion::ConversionFacts,
        crypto::CheckedCrypto,
        identifier::CheckedName,
        sink::{
            AggregateSink, ArithmeticSink, CheckedLeaf, ControlSink, ConversionSink, CryptoSink,
            ExpressionSink, LeafSink, OrdinaryView,
        },
    },
    value_lowering::carrier::NativeCarrierSink,
};

pub(crate) struct NativeSink {
    syntax: NativeLeafSink,
}
impl NativeSink {
    pub(crate) fn new() -> Self {
        Self {
            syntax: NativeLeafSink::new(),
        }
    }
}
impl LeafSink for NativeSink {
    type Value = NativeFragment;
    type Binding = syn::Ident;
    type Carriers = NativeCarrierSink;
    fn carriers(&mut self) -> &mut Self::Carriers {
        self.syntax.carriers()
    }
    fn leaf(&mut self, facts: CheckedLeaf<'_, syn::Type, syn::Ident>) -> Self::Value {
        NativeFragment::Value(self.syntax.leaf(facts))
    }
}
impl ConversionSink for NativeSink {
    fn conversion(&mut self, facts: ConversionFacts, child: Self::Value) -> Self::Value {
        NativeFragment::Value(self.syntax.conversion(facts, child.into_expression()))
    }
}
impl ArithmeticSink for NativeSink {
    fn arithmetic(
        &mut self,
        facts: ArithmeticFacts,
        left: Self::Value,
        right: Self::Value,
    ) -> Self::Value {
        NativeFragment::Value(self.syntax.arithmetic(
            facts,
            left.into_expression(),
            right.into_expression(),
        ))
    }
}
impl CryptoSink for NativeSink {
    fn crypto(&mut self, checked: CheckedCrypto<Self::Value>) -> Self::Value {
        NativeFragment::Value(self.syntax.crypto(value::crypto(checked)))
    }
}
impl AggregateSink for NativeSink {
    fn binding(&mut self, name: &CheckedName) -> Self::Binding {
        self.syntax.binding(name)
    }
    fn aggregate(
        &mut self,
        checked: CheckedAggregate<Self::Value, Self::Binding, syn::Type>,
    ) -> Self::Value {
        NativeFragment::Value(self.syntax.aggregate(value::aggregate(checked)))
    }
}
impl ControlSink for NativeSink {
    fn local_binding<'a>(
        &mut self,
        name: &CheckedName,
        raw_key: &str,
        occupied: impl Iterator<Item = (&'a str, &'a syn::Ident)> + Clone,
    ) -> syn::Ident {
        super::local::allocate(name, raw_key, occupied)
    }
    fn control<'a>(
        &mut self,
        checked: CheckedControl<'_, Self::Value, syn::Ident, syn::Type>,
        occupied: impl Iterator<Item = (&'a str, &'a syn::Ident)> + Clone,
    ) -> Self::Value {
        let names = occupied.map(|(_, name)| name);
        match checked {
            CheckedControl::Assert { condition, message } => {
                super::control::assertion(condition.into_expression(), message, names)
            }
            CheckedControl::Sequence { steps, value } => {
                let statements = steps
                    .into_iter()
                    .flat_map(|step| step.into_views().1)
                    .collect();
                super::fragment::sequence(statements, value)
            }
            CheckedControl::If {
                facts,
                condition,
                then,
                otherwise,
            } => super::control::conditional(
                super::control::IfFacts {
                    equal_arms: facts.equal_arms,
                    evaluate_condition: facts.evaluate_condition,
                    unit_result: facts.unit_result,
                },
                condition.into_expression(),
                then,
                otherwise,
                names,
            ),
            CheckedControl::Let { bindings, body } => {
                let statements = bindings
                    .into_iter()
                    .map(|local| {
                        let name = local.binding;
                        let ty = local.carrier;
                        let value = local.value.into_expression();
                        syn::parse_quote!(let #name: #ty = #value;)
                    })
                    .collect();
                super::fragment::lexical(statements, body)
            }
        }
    }
}
impl ExpressionSink for NativeSink {
    fn ordinary(&mut self, view: OrdinaryView, value: Self::Value) -> Self::Value {
        match view {
            OrdinaryView::Value => value,
            OrdinaryView::EmptyUnit => NativeFragment::unit(value.into_expression(), Vec::new()),
            OrdinaryView::ExecutableUnit => {
                let expression = value.into_expression();
                let statements = vec![syn::parse_quote!(#expression;)];
                NativeFragment::unit(expression, statements)
            }
        }
    }
    #[cfg(test)]
    fn observe_source(&mut self, source: &crate::ir::Expr) {
        super::visitation::record(source);
    }
}
