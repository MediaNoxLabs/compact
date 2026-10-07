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

//! Checked selected Set roles. A reader and paired writers must share one
//! declared slot family and enum type; the composition Audit still validates
//! every helper body, branch, argument, and effect before admission.
use super::*;

pub(super) struct SelectedSetFamily<'a> {
    pub(super) read: &'a StatefulCircuit,
    pub(super) insert: &'a StatefulCircuit,
    pub(super) remove: &'a StatefulCircuit,
    pub(super) branches: HashMap<String, &'a LedgerField>,
}

#[derive(Clone)]
pub(super) struct SelectedSetRole<'a> {
    pub(super) circuit: &'a StatefulCircuit,
    branches: HashMap<String, &'a LedgerField>,
}

impl SelectedSetRole<'_> {
    pub(super) fn same_family(&self, other: &Self) -> bool {
        self.circuit.parameters[0].ty == other.circuit.parameters[0].ty
            && self.branches.len() == other.branches.len()
            && self.branches.iter().all(|(variant, declaration)| {
                other
                    .branches
                    .get(variant)
                    .is_some_and(|candidate| std::ptr::eq(*candidate, *declaration))
            })
    }
}

pub(super) fn checked_read<'a>(
    circuit: &'a StatefulCircuit,
    fields: &HashMap<&str, &'a LedgerField>,
) -> Option<SelectedSetRole<'a>> {
    Some(SelectedSetRole {
        circuit,
        branches: read(circuit, fields)?,
    })
}

pub(super) fn checked_insert<'a>(
    circuit: &'a StatefulCircuit,
    fields: &HashMap<&str, &'a LedgerField>,
) -> Option<SelectedSetRole<'a>> {
    Some(SelectedSetRole {
        circuit,
        branches: writes(circuit, fields, Mutation::Insert)?,
    })
}

pub(super) fn checked_remove<'a>(
    circuit: &'a StatefulCircuit,
    fields: &HashMap<&str, &'a LedgerField>,
) -> Option<SelectedSetRole<'a>> {
    Some(SelectedSetRole {
        circuit,
        branches: writes(circuit, fields, Mutation::Remove)?,
    })
}

fn formals(circuit: &StatefulCircuit) -> Option<(&str, &str, &Type)> {
    let [relation, key] = circuit.parameters.as_slice() else {
        return None;
    };
    if !matches!(relation.ty, Type::Enum { .. }) || key.ty != Type::OpaqueString {
        return None;
    }
    Some((&relation.name, &key.name, &relation.ty))
}

fn variant<'a>(condition: &'a Expr, relation: &str, ty: &Type) -> Option<&'a str> {
    let Expr::Equal { left, right } = condition else {
        return None;
    };
    let (
        Expr::Parameter { name },
        Expr::EnumVariant {
            ty: variant_ty,
            variant,
        },
    ) = (left.as_ref(), right.as_ref())
    else {
        return None;
    };
    let Type::Enum { variants, .. } = ty else {
        return None;
    };
    (name == relation && variant_ty == ty && variants.contains(variant)).then_some(variant.as_str())
}

fn slot<'a>(
    fields: &HashMap<&str, &'a LedgerField>,
    field: &str,
    index: u8,
) -> Option<&'a LedgerField> {
    let declaration = fields.get(field).copied()?;
    (declaration.index == index
        && slot_path(declaration)
        && matches!(
            declaration.declaration,
            LedgerFieldKind::Set {
                ty: Type::OpaqueString
            }
        ))
    .then_some(declaration)
}

fn read<'a>(
    circuit: &StatefulCircuit,
    fields: &HashMap<&str, &'a LedgerField>,
) -> Option<HashMap<String, &'a LedgerField>> {
    let (relation, key, ty) = formals(circuit)?;
    if !circuit.actions.is_empty() || circuit.result != Type::Boolean {
        return None;
    }
    let StateReturn::Expression { value } = &circuit.return_value else {
        return None;
    };
    let mut current = value;
    let mut branches = HashMap::new();
    let mut occupied = HashSet::new();
    loop {
        match current {
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                let variant = variant(condition, relation, ty)?;
                let Expr::SetMember {
                    field,
                    index,
                    value,
                } = then.as_ref()
                else {
                    return None;
                };
                if !matches!(value.as_ref(), Expr::Parameter { name } if name == key) {
                    return None;
                }
                let declaration = slot(fields, field, *index)?;
                if !occupied.insert(declaration as *const LedgerField)
                    || branches.insert(variant.to_owned(), declaration).is_some()
                {
                    return None;
                }
                current = otherwise;
            }
            Expr::Boolean { value: false } if !branches.is_empty() => return Some(branches),
            _ => return None,
        }
    }
}

