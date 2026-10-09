// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Value expression lowering, preserving evaluation and source-path order.
use super::super::validate;
use super::*;
impl Lowerer<'_> {
    pub(super) fn expression(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        let (operation, ty) = match expression {
            Expr::Parameter { name } => {
                return scope
                    .get(name)
                    .cloned()
                    .ok_or_else(|| invalid(path, "unbound value"));
            }
            Expr::Unit => (Operation::Literal(Literal::Unit), Type::Unit),
            Expr::Boolean { value } => {
                (Operation::Literal(Literal::Boolean(*value)), Type::Boolean)
            }
            Expr::FieldLiteral { value } => (
                Operation::Literal(Literal::Field(
                    crate::field_literal_bytes(value).map_err(|e| invalid(path, e.to_string()))?,
                )),
                Type::Field,
            ),
            Expr::UnsignedLiteral { value, max } => {
                let ty = uint(max);
                let bound = validate::unsigned_bound(&ty)
                    .ok_or_else(|| invalid(path, "unsupported unsigned bound"))?;
                let bytes = crate::field_literal_bytes(value)
                    .map_err(|_| invalid(path, "noncanonical unsigned literal"))?;
                let literal = match bound {
                    crate::UnsignedMaximum::Small(max) => {
                        let low = u128::from_le_bytes(bytes[..16].try_into().expect("low limb"));
                        if bytes[16..].iter().any(|b| *b != 0) || low > max {
                            return Err(invalid(path, "unsigned literal outside bound"));
                        }
                        Literal::Unsigned(low)
                    }
                    crate::UnsignedMaximum::Wide { .. } => {
                        if !validate::wide_literal_fits(&bytes, &ty) {
                            return Err(invalid(path, "unsigned literal outside bound"));
                        }
                        Literal::WideUnsigned(bytes)
                    }
                };
                (Operation::Literal(literal), ty)
            }
            Expr::Let {
                bindings: locals,
                body,
            } => {
                let scope = self.locals(locals, scope, bindings, path)?;
                return self.expression(body, &scope, bindings, &format!("{path}.body"));
            }
            Expr::Sequence { steps, value } => {
                for (i, step) in steps.iter().enumerate() {
                    let v =
                        self.expression(step, scope, bindings, &format!("{path}.steps[{i}]"))?;
                    if v.ty != Type::Unit {
                        return Err(invalid(
                            &format!("{path}.steps[{i}]"),
                            "sequence step must be Unit",
                        ));
                    }
                }
                return self.expression(value, scope, bindings, &format!("{path}.value"));
            }
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                return self.conditional(
                    condition,
                    scope,
                    bindings,
                    path,
                    |lower, steps, child_path, is_then| {
                        lower.expression(
                            if is_then { then } else { otherwise },
                            scope,
                            steps,
                            child_path,
                        )
                    },
                );
            }
            Expr::CellRead { field, index } => {
                let (slot, ty) = self.cell(field, *index, path)?;
                (Operation::CellRead(slot), ty)
            }
            Expr::CounterLessThan {
                field,
                index,
                threshold,
            } => {
                let slot = self.counter(field, *index, path)?;
                let threshold_path = format!("{path}.threshold");
                let threshold = self.expression(threshold, scope, bindings, &threshold_path)?;
                if threshold.ty != uint("18446744073709551615") {
                    return Err(invalid(
                        &threshold_path,
                        "counter threshold must be Uint<64>",
                    ));
                }
                (
                    Operation::CounterLessThan { slot, threshold },
                    Type::Boolean,
                )
            }
            Expr::CounterRead { field, index } => {
                let slot = self.counter(field, *index, path)?;
                (
                    Operation::CounterRead { slot },
                    uint("18446744073709551615"),
                )
            }
            Expr::JubjubScalarFromNative { .. }
            | Expr::EcMulGenerator { .. }
            | Expr::EcMul { .. }
            | Expr::EcAdd { .. } => self.jubjub(expression, scope, bindings, path)?,
            Expr::Add { left, right }
            | Expr::Equal { left, right }
            | Expr::UnsignedAdd { left, right, .. } => {
                let a = self.expression(left, scope, bindings, &format!("{path}.left"))?;
                let b = self.expression(right, scope, bindings, &format!("{path}.right"))?;
                match expression {
                    Expr::Add { .. } if a.ty == Type::Field && b.ty == Type::Field => {
                        (Operation::Add(a, b), Type::Field)
                    }
                    Expr::Equal { .. } if validate::equality_type(&a.ty) && a.ty == b.ty => {
                        (Operation::Equal(a, b), Type::Boolean)
                    }
                    Expr::UnsignedAdd { max, .. }
                        if validate::bound(&a.ty).is_some()
                            && validate::bound(&b.ty).is_some()
                            && validate::bound(&uint(max)).is_some() =>
                    {
                        (Operation::UnsignedAdd(a, b), uint(max))
                    }
                    _ => return Err(invalid(path, "invalid arithmetic operand types")),
                }
            }
            Expr::Coerce { value, ty } => {
                let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
                return self.coerce(value, ty, bindings, path);
            }
            Expr::FieldCast { value } => {
                let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
                if validate::bound(&value.ty).is_none() {
                    return Err(invalid(path, "Field cast requires unsigned input"));
                }
                return self.coerce(value, &Type::Field, bindings, path);
            }
            Expr::UnsignedCast { value, max } => {
                let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
                if validate::bound(&value.ty).is_none() || validate::bound(&uint(max)).is_none() {
                    return Err(invalid(path, "unsupported unsigned cast"));
                }
                (
                    Operation::Convert {
                        value,
                        kind: Conversion::UnsignedCast,
                    },
                    uint(max),
                )
            }
            Expr::Assert { condition, message } => {
                let condition =
                    self.expression(condition, scope, bindings, &format!("{path}.condition"))?;
                if condition.ty != Type::Boolean {
                    return Err(invalid(path, "assert requires Boolean"));
                }
                (
                    Operation::Assert {
                        condition,
                        message: message.clone(),
                    },
                    Type::Unit,
                )
            }
            Expr::WitnessCall { name, arguments } => {
                let (witness_id, declaration) = self
                    .declarations
                    .resolve_witness(name)
                    .ok_or_else(|| invalid(path, "unknown witness"))?;
                if declaration.result != Type::Field
                    || arguments.len() != declaration.parameters.len()
                {
                    return Err(invalid(
                        path,
                        "prototype witnesses must return Field and match arity",
                    ));
                }
                if declaration
                    .parameters
                    .iter()
                    .any(|p| !validate::scalar(&p.ty))
                {
                    return Err(invalid(path, "unsupported witness parameter type"));
                }
                let mut args = Vec::new();
                for (i, (argument, parameter)) in
                    arguments.iter().zip(&declaration.parameters).enumerate()
                {
                    let path = format!("{path}.arguments[{i}]");
                    let value = self.expression(argument, scope, bindings, &path)?;
                    args.push(self.coerce(value, &parameter.ty, bindings, &path)?);
                }
                (
                    Operation::Witness {
                        declaration: witness_id,
                        arguments: args,
                    },
                    Type::Field,
                )
            }
            _ => return Err(RecordingGap::expression(expression, path.into())),
        };
        Ok(self.emit(operation, ty, bindings, path))
    }
}
