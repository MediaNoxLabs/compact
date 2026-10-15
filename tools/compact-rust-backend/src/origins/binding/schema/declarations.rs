use super::*;
pub(super) fn declaration(owner: Owner<'_>) -> Schema<'_> {
    let mut s = Schema::new();
    match owner {
        Owner::Contract(v) => {
            s.number("schema_version", v.schema_version);
            s.default(
                "type_aliases",
                Value::Slice(Slice::Alias(&v.type_aliases)),
                v.type_aliases.is_empty(),
            );
            s.array("ledger_fields", Slice::Ledger(&v.ledger_fields));
            s.default(
                "constructor",
                v.constructor
                    .as_ref()
                    .map_or(Value::Null, |v| Value::Node(Owner::Constructor(v))),
                v.constructor.is_none(),
            );
            s.default(
                "witnesses",
                Value::Slice(Slice::Witness(&v.witnesses)),
                v.witnesses.is_empty(),
            );
            s.array("circuits", Slice::Pure(&v.circuits));
            s.array("stateful_circuits", Slice::Stateful(&v.stateful_circuits));
        }
        Owner::Alias(v) => {
            s.source(v.source.as_ref());
            s.text("name", &v.name);
            s.node("ty", Owner::Type(&v.ty));
        }
        Owner::Constructor(v) => {
            s.source(v.source.as_ref());
            s.array("parameters", Slice::Parameter(&v.parameters));
            s.array("steps", Slice::ConstructorStep(&v.steps));
        }
        Owner::Witness(v) => {
            s.source(v.source.as_ref());
            s.text("name", &v.name);
            s.array("parameters", Slice::Parameter(&v.parameters));
            s.node("result", Owner::Type(&v.result));
        }
        Owner::Ledger(v) => {
            s.text("id", &v.id);
            s.number("index", v.index);
            s.default(
                "path",
                Value::Slice(Slice::Bytes(&v.path)),
                v.path.is_empty(),
            );
            s.source(v.source.as_ref());
            s.node("declaration", Owner::LedgerKind(&v.declaration));
        }
        Owner::Pure(v) => {
            s.source(v.source.as_ref());
            s.text("name", &v.name);
            s.default(
                "internal",
                Value::Scalar(Scalar::Bool(v.internal)),
                !v.internal,
            );
            s.array("parameters", Slice::Parameter(&v.parameters));
            s.node("result", Owner::Type(&v.result));
            s.node("body", Owner::Expr(&v.body));
        }
        Owner::Stateful(v) => {
            s.source(v.source.as_ref());
            s.text("name", &v.name);
            s.default(
                "internal",
                Value::Scalar(Scalar::Bool(v.internal)),
                !v.internal,
            );
            s.default(
                "parameters",
                Value::Slice(Slice::Parameter(&v.parameters)),
                v.parameters.is_empty(),
            );
            s.array("actions", Slice::Action(&v.actions));
            s.default(
                "result",
                Value::Node(Owner::Type(&v.result)),
                matches!(v.result, Type::Unit),
            );
            s.default(
                "return_value",
                Value::Node(Owner::StateReturn(&v.return_value)),
                matches!(v.return_value, StateReturn::Unit),
            );
        }
        Owner::Parameter(v) => {
            s.text("name", &v.name);
            s.node("ty", Owner::Type(&v.ty));
        }
        Owner::Binding(v) => {
            s.text("name", &v.name);
            s.node("ty", Owner::Type(&v.ty));
            s.node("value", Owner::Expr(&v.value));
        }
        Owner::StructField(v) => {
            s.text("name", &v.name);
            s.node("ty", Owner::Type(&v.ty));
        }
        Owner::SourcePoint(v) => {
            s.text("file", &v.file);
            s.size("line", v.line);
            s.size("column", v.column);
        }
        _ => unreachable!("describe routes enum owners to exhaustive family descriptors"),
    }
    s
}
