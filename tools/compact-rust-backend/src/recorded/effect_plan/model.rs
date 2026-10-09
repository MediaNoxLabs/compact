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
#[derive(Clone, Copy, Debug)]
pub(super) struct CellSlot {
    pub declaration: LedgerId,
    pub index: u8,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct CounterSlot {
    pub declaration: LedgerId,
    pub index: u8,
}
#[derive(Clone, Debug)]
pub(super) enum Literal {
    Unit,
    Boolean(bool),
    Field([u8; 32]),
    Unsigned(u128),
    /// Canonical little-endian value; declared bound remains on the binding type.
    WideUnsigned([u8; 32]),
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Conversion {
    Coerce,
    UnsignedCast,
}
#[derive(Clone, Debug)]
pub(super) enum Operation {
    Literal(Literal),
    Add(Value, Value),
    UnsignedAdd(Value, Value),
    Equal(Value, Value),
    Convert {
        value: Value,
        kind: Conversion,
    },
    CellRead(CellSlot),
    CellWrite {
        slot: CellSlot,
        value: Value,
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
