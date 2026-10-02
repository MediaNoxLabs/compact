// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");

//! Typed names for compiler-declared ledger fields.
//!
//! Generated contracts expose one constant per supported ledger declaration. The runtime
//! still owns VM execution; these descriptors only tie a value type to its
//! physical ledger path. Their constructors are public to support generated
//! crates and are not an access-control boundary.

use std::marker::PhantomData;

use crate::CompactError;
use crate::context::{CircuitContext, CircuitResult};
use crate::ledger::{CellValue, DB};
use crate::recording::RecordingFrame;

#[derive(Clone, Copy)]
pub struct CellSlot<T> {
    path: &'static [u8],
    value: PhantomData<fn() -> T>,
}

impl<T: CellValue> CellSlot<T> {
    pub const fn new(path: &'static [u8]) -> Self {
        Self {
            path,
            value: PhantomData,
        }
    }

    pub const fn path(self) -> &'static [u8] {
        self.path
    }

    pub fn read<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, T, D>, CompactError> {
        context.read_cell_at_path(self.path)
    }

    pub fn write<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.write_cell_at_path(self.path, value)
    }

    pub fn record_read<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, T), CompactError> {
        frame.read_cell(self.path)
    }

    pub fn record_write<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        value: T,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.write_cell(self.path, value)
    }
}

#[derive(Clone, Copy)]
pub struct CounterSlot {
    path: &'static [u8],
}

impl CounterSlot {
    pub const fn new(path: &'static [u8]) -> Self {
        Self { path }
    }

    pub const fn path(self) -> &'static [u8] {
        self.path
    }

    pub fn read<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, u64, D>, CompactError> {
        context.read_cell_at_path(self.path)
    }

    pub fn increment<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        amount: u16,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.increment_counter(self.path, amount)
    }

    pub fn decrement<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        amount: u16,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.decrement_counter(self.path, amount)
    }

    pub fn reset<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.write_cell_at_path(self.path, 0_u64)
    }

    pub fn record_read<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, u64), CompactError> {
        frame.read_cell(self.path)
    }

    pub fn record_increment<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        amount: u16,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.increment_counter(self.path, amount)
    }

    pub fn record_decrement<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        amount: u16,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.decrement_counter(self.path, amount)
    }
}

/// A compiler-declared Set with a fixed element type and physical path.
#[derive(Clone, Copy)]
pub struct SetSlot<T> {
    path: &'static [u8],
    element: PhantomData<fn() -> T>,
}

impl<T: CellValue> SetSlot<T> {
    pub const fn new(path: &'static [u8]) -> Self {
        Self {
            path,
            element: PhantomData,
        }
    }

    pub const fn path(self) -> &'static [u8] {
        self.path
    }

    pub fn insert<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.insert_set(self.path, value)
    }

    pub fn remove<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.remove_set(self.path, value)
    }

    pub fn member<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        value: T,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        context.member_set(self.path, value)
    }

    pub fn reset<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.reset_set(self.path)
    }

    pub fn size<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, u64, D>, CompactError> {
        context.size_set(self.path)
    }

    pub fn is_empty<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        context.is_empty_set(self.path)
    }

    pub fn record_insert<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        value: T,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.insert_set(self.path, value)
    }

    pub fn record_remove<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        value: T,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.remove_set(self.path, value)
    }

    pub fn record_member<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        value: T,
    ) -> Result<(RecordingFrame<Private, D>, bool), CompactError>
    where
        T: Clone,
    {
        frame.member_set(self.path, value)
    }

    pub fn record_reset<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.reset_set(self.path)
    }

    pub fn record_size<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, u64), CompactError> {
        frame.size_set(self.path)
    }

    pub fn record_is_empty<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, bool), CompactError> {
        frame.is_empty_set(self.path)
    }
}

/// A compiler-declared Map with fixed key/value types and a physical path.
#[derive(Clone, Copy)]
pub struct MapSlot<K, V> {
    path: &'static [u8],
    types: PhantomData<fn() -> (K, V)>,
}

impl<K: CellValue, V: CellValue> MapSlot<K, V> {
    pub const fn new(path: &'static [u8]) -> Self {
        Self {
            path,
            types: PhantomData,
        }
    }

    pub const fn path(self) -> &'static [u8] {
        self.path
    }

    pub fn insert<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        key: K,
        value: V,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.insert_map(self.path, key, value)
    }

    pub fn insert_default<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        key: K,
    ) -> Result<CircuitResult<Private, (), D>, CompactError>
    where
        V: Default,
    {
        self.insert(context, key, V::default())
    }

    pub fn remove<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        key: K,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.remove_map(self.path, key)
    }

    pub fn reset<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.reset_map(self.path)
    }

    pub fn member<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        key: K,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        context.member_map(self.path, key)
    }

    pub fn lookup<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        key: K,
    ) -> Result<CircuitResult<Private, V, D>, CompactError> {
        context.lookup_map(self.path, key)
    }

    pub fn size<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, u64, D>, CompactError> {
        context.size_map(self.path)
    }

    pub fn is_empty<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        context.is_empty_map(self.path)
    }

    pub fn record_insert<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        key: K,
        value: V,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.insert_map(self.path, key, value)
    }

    pub fn record_insert_default<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        key: K,
    ) -> Result<RecordingFrame<Private, D>, CompactError>
    where
        V: Default,
    {
        self.record_insert(frame, key, V::default())
    }

    pub fn record_remove<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        key: K,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.remove_map(self.path, key)
    }

    pub fn record_reset<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.reset_map(self.path)
    }

    pub fn record_member<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        key: K,
    ) -> Result<(RecordingFrame<Private, D>, bool), CompactError>
    where
        K: Clone,
    {
        frame.member_map(self.path, key)
    }

    pub fn record_lookup<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        key: K,
    ) -> Result<(RecordingFrame<Private, D>, V), CompactError>
    where
        K: Clone,
    {
        frame.lookup_map(self.path, key)
    }

    pub fn record_size<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, u64), CompactError> {
        frame.size_map(self.path)
    }

    pub fn record_is_empty<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, bool), CompactError> {
        frame.is_empty_map(self.path)
    }
}
