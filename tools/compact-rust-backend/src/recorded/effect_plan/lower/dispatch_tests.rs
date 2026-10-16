// This file is part of Compact.
// Copyright (C) 2026 contributors to Minokawa Compact
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// You may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A misrouted private node must refuse without emitting an effect or changing SSA.
use super::*;

fn lowerer<'a>(
    declarations: &'a Declarations,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Lowerer<'a> {
    Lowerer {
        observer: None,
        declarations,
        circuits,
        pure: Certifier::new(pure, circuits),
        call_stack: vec!["root".into()],
        expanded_steps: 7,
        max_call_depth: 32,
        max_expanded_steps: 128,
        next: 11,
        root_action: None,
        failure_precision: Precision::Unclassified,
        concrete_node: None,
    }
}

fn existing_binding() -> Binding {
    Binding {
        value: Value {
            id: 10,
            ty: Type::Unit,
        },
        operation: Operation::Literal(Literal::Unit),
        path: "earlier".into(),
    }
}

fn unchanged(lower: &Lowerer<'_>, bindings: &[Binding], before: &str) {
    assert_eq!(format!("{bindings:?}"), before);
    assert_eq!(lower.next, 11);
    assert_eq!(lower.expanded_steps, 7);
    assert_eq!(lower.call_stack, ["root"]);
}

#[test]
fn unrelated_expression_families_refuse_at_the_supplied_path_without_emitting() {
    let declarations = Declarations::default();
    let pure = HashMap::new();
    let circuits = HashMap::new();
    let mut lower = lowerer(&declarations, &pure, &circuits);
    let scope = Scope::new();
    let mut bindings = vec![existing_binding()];
    let before = format!("{bindings:?}");
    let path = "return_value.body.then.value";
    // Unit is legitimate IR, but it is not a query, intent or aggregate.
    let source = Expr::Unit;
    let results = [
        lower.map_expression(&source, &scope, &mut bindings, path),
        lower.list_expression(&source, path),
        lower.kernel(&source, &scope, &mut bindings, path),
        lower.zswap(&source, &scope, &mut bindings, path),
        lower.aggregate(&source, &scope, &mut bindings, path),
    ];
    for result in results {
        let gap = result.unwrap_err();
        assert_eq!(gap.path, path);
        assert_eq!(gap.ir_node, "RecordedPlan");
        assert_eq!(
            gap.code,
            super::super::RecordingGapCode::UnsupportedExpression
        );
        assert!(gap.resource.is_none());
        assert!(gap.detail.contains("dispatch requires"));
    }
    unchanged(&lower, &bindings, &before);
}

#[test]
fn unrelated_actions_refuse_before_ledger_lookup_or_any_mutation() {
    let declarations = Declarations::default();
    let pure = HashMap::new();
    let circuits = HashMap::new();
    let mut lower = lowerer(&declarations, &pure, &circuits);
    let scope = Scope::new();
    let mut bindings = vec![existing_binding()];
    let before = format!("{bindings:?}");
    let path = "actions[2].then";
    let source = StateAction::Sequence { actions: vec![] };
    let results = [
        lower.map_action(&source, &scope, &mut bindings, path),
        lower.list_action(&source, &scope, &mut bindings, path),
        lower.merkle_action(&source, &scope, &mut bindings, path),
    ];
    for result in results {
        let gap = result.unwrap_err();
        assert_eq!(gap.path, path);
        assert_eq!(gap.ir_node, "RecordedPlan");
        assert_eq!(
            gap.code,
            super::super::RecordingGapCode::UnsupportedExpression
        );
        assert!(gap.resource.is_none());
        assert!(gap.detail.contains("dispatch requires"));
    }
    unchanged(&lower, &bindings, &before);
}
