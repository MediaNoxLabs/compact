// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Root-only export of exact private checked Fold expressions for instrumented compilation.
//! Signature-only scaffolding is distinct from public frontend/generated contract qualification.
use crate::ir::{Expr, Parameter, PureCircuit, Type};
use quote::ToTokens;
use std::collections::HashMap;
#[test]
#[ignore = "parent executes this only to export exact checked expressions"]
fn export_checked_fold_runtime_order_probe() {
    let Some(output) = std::env::var_os("COMPACT_SHARED_AGGREGATE_PROBE_OUTPUT") else {
        return;
    };
    let mut rows = Vec::new();
    for length in [0, 3] {
        let vector = Type::Vector {
            element: Box::new(Type::Field),
            length,
        };
        let decl = |name: &str, parameters: Vec<Parameter>, result: Type| PureCircuit {
            source: None,
            internal: true,
            name: name.into(),
            parameters,
            body: Expr::Default { ty: result.clone() },
            result,
        };
        let circuits = [
            decl("source", vec![], vector.clone()),
            decl("initial", vec![], Type::Field),
            decl(
                "step",
                vec![
                    Parameter {
                        name: "acc".into(),
                        ty: Type::Field,
                    },
                    Parameter {
                        name: "item".into(),
                        ty: Type::Field,
                    },
                ],
                Type::Field,
            ),
        ];
        let declarations: HashMap<_, _> = circuits.iter().map(|c| (c.name.as_str(), c)).collect();
        let body = Expr::VectorFoldCall {
            name: "step".into(),
            initial: Box::new(Expr::Call {
                name: "initial".into(),
                arguments: vec![],
            }),
            source: Box::new(Expr::Call {
                name: "source".into(),
                arguments: vec![],
            }),
            accumulator: Type::Field,
            element: Type::Field,
            length,
        };
        let (expression, result) =
            crate::expression_with_calls(&body, &HashMap::new(), &declarations).unwrap();
        assert_eq!(result, Type::Field);
        assert_eq!(
            super::super::observe(&body, &HashMap::new(), &declarations),
            super::super::old_oracle::observe(&body, &HashMap::new(), &declarations).map(
                |(expression, statements, ty)| (
                    super::fold::successor(&expression),
                    statements,
                    ty
                )
            )
        );
        rows.push(serde_json::json!({"length":length,"expression":expression.to_token_stream().to_string(),"result_type":result,"source_type":vector,"callee_parameter_types":[Type::Field,Type::Field],"qualification":"Exact private checked expression; supplied signature-only callback scaffolding; not complete public frontend circuit or proof"}));
    }
    std::fs::write(output, serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
}