#[derive(Clone, Copy)]
enum Mutation {
    Insert,
    Remove,
}
fn writes<'a>(
    circuit: &StatefulCircuit,
    fields: &HashMap<&str, &'a LedgerField>,
    mutation: Mutation,
) -> Option<HashMap<String, &'a LedgerField>> {
    let (relation, key, ty) = formals(circuit)?;
    if circuit.result != Type::Unit || circuit.return_value != StateReturn::Unit {
        return None;
    }
    let [first] = circuit.actions.as_slice() else {
        return None;
    };
    let mut current = first;
    let mut branches = HashMap::new();
    let mut occupied = HashSet::new();
    loop {
        match current {
            StateAction::If {
                condition,
                then,
                otherwise,
            } => {
                let variant = variant(condition, relation, ty)?;
                let (field, index, value) = match (mutation, then.as_ref()) {
                    (
                        Mutation::Insert,
                        StateAction::SetInsert {
                            field,
                            index,
                            value,
                        },
                    )
                    | (
                        Mutation::Remove,
                        StateAction::SetRemove {
                            field,
                            index,
                            value,
                        },
                    ) => (field, index, value),
                    _ => return None,
                };
                if !matches!(value, Expr::Parameter { name } if name == key) {
                    return None;
                }
                let declaration = slot(fields, field, *index)?;
                if !occupied.insert(declaration as *const LedgerField)
                    || branches.insert(variant.to_owned(), declaration).is_some()
                {
                    return None;
                }
                current = otherwise;
            }
            StateAction::Sequence { actions } if actions.is_empty() && !branches.is_empty() => {
                return Some(branches);
            }
            _ => return None,
        }
    }
}

impl<'a> SelectedSetFamily<'a> {
    pub(super) fn join(
        read: SelectedSetRole<'a>,
        insert: &SelectedSetRole<'a>,
        remove: &SelectedSetRole<'a>,
    ) -> Option<Self> {
        if !read.same_family(insert) || !read.same_family(remove) {
            return None;
        }
        Some(Self {
            read: read.circuit,
            insert: insert.circuit,
            remove: remove.circuit,
            branches: read.branches,
        })
    }

    #[cfg(test)]
    pub(super) fn checked(
        read_circuit: &'a StatefulCircuit,
        insert: &'a StatefulCircuit,
        remove: &'a StatefulCircuit,
        fields: &HashMap<&str, &'a LedgerField>,
    ) -> Option<Self> {
        Self::join(
            checked_read(read_circuit, fields)?,
            &checked_insert(insert, fields)?,
            &checked_remove(remove, fields)?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Contract;

    fn contract(path: &str) -> Contract {
        serde_json::from_str(
            &std::fs::read_to_string(format!(
                "{}/tests/relation-composition/{path}",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap(),
        )
        .unwrap()
    }
    fn family<'a>(contract: &'a Contract, names: [&str; 3]) -> Option<SelectedSetFamily<'a>> {
        let fields = contract
            .ledger_fields
            .iter()
            .map(|f| (f.id.as_str(), f))
            .collect();
        let get = |name| {
            contract
                .stateful_circuits
                .iter()
                .find(|c| c.name == name)
                .unwrap()
        };
        SelectedSetFamily::checked(get(names[0]), get(names[1]), get(names[2]), &fields)
    }

    #[test]
    fn pinned_original_and_renamed_two_set_family_are_structurally_coupled() {
        let did = contract("original-did-schema20-ir.json");
        let small = contract("two-set-schema20-ir.json");
        let four = contract("four-set-schema20-ir.json");
        let did_names = [
            "verificationMethodRelationMember",
            "insertVerificationMethodRelation",
            "removeVerificationMethodRelationFromLedger",
        ];
        assert_eq!(family(&did, did_names).unwrap().branches.len(), 5);
        assert_eq!(
            family(&small, ["member", "insert", "remove"])
                .unwrap()
                .branches
                .len(),
            2
        );
        assert_eq!(
            family(&four, ["member", "insert", "remove"])
                .unwrap()
                .branches
                .len(),
            4
        );
        println!("checked five original and two renamed branches");
    }

    #[test]
    fn wrong_declared_slot_duplicate_variant_or_hidden_tail_write_refuses() {
        let original = contract("two-set-schema20-ir.json");
        let names = ["member", "insert", "remove"];
        let mut changed = original.clone();
        let insert = changed
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "insert")
            .unwrap();
        let StateAction::If { then, .. } = &mut insert.actions[0] else {
            unreachable!()
        };
        let StateAction::SetInsert { field, .. } = then.as_mut() else {
            unreachable!()
        };
        *field = "agreement".to_owned();
        assert!(family(&changed, names).is_none());

        let mut changed = original.clone();
        let read = changed
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "member")
            .unwrap();
        let StateReturn::Expression { value } = &mut read.return_value else {
            unreachable!()
        };
        let Expr::If {
            condition: first,
            otherwise,
            ..
        } = value
        else {
            unreachable!()
        };
        let Expr::If {
            condition: second, ..
        } = otherwise.as_mut()
        else {
            unreachable!()
        };
        **second = *first.clone();
        assert!(family(&changed, names).is_none());

        let mut changed = original;
        let insert = changed
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "insert")
            .unwrap();
        let mut tail = &mut insert.actions[0];
        while let StateAction::If { otherwise, .. } = tail {
            tail = otherwise.as_mut();
        }
        *tail = StateAction::SetInsert {
            field: "auth".to_owned(),
            index: 0,
            value: Expr::Parameter {
                name: "id".to_owned(),
            },
        };
        assert!(family(&changed, names).is_none());
    }
}
