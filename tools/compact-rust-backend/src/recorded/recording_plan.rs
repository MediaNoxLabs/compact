// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Lowered Rust syntax shared by checked recording and frozen test oracles.

use crate::coin_shapes::shielded_coin_type;
use crate::ir::Type;

pub(super) struct TypedPlan {
    pub steps: Vec<syn::Stmt>,
    pub result: syn::Expr,
}

// Public result-shape classification supplies diagnostics, never admission.
pub(super) fn shielded_send_result(ty: &Type) -> bool {
    let Type::Struct { fields, .. } = ty else {
        return false;
    };
    let [change, sent] = fields.as_slice() else {
        return false;
    };
    if change.name != "change" || sent.name != "sent" || sent.ty != shielded_coin_type() {
        return false;
    }
    matches!(&change.ty, Type::Struct { fields, .. }
        if matches!(fields.as_slice(), [present, value]
            if present.name == "is_some" && present.ty == Type::Boolean
                && value.name == "value" && value.ty == sent.ty))
}
