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

//! Bounded pure-call policies used by legacy recorded lowering.
//! These audit admitted shapes; ordinary source and renderer typing still apply.

use super::recordable_cell_type;
use crate::ir::{Expr, PureCircuit, Type};
use std::collections::{HashMap, HashSet};

pub(super) fn identity_struct_call_argument(value: &Expr) -> bool {
    match value {
        Expr::Coerce {
            value,
            ty: Type::Struct { .. },
        } => identity_struct_call_argument(value),
        Expr::Coerce { .. } => false,
        _ => true,
    }
}

// Pure struct values can reuse the native typed function once every node is
// known to be effect-free. Ledger reads, witness calls and nested calls remain
// outside this value family; recording owns their evaluation separately.
pub(super) fn closed_struct_hash_value(value: &Expr) -> bool {
    match value {
        Expr::Parameter { .. } | Expr::BytesLiteral { .. } => true,
        Expr::StructField { value, .. } | Expr::PersistentHash { value } => {
            closed_struct_hash_value(value)
        }
        Expr::Coerce { value, ty } => recordable_cell_type(ty) && closed_struct_hash_value(value),
        Expr::Tuple { elements } => elements.iter().all(closed_struct_hash_value),
        Expr::StructLiteral { ty, fields } => {
            recordable_cell_type(ty) && fields.iter().all(closed_struct_hash_value)
        }
        _ => false,
    }
}

/// A pure Field call may be evaluated while recording only when its whole
/// transitive body is scalar arithmetic. Hashes and other primitives need
/// their own VM/gas parity decision before they can join this path.
pub(super) fn closed_pure_field_call(
    name: &str,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    visiting: &mut HashSet<String>,
) -> bool {
    fn scalar_body(
        value: &Expr,
        parameters: &HashSet<&str>,
        pure_circuits: &HashMap<&str, &PureCircuit>,
        visiting: &mut HashSet<String>,
    ) -> bool {
        match value {
            Expr::FieldLiteral { .. } => true,
            Expr::Parameter { name } => parameters.contains(name.as_str()),
            Expr::Coerce { value, ty } if *ty == Type::Field => {
                scalar_body(value, parameters, pure_circuits, visiting)
            }
            Expr::Add { left, right }
            | Expr::Subtract { left, right }
            | Expr::Multiply { left, right } => {
                scalar_body(left, parameters, pure_circuits, visiting)
                    && scalar_body(right, parameters, pure_circuits, visiting)
            }
            Expr::Call { name, arguments } => {
                let Some(callee) = pure_circuits.get(name.as_str()) else {
                    return false;
                };
                arguments.len() == callee.parameters.len()
                    && arguments
                        .iter()
                        .zip(&callee.parameters)
                        .all(|(argument, parameter)| {
                            parameter.ty == Type::Field
                                && scalar_body(argument, parameters, pure_circuits, visiting)
                        })
                    && closed_pure_field_call(name, pure_circuits, visiting)
            }
            _ => false,
        }
    }

    let Some(callee) = pure_circuits.get(name) else {
        return false;
    };
    if callee.result != Type::Field
        || callee
            .parameters
            .iter()
            .any(|parameter| parameter.ty != Type::Field)
        || !visiting.insert(name.to_owned())
    {
        return false;
    }
    let parameters = callee
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect();
    let allowed = scalar_body(&callee.body, &parameters, pure_circuits, visiting);
    visiting.remove(name);
    allowed
}

// Calling the generated pure Rust function retains its exact assertion
// message and propagates failure before any recorded ledger operation. Keep
// this admission bounded to one typed argument, one assertion, and a direct
// Unit or Boolean return; other pure bodies need their own effect audit.
pub(super) fn closed_pure_assert_call(callee: &PureCircuit) -> bool {
    let [parameter] = callee.parameters.as_slice() else {
        return false;
    };
    let Expr::Sequence { steps, value } = &callee.body else {
        return false;
    };
    let [Expr::Assert { condition, .. }] = steps.as_slice() else {
        return false;
    };
    match (
        &parameter.ty,
        &callee.result,
        condition.as_ref(),
        value.as_ref(),
    ) {
        (
            Type::Boolean,
            Type::Boolean,
            Expr::Parameter { name: asserted },
            Expr::Parameter { name: returned },
        ) => asserted == &parameter.name && returned == &parameter.name,
        (Type::Field, Type::Unit, Expr::NotEqual { left, right }, Expr::Unit) => {
            matches!(left.as_ref(), Expr::Parameter { name } if name == &parameter.name)
                && matches!(right.as_ref(), Expr::FieldLiteral { value } if value == "0")
        }
        _ => false,
    }
}

