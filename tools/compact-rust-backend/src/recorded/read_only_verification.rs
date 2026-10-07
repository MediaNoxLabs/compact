// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A checked public Map read followed by an audited local Unit helper.
//! The admitted public reads retain their declaration and lexical binding identity.

use super::*;
use crate::ir::Parameter;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Guard,
    Key,
    Member,
    Lookup,
    Call,
    Done,
}

#[derive(Debug, Default)]
struct ReadDraft<'a> {
    guard: Option<&'a LedgerField>,
    guard_message: Option<&'a str>,
    key: Option<&'a LocalBinding>,
    key_source: Option<usize>,
    map: Option<&'a LedgerField>,
    member_message: Option<&'a str>,
    value: Option<&'a LocalBinding>,
    callee: Option<&'a StatefulCircuit>,
    has_map_point_argument: bool,
    arguments: Vec<ArgumentSource<'a>>,
}

// Rendering receives only a completed admission. Missing fields and an absent
// point projection remain private to the ordered audit rather than panicking.
#[derive(Debug)]
struct ReadPlan<'a> {
    guard: &'a LedgerField,
    guard_message: &'a str,
    key_source: usize,
    map: &'a LedgerField,
    member_message: &'a str,
    value: &'a LocalBinding,
    callee: &'a StatefulCircuit,
    arguments: Vec<ArgumentSource<'a>>,
}

#[derive(Debug)]
enum ArgumentSource<'a> {
    Parameter(usize),
    MapPoint(&'a crate::ir::StructField),
}

struct Audit<'a> {
    phase: Phase,
    draft: ReadDraft<'a>,
    fields: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
    parameters: HashMap<&'a str, (usize, &'a Parameter)>,
}

