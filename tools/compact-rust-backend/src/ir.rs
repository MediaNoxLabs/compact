//! The interchange format from Compact's semantic lowering to Rust syntax.
//!
//! This is a Compact model, not a bag of Rust snippets. Adding a construct
//! requires a typed variant and an explicit renderer. The schema number is
//! checked before rendering so later compiler and renderer versions cannot
//! silently disagree.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub schema_version: u32,
    pub ledger_fields: Vec<LedgerField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constructor: Option<Constructor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub witnesses: Vec<WitnessDeclaration>,
    pub circuits: Vec<PureCircuit>,
    pub stateful_circuits: Vec<StatefulCircuit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Constructor {
    pub parameters: Vec<Parameter>,
    pub steps: Vec<ConstructorStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConstructorStep {
    CellWrite {
        field: String,
        index: u8,
        value: Expr,
    },
    CounterIncrement {
        field: String,
        index: u8,
        amount: CounterAmount,
    },
    CounterDecrement {
        field: String,
        index: u8,
        amount: CounterAmount,
    },
    CounterReset {
        field: String,
        index: u8,
    },
    SetInsert {
        field: String,
        index: u8,
        value: Expr,
    },
    SetRemove {
        field: String,
        index: u8,
        value: Expr,
    },
    SetReset {
        field: String,
        index: u8,
    },
    ListPushFront {
        field: String,
        index: u8,
        value: Expr,
    },
    ListPopFront {
        field: String,
        index: u8,
    },
    ListReset {
        field: String,
        index: u8,
    },
    MapInsert {
        field: String,
        index: u8,
        key: Expr,
        value: Expr,
    },
    MapInsertDefault {
        field: String,
        index: u8,
        key: Expr,
    },
    MapRemove {
        field: String,
        index: u8,
        key: Expr,
    },
    MapReset {
        field: String,
        index: u8,
    },
    ForEach {
        binding: Parameter,
        values: Vec<Expr>,
        steps: Vec<ConstructorStep>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WitnessDeclaration {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub result: Type,
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
    Set { ty: Type },
    List { ty: Type },
    Map { key: Type, value: Type },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatefulCircuit {
    pub name: String,
    #[serde(default)]
    pub internal: bool,
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
    Expression {
        value: Expr,
    },
    CellRead {
        field: String,
        index: u8,
    },
    CounterRead {
        field: String,
        index: u8,
    },
    SetMember {
        field: String,
        index: u8,
        value: Expr,
    },
    SetSize {
        field: String,
        index: u8,
    },
    SetIsEmpty {
        field: String,
        index: u8,
    },
    MapMember {
        field: String,
        index: u8,
        key: Expr,
    },
    MapLookup {
        field: String,
        index: u8,
        key: Expr,
    },
    MapSize {
        field: String,
        index: u8,
    },
    MapIsEmpty {
        field: String,
        index: u8,
    },
    ListLength {
        field: String,
        index: u8,
    },
    ListIsEmpty {
        field: String,
        index: u8,
    },
    ListHead {
        field: String,
        index: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StateAction {
    Expression {
        value: Expr,
    },
    PureCall {
        name: String,
        arguments: Vec<Expr>,
    },
    CircuitCall {
        name: String,
        arguments: Vec<Expr>,
    },
    Assert {
        condition: Expr,
        message: String,
    },
    Let {
        bindings: Vec<LocalBinding>,
        action: Box<StateAction>,
    },
    CounterIncrement {
        field: String,
        index: u8,
        amount: CounterAmount,
    },
    CounterDecrement {
        field: String,
        index: u8,
        amount: CounterAmount,
    },
    CounterReset {
        field: String,
        index: u8,
    },
    CellWrite {
        field: String,
        index: u8,
        value: Expr,
    },
    SetInsert {
        field: String,
        index: u8,
        value: Expr,
    },
    SetRemove {
        field: String,
        index: u8,
        value: Expr,
    },
    SetReset {
        field: String,
        index: u8,
    },
    ListPushFront {
        field: String,
        index: u8,
        value: Expr,
    },
    ListPopFront {
        field: String,
        index: u8,
    },
    ListReset {
        field: String,
        index: u8,
    },
    MapInsert {
        field: String,
        index: u8,
        key: Expr,
        value: Expr,
    },
    MapInsertDefault {
        field: String,
        index: u8,
        key: Expr,
    },
    MapRemove {
        field: String,
        index: u8,
        key: Expr,
    },
    MapReset {
        field: String,
        index: u8,
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
    #[serde(default)]
    pub internal: bool,
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
#[serde(deny_unknown_fields)]
pub struct LocalBinding {
    pub name: String,
    pub ty: Type,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Type {
    Unit,
    Boolean,
    Field,
    JubjubPoint,
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
    Default {
        ty: Type,
    },
    Boolean {
        value: bool,
    },
    FieldLiteral {
        value: String,
    },
    BytesLiteral {
        bytes: Vec<u8>,
    },
    UnsignedLiteral {
        value: String,
        max: String,
    },
    UnsignedCast {
        max: String,
        value: Box<Expr>,
    },
    FieldCast {
        value: Box<Expr>,
    },
    Coerce {
        value: Box<Expr>,
        ty: Type,
    },
    Parameter {
        name: String,
    },
    StructField {
        value: Box<Expr>,
        field: String,
        index: usize,
    },
    StructLiteral {
        ty: Type,
        fields: Vec<Expr>,
    },
    SetMember {
        field: String,
        index: u8,
        value: Box<Expr>,
    },
    CellRead {
        field: String,
        index: u8,
    },
    SetIsEmpty {
        field: String,
        index: u8,
    },
    MapIsEmpty {
        field: String,
        index: u8,
    },
    Tuple {
        elements: Vec<Expr>,
    },
    Vector {
        element: Type,
        elements: Vec<Expr>,
    },
    If {
        condition: Box<Expr>,
        then: Box<Expr>,
        otherwise: Box<Expr>,
    },
    Let {
        bindings: Vec<LocalBinding>,
        body: Box<Expr>,
    },
    Sequence {
        steps: Vec<Expr>,
        value: Box<Expr>,
    },
    Assert {
        condition: Box<Expr>,
        message: String,
    },
    Equal {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    NotEqual {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Compare {
        operator: ComparisonOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        name: String,
        arguments: Vec<Expr>,
    },
    TransientHash {
        value: Box<Expr>,
    },
    TransientCommit {
        value: Box<Expr>,
        opening: Box<Expr>,
    },
    PersistentHash {
        value: Box<Expr>,
    },
    Keccak256 {
        value: Box<Expr>,
    },
    PersistentCommit {
        value: Box<Expr>,
        opening: Box<Expr>,
    },
    DegradeToTransient {
        value: Box<Expr>,
    },
    UpgradeFromTransient {
        value: Box<Expr>,
    },
    HashToCurve {
        value: Box<Expr>,
    },
    JubjubPointX {
        value: Box<Expr>,
    },
    JubjubPointY {
        value: Box<Expr>,
    },
    EcAdd {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    ConstructJubjubPoint {
        x: Box<Expr>,
        y: Box<Expr>,
    },
    EcNeg {
        value: Box<Expr>,
    },
    EcMul {
        point: Box<Expr>,
        scalar: Box<Expr>,
    },
    EcMulGenerator {
        scalar: Box<Expr>,
    },
    JubjubScalarFromNative {
        value: Box<Expr>,
    },
    WitnessCall {
        name: String,
        arguments: Vec<Expr>,
    },
    Add {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Subtract {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Multiply {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnsignedAdd {
        max: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnsignedSubtract {
        max: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnsignedMultiply {
        max: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}
