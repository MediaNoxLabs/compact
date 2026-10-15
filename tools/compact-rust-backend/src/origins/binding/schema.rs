//! Serialized schema descriptors only: no semantic predicate or checker.
use crate::origins::binding::{identity::Owner, ir::*};
mod actions;
mod constructors;
mod declarations;
mod expressions;
mod returns;
mod types;

#[derive(Clone, Copy)]
pub(crate) enum Scalar<'a> {
    Text(&'a str),
    Bool(bool),
    Integer(u64),
}

#[derive(Clone, Copy)]
pub(crate) enum Slice<'a> {
    Alias(&'a [TypeAlias]),
    Ledger(&'a [LedgerField]),
    Witness(&'a [WitnessDeclaration]),
    Pure(&'a [PureCircuit]),
    Stateful(&'a [StatefulCircuit]),
    Parameter(&'a [Parameter]),
    Binding(&'a [LocalBinding]),
    StructField(&'a [StructField]),
    Type(&'a [Type]),
    Expr(&'a [Expr]),
    Action(&'a [StateAction]),
    ConstructorStep(&'a [ConstructorStep]),
    Text(&'a [String]),
    Bytes(&'a [u8]),
}

#[derive(Clone, Copy)]
pub(crate) enum Value<'a> {
    Node(Owner<'a>),
    Scalar(Scalar<'a>),
    Slice(Slice<'a>),
    Null,
}
impl Slice<'_> {
    pub(crate) fn len(self) -> usize {
        match self {
            Self::Alias(v) => v.len(),
            Self::Ledger(v) => v.len(),
            Self::Witness(v) => v.len(),
            Self::Pure(v) => v.len(),
            Self::Stateful(v) => v.len(),
            Self::Parameter(v) => v.len(),
            Self::Binding(v) => v.len(),
            Self::StructField(v) => v.len(),
            Self::Type(v) => v.len(),
            Self::Expr(v) => v.len(),
            Self::Action(v) => v.len(),
            Self::ConstructorStep(v) => v.len(),
            Self::Text(v) => v.len(),
            Self::Bytes(v) => v.len(),
        }
    }
}
impl<'a> Slice<'a> {
    pub(crate) fn get(self, index: usize) -> Value<'a> {
        match self {
            Self::Alias(v) => Value::Node(Owner::Alias(&v[index])),
            Self::Ledger(v) => Value::Node(Owner::Ledger(&v[index])),
            Self::Witness(v) => Value::Node(Owner::Witness(&v[index])),
            Self::Pure(v) => Value::Node(Owner::Pure(&v[index])),
            Self::Stateful(v) => Value::Node(Owner::Stateful(&v[index])),
            Self::Parameter(v) => Value::Node(Owner::Parameter(&v[index])),
            Self::Binding(v) => Value::Node(Owner::Binding(&v[index])),
            Self::StructField(v) => Value::Node(Owner::StructField(&v[index])),
            Self::Type(v) => Value::Node(Owner::Type(&v[index])),
            Self::Expr(v) => Value::Node(Owner::Expr(&v[index])),
            Self::Action(v) => Value::Node(Owner::Action(&v[index])),
            Self::ConstructorStep(v) => Value::Node(Owner::ConstructorStep(&v[index])),
            Self::Text(v) => Value::Scalar(Scalar::Text(&v[index])),
            Self::Bytes(v) => Value::Scalar(Scalar::Integer(u64::from(v[index]))),
        }
    }
}

pub(crate) struct Member<'a> {
    pub(crate) name: &'static str,
    pub(crate) value: Value<'a>,
    pub(crate) omission: bool,
}
pub(crate) struct Schema<'a> {
    pub(crate) members: Vec<Member<'a>>,
    // Internally tagged serde unit variants can discard raw fields. Those
    // addresses stay transport-only; they are never promoted into typed nodes.
    pub(crate) unit: bool,
}
impl<'a> Schema<'a> {
    pub(crate) fn new() -> Self {
        Self {
            members: Vec::new(),
            unit: false,
        }
    }
    pub(crate) fn field(&mut self, name: &'static str, value: Value<'a>) {
        self.members.push(Member {
            name,
            value,
            omission: false,
        });
    }
    pub(crate) fn default(&mut self, name: &'static str, value: Value<'a>, omission: bool) {
        self.members.push(Member {
            name,
            value,
            omission,
        });
    }
    pub(crate) fn text(&mut self, name: &'static str, text: &'a str) {
        self.field(name, Value::Scalar(Scalar::Text(text)));
    }
    pub(crate) fn number(&mut self, name: &'static str, value: impl Into<u64>) {
        self.field(name, Value::Scalar(Scalar::Integer(value.into())));
    }
    pub(crate) fn size(&mut self, name: &'static str, value: usize) {
        self.number(name, value as u64);
    }
    pub(crate) fn node(&mut self, name: &'static str, value: Owner<'a>) {
        self.field(name, Value::Node(value));
    }
    pub(crate) fn array(&mut self, name: &'static str, value: Slice<'a>) {
        self.field(name, Value::Slice(value));
    }
    pub(crate) fn tag(&mut self, value: &'static str) {
        self.text("kind", value);
    }
    pub(crate) fn unit(&mut self, value: &'static str) {
        self.tag(value);
        self.unit = true;
    }
    pub(crate) fn source(&mut self, source: Option<&'a SourceLocation>) {
        self.default(
            "source",
            source.map_or(Value::Null, |p| Value::Node(Owner::SourcePoint(p))),
            source.is_none(),
        );
    }
}
pub(crate) fn describe(owner: Owner<'_>) -> Schema<'_> {
    match owner {
        Owner::Type(v) => types::ty(v),
        Owner::LedgerKind(v) => types::ledger(v),
        Owner::Expr(v) => expressions::expression(v),
        Owner::Action(v) => actions::action(v),
        Owner::StateReturn(v) => returns::state_return(v),
        Owner::ReturnPlan(v) => returns::return_plan(v),
        Owner::ConstructorStep(v) => constructors::step(v),
        Owner::CounterAmount(v) => constructors::amount(v),
        Owner::Contract(_)
        | Owner::Alias(_)
        | Owner::Constructor(_)
        | Owner::Witness(_)
        | Owner::Ledger(_)
        | Owner::Pure(_)
        | Owner::Stateful(_)
        | Owner::Parameter(_)
        | Owner::Binding(_)
        | Owner::StructField(_)
        | Owner::SourcePoint(_) => declarations::declaration(owner),
    }
}
