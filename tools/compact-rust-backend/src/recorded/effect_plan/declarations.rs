// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Immutable contract declarations, retained independently of lowered operations.
//! IDs are deterministic within this environment and are never serialized.
use super::model::{LedgerId, WitnessId};
use crate::ir::{LedgerField, WitnessDeclaration};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub(super) struct Declarations {
    ledger: Vec<LedgerField>,
    witnesses: Vec<WitnessDeclaration>,
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
        Self { ledger, witnesses }
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
