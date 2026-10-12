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

//! Stable recording availability diagnostics and admission outcomes.
//!
//! Serialize the same public gap representation used by capability reports.
//! Constructors preserve existing codes, IR paths and details; the renderer
//! retains responsibility for selecting and refining the reported cause.

use crate::ir::{Expr, StateAction, StateReturn, Type};
use serde::Serialize;

mod resource;
pub use resource::{RecordingResource, RecordingResourceKind, RecordingResourcePhase};

/// The first definite reason an exported circuit has no recorded Rust API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecordingGap {
    pub code: RecordingGapCode,
    pub ir_node: String,
    pub path: String,
    pub detail: String,
    /// Additive capability-schema-3 metadata; absent for ordinary refusals.
    /// Rust callers constructing this public struct must set this field explicitly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<Box<RecordingResource>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingGapCode {
    UnsupportedAction,
    UnsupportedExpression,
    UnsupportedType,
    UnsupportedReturn,
    NoRecordedEffect,
    RecordingUnavailable,
    NameCollision,
}

impl RecordingGapCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedAction => "unsupported_action",
            Self::UnsupportedExpression => "unsupported_expression",
            Self::UnsupportedType => "unsupported_type",
            Self::UnsupportedReturn => "unsupported_return",
            Self::NoRecordedEffect => "no_recorded_effect",
            Self::RecordingUnavailable => "recording_unavailable",
            Self::NameCollision => "name_collision",
        }
    }
}

impl RecordingGap {
    pub(super) fn action(action: &StateAction, path: String) -> Self {
        let ir_node = node_name(action, "StateAction");
        Self {
            resource: None,
            code: RecordingGapCode::UnsupportedAction,
            ir_node: ir_node.clone(),
            path,
            detail: format!("{ir_node} is not supported by recorded Rust lowering"),
        }
    }

    pub(super) fn returned(value: &StateReturn) -> Self {
        let ir_node = node_name(value, "StateReturn");
        Self {
            resource: None,
            code: RecordingGapCode::UnsupportedReturn,
            ir_node: ir_node.clone(),
            path: "return_value".to_owned(),
            detail: format!("{ir_node} is not supported by recorded Rust lowering"),
        }
    }

    pub(super) fn expression(value: &Expr, path: String) -> Self {
        let ir_node = node_name(value, "Expr");
        Self {
            resource: None,
            code: RecordingGapCode::UnsupportedExpression,
            ir_node: ir_node.clone(),
            path,
            detail: format!("{ir_node} is not supported in this recorded expression"),
        }
    }

    pub(super) fn unsupported_type(action: &StateAction, ty: &Type, path: String) -> Self {
        Self {
            resource: None,
            code: RecordingGapCode::UnsupportedType,
            ir_node: node_name(action, "StateAction"),
            path,
            detail: format!("{ty:?} Cell recording is unsupported"),
        }
    }

    pub(super) fn no_effect() -> Self {
        Self {
            resource: None,
            code: RecordingGapCode::NoRecordedEffect,
            ir_node: "StatefulCircuit".to_owned(),
            path: "actions".to_owned(),
            detail: "the circuit has no replayable ledger read or write".to_owned(),
        }
    }

    pub(crate) fn recording_dependency(gap: Self) -> Self {
        let mut detail = "the observed-call API requires a recorded circuit".to_owned();
        if gap.resource.is_some() {
            detail.push_str(": ");
            detail.push_str(&gap.detail);
        }
        Self {
            resource: gap.resource,
            code: RecordingGapCode::RecordingUnavailable,
            ir_node: gap.ir_node,
            path: gap.path,
            detail,
        }
    }

    pub(crate) fn name_collision(name: &str) -> Self {
        Self {
            resource: None,
            code: RecordingGapCode::NameCollision,
            ir_node: "StatefulCircuit".to_owned(),
            path: "name".to_owned(),
            detail: format!("observed-call method {name:?} collides with an exported circuit"),
        }
    }
}

fn node_name(value: &impl Serialize, prefix: &str) -> String {
    // The tagged IR enum is the source of the stable machine discriminator.
    // This is only invoked on a failed lowering branch, never on supported output.
    let node = serde_json::to_value(value).expect("typed IR serializes");
    let kind = node["kind"].as_str().expect("tagged IR variant has kind");
    let variant = kind
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<String>();
    format!("{prefix}::{variant}")
}

pub(crate) enum RecordingOutcome<T> {
    Supported(T),
    Unsupported(RecordingGap),
}

impl<T> RecordingOutcome<T> {
    pub(crate) fn is_supported(&self) -> bool {
        matches!(self, Self::Supported(_))
    }

    pub(crate) fn gap(&self) -> Option<&RecordingGap> {
        match self {
            Self::Supported(_) => None,
            Self::Unsupported(gap) => Some(gap),
        }
    }
}
