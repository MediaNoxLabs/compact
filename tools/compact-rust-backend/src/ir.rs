//! The interchange format from Compact's semantic lowering to Rust syntax.
//!
//! This is a Compact model, not a bag of Rust snippets. Adding a construct
//! requires a typed variant and an explicit renderer. The schema number is
//! checked before rendering so later compiler and renderer versions cannot
//! silently disagree.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub schema_version: u32,
    pub circuits: Vec<PureCircuit>,
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
    Bytes { length: usize },
    Unsigned { max: String },
    Tuple { elements: Vec<Type> },
    Vector { element: Box<Type>, length: usize },
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
