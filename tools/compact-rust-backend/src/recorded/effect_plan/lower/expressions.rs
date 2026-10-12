// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Value expression lowering, preserving evaluation and source-path order.
use super::super::validate;
use super::*;

/// A selected operation lowerer with one shared result slot in recursive dispatch.
type OperationLowerer<'env> = fn(
    &mut Lowerer<'env>,
    &Expr,
    &Scope,
    &mut Vec<Binding>,
    &str,
) -> Result<(Operation, Type), RecordingGap>;
impl Lowerer<'_> {
    pub(super) fn expression(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        self.spend(path)?;
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
            Expr::BytesLiteral { .. }
            | Expr::EnumVariant { .. }
            | Expr::Default { .. }
            | Expr::StructLiteral { .. }
            | Expr::StructField { .. }
            | Expr::TupleIndex { .. }
            | Expr::Tuple { .. }
            | Expr::Vector { .. } => self.aggregate(expression, scope, bindings, path)?,
            Expr::CreateZswapInput { .. } | Expr::CreateZswapOutput { .. } => {
                self.zswap(expression, scope, bindings, path)?
            }
            Expr::KernelClaim { .. } | Expr::KernelMintShielded { .. } => {
                self.kernel(expression, scope, bindings, path)?
            }
            Expr::NativeWitnessCall {
                builtin: NativeWitnessBuiltin::OwnPublicKey,
            } => {
                let ty = NativeWitnessBuiltin::OwnPublicKey.result_type();
                self.declarations.types().validate_at(&ty, path)?;
                (Operation::OwnPublicKey, ty)
            }
            Expr::KernelSelf { ty } => {
                self.declarations.types().validate_at(ty, path)?;
                if *ty != crate::ir::contract_address_type() {
                    return Err(invalid(
                        path,
                        "kernel self requires canonical ContractAddress",
                    ));
                }
                (Operation::KernelSelf, ty.clone())
            }
            Expr::DegradeToTransient { .. }
            | Expr::UpgradeFromTransient { .. }
            | Expr::TransientHash { .. }
            | Expr::TransientCommit { .. } => {
                self.transient_crypto(expression, scope, bindings, path)?
            }
            Expr::MerkleCheckRoot { field, index, root }
            | Expr::HistoricMerkleCheckRoot { field, index, root } => self.merkle_query(
                field,
                *index,
                matches!(expression, Expr::HistoricMerkleCheckRoot { .. }),
                Some(root),
                scope,
                bindings,
                path,
            )?,
            Expr::FieldToBytes32 { value } => {
                let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
                if value.ty != Type::Field {
                    return Err(invalid(path, "FieldToBytes32 requires a Field operand"));
                }
                (Operation::FieldToBytes32(value), Type::Bytes { length: 32 })
            }
            Expr::PersistentHash { value } => {
                let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
                if !validate::hash_input_type(&value.ty) {
                    return Err(invalid(
                        path,
                        "persistent hash requires a fixed-byte product",
                    ));
                }
                (Operation::PersistentHash(value), Type::Bytes { length: 32 })
            }
            Expr::PersistentCommit { value, opening } => {
                let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
                let opening =
                    self.expression(opening, scope, bindings, &format!("{path}.opening"))?;
                if value.ty
                    != (Type::Tuple {
                        elements: vec![Type::Bytes { length: 32 }; 2],
                    })
                    || opening.ty != (Type::Bytes { length: 32 })
                {
                    return Err(invalid(
                        path,
                        "persistent commitment requires a Bytes32 pair and Bytes32 opening",
                    ));
                }
                (
                    Operation::PersistentCommit { value, opening },
                    Type::Bytes { length: 32 },
                )
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
            Expr::SetMember {
                field,
                index,
                value,
            } => self.set_member(field, *index, value, scope, bindings, path)?,
            Expr::SetSize { field, index } => {
                let (slot, _) = self.set(field, *index, path)?;
                (Operation::SetSize { slot }, uint("18446744073709551615"))
            }
            Expr::SetIsEmpty { field, index } => {
                let (slot, _) = self.set(field, *index, path)?;
                (Operation::SetIsEmpty { slot }, Type::Boolean)
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
            Expr::JubjubPointX { .. }
            | Expr::JubjubPointY { .. }
            | Expr::JubjubScalarFromNative { .. }
            | Expr::EcMulGenerator { .. }
            | Expr::EcMul { .. }
            | Expr::EcAdd { .. } => {
                // One result temporary keeps this recursive dispatch frame bounded.
                let lower: OperationLowerer<'_> = if matches!(
                    expression,
                    Expr::JubjubPointX { .. } | Expr::JubjubPointY { .. }
                ) {
                    Self::jubjub_coordinate
                } else {
                    Self::jubjub
                };
                lower(self, expression, scope, bindings, path)?
            }
            Expr::Add { left, right }
            | Expr::Equal { left, right }
            | Expr::NotEqual { left, right }
            | Expr::UnsignedAdd { left, right, .. }
            | Expr::UnsignedSubtract { left, right, .. }
            | Expr::UnsignedMultiply { left, right, .. } => {
                let a = self.expression(left, scope, bindings, &format!("{path}.left"))?;
                let b = self.expression(right, scope, bindings, &format!("{path}.right"))?;
                match expression {
                    Expr::Add { .. } if a.ty == Type::Field && b.ty == Type::Field => {
                        (Operation::Add(a, b), Type::Field)
                    }
                    Expr::Equal { .. } | Expr::NotEqual { .. }
                        if validate::equality_type(&a.ty) && a.ty == b.ty =>
                    {
                        let operation = if matches!(expression, Expr::NotEqual { .. }) {
                            Operation::NotEqual(a, b)
                        } else {
                            Operation::Equal(a, b)
                        };
                        (operation, Type::Boolean)
                    }
                    Expr::UnsignedAdd { max, .. }
                        if validate::unsigned_add_type(&a.ty, &b.ty, &uint(max)) =>
                    {
                        (Operation::UnsignedAdd(a, b), uint(max))
                    }
                    Expr::UnsignedSubtract { max, .. } | Expr::UnsignedMultiply { max, .. }
                        if validate::bound(&a.ty).is_some()
                            && validate::bound(&b.ty).is_some()
                            && validate::bound(&uint(max)).is_some() =>
                    {
                        let operation = match expression {
                            Expr::UnsignedSubtract { .. } => Operation::UnsignedSubtract(a, b),
                            Expr::UnsignedMultiply { .. } => Operation::UnsignedMultiply(a, b),
                            _ => unreachable!("unsigned arithmetic arm"),
                        };
                        (operation, uint(max))
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
                if validate::unsigned_bound(&value.ty).is_none()
                    || validate::unsigned_bound(&uint(max)).is_none()
                {
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
            Expr::Call { name, arguments } => {
                return self.call(
                    calls::CallSite::Expression(expression),
                    name,
                    arguments,
                    scope,
                    bindings,
                    path,
                );
            }
            Expr::WitnessCall { name, arguments } => {
                let (witness_id, declaration) = self
                    .declarations
                    .resolve_witness(name)
                    .ok_or_else(|| invalid(path, "unknown witness"))?;
                self.declarations
                    .types()
                    .validate_at(&declaration.result, path)?;
                for parameter in &declaration.parameters {
                    self.declarations.types().validate_at(&parameter.ty, path)?;
                }
                if !validate::witness_result_type(&declaration.result)
                    || arguments.len() != declaration.parameters.len()
                {
                    return Err(invalid(
                        path,
                        "witness result must be supported and arguments must match arity",
                    ));
                }
                if declaration
                    .parameters
                    .iter()
                    .any(|p| !validate::witness_parameter_type(&p.ty))
                {
                    return Err(invalid(path, "unsupported witness parameter type"));
                }
                let result = declaration.result.clone();
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
                    result,
                )
            }
            _ => return Err(RecordingGap::expression(expression, path.into())),
        };
        self.declarations.types().validate_at(&ty, path)?;
        Ok(self.emit(operation, ty, bindings, path))
    }
}
