use super::*;
pub(super) fn expression(value: &Expr) -> Schema<'_> {
    let mut s = Schema::new();
    macro_rules! unary {
        ($tag:literal,$field:literal,$v:expr) => {{
            s.tag($tag);
            s.node($field, Owner::Expr($v));
        }};
    }
    macro_rules! binary {
        ($tag:literal,$a:literal,$l:expr,$b:literal,$r:expr) => {{
            s.tag($tag);
            s.node($a, Owner::Expr($l));
            s.node($b, Owner::Expr($r));
        }};
    }
    macro_rules! slot {
        ($tag:literal,$field:expr,$index:expr) => {{
            s.tag($tag);
            s.text("field", $field);
            s.number("index", *$index);
        }};
    }
    match value {
        Expr::Unit => s.unit("unit"),
        Expr::Default { ty } => {
            s.tag("default");
            s.node("ty", Owner::Type(ty));
        }
        Expr::Boolean { value } => {
            s.tag("boolean");
            s.field("value", Value::Scalar(Scalar::Bool(*value)));
        }
        Expr::FieldLiteral { value } => {
            s.tag("field_literal");
            s.text("value", value);
        }
        Expr::BytesLiteral { bytes } => {
            s.tag("bytes_literal");
            s.array("bytes", Slice::Bytes(bytes));
        }
        Expr::UnsignedLiteral { value, max } => {
            s.tag("unsigned_literal");
            s.text("value", value);
            s.text("max", max);
        }
        Expr::UnsignedCast { max, value } => {
            unary!("unsigned_cast", "value", value);
            s.text("max", max);
        }
        Expr::FieldCast { value } => unary!("field_cast", "value", value),
        Expr::FieldToBytes32 { value } => unary!("field_to_bytes32", "value", value),
        Expr::Coerce { value, ty } => {
            unary!("coerce", "value", value);
            s.node("ty", Owner::Type(ty));
        }
        Expr::Parameter { name } => {
            s.tag("parameter");
            s.text("name", name);
        }
        Expr::EnumVariant { ty, variant } => {
            s.tag("enum_variant");
            s.node("ty", Owner::Type(ty));
            s.text("variant", variant);
        }
        Expr::StructField {
            value,
            field,
            index,
        } => {
            unary!("struct_field", "value", value);
            s.text("field", field);
            s.size("index", *index);
        }
        Expr::TupleIndex { value, index } => {
            unary!("tuple_index", "value", value);
            s.size("index", *index);
        }
        Expr::StructLiteral { ty, fields } => {
            s.tag("struct_literal");
            s.node("ty", Owner::Type(ty));
            s.array("fields", Slice::Expr(fields));
        }
        Expr::SetMember {
            field,
            index,
            value,
        } => {
            slot!("set_member", field, index);
            s.node("value", Owner::Expr(value));
        }
        Expr::MapMember { field, index, key } => {
            slot!("map_member", field, index);
            s.node("key", Owner::Expr(key));
        }
        Expr::MapLookup { field, index, key } => {
            slot!("map_lookup", field, index);
            s.node("key", Owner::Expr(key));
        }
        Expr::ListLength { field, index } => slot!("list_length", field, index),
        Expr::ListIsEmpty { field, index } => slot!("list_is_empty", field, index),
        Expr::ListHead { field, index, ty } => {
            slot!("list_head", field, index);
            s.node("ty", Owner::Type(ty));
        }
        Expr::MerkleCheckRoot { field, index, root } => {
            slot!("merkle_check_root", field, index);
            s.node("root", Owner::Expr(root));
        }
        Expr::HistoricMerkleCheckRoot { field, index, root } => {
            slot!("historic_merkle_check_root", field, index);
            s.node("root", Owner::Expr(root));
        }
        Expr::CellRead { field, index } => slot!("cell_read", field, index),
        Expr::CounterLessThan {
            field,
            index,
            threshold,
        } => {
            slot!("counter_less_than", field, index);
            s.node("threshold", Owner::Expr(threshold));
        }
        Expr::CounterRead { field, index } => slot!("counter_read", field, index),
        Expr::KernelSelf { ty } => {
            s.tag("kernel_self");
            s.node("ty", Owner::Type(ty));
        }
        Expr::SetSize { field, index } => slot!("set_size", field, index),
        Expr::SetIsEmpty { field, index } => slot!("set_is_empty", field, index),
        Expr::MapIsEmpty { field, index } => slot!("map_is_empty", field, index),
        Expr::Tuple { elements } => {
            s.tag("tuple");
            s.array("elements", Slice::Expr(elements));
        }
        Expr::Vector { element, elements } => {
            s.tag("vector");
            s.node("element", Owner::Type(element));
            s.array("elements", Slice::Expr(elements));
        }
        Expr::VectorMap {
            parameter,
            source,
            body,
            result,
            length,
        } => {
            s.tag("vector_map");
            s.node("parameter", Owner::Parameter(parameter));
            s.node("source", Owner::Expr(source));
            s.node("body", Owner::Expr(body));
            s.node("result", Owner::Type(result));
            s.size("length", *length);
        }
        Expr::VectorFoldCall {
            name,
            initial,
            source,
            accumulator,
            element,
            length,
        } => {
            s.tag("vector_fold_call");
            s.text("name", name);
            s.node("initial", Owner::Expr(initial));
            s.node("source", Owner::Expr(source));
            s.node("accumulator", Owner::Type(accumulator));
            s.node("element", Owner::Type(element));
            s.size("length", *length);
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            s.tag("if");
            s.node("condition", Owner::Expr(condition));
            s.node("then", Owner::Expr(then));
            s.node("otherwise", Owner::Expr(otherwise));
        }
        Expr::Let { bindings, body } => {
            s.tag("let");
            s.array("bindings", Slice::Binding(bindings));
            s.node("body", Owner::Expr(body));
        }
        Expr::Sequence { steps, value } => {
            s.tag("sequence");
            s.array("steps", Slice::Expr(steps));
            s.node("value", Owner::Expr(value));
        }
        Expr::Assert { condition, message } => {
            s.tag("assert");
            s.node("condition", Owner::Expr(condition));
            s.text("message", message);
        }
        Expr::Equal { left, right } => binary!("equal", "left", left, "right", right),
        Expr::NotEqual { left, right } => binary!("not_equal", "left", left, "right", right),
        Expr::Compare {
            operator,
            left,
            right,
        } => {
            binary!("compare", "left", left, "right", right);
            s.text(
                "operator",
                match operator {
                    ComparisonOperator::Less => "less",
                    ComparisonOperator::LessEqual => "less_equal",
                    ComparisonOperator::Greater => "greater",
                    ComparisonOperator::GreaterEqual => "greater_equal",
                },
            );
        }
        Expr::Call { name, arguments } => {
            s.tag("call");
            s.text("name", name);
            s.array("arguments", Slice::Expr(arguments));
        }
        Expr::TransientHash { value } => unary!("transient_hash", "value", value),
        Expr::TransientCommit { value, opening } => {
            binary!("transient_commit", "value", value, "opening", opening)
        }
        Expr::PersistentHash { value } => unary!("persistent_hash", "value", value),
        Expr::Keccak256 { value } => unary!("keccak256", "value", value),
        Expr::PersistentCommit { value, opening } => {
            binary!("persistent_commit", "value", value, "opening", opening)
        }
        Expr::DegradeToTransient { value } => unary!("degrade_to_transient", "value", value),
        Expr::UpgradeFromTransient { value } => unary!("upgrade_from_transient", "value", value),
        Expr::HashToCurve { value } => unary!("hash_to_curve", "value", value),
        Expr::JubjubPointX { value } => unary!("jubjub_point_x", "value", value),
        Expr::JubjubPointY { value } => unary!("jubjub_point_y", "value", value),
        Expr::EcAdd { left, right } => binary!("ec_add", "left", left, "right", right),
        Expr::ConstructJubjubPoint { x, y } => binary!("construct_jubjub_point", "x", x, "y", y),
        Expr::EcNeg { value } => unary!("ec_neg", "value", value),
        Expr::EcMul { point, scalar } => binary!("ec_mul", "point", point, "scalar", scalar),
        Expr::EcMulGenerator { scalar } => unary!("ec_mul_generator", "scalar", scalar),
        Expr::JubjubScalarFromNative { value } => {
            unary!("jubjub_scalar_from_native", "value", value)
        }
        Expr::WitnessCall { name, arguments } => {
            s.tag("witness_call");
            s.text("name", name);
            s.array("arguments", Slice::Expr(arguments));
        }
        Expr::KernelClaim { claim, value } => {
            unary!("kernel_claim", "value", value);
            s.text(
                "claim",
                match claim {
                    KernelClaimKind::Nullifier => "nullifier",
                    KernelClaimKind::CoinSpend => "coin_spend",
                    KernelClaimKind::CoinReceive => "coin_receive",
                },
            );
        }
        Expr::KernelMintShielded { domain, amount } => {
            binary!("kernel_mint_shielded", "domain", domain, "amount", amount)
        }
        Expr::CreateZswapInput { coin } => unary!("create_zswap_input", "coin", coin),
        Expr::CreateZswapOutput { coin, recipient } => {
            binary!("create_zswap_output", "coin", coin, "recipient", recipient)
        }
        Expr::NativeWitnessCall { builtin } => {
            s.tag("native_witness_call");
            s.text(
                "builtin",
                match builtin {
                    NativeWitnessBuiltin::OwnPublicKey => "own_public_key",
                },
            );
        }
        Expr::Add { left, right } => binary!("add", "left", left, "right", right),
        Expr::Subtract { left, right } => binary!("subtract", "left", left, "right", right),
        Expr::Multiply { left, right } => binary!("multiply", "left", left, "right", right),
        Expr::UnsignedAdd { max, left, right } => {
            binary!("unsigned_add", "left", left, "right", right);
            s.text("max", max);
        }
        Expr::UnsignedSubtract { max, left, right } => {
            binary!("unsigned_subtract", "left", left, "right", right);
            s.text("max", max);
        }
        Expr::UnsignedMultiply { max, left, right } => {
            binary!("unsigned_multiply", "left", left, "right", right);
            s.text("max", max);
        }
    }
    s
}
