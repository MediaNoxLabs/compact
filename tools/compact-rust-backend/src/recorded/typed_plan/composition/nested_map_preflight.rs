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

//! Checked lazy nested Map read role. It proves the short-circuit membership
//! guard and enum projection within one declared Map; root coupling and the
//! complete helper/effect audit remain with the relation owner and Audit.
use super::*;

#[derive(Debug, Clone)]
pub(super) struct NestedMapRead<'a> {
    pub(super) circuit: &'a StatefulCircuit,
    pub(super) declaration: &'a LedgerField,
    pub(super) key_formal: String,
    pub(super) projection: Vec<usize>,
    pub(super) curve_type: Type,
    pub(super) excluded_variant: String,
}

fn map<'a>(
    fields: &HashMap<&str, &'a LedgerField>,
    name: &str,
    index: u8,
) -> Option<(&'a LedgerField, &'a Type)> {
    let declaration = fields.get(name).copied()?;
    let LedgerFieldKind::Map {
        key: Type::OpaqueString,
        value,
    } = &declaration.declaration
    else {
        return None;
    };
    (declaration.index == index && slot_path(declaration)).then_some((declaration, value))
}

fn direct_key(value: &Expr, names: &HashSet<String>) -> bool {
    matches!(value, Expr::Parameter { name } if names.contains(name))
}

fn member<'a>(
    value: &Expr,
    fields: &HashMap<&str, &'a LedgerField>,
    keys: &HashSet<String>,
) -> Option<&'a LedgerField> {
    let Expr::MapMember { field, index, key } = value else {
        return None;
    };
    let (declaration, _) = map(fields, field, *index)?;
    direct_key(key, keys).then_some(declaration)
}

fn lookup<'a>(
    value: &Expr,
    fields: &HashMap<&str, &'a LedgerField>,
    keys: &HashSet<String>,
) -> Option<(&'a LedgerField, &'a Type)> {
    let Expr::MapLookup { field, index, key } = value else {
        return None;
    };
    let (declaration, ty) = map(fields, field, *index)?;
    direct_key(key, keys).then_some((declaration, ty))
}

fn enum_variant(value: &Expr, ty: &Type) -> Option<String> {
    let Expr::EnumVariant { ty: found, variant } = value else {
        return None;
    };
    let Type::Enum { variants, .. } = ty else {
        return None;
    };
    (found == ty && variants.contains(variant)).then(|| variant.clone())
}

struct DirectAliases<'a> {
    keys: HashSet<String>,
    types: HashMap<String, Type>,
    outer: &'a StateAction,
    key_formal: String,
}
fn direct_aliases(circuit: &StatefulCircuit) -> Option<DirectAliases<'_>> {
    let key = circuit
        .parameters
        .iter()
        .find(|p| p.ty == Type::OpaqueString)?;
    if circuit
        .parameters
        .iter()
        .filter(|p| p.ty == Type::OpaqueString)
        .count()
        != 1
    {
        return None;
    }
    let mut types = circuit
        .parameters
        .iter()
        .map(|p| (p.name.clone(), p.ty.clone()))
        .collect::<HashMap<_, _>>();
    if types.len() != circuit.parameters.len() || circuit.actions.len() != 1 {
        return None;
    }
    let mut keys = HashSet::from([key.name.clone()]);
    let mut action = &circuit.actions[0];
    while let StateAction::Let {
        bindings,
        action: body,
    } = action
    {
        for binding in bindings {
            let Expr::Parameter { name: source } = &binding.value else {
                return None;
            };
            if types.get(source) != Some(&binding.ty)
                || types
                    .insert(binding.name.clone(), binding.ty.clone())
                    .is_some()
            {
                return None;
            }
            if binding.ty == Type::OpaqueString {
                if !keys.contains(source) {
                    return None;
                }
                keys.insert(binding.name.clone());
            }
        }
        action = body;
    }
    Some(DirectAliases {
        keys,
        types,
        outer: action,
        key_formal: key.name.clone(),
    })
}

