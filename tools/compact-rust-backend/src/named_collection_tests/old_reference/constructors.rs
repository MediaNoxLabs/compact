// Literal H197 old traversal bodies; headers permit test-only sibling calls.
use super::*;

pub(in crate::named_collection_tests) fn collect_constructor_step_types<'ir>(
    step: &'ir ConstructorStep,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
    origins: &mut type_declarations::DefinitionOrigins<'ir>,
) -> Result<(), RenderError> {
    match step {
        ConstructorStep::Expression { value } => {
            collect_expression_types(value, structs, enums, origins)?;
        }
        ConstructorStep::Let { bindings, step } => {
            for binding in bindings {
                collect_named_types_with_origins(&binding.ty, structs, enums, Some(origins))?;
                collect_expression_types(&binding.value, structs, enums, origins)?;
            }
            collect_constructor_step_types(step, structs, enums, origins)?;
        }
        ConstructorStep::Sequence { steps } => {
            for step in steps {
                collect_constructor_step_types(step, structs, enums, origins)?;
            }
        }
        ConstructorStep::Assert { condition, .. } => {
            collect_expression_types(condition, structs, enums, origins)?
        }
        ConstructorStep::CellWrite { value, .. }
        | ConstructorStep::SetInsert { value, .. }
        | ConstructorStep::SetRemove { value, .. }
        | ConstructorStep::ListPushFront { value, .. } => {
            collect_expression_types(value, structs, enums, origins)?
        }
        ConstructorStep::MapInsert { key, value, .. } => {
            collect_expression_types(key, structs, enums, origins)?;
            collect_expression_types(value, structs, enums, origins)?;
        }
        ConstructorStep::MapInsertDefault { key, .. } | ConstructorStep::MapRemove { key, .. } => {
            collect_expression_types(key, structs, enums, origins)?
        }
        ConstructorStep::CounterIncrement { .. }
        | ConstructorStep::CounterDecrement { .. }
        | ConstructorStep::CounterReset { .. }
        | ConstructorStep::SetReset { .. }
        | ConstructorStep::ListPopFront { .. }
        | ConstructorStep::ListReset { .. }
        | ConstructorStep::MapReset { .. } => {}
        ConstructorStep::ForEach {
            binding,
            values,
            steps,
        } => {
            collect_named_types_with_origins(&binding.ty, structs, enums, Some(origins))?;
            for value in values {
                collect_expression_types(value, structs, enums, origins)?;
            }
            for step in steps {
                collect_constructor_step_types(step, structs, enums, origins)?;
            }
        }
        ConstructorStep::ForEachVector {
            binding,
            source,
            steps,
            ..
        } => {
            collect_named_types_with_origins(&binding.ty, structs, enums, Some(origins))?;
            collect_expression_types(source, structs, enums, origins)?;
            for step in steps {
                collect_constructor_step_types(step, structs, enums, origins)?;
            }
        }
        ConstructorStep::If {
            condition,
            then_steps,
            otherwise_steps,
        } => {
            collect_expression_types(condition, structs, enums, origins)?;
            for step in then_steps.iter().chain(otherwise_steps) {
                collect_constructor_step_types(step, structs, enums, origins)?;
            }
        }
    }
    Ok(())
}
