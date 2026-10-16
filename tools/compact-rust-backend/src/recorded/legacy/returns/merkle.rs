// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Merkle observations and witnessed path-root return lowering.
//!
//! The parent match owns return-type and action-shape admission. Root-check
//! declaration mismatches remain recording refusals, while fullness mismatches
//! retain their ledger-field errors. Witnessed paths retain expression diagnostics.

use crate::ir::{
    Expr, LedgerField, LedgerFieldKind, LocalBinding, PureCircuit, StateReturn, Type,
    WitnessDeclaration,
};
use crate::recorded::legacy::value_sources::cell_source;
use crate::recorded::{RecordingGap, RecordingOutcome};
use crate::{RenderError, ident, rust_type};
use std::collections::HashMap;

pub(in crate::recorded) fn check_root(
    field: &str,
    index: u8,
    root: &Expr,
    return_value: &StateReturn,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
) -> Result<RecordingOutcome<(Vec<syn::Stmt>, syn::Expr)>, RenderError> {
    let declaration = ledger_fields
        .get(field)
        .ok_or_else(|| RenderError::UnknownLedgerField(field.to_owned()))?;
    let Expr::Parameter { name } = root else {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    };
    let Some((root_ty, _)) = parameters.get(name.as_str()) else {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    };
    let historic = matches!(return_value, StateReturn::HistoricMerkleCheckRoot { .. });
    let declaration_historic = match declaration.declaration {
        LedgerFieldKind::MerkleTree { .. } => false,
        LedgerFieldKind::HistoricMerkleTree { .. } => true,
        _ => {
            return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                return_value,
            )));
        }
    };
    if declaration.index != index
        || (historic && declaration.physical_path().len() != 1)
        || historic != declaration_historic
        || !matches!(root_ty, Type::Struct { name, fields }
            if name == "MerkleTreeDigest" && fields.len() == 1
                && fields[0].name == "field" && fields[0].ty == Type::Field)
    {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    }
    let Some(root) = cell_source(root, root_ty, &HashMap::new(), parameters) else {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
            return_value,
        )));
    };
    let slot = ident(field)?;
    Ok(RecordingOutcome::Supported((
        vec![syn::parse_quote!(
            let (frame, observed): (_, bool) =
                crate::ledger_slots::#slot.record_check_root(frame, #root)?;
        )],
        syn::parse_quote!(observed),
    )))
}

pub(in crate::recorded) fn is_full(
    field: &str,
    index: u8,
    return_value: &StateReturn,
    ledger_fields: &HashMap<&str, &LedgerField>,
) -> Result<RecordingOutcome<(Vec<syn::Stmt>, syn::Expr)>, RenderError> {
    let declaration = ledger_fields
        .get(field)
        .ok_or_else(|| RenderError::UnknownLedgerField(field.to_owned()))?;
    let historic = matches!(return_value, StateReturn::HistoricMerkleIsFull { .. });
    let declaration_historic = match declaration.declaration {
        LedgerFieldKind::MerkleTree { .. } => false,
        LedgerFieldKind::HistoricMerkleTree { .. } => true,
        _ => return Err(RenderError::UnknownLedgerField(field.to_owned())),
    };
    if declaration.index != index || historic != declaration_historic {
        return Err(RenderError::UnknownLedgerField(field.to_owned()));
    }
    let slot = ident(field)?;
    Ok(RecordingOutcome::Supported((
        vec![syn::parse_quote!(
            let (frame, observed): (_, bool) =
                crate::ledger_slots::#slot.record_is_full(frame)?;
        )],
        syn::parse_quote!(observed),
    )))
}

pub(in crate::recorded) fn witnessed_path_root(
    bindings: &[LocalBinding],
    body: &Expr,
    ledger_fields: &HashMap<&str, &LedgerField>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
) -> Result<RecordingOutcome<(Vec<syn::Stmt>, syn::Expr)>, RenderError> {
    let binding = &bindings[0];
    let Expr::MerkleCheckRoot { field, index, root } = body else {
        unreachable!("guarded by MerkleCheckRoot expression")
    };
    let declaration = ledger_fields
        .get(field.as_str())
        .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
    if !matches!(declaration.declaration, LedgerFieldKind::MerkleTree { .. })
        || declaration.index != *index
        || !matches!(root.as_ref(), Expr::Parameter { name } if *name == binding.name)
    {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
            body,
            "return_value.value".to_owned(),
        )));
    }
    let Expr::Call { name, arguments } = &binding.value else {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
            &binding.value,
            "return_value.value.bindings[0].value".to_owned(),
        )));
    };
    let pure = pure_circuits
        .get(name.as_str())
        .ok_or_else(|| RenderError::UnknownCircuit(name.clone()))?;
    if name != "merkleTreePathRoot"
        || pure.result != binding.ty
        || !matches!(&binding.ty, Type::Struct { name, .. } if name == "MerkleTreeDigest")
        || pure.parameters.len() != 1
        || arguments.len() != 1
    {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
            &binding.value,
            "return_value.value.bindings[0].value".to_owned(),
        )));
    }
    let path_ty = &pure.parameters[0].ty;
    let path = match &arguments[0] {
        Expr::Coerce { value, ty } if ty == path_ty => value.as_ref(),
        other => other,
    };
    let Expr::WitnessCall {
        name: witness_name,
        arguments: witness_args,
    } = path
    else {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
            path,
            "return_value.value.bindings[0].value.arguments[0]".to_owned(),
        )));
    };
    let witness = witnesses
        .get(witness_name.as_str())
        .ok_or_else(|| RenderError::UnknownWitness(witness_name.clone()))?;
    if witness.result != *path_ty || !witness.parameters.is_empty() || !witness_args.is_empty() {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
            path,
            "return_value.value.bindings[0].value.arguments[0]".to_owned(),
        )));
    }
    let slot = ident(field)?;
    let witness_method = ident(witness_name)?;
    let pure_method = ident(name)?;
    let root_ty = rust_type(&binding.ty)?;
    Ok(RecordingOutcome::Supported((
        vec![
            syn::parse_quote! {
                let (frame, path) = frame.try_witness_metered(|context, meter| {
                    witnesses.#witness_method(
                        context.witness_context_with(super::LedgerView {
                            state: context.query.state.get_ref(),
                            meter,
                        }),
                    )
                })?;
            },
            syn::parse_quote! {
                let root: #root_ty = crate::pure_circuits::#pure_method(path)?;
            },
            syn::parse_quote! {
                let (frame, observed): (_, bool) =
                    crate::ledger_slots::#slot.record_check_root(frame, root)?;
            },
        ],
        syn::parse_quote!(observed),
    )))
}