/// Admit a pure unsigned helper only when its complete body consists of
/// bounded unsigned arithmetic and casts. Other pure primitives may carry
/// VM effects or different gas and need an explicit recording decision.
pub(super) fn closed_pure_unsigned_call(
    name: &str,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    visiting: &mut HashSet<String>,
) -> bool {
    fn unsigned_body(
        value: &Expr,
        parameters: &HashSet<&str>,
        pure_circuits: &HashMap<&str, &PureCircuit>,
        visiting: &mut HashSet<String>,
    ) -> bool {
        match value {
            Expr::UnsignedLiteral { .. } => true,
            Expr::Parameter { name } => parameters.contains(name.as_str()),
            Expr::Coerce {
                value,
                ty: Type::Unsigned { .. },
            } => unsigned_body(value, parameters, pure_circuits, visiting),
            Expr::UnsignedCast { value, .. } => {
                unsigned_body(value, parameters, pure_circuits, visiting)
            }
            Expr::UnsignedAdd { left, right, .. }
            | Expr::UnsignedSubtract { left, right, .. }
            | Expr::UnsignedMultiply { left, right, .. } => {
                unsigned_body(left, parameters, pure_circuits, visiting)
                    && unsigned_body(right, parameters, pure_circuits, visiting)
            }
            Expr::Call { name, arguments } => {
                let Some(callee) = pure_circuits.get(name.as_str()) else {
                    return false;
                };
                arguments.len() == callee.parameters.len()
                    && arguments
                        .iter()
                        .zip(&callee.parameters)
                        .all(|(argument, parameter)| {
                            matches!(parameter.ty, Type::Unsigned { .. })
                                && unsigned_body(argument, parameters, pure_circuits, visiting)
                        })
                    && closed_pure_unsigned_call(name, pure_circuits, visiting)
            }
            _ => false,
        }
    }

    let Some(callee) = pure_circuits.get(name) else {
        return false;
    };
    if !matches!(callee.result, Type::Unsigned { .. })
        || callee
            .parameters
            .iter()
            .any(|parameter| !matches!(parameter.ty, Type::Unsigned { .. }))
        || !visiting.insert(name.to_owned())
    {
        return false;
    }
    let parameters = callee
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect();
    let allowed = unsigned_body(&callee.body, &parameters, pure_circuits, visiting);
    visiting.remove(name);
    allowed
}

pub(super) fn field_pair_type(ty: &Type) -> bool {
    matches!(ty, Type::Vector { element, length } if **element == Type::Field && *length == 2)
        || matches!(ty, Type::Tuple { elements } if elements == &[Type::Field, Type::Field])
}

/// A pure Field helper may be evaluated during recording when its entire
/// transitive body only constructs and hashes a pair of Fields. Its declared
/// input is either empty or one typed pair; the caller must separately lower
/// that input through the typed Cell source. Tuple/vector coercions are checked
/// at their declared types instead of assuming the representations match.
pub(super) fn closed_pure_field_pair_hash_call(
    name: &str,
    pure_circuits: &HashMap<&str, &PureCircuit>,
) -> bool {
    fn body_type(
        value: &Expr,
        locals: &HashMap<String, Type>,
        pure_circuits: &HashMap<&str, &PureCircuit>,
        visiting: &mut HashSet<String>,
        saw_hash: &mut bool,
    ) -> Option<Type> {
        match value {
            Expr::FieldLiteral { .. } => Some(Type::Field),
            Expr::Parameter { name } => locals.get(name).cloned(),
            Expr::Vector { element, elements }
                if *element == Type::Field && elements.len() == 2 =>
            {
                elements
                    .iter()
                    .all(|element| {
                        body_type(element, locals, pure_circuits, visiting, saw_hash)
                            == Some(Type::Field)
                    })
                    .then(|| Type::Vector {
                        element: Box::new(Type::Field),
                        length: 2,
                    })
            }
            Expr::Tuple { elements } if elements.len() == 2 => elements
                .iter()
                .all(|element| {
                    body_type(element, locals, pure_circuits, visiting, saw_hash)
                        == Some(Type::Field)
                })
                .then(|| Type::Tuple {
                    elements: vec![Type::Field, Type::Field],
                }),
            Expr::Coerce { value, ty } => {
                let source = body_type(value, locals, pure_circuits, visiting, saw_hash)?;
                (source == *ty || (field_pair_type(&source) && field_pair_type(ty)))
                    .then(|| ty.clone())
            }
            Expr::Let { bindings, body } => {
                let mut scoped = locals.clone();
                for binding in bindings {
                    let actual =
                        body_type(&binding.value, &scoped, pure_circuits, visiting, saw_hash)?;
                    if actual != binding.ty {
                        return None;
                    }
                    scoped.insert(binding.name.clone(), actual);
                }
                body_type(body, &scoped, pure_circuits, visiting, saw_hash)
            }
            Expr::Call { name, arguments } => {
                let callee = pure_circuits.get(name.as_str())?;
                if arguments.len() != callee.parameters.len() || !visiting.insert(name.clone()) {
                    return None;
                }
                let valid_arguments =
                    arguments
                        .iter()
                        .zip(&callee.parameters)
                        .all(|(arg, formal)| {
                            body_type(arg, locals, pure_circuits, visiting, saw_hash)
                                == Some(formal.ty.clone())
                        });
                let mut callee_locals = HashMap::new();
                for parameter in &callee.parameters {
                    callee_locals.insert(parameter.name.clone(), parameter.ty.clone());
                }
                let result = if valid_arguments {
                    body_type(
                        &callee.body,
                        &callee_locals,
                        pure_circuits,
                        visiting,
                        saw_hash,
                    )
                    .filter(|actual| *actual == callee.result)
                } else {
                    None
                };
                visiting.remove(name);
                result
            }
            Expr::TransientHash { value } => {
                let ty = body_type(value, locals, pure_circuits, visiting, saw_hash)?;
                if !field_pair_type(&ty) {
                    return None;
                }
                *saw_hash = true;
                Some(Type::Field)
            }
            _ => None,
        }
    }

    let Some(callee) = pure_circuits.get(name) else {
        return false;
    };
    if callee.result != Type::Field
        || !(callee.parameters.is_empty()
            || (callee.parameters.len() == 1 && field_pair_type(&callee.parameters[0].ty)))
    {
        return false;
    }
    let locals = callee
        .parameters
        .iter()
        .map(|parameter| (parameter.name.clone(), parameter.ty.clone()))
        .collect();
    let mut saw_hash = false;
    let mut visiting = HashSet::from([name.to_owned()]);
    body_type(
        &callee.body,
        &locals,
        pure_circuits,
        &mut visiting,
        &mut saw_hash,
    ) == Some(Type::Field)
        && saw_hash
}

