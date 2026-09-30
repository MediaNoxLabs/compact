//! The interchange format from Compact's semantic lowering to Rust syntax.
//!
//! This is a Compact model, not a bag of Rust snippets. Adding a construct
//! requires a typed variant and an explicit renderer. The schema number is
//! checked before rendering so later compiler and renderer versions cannot
//! silently disagree.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub schema_version: u32,
    pub ledger_fields: Vec<LedgerField>,
    pub circuits: Vec<PureCircuit>,
    pub stateful_circuits: Vec<StatefulCircuit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerField {
    /// Frontend identity, distinct from any Rust identifier.
    pub id: String,
    pub index: u8,
    pub declaration: LedgerFieldKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LedgerFieldKind {
    Counter,
    Cell { ty: Type },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatefulCircuit {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
    pub actions: Vec<StateAction>,
    #[serde(default)]
    pub result: Type,
    #[serde(default)]
    pub return_value: StateReturn,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StateReturn {
    #[default]
    Unit,
    CellRead {
        field: String,
        index: u8,
    },
    CounterRead {
        field: String,
        index: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StateAction {
    CounterIncrement {
        field: String,
        index: u8,
        amount: CounterAmount,
    },
    CellWrite {
        field: String,
        index: u8,
        value: Expr,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CounterAmount {
    Literal { value: u16 },
    Parameter { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PureCircuit {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub result: Type,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Type {
    Unit,
    Boolean,
    Field,
    Bytes {
        length: usize,
    },
    Struct {
        name: String,
        fields: Vec<StructField>,
    },
    Enum {
        name: String,
        variants: Vec<String>,
    },
    Unsigned {
        max: String,
    },
    Tuple {
        elements: Vec<Type>,
    },
    Vector {
        element: Box<Type>,
        length: usize,
    },
}

impl Default for Type {
    fn default() -> Self {
        Self::Unit
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Expr {
    Unit,
    Boolean { value: bool },
    Parameter { name: String },
    Tuple { elements: Vec<Expr> },
    Add { left: Box<Expr>, right: Box<Expr> },
}
