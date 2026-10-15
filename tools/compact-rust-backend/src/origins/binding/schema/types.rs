use super::*;
pub(super) fn ty(value: &Type) -> Schema<'_> {
    let mut s = Schema::new();
    match value {
        Type::Unit => s.unit("unit"),
        Type::Boolean => s.unit("boolean"),
        Type::Field => s.unit("field"),
        Type::JubjubPoint => s.unit("jubjub_point"),
        Type::OpaqueString => s.unit("opaque_string"),
        Type::OpaqueBytes => s.unit("opaque_bytes"),
        Type::Bytes { length } => {
            s.tag("bytes");
            s.size("length", *length);
        }
        Type::Struct { name, fields } => {
            s.tag("struct");
            s.text("name", name);
            s.array("fields", Slice::StructField(fields));
        }
        Type::Enum { name, variants } => {
            s.tag("enum");
            s.text("name", name);
            s.array("variants", Slice::Text(variants));
        }
        Type::Unsigned { max } => {
            s.tag("unsigned");
            s.text("max", max);
        }
        Type::Tuple { elements } => {
            s.tag("tuple");
            s.array("elements", Slice::Type(elements));
        }
        Type::Vector { element, length } => {
            s.tag("vector");
            s.node("element", Owner::Type(element));
            s.size("length", *length);
        }
        Type::LedgerMap { key, value } => {
            s.tag("ledger_map");
            s.node("key", Owner::Type(key));
            s.node("value", Owner::Type(value));
        }
    }
    s
}
pub(super) fn ledger(value: &LedgerFieldKind) -> Schema<'_> {
    let mut s = Schema::new();
    match value {
        LedgerFieldKind::Counter => s.unit("counter"),
        LedgerFieldKind::Cell { ty } => {
            s.tag("cell");
            s.node("ty", Owner::Type(ty));
        }
        LedgerFieldKind::Set { ty } => {
            s.tag("set");
            s.node("ty", Owner::Type(ty));
        }
        LedgerFieldKind::List { ty } => {
            s.tag("list");
            s.node("ty", Owner::Type(ty));
        }
        LedgerFieldKind::Map { key, value } => {
            s.tag("map");
            s.node("key", Owner::Type(key));
            s.node("value", Owner::Type(value));
        }
        LedgerFieldKind::MerkleTree { depth, ty } => {
            s.tag("merkle_tree");
            s.number("depth", *depth);
            s.node("ty", Owner::Type(ty));
        }
        LedgerFieldKind::HistoricMerkleTree { depth, ty } => {
            s.tag("historic_merkle_tree");
            s.number("depth", *depth);
            s.node("ty", Owner::Type(ty));
        }
    }
    s
}
