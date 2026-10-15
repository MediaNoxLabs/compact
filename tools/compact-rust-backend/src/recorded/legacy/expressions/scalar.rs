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

//! Scalar expression dispatch over the existing Field, Boolean, and source domains.
//!
//! This module preserves the legacy lowering semantics. Explicit arguments own
//! scopes, temporary identifiers, declarations, and the current recording frame.

use crate::RenderError;
use crate::ir::{Expr, LedgerField, StatefulCircuit, Type, WitnessDeclaration};
use crate::recorded::legacy::expressions::{boolean_expression, field_expression};
use crate::recorded::legacy::value_sources::cell_source;
use std::collections::{HashMap, HashSet};

#[expect(
    clippy::too_many_arguments,
    reason = "recursive expression lowering threads typed lookups and ordered recording state explicitly"
)]
pub(in crate::recorded) fn scalar_expression(
    value: &Expr,
    ty: &Type,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
) -> Result<Option<syn::Expr>, RenderError> {
    match ty {
        Type::Field => field_expression(
            value,
            locals,
            parameters,
            ledger_fields,
            witnesses,
            circuits,
            &HashSet::new(),
            steps,
            next_temp,
            visiting,
        ),
        Type::Boolean => boolean_expression(
            value,
            locals,
            parameters,
            ledger_fields,
            witnesses,
            circuits,
            steps,
            next_temp,
            visiting,
        ),
        Type::Bytes { .. }
        | Type::Unsigned { .. }
        | Type::OpaqueString
        | Type::Enum { .. }
        | Type::Struct { .. }
        | Type::Tuple { .. }
        | Type::Vector { .. } => Ok(cell_source(value, ty, locals, parameters)),
        _ => Ok(None),
    }
}
