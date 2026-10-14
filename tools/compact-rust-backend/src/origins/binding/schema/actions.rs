use super::*;
pub(super) fn action(value: &StateAction) -> Schema<'_> {
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
        StateAction::Sequence { actions } => {
            s.tag("sequence");
            s.array("actions", Slice::Action(actions));
        }
        StateAction::If {
            condition,
            then,
            otherwise,
        } => {
            s.tag("if");
            s.node("condition", Owner::Expr(condition));
            s.node("then", Owner::Action(then));
            s.node("otherwise", Owner::Action(otherwise));
        }
        StateAction::Expression { value } => {
            s.tag("expression");
            s.node("value", Owner::Expr(value));
        }
        StateAction::PureCall { name, arguments } => {
            s.tag("pure_call");
            s.text("name", name);
            s.array("arguments", Slice::Expr(arguments));
        }
        StateAction::CircuitCall { name, arguments } => {
            s.tag("circuit_call");
            s.text("name", name);
            s.array("arguments", Slice::Expr(arguments));
        }
        StateAction::NativeWitnessCall { builtin } => {
            s.tag("native_witness_call");
            s.text(
                "builtin",
                match builtin {
                    NativeWitnessBuiltin::OwnPublicKey => "own_public_key",
                },
            );
        }
        StateAction::Assert { condition, message } => {
            s.tag("assert");
            s.node("condition", Owner::Expr(condition));
            s.text("message", message);
        }
        StateAction::Let { bindings, action } => {
            s.tag("let");
            s.array("bindings", Slice::Binding(bindings));
            s.node("action", Owner::Action(action));
        }
        StateAction::CounterIncrement {
            field,
            index,
            amount,
        } => {
            slot!("counter_increment", field, index);
            s.node("amount", Owner::CounterAmount(amount));
        }
        StateAction::CounterDecrement {
            field,
            index,
            amount,
        } => {
            slot!("counter_decrement", field, index);
            s.node("amount", Owner::CounterAmount(amount));
        }
        StateAction::CounterReset { field, index } => slot!("counter_reset", field, index),
        StateAction::CellWrite {
            field,
            index,
            value,
        } => slot_expr!("cell_write", field, index, "value", value),
        StateAction::SetInsert {
            field,
            index,
            value,
        } => slot_expr!("set_insert", field, index, "value", value),
        StateAction::CellWriteCoin {
            field,
            index,
            coin,
            recipient,
        } => {
            slot_expr!("cell_write_coin", field, index, "coin", coin);
            s.node("recipient", Owner::Expr(recipient));
        }
        StateAction::SetInsertCoin {
            field,
            index,
            coin,
            recipient,
        } => {
            slot_expr!("set_insert_coin", field, index, "coin", coin);
            s.node("recipient", Owner::Expr(recipient));
        }
        StateAction::SetRemove {
            field,
            index,
            value,
        } => slot_expr!("set_remove", field, index, "value", value),
        StateAction::SetReset { field, index } => slot!("set_reset", field, index),
        StateAction::ListPushFront {
            field,
            index,
            value,
        } => slot_expr!("list_push_front", field, index, "value", value),
        StateAction::ListPopFront { field, index } => slot!("list_pop_front", field, index),
        StateAction::ListReset { field, index } => slot!("list_reset", field, index),
        StateAction::MapInsert {
            field,
            index,
            key,
            value,
        } => {
            slot_expr!("map_insert", field, index, "key", key);
            s.node("value", Owner::Expr(value));
        }
        StateAction::MapInsertDefault { field, index, key } => {
            slot_expr!("map_insert_default", field, index, "key", key)
        }
        StateAction::MapRemove { field, index, key } => {
            slot_expr!("map_remove", field, index, "key", key)
        }
        StateAction::MapReset { field, index } => slot!("map_reset", field, index),
        StateAction::HistoricMerkleInsertIndexDefault {
            field,
            index,
            position,
        } => slot_expr!(
            "historic_merkle_insert_index_default",
            field,
            index,
            "position",
            position
        ),
        StateAction::HistoricMerkleInsert {
            field,
            index,
            value,
        } => slot_expr!("historic_merkle_insert", field, index, "value", value),
        StateAction::HistoricMerkleInsertIndex {
            field,
            index,
            value,
            position,
        } => {
            slot_expr!("historic_merkle_insert_index", field, index, "value", value);
            s.node("position", Owner::Expr(position));
        }
        StateAction::HistoricMerkleInsertHash { field, index, hash } => {
            slot_expr!("historic_merkle_insert_hash", field, index, "hash", hash)
        }
        StateAction::HistoricMerkleInsertHashIndex {
            field,
            index,
            hash,
            position,
        } => {
            slot_expr!(
                "historic_merkle_insert_hash_index",
                field,
                index,
                "hash",
                hash
            );
            s.node("position", Owner::Expr(position));
        }
        StateAction::HistoricMerkleResetHistory { field, index } => {
            slot!("historic_merkle_reset_history", field, index)
        }
        StateAction::HistoricMerkleResetToDefault { field, index } => {
            slot!("historic_merkle_reset_to_default", field, index)
        }
        StateAction::MerkleInsertIndexDefault {
            field,
            index,
            position,
        } => slot_expr!(
            "merkle_insert_index_default",
            field,
            index,
            "position",
            position
        ),
        StateAction::MerkleInsert {
            field,
            index,
            value,
        } => slot_expr!("merkle_insert", field, index, "value", value),
        StateAction::MerkleInsertIndex {
            field,
            index,
            value,
            position,
        } => {
            slot_expr!("merkle_insert_index", field, index, "value", value);
            s.node("position", Owner::Expr(position));
        }
        StateAction::MerkleInsertHash { field, index, hash } => {
            slot_expr!("merkle_insert_hash", field, index, "hash", hash)
        }
        StateAction::MerkleInsertHashIndex {
            field,
            index,
            hash,
            position,
        } => {
            slot_expr!("merkle_insert_hash_index", field, index, "hash", hash);
            s.node("position", Owner::Expr(position));
        }
        StateAction::MerkleResetToDefault { field, index } => {
            slot!("merkle_reset_to_default", field, index)
        }
    }
    s
}
