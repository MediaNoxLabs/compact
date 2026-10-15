// Literal H197 old traversal bodies; headers permit test-only sibling calls.
use super::*;

pub(in crate::named_collection_tests) fn collect_expression_types<'ir>(
    expr: &'ir Expr,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
    origins: &mut type_declarations::DefinitionOrigins<'ir>,
) -> Result<(), RenderError> {
    match expr {
        Expr::Default { ty } | Expr::EnumVariant { ty, .. } => {
            collect_named_types_with_origins(ty, structs, enums, Some(origins))?
        }
        Expr::StructField { value, .. } | Expr::TupleIndex { value, .. } => {
            collect_expression_types(value, structs, enums, origins)?
        }
        Expr::StructLiteral { ty, fields } => {
            collect_named_types_with_origins(ty, structs, enums, Some(origins))?;
            for field in fields {
                collect_expression_types(field, structs, enums, origins)?;
            }
        }
        Expr::Tuple { elements } => {
            for element in elements {
                collect_expression_types(element, structs, enums, origins)?;
            }
        }
        Expr::Vector { element, elements } => {
            collect_named_types_with_origins(element, structs, enums, Some(origins))?;
            for value in elements {
                collect_expression_types(value, structs, enums, origins)?;
            }
        }
        Expr::VectorMap {
            parameter,
            source,
            body,
            result,
            ..
        } => {
            collect_named_types_with_origins(&parameter.ty, structs, enums, Some(origins))?;
            collect_named_types_with_origins(result, structs, enums, Some(origins))?;
            collect_expression_types(source, structs, enums, origins)?;
            collect_expression_types(body, structs, enums, origins)?;
        }
        Expr::VectorFoldCall {
            initial,
            source,
            accumulator,
            element,
            ..
        } => {
            collect_named_types_with_origins(accumulator, structs, enums, Some(origins))?;
            collect_named_types_with_origins(element, structs, enums, Some(origins))?;
            collect_expression_types(initial, structs, enums, origins)?;
            collect_expression_types(source, structs, enums, origins)?;
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            collect_expression_types(condition, structs, enums, origins)?;
            collect_expression_types(then, structs, enums, origins)?;
            collect_expression_types(otherwise, structs, enums, origins)?;
        }
        Expr::Let { bindings, body } => {
            for binding in bindings {
                collect_named_types_with_origins(&binding.ty, structs, enums, Some(origins))?;
                collect_expression_types(&binding.value, structs, enums, origins)?;
            }
            collect_expression_types(body, structs, enums, origins)?;
        }
        Expr::Sequence { steps, value } => {
            for step in steps {
                collect_expression_types(step, structs, enums, origins)?;
            }
            collect_expression_types(value, structs, enums, origins)?;
        }
        Expr::Assert { condition, .. } => {
            collect_expression_types(condition, structs, enums, origins)?;
        }
        Expr::SetMember { value, .. }
        | Expr::CounterLessThan {
            threshold: value, ..
        } => collect_expression_types(value, structs, enums, origins)?,
        Expr::MapMember { key, .. } | Expr::MapLookup { key, .. } => {
            collect_expression_types(key, structs, enums, origins)?
        }
        Expr::MerkleCheckRoot { root, .. } | Expr::HistoricMerkleCheckRoot { root, .. } => {
            collect_expression_types(root, structs, enums, origins)?
        }
        Expr::Call { arguments, .. } | Expr::WitnessCall { arguments, .. } => {
            for argument in arguments {
                collect_expression_types(argument, structs, enums, origins)?;
            }
        }
        Expr::TransientHash { value }
        | Expr::PersistentHash { value }
        | Expr::Keccak256 { value }
        | Expr::DegradeToTransient { value }
        | Expr::UpgradeFromTransient { value }
        | Expr::HashToCurve { value }
        | Expr::JubjubPointX { value }
        | Expr::JubjubPointY { value }
        | Expr::EcNeg { value }
        | Expr::JubjubScalarFromNative { value } => {
            collect_expression_types(value, structs, enums, origins)?
        }
        Expr::EcMulGenerator { scalar } => {
            collect_expression_types(scalar, structs, enums, origins)?
        }
        Expr::EcMul { point, scalar } => {
            collect_expression_types(point, structs, enums, origins)?;
            collect_expression_types(scalar, structs, enums, origins)?;
        }
        Expr::ConstructJubjubPoint { x, y } => {
            collect_expression_types(x, structs, enums, origins)?;
            collect_expression_types(y, structs, enums, origins)?;
        }
        Expr::TransientCommit { value, opening } | Expr::PersistentCommit { value, opening } => {
            collect_expression_types(value, structs, enums, origins)?;
            collect_expression_types(opening, structs, enums, origins)?;
        }
        Expr::UnsignedCast { value, .. }
        | Expr::FieldCast { value }
        | Expr::FieldToBytes32 { value } => {
            collect_expression_types(value, structs, enums, origins)?
        }
        Expr::Coerce { value, ty } => {
            collect_expression_types(value, structs, enums, origins)?;
            collect_named_types_with_origins(ty, structs, enums, Some(origins))?;
        }
        Expr::UnsignedAdd { left, right, .. }
        | Expr::UnsignedSubtract { left, right, .. }
        | Expr::UnsignedMultiply { left, right, .. } => {
            collect_expression_types(left, structs, enums, origins)?;
            collect_expression_types(right, structs, enums, origins)?;
        }
        Expr::Add { left, right }
        | Expr::Subtract { left, right }
        | Expr::Multiply { left, right }
        | Expr::Equal { left, right }
        | Expr::NotEqual { left, right }
        | Expr::Compare { left, right, .. }
        | Expr::EcAdd { left, right } => {
            collect_expression_types(left, structs, enums, origins)?;
            collect_expression_types(right, structs, enums, origins)?;
        }
        Expr::Unit
        | Expr::Boolean { .. }
        | Expr::FieldLiteral { .. }
        | Expr::BytesLiteral { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::Parameter { .. } => {}
        Expr::KernelClaim { value, .. } => {
            collect_expression_types(value, structs, enums, origins)?
        }
        Expr::KernelMintShielded { domain, amount } => {
            collect_expression_types(domain, structs, enums, origins)?;
            collect_expression_types(amount, structs, enums, origins)?;
        }
        Expr::CreateZswapInput { coin } => collect_expression_types(coin, structs, enums, origins)?,
        Expr::CreateZswapOutput { coin, recipient } => {
            collect_expression_types(coin, structs, enums, origins)?;
            collect_expression_types(recipient, structs, enums, origins)?;
        }
        Expr::NativeWitnessCall { builtin } => {
            collect_named_types_with_origins(&builtin.result_type(), structs, enums, None)?;
        }
        Expr::KernelSelf { ty } | Expr::ListHead { ty, .. } => {
            collect_named_types_with_origins(ty, structs, enums, Some(origins))?
        }
        Expr::SetSize { .. }
        | Expr::SetIsEmpty { .. }
        | Expr::MapIsEmpty { .. }
        | Expr::ListLength { .. }
        | Expr::ListIsEmpty { .. }
        | Expr::CellRead { .. }
        | Expr::CounterRead { .. } => {}
    }
    Ok(())
}
