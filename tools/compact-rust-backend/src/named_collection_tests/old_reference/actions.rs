// Literal H197 old traversal bodies; headers permit test-only sibling calls.
use super::*;

pub(in crate::named_collection_tests) fn collect_action_types<'ir>(
    action: &'ir StateAction,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
    origins: &mut type_declarations::DefinitionOrigins<'ir>,
) -> Result<(), RenderError> {
    match action {
        StateAction::Sequence { actions } => {
            for action in actions {
                collect_action_types(action, structs, enums, origins)?;
            }
        }
        StateAction::If {
            condition,
            then,
            otherwise,
        } => {
            collect_expression_types(condition, structs, enums, origins)?;
            collect_action_types(then, structs, enums, origins)?;
            collect_action_types(otherwise, structs, enums, origins)?;
        }
        StateAction::PureCall { arguments, .. } | StateAction::CircuitCall { arguments, .. } => {
            for argument in arguments {
                collect_expression_types(argument, structs, enums, origins)?;
            }
        }
        StateAction::Expression { value } => {
            collect_expression_types(value, structs, enums, origins)?;
        }
        StateAction::Assert { condition, .. } => {
            collect_expression_types(condition, structs, enums, origins)?;
        }
        StateAction::Let { bindings, action } => {
            for binding in bindings {
                collect_named_types_with_origins(&binding.ty, structs, enums, Some(origins))?;
                collect_expression_types(&binding.value, structs, enums, origins)?;
            }
            collect_action_types(action, structs, enums, origins)?;
        }
        StateAction::CellWrite { value, .. }
        | StateAction::SetInsert { value, .. }
        | StateAction::SetRemove { value, .. }
        | StateAction::ListPushFront { value, .. }
        | StateAction::MerkleInsert { value, .. }
        | StateAction::MerkleInsertHash { hash: value, .. }
        | StateAction::HistoricMerkleInsert { value, .. }
        | StateAction::HistoricMerkleInsertHash { hash: value, .. } => {
            collect_expression_types(value, structs, enums, origins)?;
        }
        StateAction::SetInsertCoin {
            coin, recipient, ..
        }
        | StateAction::CellWriteCoin {
            coin, recipient, ..
        } => {
            collect_expression_types(coin, structs, enums, origins)?;
            collect_expression_types(recipient, structs, enums, origins)?;
        }
        StateAction::MerkleInsertIndex {
            value, position, ..
        }
        | StateAction::MerkleInsertHashIndex {
            hash: value,
            position,
            ..
        }
        | StateAction::HistoricMerkleInsertIndex {
            value, position, ..
        }
        | StateAction::HistoricMerkleInsertHashIndex {
            hash: value,
            position,
            ..
        } => {
            collect_expression_types(value, structs, enums, origins)?;
            collect_expression_types(position, structs, enums, origins)?;
        }
        StateAction::MapInsert { key, value, .. } => {
            collect_expression_types(key, structs, enums, origins)?;
            collect_expression_types(value, structs, enums, origins)?;
        }
        StateAction::MapInsertDefault { key, .. } | StateAction::MapRemove { key, .. } => {
            collect_expression_types(key, structs, enums, origins)?;
        }
        StateAction::MerkleInsertIndexDefault { position, .. }
        | StateAction::HistoricMerkleInsertIndexDefault { position, .. } => {
            collect_expression_types(position, structs, enums, origins)?;
        }
        StateAction::NativeWitnessCall { builtin } => {
            collect_named_types_with_origins(&builtin.result_type(), structs, enums, None)?;
        }
        StateAction::CounterIncrement { .. }
        | StateAction::CounterDecrement { .. }
        | StateAction::CounterReset { .. }
        | StateAction::SetReset { .. }
        | StateAction::ListPopFront { .. }
        | StateAction::ListReset { .. }
        | StateAction::MapReset { .. }
        | StateAction::HistoricMerkleResetHistory { .. }
        | StateAction::HistoricMerkleResetToDefault { .. }
        | StateAction::MerkleResetToDefault { .. } => {}
    }
    Ok(())
}

pub(in crate::named_collection_tests) fn collect_return_plan_types<'ir>(
    plan: &'ir ir::ReturnPlan,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
    origins: &mut type_declarations::DefinitionOrigins<'ir>,
) -> Result<(), RenderError> {
    match plan {
        ir::ReturnPlan::Value { value } => collect_expression_types(value, structs, enums, origins),
        ir::ReturnPlan::Sequence { actions, result } => {
            for action in actions {
                collect_action_types(action, structs, enums, origins)?;
            }
            collect_return_plan_types(result, structs, enums, origins)
        }
        ir::ReturnPlan::Let { bindings, result } => {
            for binding in bindings {
                collect_named_types_with_origins(&binding.ty, structs, enums, Some(origins))?;
                collect_expression_types(&binding.value, structs, enums, origins)?;
            }
            collect_return_plan_types(result, structs, enums, origins)
        }
        ir::ReturnPlan::Conditional {
            condition,
            then,
            otherwise,
        } => {
            collect_expression_types(condition, structs, enums, origins)?;
            collect_return_plan_types(then, structs, enums, origins)?;
            collect_return_plan_types(otherwise, structs, enums, origins)
        }
    }
}
