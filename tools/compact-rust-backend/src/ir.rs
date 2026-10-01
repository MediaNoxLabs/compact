// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The interchange format from Compact's semantic lowering to Rust syntax.
//!
//! This is a Compact model, not a bag of Rust snippets. Adding a construct
//! requires a typed variant and an explicit renderer. The schema number is
//! checked before rendering so later compiler and renderer versions cannot
//! silently disagree.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 6;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub type_aliases: Vec<TypeAlias>,
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
pub struct TypeAlias {
    pub name: String,
    pub ty: Type,
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
    Expression {
        value: Expr,
    },
    Let {
        bindings: Vec<LocalBinding>,
        step: Box<ConstructorStep>,
    },
    Sequence {
        steps: Vec<ConstructorStep>,
    },
    Assert {
        condition: Expr,
        message: String,
    },
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
    /// Full physical path when the compiler chunks more than 15 fields.
    /// Empty for the legacy single-segment `index` representation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<u8>,
    pub declaration: LedgerFieldKind,
}

impl LedgerField {
    pub fn physical_path(&self) -> Vec<u8> {
        if self.path.is_empty() {
            vec![self.index]
        } else {
            self.path.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LedgerFieldKind {
    Counter,
    Cell { ty: Type },
    Set { ty: Type },
    List { ty: Type },
    Map { key: Type, value: Type },
    MerkleTree { depth: u8, ty: Type },
    HistoricMerkleTree { depth: u8, ty: Type },
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
    HistoricMerkleIsFull {
        field: String,
        index: u8,
    },
    HistoricMerkleCheckRoot {
        field: String,
        index: u8,
        root: Expr,
    },
    MerkleIsFull {
        field: String,
        index: u8,
    },
    MerkleCheckRoot {
        field: String,
        index: u8,
        root: Expr,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StateAction {
    Sequence {
        actions: Vec<StateAction>,
    },
    If {
        condition: Expr,
        then: Box<StateAction>,
        otherwise: Box<StateAction>,
    },
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
    HistoricMerkleInsertIndexDefault {
        field: String,
        index: u8,
        position: Expr,
    },
    HistoricMerkleInsert {
        field: String,
        index: u8,
        value: Expr,
    },
    HistoricMerkleInsertIndex {
        field: String,
        index: u8,
        value: Expr,
        position: Expr,
    },
    HistoricMerkleInsertHash {
        field: String,
        index: u8,
        hash: Expr,
    },
    HistoricMerkleInsertHashIndex {
        field: String,
        index: u8,
        hash: Expr,
        position: Expr,
    },
    HistoricMerkleResetHistory {
        field: String,
        index: u8,
    },
    HistoricMerkleResetToDefault {
        field: String,
        index: u8,
    },
    MerkleInsertIndexDefault {
        field: String,
        index: u8,
        position: Expr,
    },
    MerkleInsert {
        field: String,
        index: u8,
        value: Expr,
    },
    MerkleInsertIndex {
        field: String,
        index: u8,
        value: Expr,
        position: Expr,
    },
    MerkleInsertHash {
        field: String,
        index: u8,
        hash: Expr,
    },
    MerkleInsertHashIndex {
        field: String,
        index: u8,
        hash: Expr,
        position: Expr,
    },
    MerkleResetToDefault {
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
    OpaqueString,
    OpaqueBytes,
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
    /// A ledger Map value nested inside another collection, rather than a
    /// Compact value that can be encoded in a Cell.
    LedgerMap {
        key: Box<Type>,
        value: Box<Type>,
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
    EnumVariant {
        ty: Type,
        variant: String,
    },
    StructField {
        value: Box<Expr>,
        field: String,
        index: usize,
    },
    TupleIndex {
        value: Box<Expr>,
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
    MapMember {
        field: String,
        index: u8,
        key: Box<Expr>,
    },
    MapLookup {
        field: String,
        index: u8,
        key: Box<Expr>,
    },
    MerkleCheckRoot {
        field: String,
        index: u8,
        root: Box<Expr>,
    },
    HistoricMerkleCheckRoot {
        field: String,
        index: u8,
        root: Box<Expr>,
    },
    CellRead {
        field: String,
        index: u8,
    },
    KernelSelf {
        ty: Type,
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
    VectorMap {
        parameter: Parameter,
        source: Box<Expr>,
        body: Box<Expr>,
        result: Type,
        length: usize,
    },
    VectorFoldCall {
        name: String,
        initial: Box<Expr>,
        source: Box<Expr>,
        accumulator: Type,
        element: Type,
        length: usize,
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
