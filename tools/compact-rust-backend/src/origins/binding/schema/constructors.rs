use super::*;
pub(super) fn amount(value: &CounterAmount) -> Schema<'_> {
    let mut s = Schema::new();
    match value {
        CounterAmount::Literal { value } => {
            s.tag("literal");
            s.number("value", *value);
        }
        CounterAmount::Parameter { name } => {
            s.tag("parameter");
            s.text("name", name);
        }
    }
    s
}
pub(super) fn step(value: &ConstructorStep) -> Schema<'_> {
    let mut s = Schema::new();
    macro_rules! slot {
        ($tag:literal,$field:expr,$index:expr) => {{
            s.tag($tag);
            s.text("field", $field);
            s.number("index", *$index);
        }};
    }
    macro_rules! slot_expr {
        ($tag:literal,$field:expr,$index:expr,$key:literal,$expr:expr) => {{
            slot!($tag, $field, $index);
            s.node($key, Owner::Expr($expr));
        }};
    }
    match value {
        ConstructorStep::Expression { value } => {
            s.tag("expression");
            s.node("value", Owner::Expr(value));
        }
        ConstructorStep::Let { bindings, step } => {
            s.tag("let");
            s.array("bindings", Slice::Binding(bindings));
            s.node("step", Owner::ConstructorStep(step));
        }
        ConstructorStep::Sequence { steps } => {
            s.tag("sequence");
            s.array("steps", Slice::ConstructorStep(steps));
        }
        ConstructorStep::Assert { condition, message } => {
            s.tag("assert");
            s.node("condition", Owner::Expr(condition));
            s.text("message", message);
        }
        ConstructorStep::CellWrite {
            field,
            index,
            value,
        } => slot_expr!("cell_write", field, index, "value", value),
        ConstructorStep::CounterIncrement {
            field,
            index,
            amount,
        } => {
            slot!("counter_increment", field, index);
            s.node("amount", Owner::CounterAmount(amount));
        }
        ConstructorStep::CounterDecrement {
            field,
            index,
            amount,
        } => {
            slot!("counter_decrement", field, index);
            s.node("amount", Owner::CounterAmount(amount));
        }
        ConstructorStep::CounterReset { field, index } => slot!("counter_reset", field, index),
        ConstructorStep::SetInsert {
            field,
            index,
            value,
        } => slot_expr!("set_insert", field, index, "value", value),
        ConstructorStep::SetRemove {
            field,
            index,
            value,
        } => slot_expr!("set_remove", field, index, "value", value),
        ConstructorStep::SetReset { field, index } => slot!("set_reset", field, index),
        ConstructorStep::ListPushFront {
            field,
            index,
            value,
        } => slot_expr!("list_push_front", field, index, "value", value),
        ConstructorStep::ListPopFront { field, index } => slot!("list_pop_front", field, index),
        ConstructorStep::ListReset { field, index } => slot!("list_reset", field, index),
        ConstructorStep::MapInsert {
            field,
            index,
            key,
            value,
        } => {
            slot_expr!("map_insert", field, index, "key", key);
            s.node("value", Owner::Expr(value));
        }
        ConstructorStep::MapInsertDefault { field, index, key } => {
            slot_expr!("map_insert_default", field, index, "key", key)
        }
        ConstructorStep::MapRemove { field, index, key } => {
            slot_expr!("map_remove", field, index, "key", key)
        }
        ConstructorStep::MapReset { field, index } => slot!("map_reset", field, index),
        ConstructorStep::ForEach {
            binding,
            values,
            steps,
        } => {
            s.tag("for_each");
            s.node("binding", Owner::Parameter(binding));
            s.array("values", Slice::Expr(values));
            s.array("steps", Slice::ConstructorStep(steps));
        }
        ConstructorStep::ForEachVector {
            binding,
            source,
            length,
            steps,
        } => {
            s.tag("for_each_vector");
            s.node("binding", Owner::Parameter(binding));
            s.node("source", Owner::Expr(source));
            s.size("length", *length);
            s.array("steps", Slice::ConstructorStep(steps));
        }
        ConstructorStep::If {
            condition,
            then_steps,
            otherwise_steps,
        } => {
            s.tag("if");
            s.node("condition", Owner::Expr(condition));
            s.array("then_steps", Slice::ConstructorStep(then_steps));
            s.array("otherwise_steps", Slice::ConstructorStep(otherwise_steps));
        }
    }
    s
}
