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

//! The shared identity-contract fixture used by the render tests.

use compact_rust_backend::ir::{Contract, Expr, Parameter, PureCircuit, SCHEMA_VERSION, Type};

pub(super) fn identity(result: Type, body: Expr) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "identity".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Field,
            }],
            result,
            body,
        }],
        stateful_circuits: vec![],
    }
}