/// A closed literal Vector helper has no ledger, witness, or VM effects. Keep
/// this narrower than general pure calls until their recording parity is known.
pub(super) fn closed_literal_field_vector_call(
    name: &str,
    ty: &Type,
    pure_circuits: &HashMap<&str, &PureCircuit>,
) -> bool {
    let Type::Vector { element, length } = ty else {
        return false;
    };
    if **element != Type::Field {
        return false;
    }
    let Some(callee) = pure_circuits.get(name) else {
        return false;
    };
    if !callee.parameters.is_empty() || callee.result != *ty {
        return false;
    }
    let Expr::Vector {
        element: body_element,
        elements,
    } = &callee.body
    else {
        return false;
    };
    *body_element == Type::Field
        && elements.len() == *length
        && elements
            .iter()
            .all(|element| matches!(element, Expr::FieldLiteral { .. }))
}

#[cfg(test)]
mod unsigned_call_tests {
    use super::*;
    use crate::ir::Parameter;

    fn uint(max: &str) -> Type {
        Type::Unsigned { max: max.into() }
    }

    #[test]
    fn unsigned_pure_call_whitelist_is_transitive_and_rejects_other_primitives() {
        let leaf = PureCircuit {
            source: None,
            name: "product".into(),
            internal: false,
            parameters: vec![Parameter {
                name: "x".into(),
                ty: uint("65535"),
            }],
            result: uint("4294967295"),
            body: Expr::UnsignedCast {
                max: "4294967295".into(),
                value: Box::new(Expr::UnsignedMultiply {
                    max: "4294836225".into(),
                    left: Box::new(Expr::Parameter { name: "x".into() }),
                    right: Box::new(Expr::Parameter { name: "x".into() }),
                }),
            },
        };
        let wrapper = PureCircuit {
            source: None,
            name: "wrapper".into(),
            internal: false,
            parameters: vec![Parameter {
                name: "y".into(),
                ty: uint("65535"),
            }],
            result: uint("4294967295"),
            body: Expr::Call {
                name: "product".into(),
                arguments: vec![Expr::Parameter { name: "y".into() }],
            },
        };
        let mut circuits = HashMap::new();
        circuits.insert("product", &leaf);
        circuits.insert("wrapper", &wrapper);
        assert!(closed_pure_unsigned_call(
            "wrapper",
            &circuits,
            &mut HashSet::new()
        ));

        let effectful = PureCircuit {
            body: Expr::TransientHash {
                value: Box::new(Expr::Parameter { name: "x".into() }),
            },
            ..leaf.clone()
        };
        circuits.insert("product", &effectful);
        assert!(!closed_pure_unsigned_call(
            "wrapper",
            &circuits,
            &mut HashSet::new()
        ));

        let recursive = PureCircuit {
            body: Expr::Call {
                name: "wrapper".into(),
                arguments: vec![Expr::Parameter { name: "x".into() }],
            },
            ..leaf.clone()
        };
        circuits.insert("product", &recursive);
        assert!(!closed_pure_unsigned_call(
            "wrapper",
            &circuits,
            &mut HashSet::new()
        ));
    }
}

#[cfg(test)]
mod tests;