impl<'a> Audit<'a> {
    fn slot(&self, name: &str, index: u8) -> Option<&'a LedgerField> {
        self.fields
            .get(name)
            .copied()
            .filter(|field| field.index == index)
    }

    fn bound<'s>(
        scope: &'s HashMap<&str, &'a LocalBinding>,
        value: &Expr,
    ) -> Option<&'a LocalBinding> {
        let Expr::Parameter { name } = value else {
            return None;
        };
        scope.get(name.as_str()).copied()
    }

    fn argument(
        &mut self,
        argument: &'a Expr,
        expected: &Type,
        scope: &HashMap<&str, &'a LocalBinding>,
    ) -> Option<ArgumentSource<'a>> {
        let Expr::Coerce { value, ty } = argument else {
            return None;
        };
        if ty != expected {
            return None;
        }
        match value.as_ref() {
            Expr::Parameter { name } => {
                let (index, parameter) = self.parameters.get(name.as_str())?;
                (&parameter.ty == expected && !scope.contains_key(name.as_str()))
                    .then_some(ArgumentSource::Parameter(*index))
            }
            Expr::StructField {
                value,
                field,
                index,
            } => {
                let bound = Self::bound(scope, value)?;
                if !self
                    .draft
                    .value
                    .is_some_and(|source| std::ptr::eq(source, bound))
                {
                    return None;
                }
                let Type::Struct { fields, .. } = &bound.ty else {
                    return None;
                };
                let declared = fields.get(*index)?;
                if declared.name != *field
                    || &declared.ty != expected
                    || *expected != Type::JubjubPoint
                {
                    return None;
                }
                self.draft.has_map_point_argument = true;
                Some(ArgumentSource::MapPoint(declared))
            }
            _ => None,
        }
    }

    fn action(
        &mut self,
        action: &'a StateAction,
        scope: &mut HashMap<&'a str, &'a LocalBinding>,
    ) -> bool {
        match action {
            StateAction::Sequence { actions } => {
                actions.iter().all(|action| self.action(action, scope))
            }
            StateAction::Assert {
                condition: Expr::CellRead { field, index },
                message,
            } if self.phase == Phase::Guard => {
                let Some(slot) = self.slot(field, *index) else {
                    return false;
                };
                if !matches!(
                    slot.declaration,
                    LedgerFieldKind::Cell { ty: Type::Boolean }
                ) {
                    return false;
                }
                self.draft.guard = Some(slot);
                self.draft.guard_message = Some(message);
                self.phase = Phase::Key;
                true
            }
            StateAction::Let { bindings, action } if self.phase == Phase::Key => {
                let [binding] = bindings.as_slice() else {
                    return false;
                };
                let Expr::Parameter { name } = &binding.value else {
                    return false;
                };
                if binding.ty != Type::OpaqueString
                    || !self
                        .parameters
                        .get(name.as_str())
                        .is_some_and(|(_, p)| p.ty == Type::OpaqueString)
                    || self.parameters.contains_key(binding.name.as_str())
                    || scope.contains_key(binding.name.as_str())
                {
                    return false;
                }
                self.draft.key = Some(binding);
                self.draft.key_source = self.parameters.get(name.as_str()).map(|(index, _)| *index);
                self.phase = Phase::Member;
                scope.insert(binding.name.as_str(), binding);
                let accepted = self.action(action, scope);
                scope.remove(binding.name.as_str());
                accepted
            }
            StateAction::Assert {
                condition: Expr::MapMember { field, index, key },
                message,
            } if self.phase == Phase::Member => {
                let Some(bound) = Self::bound(scope, key) else {
                    return false;
                };
                if !self
                    .draft
                    .key
                    .is_some_and(|source| std::ptr::eq(source, bound))
                {
                    return false;
                }
                let Some(slot) = self.slot(field, *index) else {
                    return false;
                };
                let LedgerFieldKind::Map {
                    key: Type::OpaqueString,
                    value: Type::Struct { fields, .. },
                } = &slot.declaration
                else {
                    return false;
                };
                if fields.is_empty()
                    || !fields
                        .iter()
                        .all(|f| matches!(f.ty, Type::OpaqueString | Type::JubjubPoint))
                    || !fields.iter().any(|f| f.ty == Type::JubjubPoint)
                {
                    return false;
                }
                self.draft.map = Some(slot);
                self.draft.member_message = Some(message);
                self.phase = Phase::Lookup;
                true
            }
            StateAction::Let { bindings, action } if self.phase == Phase::Lookup => {
                let [binding] = bindings.as_slice() else {
                    return false;
                };
                let Expr::MapLookup { field, index, key } = &binding.value else {
                    return false;
                };
                let Some(slot) = self.slot(field, *index) else {
                    return false;
                };
                if !self
                    .draft
                    .map
                    .is_some_and(|source| std::ptr::eq(source, slot))
                    || !matches!(&slot.declaration, LedgerFieldKind::Map { value, .. } if *value == binding.ty)
                    || !Self::bound(scope, key).is_some_and(|bound| {
                        self.draft
                            .key
                            .is_some_and(|source| std::ptr::eq(source, bound))
                    })
                    || self.parameters.contains_key(binding.name.as_str())
                    || scope.contains_key(binding.name.as_str())
                {
                    return false;
                }
                self.draft.value = Some(binding);
                self.phase = Phase::Call;
                scope.insert(binding.name.as_str(), binding);
                let accepted = self.action(action, scope);
                scope.remove(binding.name.as_str());
                accepted
            }
            StateAction::CircuitCall { name, arguments } if self.phase == Phase::Call => {
                let Some(callee) = self.circuits.get(name.as_str()).copied() else {
                    return false;
                };
                if arguments.len() != callee.parameters.len()
                    || !super::audited_local::unit_helper(callee, self.witnesses, self.circuits)
                {
                    return false;
                }
                for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                    let Some(source) = self.argument(argument, &parameter.ty, scope) else {
                        return false;
                    };
                    self.draft.arguments.push(source);
                }
                if !self.draft.has_map_point_argument {
                    return false;
                }
                self.draft.callee = Some(callee);
                // A second action is rejected by the default arm below.
                self.phase = Phase::Done;
                true
            }
            _ => false,
        }
    }
}

