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
//! Shared legacy value/signature predicates still used by shielded profiles.
//! Unit-action shape admission itself is preserved only in the test oracle.
use super::*;

pub(super) fn value_type(ty: &Type) -> bool {
    match ty {
        Type::Field | Type::Boolean | Type::Bytes { length: 32 } | Type::Enum { .. } => true,
        Type::Struct { fields, .. } => {
            !fields.is_empty() && fields.iter().all(|f| value_type(&f.ty))
        }
        _ => false,
    }
}
pub(super) fn field_structure(ty: &Type) -> bool {
    match ty {
        Type::Field => true,
        Type::Struct { fields, .. } => {
            !fields.is_empty() && fields.iter().all(|f| field_structure(&f.ty))
        }
        _ => false,
    }
}
pub(super) fn helper_signature(c: &StatefulCircuit) -> bool {
    c.result == Type::Unit
        && matches!(c.return_value, StateReturn::Unit)
        && c.parameters.len() == 1
        && c.parameters[0].ty == Type::Field
        && !c.actions.is_empty()
}
