// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Typed SSA values and ordered blocks. No Rust syntax or profile counters.
use super::declarations::Declarations;
use crate::ir::Type;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Value {
    pub id: usize,
    pub ty: Type,
}
/// References are local to the immutable declaration environment of one plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct LedgerId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WitnessId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CallId(pub usize);
#[derive(Clone, Copy, Debug)]
pub(super) struct CellSlot {
    pub declaration: LedgerId,
    pub index: u8,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct SetSlot {
    pub declaration: LedgerId,
    pub index: u8,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct CounterSlot {
    pub declaration: LedgerId,
    pub index: u8,
}
/// Tree identity is checked against the immutable declaration registry.
#[derive(Clone, Copy, Debug)]
pub(super) struct MerkleSlot {
    pub declaration: LedgerId,
    pub index: u8,
    pub depth: u8,
    pub historic: bool,
}
/// Small tree vocabulary; an optional position distinguishes append from indexed insert.
#[derive(Clone, Debug)]
pub(super) enum MerkleOperation {
    Insert {
        value: Value,
        position: Option<Value>,
    },
    InsertDefault {
        position: Value,
    },
    InsertHash {
        hash: Value,
        position: Option<Value>,
    },
    ResetDefault,
    ResetHistory,
    CheckRoot {
        root: Value,
    },
    IsFull,
}
#[derive(Clone, Debug)]
pub(super) enum Literal {
    Unit,
    Boolean(bool),
    Field([u8; 32]),
    Unsigned(u128),
    /// Canonical little-endian value; declared bound remains on the binding type.
    WideUnsigned([u8; 32]),
    Bytes(Vec<u8>),
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Conversion {
    Coerce,
    UnsignedCast,
}
#[derive(Clone, Debug)]
pub(super) enum Operation {
    Literal(Literal),
    EnumVariant {
        variant: String,
    },
    Default,
    StructConstruct {
        fields: Vec<Value>,
    },
    StructProject {
        value: Value,
        field: String,
        index: usize,
    },
    Tuple {
        elements: Vec<Value>,
    },
    Vector {
        elements: Vec<Value>,
    },
    /// Private intent operations delegate allocation and transcript policy to the frame.
    CreateZswapInput {
        coin: Value,
    },
    CreateZswapOutput {
        coin: Value,
        recipient: Value,
    },
    /// Ordered official VM observation of the current contract address.
    KernelSelf,
    KernelClaim {
        kind: crate::ir::KernelClaimKind,
        value: Value,
    },
    KernelMintShielded {
        domain: Value,
        amount: Value,
    },
    /// The ledger8 commitment signature qualified by this rollout is a Bytes32 pair.
    PersistentCommit {
        value: Value,
        opening: Value,
    },
    /// Canonical little-endian Field encoding, distinct from transient degradation.
    FieldToBytes32(Value),
    /// Persistent FAB hashing of independently checked fixed-byte products.
    PersistentHash(Value),
    /// Official ledger8 digest/field conversions and transient FAB operations.
    DegradeToTransient(Value),
    UpgradeFromTransient(Value),
    TransientHash(Value),
    TransientCommit {
        value: Value,
        opening: Value,
    },
    Add(Value, Value),
    /// Checked small unsigned arithmetic; operand and result bounds are independent.
    UnsignedAdd(Value, Value),
    UnsignedSubtract(Value, Value),
    UnsignedMultiply(Value, Value),
    Equal(Value, Value),
    JubjubScalarFromNative(Value),
    EcMulGenerator(Value),
    EcMul {
        point: Value,
        scalar: Value,
    },
    EcAdd {
        left: Value,
        right: Value,
    },
    Convert {
        value: Value,
        kind: Conversion,
    },
    Merkle {
        slot: MerkleSlot,
        operation: MerkleOperation,
    },
    CellRead(CellSlot),
    CellWrite {
        slot: CellSlot,
        value: Value,
    },
    /// Store a coin using the recipient-bound index allocated by the ledger context.
    CellWriteCoin {
        slot: CellSlot,
        coin: Value,
        recipient: Value,
    },
    /// Ordinary typed membership/insertion; qualified-coin allocation is separate.
    SetMember {
        slot: SetSlot,
        value: Value,
    },
    SetInsert {
        slot: SetSlot,
        value: Value,
    },
    /// Insert a coin qualified by the recipient-bound allocation in the ledger context.
    SetInsertCoin {
        slot: SetSlot,
        coin: Value,
        recipient: Value,
    },
    SetRemove {
        slot: SetSlot,
        value: Value,
    },
    SetReset {
        slot: SetSlot,
    },
    SetSize {
        slot: SetSlot,
    },
    SetIsEmpty {
        slot: SetSlot,
    },
    CounterRead {
        slot: CounterSlot,
    },
    CounterIncrement {
        slot: CounterSlot,
        amount: Value,
    },
    CounterDecrement {
        slot: CounterSlot,
        amount: Value,
    },
    CounterReset {
        slot: CounterSlot,
    },
    CounterLessThan {
        slot: CounterSlot,
        threshold: Value,
    },
    Witness {
        declaration: WitnessId,
        arguments: Vec<Value>,
    },
    /// Arguments live in the caller; only fresh formals enter the callee block.
    Call {
        declaration: CallId,
        arguments: Vec<Value>,
        parameters: Vec<Value>,
        body: Block,
    },
    /// Invoke a certified context-free function after evaluating its arguments.
    PureCall {
        declaration: CallId,
        arguments: Vec<Value>,
    },
    Assert {
        condition: Value,
        message: String,
    },
    If {
        condition: Value,
        then: Block,
        otherwise: Block,
    },
}
#[derive(Clone, Debug)]
pub(super) struct Binding {
    pub value: Value,
    pub operation: Operation,
    pub path: String,
}
#[derive(Clone, Debug)]
pub(super) struct Block {
    pub bindings: Vec<Binding>,
    pub result: Value,
}
#[derive(Clone, Debug)]
pub(super) struct Plan {
    pub parameters: Vec<Value>,
    pub body: Block,
    pub result_type: Type,
}

/// Only the validator can mint a checked plan. Access is read-only elsewhere.
#[derive(Debug)]
pub(in crate::recorded) struct CheckedPlan {
    plan: Plan,
    declarations: Declarations,
}
impl CheckedPlan {
    pub(super) fn plan(&self) -> &Plan {
        &self.plan
    }
    pub(super) fn declarations(&self) -> &Declarations {
        &self.declarations
    }
    pub(super) fn validate(
        plan: Plan,
        declarations: Declarations,
    ) -> Result<Self, super::RecordingGap> {
        super::validate::validate(&plan, &declarations)?;
        Ok(Self { plan, declarations })
    }
}
