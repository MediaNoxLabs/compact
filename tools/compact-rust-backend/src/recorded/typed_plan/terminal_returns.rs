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
//! Attach extracted returns to their terminal lexical continuation. This is a
//! structural adapter retained for other legacy evaluators; admission is separate.
use super::*;

/// Only the final Sequence/Let suffix encloses an extracted return. Earlier
/// actions and If branches remain ordinary action scopes. Already explicit
/// Effectful ReturnPlans never pass through this adapter.
pub(super) fn adapt(actions: &[StateAction], value: &Expr) -> ReturnPlan {
    let Some((last, prefix)) = actions.split_last() else {
        return ReturnPlan::Value {
            value: value.clone(),
        };
    };
    let continuation = attach(last, value);
    if prefix.is_empty() {
        continuation
    } else {
        ReturnPlan::Sequence {
            actions: prefix.to_vec(),
            result: Box::new(continuation),
        }
    }
}
fn attach(action: &StateAction, value: &Expr) -> ReturnPlan {
    match action {
        StateAction::Let { bindings, action } => ReturnPlan::Let {
            bindings: bindings.clone(),
            result: Box::new(attach(action, value)),
        },
        StateAction::Sequence { actions } => adapt(actions, value),
        _ => ReturnPlan::Sequence {
            actions: vec![action.clone()],
            result: Box::new(ReturnPlan::Value {
                value: value.clone(),
            }),
        },
    }
}
