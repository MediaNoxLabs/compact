use super::*;
pub(super) fn state_return(value: &StateReturn) -> Schema<'_> {
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
        StateReturn::Unit => s.unit("unit"),
        StateReturn::Expression { value } => {
            s.tag("expression");
            s.node("value", Owner::Expr(value));
        }
        StateReturn::Effectful { body } => {
            s.tag("effectful");
            s.node("body", Owner::ReturnPlan(body));
        }
        StateReturn::CellRead { field, index } => slot!("cell_read", field, index),
        StateReturn::CounterRead { field, index } => slot!("counter_read", field, index),
        StateReturn::SetMember {
            field,
            index,
            value,
        } => slot_expr!("set_member", field, index, "value", value),
        StateReturn::SetSize { field, index } => slot!("set_size", field, index),
        StateReturn::SetIsEmpty { field, index } => slot!("set_is_empty", field, index),
        StateReturn::MapMember { field, index, key } => {
            slot_expr!("map_member", field, index, "key", key)
        }
        StateReturn::MapLookup { field, index, key } => {
            slot_expr!("map_lookup", field, index, "key", key)
        }
        StateReturn::MapSize { field, index } => slot!("map_size", field, index),
        StateReturn::MapIsEmpty { field, index } => slot!("map_is_empty", field, index),
        StateReturn::ListLength { field, index } => slot!("list_length", field, index),
        StateReturn::ListIsEmpty { field, index } => slot!("list_is_empty", field, index),
        StateReturn::ListHead { field, index } => slot!("list_head", field, index),
        StateReturn::HistoricMerkleIsFull { field, index } => {
            slot!("historic_merkle_is_full", field, index)
        }
        StateReturn::HistoricMerkleCheckRoot { field, index, root } => {
            slot_expr!("historic_merkle_check_root", field, index, "root", root)
        }
        StateReturn::MerkleIsFull { field, index } => slot!("merkle_is_full", field, index),
        StateReturn::MerkleCheckRoot { field, index, root } => {
            slot_expr!("merkle_check_root", field, index, "root", root)
        }
    }
    s
}
pub(super) fn return_plan(value: &ReturnPlan) -> Schema<'_> {
    let mut s = Schema::new();
    match value {
        ReturnPlan::Value { value } => {
            s.tag("value");
            s.node("value", Owner::Expr(value));
        }
        ReturnPlan::Sequence { actions, result } => {
            s.tag("sequence");
            s.array("actions", Slice::Action(actions));
            s.node("result", Owner::ReturnPlan(result));
        }
        ReturnPlan::Let { bindings, result } => {
            s.tag("let");
            s.array("bindings", Slice::Binding(bindings));
            s.node("result", Owner::ReturnPlan(result));
        }
        ReturnPlan::Conditional {
            condition,
            then,
            otherwise,
        } => {
            s.tag("conditional");
            s.node("condition", Owner::Expr(condition));
            s.node("then", Owner::ReturnPlan(then));
            s.node("otherwise", Owner::ReturnPlan(otherwise));
        }
    }
    s
}