fn agreement<'a>(
    action: &StateAction,
    fields: &HashMap<&str, &'a LedgerField>,
    keys: &HashSet<String>,
) -> Option<(&'a LedgerField, Vec<usize>, Type, String)> {
    let StateAction::Sequence { actions } = action else {
        return None;
    };
    let [
        StateAction::Assert {
            condition: guarded, ..
        },
        StateAction::Let { bindings, action },
    ] = actions.as_slice()
    else {
        return None;
    };
    let guarded_map = member(guarded, fields, keys)?;
    let [binding] = bindings.as_slice() else {
        return None;
    };
    let (lookup_map, value_ty) = lookup(&binding.value, fields, keys)?;
    if !std::ptr::eq(guarded_map, lookup_map) || &binding.ty != value_ty {
        return None;
    }
    let StateAction::Assert {
        condition: Expr::Equal { left, right },
        ..
    } = action.as_ref()
    else {
        return None;
    };
    let (path, ty) = projected_binding(left, &binding.name, value_ty)?;
    let variant = enum_variant(right, &ty)?;
    Some((lookup_map, path, ty, variant))
}

fn projected_binding(value: &Expr, name: &str, source: &Type) -> Option<(Vec<usize>, Type)> {
    let mut steps = Vec::new();
    let mut cursor = value;
    while let Expr::StructField {
        field,
        index,
        value,
    } = cursor
    {
        steps.push((field.as_str(), *index));
        cursor = value;
    }
    if !matches!(cursor, Expr::Parameter { name: source } if source == name) || steps.len() < 2 {
        return None;
    }
    steps.reverse();
    let mut ty = source;
    for (field, index) in &steps {
        let Type::Struct { fields, .. } = ty else {
            return None;
        };
        let next = fields.get(*index)?;
        if &next.name != field {
            return None;
        }
        ty = &next.ty;
    }
    matches!(ty, Type::Enum { .. }).then(|| {
        (
            steps.into_iter().map(|(_, index)| index).collect(),
            ty.clone(),
        )
    })
}

fn projected_lookup<'a>(
    value: &Expr,
    fields: &HashMap<&str, &'a LedgerField>,
    keys: &HashSet<String>,
) -> Option<(&'a LedgerField, Vec<usize>, Type)> {
    let mut steps = Vec::new();
    let mut cursor = value;
    while let Expr::StructField {
        field,
        index,
        value,
    } = cursor
    {
        steps.push((field.as_str(), *index));
        cursor = value;
    }
    if steps.len() < 2 {
        return None;
    }
    steps.reverse();
    let (declaration, ty) = lookup(cursor, fields, keys)?;
    let mut source = ty;
    for (field, index) in &steps {
        let Type::Struct { fields, .. } = source else {
            return None;
        };
        let next = fields.get(*index)?;
        if &next.name != field {
            return None;
        }
        source = &next.ty;
    }
    matches!(source, Type::Enum { .. }).then(|| {
        (
            declaration,
            steps.into_iter().map(|(_, index)| index).collect(),
            source.clone(),
        )
    })
}

fn signing<'a>(
    action: &StateAction,
    fields: &HashMap<&str, &'a LedgerField>,
    keys: &HashSet<String>,
) -> Option<(&'a LedgerField, Vec<usize>, Type, String)> {
    let StateAction::Assert {
        condition:
            Expr::If {
                condition,
                then,
                otherwise,
            },
        ..
    } = action
    else {
        return None;
    };
    let Expr::If {
        condition: present,
        then: absent_false,
        otherwise: absent_true,
    } = condition.as_ref()
    else {
        return None;
    };
    if !matches!(absent_false.as_ref(), Expr::Boolean { value: false })
        || !matches!(absent_true.as_ref(), Expr::Boolean { value: true })
        || !matches!(then.as_ref(), Expr::Boolean { value: true })
    {
        return None;
    }
    let guarded_map = member(present, fields, keys)?;
    let Expr::NotEqual { left, right } = otherwise.as_ref() else {
        return None;
    };
    let (lookup_map, path, ty) = projected_lookup(left, fields, keys)?;
    if !std::ptr::eq(guarded_map, lookup_map) {
        return None;
    }
    let variant = enum_variant(right, &ty)?;
    Some((lookup_map, path, ty, variant))
}

