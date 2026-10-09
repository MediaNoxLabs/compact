// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Immutable contract declarations, retained independently of lowered operations.
//! IDs are deterministic within this environment and are never serialized.
use super::model::{CallId, LedgerId, WitnessId};
use super::pure_certification::{Certificates, CertifiedPure};
use crate::ir::{LedgerField, Parameter, PureCircuit, StatefulCircuit, Type, WitnessDeclaration};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub(super) struct Declarations {
    ledger: Vec<LedgerField>,
    witnesses: Vec<WitnessDeclaration>,
    calls: Vec<CallSignature>,
    root: Option<CallSignature>,
    pure: Certificates,
    types: super::type_environment::TypeEnvironment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CallKind {
    Pure,
    Stateful,
}

/// Source signatures, independent of lowered argument/formal/body annotations.
#[derive(Debug)]
pub(super) struct CallSignature {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub result: Type,
    pub kind: CallKind,
}
impl CallSignature {
    fn matches_pure(&self, source: &PureCircuit) -> bool {
        self.kind == CallKind::Pure
            && self.name == source.name
            && self.parameters == source.parameters
            && self.result == source.result
    }
}
impl Declarations {
    pub(super) fn new(
        ledger: &HashMap<&str, &LedgerField>,
        witnesses: &HashMap<&str, &WitnessDeclaration>,
    ) -> Self {
        let mut ledger: Vec<_> = ledger.values().map(|f| (*f).clone()).collect();
        let mut witnesses: Vec<_> = witnesses.values().map(|w| (*w).clone()).collect();
        ledger.sort_by(|a, b| a.id.cmp(&b.id));
        witnesses.sort_by(|a, b| a.name.cmp(&b.name));
        Self {
            ledger,
            witnesses,
            calls: Vec::new(),
            root: None,
            pure: Certificates::default(),
            types: Default::default(),
        }
    }
    pub(super) fn with_type_environment(
        mut self,
        types: super::type_environment::TypeEnvironment,
    ) -> Self {
        self.types = types;
        self
    }
    pub(super) fn types(&self) -> &super::type_environment::TypeEnvironment {
        &self.types
    }
    pub(super) fn with_certificates(
        mut self,
        certificates: Certificates,
        pure: &HashMap<&str, &PureCircuit>,
    ) -> Result<Self, super::RecordingGap> {
        certificates.verify_bindings(pure)?;
        // The whole reachable graph must belong to this destination registry,
        // including helpers never invoked directly by a recording operation.
        for (name, certified) in certificates.iter() {
            let path = format!("pure[{name}]");
            let (_, signature) = self.resolve_call(name).ok_or_else(|| {
                super::invalid(&path, "certified pure declaration is missing or ambiguous")
            })?;
            if !signature.matches_pure(certified.source()) {
                return Err(super::invalid(
                    &path,
                    "certified pure signature differs from destination declaration",
                ));
            }
        }
        self.pure = certificates;
        Ok(self)
    }
    pub(super) fn certified_pure(&self, id: CallId) -> Option<&CertifiedPure> {
        let signature = self.call(id)?;
        let certified = self.pure.get(&signature.name)?;
        signature
            .matches_pure(certified.source())
            .then_some(certified)
    }
    pub(super) fn with_root(mut self, circuit: &StatefulCircuit) -> Self {
        self.root = Some(CallSignature {
            name: circuit.name.clone(),
            parameters: circuit.parameters.clone(),
            result: circuit.result.clone(),
            kind: CallKind::Stateful,
        });
        self
    }
    pub(super) fn root(&self) -> Option<&CallSignature> {
        self.root.as_ref()
    }
    pub(super) fn with_calls(
        ledger: &HashMap<&str, &LedgerField>,
        witnesses: &HashMap<&str, &WitnessDeclaration>,
        pure: &HashMap<&str, &PureCircuit>,
        stateful: &HashMap<&str, &StatefulCircuit>,
    ) -> Self {
        let mut declarations = Self::new(ledger, witnesses);
        declarations
            .calls
            .extend(pure.values().map(|c| CallSignature {
                name: c.name.clone(),
                parameters: c.parameters.clone(),
                result: c.result.clone(),
                kind: CallKind::Pure,
            }));
        declarations
            .calls
            .extend(stateful.values().map(|c| CallSignature {
                name: c.name.clone(),
                parameters: c.parameters.clone(),
                result: c.result.clone(),
                kind: CallKind::Stateful,
            }));
        declarations
            .calls
            .sort_by(|a, b| (&a.name, a.kind).cmp(&(&b.name, b.kind)));
        declarations
    }
    pub(super) fn call(&self, id: CallId) -> Option<&CallSignature> {
        let signature = self.calls.get(id.0)?;
        let (resolved, _) = self.resolve_call(&signature.name)?;
        (resolved == id).then_some(signature)
    }
    /// Ambiguous declarations cannot acquire a usable call identity.
    pub(super) fn resolve_call(&self, name: &str) -> Option<(CallId, &CallSignature)> {
        let index = self
            .calls
            .binary_search_by(|c| c.name.as_str().cmp(name))
            .ok()?;
        if index
            .checked_sub(1)
            .and_then(|i| self.calls.get(i))
            .is_some_and(|c| c.name == name)
            || self.calls.get(index + 1).is_some_and(|c| c.name == name)
        {
            return None;
        }
        Some((CallId(index), &self.calls[index]))
    }
    pub(super) fn field(&self, id: LedgerId) -> Option<&LedgerField> {
        self.ledger.get(id.0)
    }
    pub(super) fn witness(&self, id: WitnessId) -> Option<&WitnessDeclaration> {
        self.witnesses.get(id.0)
    }
    pub(super) fn resolve_field(&self, name: &str) -> Option<(LedgerId, &LedgerField)> {
        let id = LedgerId(
            self.ledger
                .binary_search_by(|f| f.id.as_str().cmp(name))
                .ok()?,
        );
        Some((id, &self.ledger[id.0]))
    }
    pub(super) fn resolve_witness(&self, name: &str) -> Option<(WitnessId, &WitnessDeclaration)> {
        let id = WitnessId(
            self.witnesses
                .binary_search_by(|w| w.name.as_str().cmp(name))
                .ok()?,
        );
        Some((id, &self.witnesses[id.0]))
    }
}
