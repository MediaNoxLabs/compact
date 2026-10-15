// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Independently validate vector cardinality, lexical SSA, and semantic purity.
use super::*;

pub(super) fn validate(
    binding: &Binding,
    declarations: &Declarations,
    incoming: &HashMap<usize, Type>,
    state: &mut Validation,
) -> Result<(), RecordingGap> {
    let path = &binding.path;
    let Operation::VectorMap {
        source,
        parameter,
        body,
    } = &binding.operation
    else {
        return Err(invalid(path, "expected VectorMap operation"));
    };
    used(source, incoming, declarations, path)?;
    declarations.types().validate_at(&parameter.ty, path)?;
    let (
        Type::Vector {
            element: input,
            length,
        },
        Type::Vector {
            element: output,
            length: output_length,
        },
    ) = (&source.ty, &binding.value.ty)
    else {
        return Err(invalid(path, "VectorMap requires vector input and result"));
    };
    if length != output_length
        || **input != parameter.ty
        || **output != body.result.ty
        || !value_type(&parameter.ty)
    {
        return Err(invalid(
            path,
            "VectorMap differs from declared input/result",
        ));
    }
    state.charge(1, path)?;
    if !state.allocated.insert(parameter.id) {
        return Err(invalid(path, "invalid or duplicate VectorMap formal"));
    }
    let mut local = incoming.clone();
    local.insert(parameter.id, parameter.ty.clone());
    block(body, declarations, &local, state, path)?;
    if !super::super::context_free::bindings(&body.bindings) {
        return Err(invalid(path, "VectorMap body must be context-free"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_map_charges_formal_and_empty_body_against_remaining_work() {
        let source = Value {
            id: 0,
            ty: Type::Vector {
                element: Box::new(Type::Field),
                length: 0,
            },
        };
        let parameter = Value {
            id: 1,
            ty: Type::Field,
        };
        let binding = Binding {
            value: Value {
                id: 2,
                ty: source.ty.clone(),
            },
            operation: Operation::VectorMap {
                source: source.clone(),
                parameter: parameter.clone(),
                body: Block {
                    bindings: vec![],
                    result: parameter,
                },
            },
            path: "map".into(),
        };
        let incoming = HashMap::from([(source.id, source.ty)]);
        // The enclosing block has already charged its binding. The nested
        // formal and empty body each consume one more unit, even for length0.
        for (remaining, accepted) in [(2, true), (1, false)] {
            let mut state = Validation {
                allocated: HashSet::from([0]),
                remaining,
                active_calls: vec![],
                block_depth: 1,
            };
            let result = validate(&binding, &Declarations::default(), &incoming, &mut state);
            if accepted {
                result.unwrap();
                assert_eq!(state.remaining, 0);
            } else {
                assert_eq!(
                    result.unwrap_err().detail,
                    "recording plan expanded work limit exceeded"
                );
            }
        }
    }
}