fn analyze<'a>(
    circuit: &'a StatefulCircuit,
    fields: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<ReadPlan<'a>> {
    if circuit.result != Type::Unit || circuit.return_value != StateReturn::Unit {
        return None;
    }
    let parameters: HashMap<_, _> = circuit
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| (p.name.as_str(), (i, p)))
        .collect();
    if parameters.len() != circuit.parameters.len() {
        return None;
    }
    let mut audit = Audit {
        phase: Phase::Guard,
        draft: ReadDraft::default(),
        fields,
        witnesses,
        circuits,
        parameters,
    };
    let mut scope = HashMap::new();
    if !circuit
        .actions
        .iter()
        .all(|action| audit.action(action, &mut scope))
    {
        return None;
    }
    if audit.phase != Phase::Done || !audit.draft.has_map_point_argument {
        return None;
    }
    let draft = audit.draft;
    draft.key?;
    Some(ReadPlan {
        guard: draft.guard?,
        guard_message: draft.guard_message?,
        key_source: draft.key_source?,
        map: draft.map?,
        member_message: draft.member_message?,
        value: draft.value?,
        callee: draft.callee?,
        arguments: draft.arguments,
    })
}

pub(super) fn render(
    circuit: &StatefulCircuit,
    fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<RecordingOutcome<syn::Item>>, RenderError> {
    let Some(plan) = analyze(circuit, fields, witnesses, circuits) else {
        return Ok(None);
    };
    let guard = ident(&plan.guard.id)?;
    let guard_message = plan.guard_message;
    let slot = ident(&plan.map.id)?;
    let member_message = plan.member_message;
    let value_ty = rust_type(&plan.value.ty)?;
    let callee = ident(&plan.callee.name)?;
    let name = ident(&circuit.name)?;
    let key_source = syn::Ident::new(
        &format!("__compact_param_{}", plan.key_source),
        Span::call_site(),
    );
    let mut parameters = Vec::<syn::FnArg>::new();
    for (index, parameter) in circuit.parameters.iter().enumerate() {
        ident(&parameter.name)?;
        let name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
        let ty = rust_type(&parameter.ty)?;
        parameters.push(syn::parse_quote!(#name: #ty));
    }
    let mut arguments = Vec::<syn::Expr>::new();
    for source in plan.arguments {
        let argument = match source {
            ArgumentSource::Parameter(index) => {
                let name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
                retained_value(syn::parse_quote!(#name), &circuit.parameters[index].ty)
            }
            ArgumentSource::MapPoint(field) => {
                let name = ident(&field.name)?;
                retained_value(
                    syn::parse_quote!(__compact_recorded_map_value.#name),
                    &field.ty,
                )
            }
        };
        arguments.push(argument);
    }
    let item = syn::parse_quote! {
        pub fn #name<Private, W: super::TryWitnesses<Private>>(
            context: runtime::context::CircuitContext<Private>,
            witnesses: &W,
            #(#parameters),*
        ) -> Result<runtime::recording::RecordedCircuitResult<Private, ()>, runtime::CompactError> {
            let frame = runtime::recording::RecordingFrame::new(context);
            let (frame, open): (_, bool) = crate::ledger_slots::#guard.record_read(frame)?;
            if !open {
                return Err(runtime::CompactError::AssertionFailed(#guard_message.to_owned()));
            }
            let __compact_recorded_map_key: runtime::OpaqueString = (#key_source).clone();
            let (frame, present): (_, bool) = crate::ledger_slots::#slot
                .record_member(frame, __compact_recorded_map_key.clone())?;
            if !present {
                return Err(runtime::CompactError::AssertionFailed(#member_message.to_owned()));
            }
            let (frame, __compact_recorded_map_value): (_, #value_ty) = crate::ledger_slots::#slot
                .record_lookup(frame, __compact_recorded_map_key)?;
            let (frame, ()): (_, ()) = frame.call_local(|context| {
                super::#callee(context, witnesses, #(#arguments),*)
            })?;
            Ok(frame.finish(()))
        }
    };
    Ok(Some(RecordingOutcome::Supported(item)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> crate::ir::Contract {
        serde_json::from_str(include_str!("../../tests/did-digest-schema20-ir.json")).unwrap()
    }

    fn accepts(contract: &crate::ir::Contract) -> bool {
        let fields = contract
            .ledger_fields
            .iter()
            .map(|f| (f.id.as_str(), f))
            .collect();
        let witnesses = contract
            .witnesses
            .iter()
            .map(|w| (w.name.as_str(), w))
            .collect();
        let circuits = contract
            .stateful_circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        let export = contract
            .stateful_circuits
            .iter()
            .find(|c| c.name == "verifySchnorrJubjubDigestSignature")
            .unwrap();
        analyze(export, &fields, &witnesses, &circuits).is_some()
    }

    #[test]
    fn original_did_read_and_imported_helper_are_admitted() {
        assert!(accepts(&contract()));
    }

    #[test]
    fn changed_map_lookup_slot_refuses_even_with_same_key_and_type() {
        let mut contract = contract();
        let export = contract
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "verifySchnorrJubjubDigestSignature")
            .unwrap();
        let StateAction::Sequence { actions } = &mut export.actions[0] else {
            panic!()
        };
        let StateAction::Let { action, .. } = &mut actions[1] else {
            panic!()
        };
        let StateAction::Sequence { actions } = action.as_mut() else {
            panic!()
        };
        let StateAction::Let { bindings, .. } = &mut actions[1] else {
            panic!()
        };
        let Expr::MapLookup { field, .. } = &mut bindings[0].value else {
            panic!()
        };
        *field = "verificationMethods".into();
        assert!(!accepts(&contract));
    }

    #[test]
    fn hidden_extra_public_read_refuses() {
        let mut contract = contract();
        let export = contract
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "verifySchnorrJubjubDigestSignature")
            .unwrap();
        let StateAction::Sequence { actions } = &mut export.actions[0] else {
            panic!()
        };
        let extra = actions[0].clone();
        actions.push(extra);
        assert!(!accepts(&contract));
    }

    #[test]
    fn changed_member_key_and_unselected_effect_refuse() {
        let mut changed_key = contract();
        let export = changed_key
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "verifySchnorrJubjubDigestSignature")
            .unwrap();
        export.parameters.push(Parameter {
            name: "otherMethodId".into(),
            ty: Type::OpaqueString,
        });
        let StateAction::Sequence { actions } = &mut export.actions[0] else {
            panic!()
        };
        let StateAction::Let { action, .. } = &mut actions[1] else {
            panic!()
        };
        let StateAction::Sequence { actions } = action.as_mut() else {
            panic!()
        };
        let StateAction::Assert {
            condition: Expr::MapMember { key, .. },
            ..
        } = &mut actions[0]
        else {
            panic!()
        };
        **key = Expr::Parameter {
            name: "otherMethodId".into(),
        };
        assert!(!accepts(&changed_key));

        let mut hidden = contract();
        let export = hidden
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "verifySchnorrJubjubDigestSignature")
            .unwrap();
        let StateAction::Sequence { actions } = &mut export.actions[0] else {
            panic!()
        };
        actions.insert(
            1,
            StateAction::If {
                condition: Expr::Boolean { value: false },
                then: Box::new(StateAction::Assert {
                    condition: Expr::CellRead {
                        field: "active".into(),
                        index: 1,
                    },
                    message: "hidden public read".into(),
                }),
                otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
            },
        );
        assert!(!accepts(&hidden));
    }

    #[test]
    fn same_typed_second_map_declaration_cannot_replace_member_slot() {
        let mut contract = contract();
        let mut other = contract
            .ledger_fields
            .iter()
            .find(|f| f.id == "schnorrJubjubVerificationMethods")
            .unwrap()
            .clone();
        other.id = "sameShapeOtherMethods".into();
        other.path = vec![1, 9];
        contract.ledger_fields.push(other);
        let export = contract
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "verifySchnorrJubjubDigestSignature")
            .unwrap();
        let StateAction::Sequence { actions } = &mut export.actions[0] else {
            panic!()
        };
        let StateAction::Let { action, .. } = &mut actions[1] else {
            panic!()
        };
        let StateAction::Sequence { actions } = action.as_mut() else {
            panic!()
        };
        let StateAction::Let { bindings, .. } = &mut actions[1] else {
            panic!()
        };
        let Expr::MapLookup { field, .. } = &mut bindings[0].value else {
            panic!()
        };
        *field = "sameShapeOtherMethods".into();
        assert!(!accepts(&contract));
    }

    #[test]
    fn root_parameter_shadowing_and_rebound_projection_refuse() {
        let mut shadowed = contract();
        let export = shadowed
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "verifySchnorrJubjubDigestSignature")
            .unwrap();
        let StateAction::Sequence { actions } = &mut export.actions[0] else {
            panic!()
        };
        let StateAction::Let { bindings, .. } = &mut actions[1] else {
            panic!()
        };
        bindings[0].name = "methodId".into();
        assert!(!accepts(&shadowed));

        let mut rebound = contract();
        let export = rebound
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "verifySchnorrJubjubDigestSignature")
            .unwrap();
        let StateAction::Sequence { actions } = &mut export.actions[0] else {
            panic!()
        };
        let StateAction::Let { action, .. } = &mut actions[1] else {
            panic!()
        };
        let StateAction::Sequence { actions } = action.as_mut() else {
            panic!()
        };
        let StateAction::Let { action, .. } = &mut actions[1] else {
            panic!()
        };
        let StateAction::CircuitCall { arguments, .. } = action.as_mut() else {
            panic!()
        };
        let Expr::Coerce { value, .. } = &mut arguments[2] else {
            panic!()
        };
        let Expr::StructField { value, .. } = value.as_mut() else {
            panic!()
        };
        **value = Expr::Parameter {
            name: "methodId".into(),
        };
        assert!(!accepts(&rebound));
    }

    #[test]
    fn recursive_or_effectful_verifier_and_reordered_arguments_refuse() {
        let mut recursive = contract();
        let helper = recursive
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "schnorrVerifyDigest")
            .unwrap();
        let StateAction::CircuitCall { name, .. } = &mut helper.actions[0] else {
            panic!()
        };
        *name = "schnorrVerifyDigest".into();
        assert!(!accepts(&recursive));

        let mut effectful = contract();
        let helper = effectful
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "schnorrVerifyDigest")
            .unwrap();
        helper.actions.push(StateAction::Assert {
            condition: Expr::CellRead {
                field: "active".into(),
                index: 1,
            },
            message: "hidden query".into(),
        });
        assert!(!accepts(&effectful));

        let mut reordered = contract();
        let export = reordered
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "verifySchnorrJubjubDigestSignature")
            .unwrap();
        let StateAction::Sequence { actions } = &mut export.actions[0] else {
            panic!()
        };
        let StateAction::Let { action, .. } = &mut actions[1] else {
            panic!()
        };
        let StateAction::Sequence { actions } = action.as_mut() else {
            panic!()
        };
        let StateAction::Let { action, .. } = &mut actions[1] else {
            panic!()
        };
        let StateAction::CircuitCall { arguments, .. } = action.as_mut() else {
            panic!()
        };
        arguments.swap(0, 1);
        assert!(!accepts(&reordered));
    }
}
