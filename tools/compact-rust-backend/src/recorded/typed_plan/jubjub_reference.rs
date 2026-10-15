// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Frozen pre-algebra Jubjub admission oracle, compiled only by tests.
use super::*;

fn value_type(ty: &Type) -> bool {
    matches!(ty, Type::Field | Type::JubjubPoint)
}
fn value(v: &Expr) -> bool {
    match v {
        Expr::Parameter { .. } | Expr::FieldLiteral { .. } => true,
        Expr::JubjubScalarFromNative { value: v } | Expr::EcMulGenerator { scalar: v } => value(v),
        Expr::EcMul { point, scalar } => value(point) && value(scalar),
        Expr::EcAdd { left, right } => value(left) && value(right),
        Expr::Let { bindings: b, body } => bindings(b) && value(body),
        _ => false,
    }
}
fn bindings(b: &[LocalBinding]) -> bool {
    b.iter().all(|b| value_type(&b.ty) && value(&b.value))
}
struct Audit<'a> {
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    writes: usize,
}
impl Audit<'_> {
    fn action(&mut self, a: &StateAction) -> bool {
        match a {
            StateAction::Sequence { actions } => actions.iter().all(|a| self.action(a)),
            StateAction::Let {
                bindings: b,
                action,
            } => bindings(b) && self.action(action),
            StateAction::CellWrite {
                field,
                index,
                value: v,
            } => {
                self.writes += 1;
                self.writes == 1
                    && self.ledger.get(field.as_str()).is_some_and(|f| {
                        f.index == *index
                            && f.physical_path() == [*index]
                            && f.declaration
                                == (LedgerFieldKind::Cell {
                                    ty: Type::JubjubPoint,
                                })
                    })
                    && value(v)
            }
            _ => false,
        }
    }
}
pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    let StateReturn::Expression { value: returned } = &circuit.return_value else {
        return None;
    };
    let mut audit = Audit { ledger, writes: 0 };
    if circuit.result != Type::JubjubPoint
        || !circuit.parameters.iter().all(|p| value_type(&p.ty))
        || !circuit.actions.iter().all(|a| audit.action(a))
        || audit.writes != 1
        || !value(returned)
    {
        return None;
    }
    let mut plan = Plan {
        declarations: Declarations {
            ledger,
            witnesses,
            pure,
            stateful_circuits: Some(circuits),
        },
        evidence: EffectEvidence::default(),
        admission: AdmissionPolicy {
            field_cell_slot: None,
            effectful_field_cells: false,
            read_only_assertions: false,
            unit_actions: false,
            phase_reset: false,
            composite_domain: CompositeDomain::JubjubValues,
            composition_calls: None,
            composition_lookup_sites: Default::default(),
            counter_hash_helpers: false,
        },
        evaluation: EvaluationState {
            next: 0,
            context_query: false,
            scalar_arguments: false,
            scalar_body_depth: 0,
            active_calls: HashSet::new(),
        },
    };

    let scope: Scope = circuit
        .parameters
        .iter()
        .enumerate()
        .map(|(index, p)| {
            let name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
            (
                p.name.clone(),
                TypedValue {
                    ty: p.ty.clone(),
                    value: syn::parse_quote!(#name),
                },
            )
        })
        .collect();
    if scope.len() != circuit.parameters.len() {
        return None;
    }
    let mut steps = Vec::new();
    let body = terminal_returns::adapt(&circuit.actions, returned);
    let result = plan.return_plan(&body, &scope, &mut steps)?;
    (result.ty == Type::JubjubPoint && plan.evidence.cell_writes == 1).then_some(TypedPlan {
        steps,
        result: result.value,
    })
}
