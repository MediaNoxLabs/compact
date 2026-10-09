// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Typed SSA values and ordered blocks. No Rust syntax or profile counters.
use crate::ir::Type;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Value {
    pub id: usize,
    pub ty: Type,
}
#[derive(Clone, Debug)]
pub(super) struct Slot {
    pub name: String,
    pub ty: Type,
}
#[derive(Clone, Debug)]
pub(super) enum Literal {
    Unit,
    Boolean(bool),
    Field([u8; 32]),
    Unsigned(u128),
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
    CellRead(Slot),
    CellWrite {
        slot: Slot,
        value: Value,
    },
    CounterRead {
        field: String,
    },
    CounterIncrement {
        field: String,
        amount: Value,
    },
    Witness {
        name: String,
        parameters: Vec<Type>,
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
pub(in crate::recorded) struct CheckedPlan(Plan);
impl CheckedPlan {
    pub(super) fn plan(&self) -> &Plan {
        &self.0
    }
    pub(super) fn validate(plan: Plan) -> Result<Self, super::RecordingGap> {
        super::validate::validate(&plan)?;
        Ok(Self(plan))
    }
}
