// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Borrow raw-name pure signatures without checking or retyping callee bodies.
//! Existing declaration/resource/purity/call-graph owners retain their authority.
use crate::ir::{Parameter, PureCircuit, Type};
use std::collections::HashMap;

pub(crate) struct PureSignature<'a> {
    pub(crate) parameters: &'a [Parameter],
    pub(crate) result: &'a Type,
}
pub(crate) trait PureDeclarations {
    fn signature(&self, raw_source_name: &str) -> Option<PureSignature<'_>>;
}
impl PureDeclarations for HashMap<&str, &PureCircuit> {
    fn signature(&self, raw_source_name: &str) -> Option<PureSignature<'_>> {
        self.get(raw_source_name).map(|circuit| PureSignature {
            parameters: &circuit.parameters,
            result: &circuit.result,
        })
    }
}