impl<'a> NestedMapRead<'a> {
    pub(super) fn retains_checked_projection(&self) -> bool {
        let LedgerFieldKind::Map { value, .. } = &self.declaration.declaration else {
            return false;
        };
        let mut ty = value;
        for index in &self.projection {
            let Type::Struct { fields, .. } = ty else {
                return false;
            };
            let Some(field) = fields.get(*index) else {
                return false;
            };
            ty = &field.ty;
        }
        ty == &self.curve_type
            && matches!(ty, Type::Enum { variants, .. } if variants.contains(&self.excluded_variant))
            && self
                .circuit
                .parameters
                .iter()
                .any(|p| p.name == self.key_formal && p.ty == Type::OpaqueString)
    }
    pub(super) fn checked(
        circuit: &'a StatefulCircuit,
        fields: &HashMap<&str, &'a LedgerField>,
    ) -> Option<Self> {
        if circuit.result != Type::Unit || circuit.return_value != StateReturn::Unit {
            return None;
        }
        let DirectAliases {
            keys,
            types,
            outer,
            key_formal,
        } = direct_aliases(circuit)?;
        let StateAction::If {
            condition,
            then,
            otherwise,
        } = outer
        else {
            return None;
        };
        // Only a typed scalar selector may run before either Map branch. The
        // complete expression and any referenced pure call receive the
        // existing recursive Audit later.
        let scalar_selector = match condition {
            Expr::Parameter { name } => types.get(name) == Some(&Type::Boolean),
            Expr::Equal { left, right } => match (left.as_ref(), right.as_ref()) {
                (Expr::Parameter { name }, Expr::EnumVariant { ty, variant }) => {
                    types.get(name) == Some(ty)
                        && matches!(ty, Type::Enum { variants, .. } if variants.contains(variant))
                }
                _ => false,
            },
            _ => false,
        };
        if !scalar_selector {
            return None;
        }
        let first = agreement(then, fields, &keys)?;
        let second = match otherwise.as_ref() {
            StateAction::If {
                condition: Expr::Call { .. },
                then,
                otherwise,
            } if matches!(otherwise.as_ref(), StateAction::Sequence { actions } if actions.is_empty()) => {
                signing(then, fields, &keys)?
            }
            other => signing(other, fields, &keys)?,
        };
        if !std::ptr::eq(first.0, second.0)
            || first.1 != second.1
            || first.2 != second.2
            || first.3 != second.3
        {
            return None;
        }
        Some(Self {
            circuit,
            declaration: first.0,
            key_formal,
            projection: first.1,
            curve_type: first.2,
            excluded_variant: first.3,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Contract;
    use serde_json::Value;

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
    fn check<'a>(c: &'a Contract, name: &str) -> Option<NestedMapRead<'a>> {
        let fields = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
        NestedMapRead::checked(
            c.stateful_circuits.iter().find(|c| c.name == name)?,
            &fields,
        )
    }
    #[test]
    fn original_and_renamed_reducer_preserve_branch_local_guard() {
        let original = contract("original-did-schema20-ir.json");
        let small = contract("nested-map-schema20-ir.json");
        let a = check(&original, "assertVerificationMethodRelationCompatible").unwrap();
        let b = check(&small, "check").unwrap();
        assert_eq!(a.projection, vec![2, 1]);
        assert_eq!(b.projection, vec![1, 0]);
        assert_eq!(a.declaration.id, "verificationMethods");
        assert_eq!(b.declaration.id, "methods");
        assert_eq!(a.key_formal, "methodId");
        assert_eq!(b.key_formal, "id");
        assert!(matches!(a.curve_type, Type::Enum { .. }));
        assert!(matches!(b.curve_type, Type::Enum { .. }));
        assert_eq!(a.excluded_variant, "X25519");
        assert_eq!(b.excluded_variant, "X25519");
    }
    #[test]
    fn unrelated_map_or_eager_lookup_refuses() {
        let original = contract("nested-map-schema20-ir.json");
        let mut altered = original.clone();
        let mut other = altered.ledger_fields[0].clone();
        other.id = "otherSameTypedMap".to_owned();
        other.index = 1;
        other.path = vec![1];
        altered.ledger_fields.push(other);
        let StateAction::Let { action, .. } = &mut altered.stateful_circuits[0].actions[0] else {
            unreachable!()
        };
        let StateAction::Let { action, .. } = action.as_mut() else {
            unreachable!()
        };
        let StateAction::If { then, .. } = action.as_mut() else {
            unreachable!()
        };
        let StateAction::Sequence { actions } = then.as_mut() else {
            unreachable!()
        };
        let StateAction::Let { bindings, .. } = &mut actions[1] else {
            unreachable!()
        };
        let Expr::MapLookup { field, index, .. } = &mut bindings[0].value else {
            unreachable!()
        };
        *field = "otherSameTypedMap".to_owned();
        *index = 1;
        assert!(check(&altered, "check").is_none());

        let mut altered = original;
        let StateAction::Let { action, .. } = &mut altered.stateful_circuits[0].actions[0] else {
            unreachable!()
        };
        let StateAction::Let { action, .. } = action.as_mut() else {
            unreachable!()
        };
        let StateAction::If { otherwise, .. } = action.as_mut() else {
            unreachable!()
        };
        let StateAction::Assert { condition, .. } = otherwise.as_mut() else {
            unreachable!()
        };
        let Expr::If {
            condition: nested,
            otherwise,
            ..
        } = condition
        else {
            unreachable!()
        };
        std::mem::swap(nested, otherwise);
        assert!(check(&altered, "check").is_none());
    }

    fn change_first(value: &mut Value, kind: &str, update: &mut impl FnMut(&mut Value)) -> bool {
        if value["kind"] == kind {
            update(value);
            return true;
        }
        match value {
            Value::Object(fields) => fields.values_mut().any(|v| change_first(v, kind, update)),
            Value::Array(values) => values.iter_mut().any(|v| change_first(v, kind, update)),
            _ => false,
        }
    }

    #[test]
    fn original_and_renamed_reducer_reject_wrong_key_projection_and_hidden_branch() {
        for (path, helper, curve_index, map_name, key_formal) in [
            (
                "original-did-schema20-ir.json",
                "assertVerificationMethodRelationCompatible",
                1,
                "verificationMethods",
                "methodId",
            ),
            ("nested-map-schema20-ir.json", "check", 0, "methods", "id"),
        ] {
            let original = contract(path);
            assert!(check(&original, helper).is_some());
            fn target<'a>(value: &'a mut Value, helper: &str) -> &'a mut Value {
                value["stateful_circuits"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|c| c["name"] == helper)
                    .unwrap()
            }

            let mut wrong_key = serde_json::to_value(&original).unwrap();
            let circuit = target(&mut wrong_key, helper);
            let mut second = circuit["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["ty"]["kind"] == "opaque_string")
                .unwrap()
                .clone();
            second["name"] = "differentKey".into();
            circuit["parameters"].as_array_mut().unwrap().push(second);
            assert!(change_first(
                &mut circuit["actions"],
                "map_lookup",
                &mut |v| v["key"]["name"] = "differentKey".into()
            ));
            assert!(check(&serde_json::from_value(wrong_key).unwrap(), helper).is_none());

            let mut wrong_projection = serde_json::to_value(&original).unwrap();
            fn change_projection(value: &mut Value, curve_index: usize) -> bool {
                if value["kind"] == "struct_field"
                    && (value["field"] == "crv" || value["field"] == "curve")
                {
                    assert_eq!(value["index"], curve_index);
                    value["index"] = (curve_index + 1).into();
                    return true;
                }
                match value {
                    Value::Object(fields) => fields
                        .values_mut()
                        .any(|v| change_projection(v, curve_index)),
                    Value::Array(values) => {
                        values.iter_mut().any(|v| change_projection(v, curve_index))
                    }
                    _ => false,
                }
            }
            assert!(change_projection(
                &mut target(&mut wrong_projection, helper)["actions"],
                curve_index
            ));
            assert!(check(&serde_json::from_value(wrong_projection).unwrap(), helper).is_none());

            let mut hidden = serde_json::to_value(&original).unwrap();
            fn wrap_assert(value: &mut Value, map_name: &str, key_formal: &str) -> bool {
                if value["kind"] == "assert" {
                    let assertion = std::mem::take(value);
                    *value = serde_json::json!({
                        "kind":"sequence", "actions":[assertion, {
                            "kind":"map_remove", "field":map_name, "index":1,
                            "key":{"kind":"parameter","name":key_formal}
                        }]
                    });
                    return true;
                }
                match value {
                    Value::Object(fields) => fields
                        .values_mut()
                        .any(|v| wrap_assert(v, map_name, key_formal)),
                    Value::Array(values) => values
                        .iter_mut()
                        .any(|v| wrap_assert(v, map_name, key_formal)),
                    _ => false,
                }
            }
            fn insert_hidden(value: &mut Value, map_name: &str, key_formal: &str) -> bool {
                if value["kind"] == "if"
                    && wrap_assert(&mut value["otherwise"], map_name, key_formal)
                {
                    return true;
                }
                match value {
                    Value::Object(fields) => fields
                        .values_mut()
                        .any(|v| insert_hidden(v, map_name, key_formal)),
                    Value::Array(values) => values
                        .iter_mut()
                        .any(|v| insert_hidden(v, map_name, key_formal)),
                    _ => false,
                }
            }
            assert!(insert_hidden(
                &mut target(&mut hidden, helper)["actions"],
                map_name,
                key_formal
            ));
            assert!(check(&serde_json::from_value(hidden).unwrap(), helper).is_none());
        }
    }
}
