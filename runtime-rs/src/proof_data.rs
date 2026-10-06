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

use crate::{
    AlignedValue, CompactError, ContractAddress, DefaultDB, Op, QueryContext, ResultModeGather,
    ResultModeVerify, DB,
};
use std::slice;

/// Proof data accumulated while a generated Compact circuit executes.
///
/// This is the Rust equivalent of the TypeScript runtime's minimum
/// `PartialProofData`: circuit input, public ledger transcript, and private
/// witness transcript outputs. Private witness outputs are deliberately wrapped
/// in [`PrivateTranscriptOutputs`], which does not implement `Debug` or serde
/// traits, so secrets are not accidentally logged or persisted through common
/// derives.
#[derive(Clone, PartialEq, Eq)]
pub struct PartialProofData<D: DB = DefaultDB> {
    pub input: AlignedValue,
    pub public_transcript: Vec<Op<ResultModeVerify, D>>,
    private_transcript_outputs: PrivateTranscriptOutputs,
}

impl<D: DB> PartialProofData<D> {
    pub fn new(input: AlignedValue) -> Self {
        Self {
            input,
            public_transcript: Vec::new(),
            private_transcript_outputs: PrivateTranscriptOutputs::default(),
        }
    }

    pub fn push_public_ops(&mut self, ops: impl IntoIterator<Item = Op<ResultModeVerify, D>>) {
        self.public_transcript.extend(ops);
    }

    pub fn push_private_output(&mut self, output: AlignedValue) {
        self.private_transcript_outputs.push(output);
    }

    pub fn private_transcript_outputs(&self) -> &PrivateTranscriptOutputs {
        &self.private_transcript_outputs
    }

    pub fn finalize(self, output: AlignedValue) -> ProofData<D> {
        ProofData {
            input: self.input,
            public_transcript: self.public_transcript,
            private_transcript_outputs: self.private_transcript_outputs,
            output,
        }
    }
}

/// Complete per-circuit proof data: [`PartialProofData`] plus primary output.
#[derive(Clone, PartialEq, Eq)]
pub struct ProofData<D: DB = DefaultDB> {
    pub input: AlignedValue,
    pub public_transcript: Vec<Op<ResultModeVerify, D>>,
    private_transcript_outputs: PrivateTranscriptOutputs,
    pub output: AlignedValue,
}

impl<D: DB> ProofData<D> {
    pub fn private_transcript_outputs(&self) -> &PrivateTranscriptOutputs {
        &self.private_transcript_outputs
    }

    pub fn into_parts(
        self,
    ) -> (
        AlignedValue,
        Vec<Op<ResultModeVerify, D>>,
        PrivateTranscriptOutputs,
        AlignedValue,
    ) {
        (
            self.input,
            self.public_transcript,
            self.private_transcript_outputs,
            self.output,
        )
    }
}

/// Private witness outputs in Compact order.
///
/// This type intentionally does not implement `Debug`, `Serialize`, or
/// `Deserialize`. Use the explicit accessors when handing the values to a
/// prover/ledger adapter.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct PrivateTranscriptOutputs {
    outputs: Vec<AlignedValue>,
}

impl PrivateTranscriptOutputs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, output: AlignedValue) {
        self.outputs.push(output);
    }

    pub fn len(&self) -> usize {
        self.outputs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }

    pub fn as_slice(&self) -> &[AlignedValue] {
        &self.outputs
    }

    pub fn into_vec(self) -> Vec<AlignedValue> {
        self.outputs
    }
}

impl<'a> IntoIterator for &'a PrivateTranscriptOutputs {
    type Item = &'a AlignedValue;
    type IntoIter = slice::Iter<'a, AlignedValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.outputs.iter()
    }
}

/// One finalized Compact circuit call, with enough metadata for a downstream
/// Ledger8 adapter to build pre-transcripts and contract-call partitions.
#[derive(Clone)]
pub struct CallProofData<D: DB = DefaultDB> {
    pub circuit_id: String,
    pub contract_address: ContractAddress,
    pub initial_query_context: QueryContext<D>,
    pub final_query_context: QueryContext<D>,
    pub proof_data: ProofData<D>,
}

impl<D: DB> CallProofData<D> {
    pub fn new(
        circuit_id: impl Into<String>,
        contract_address: ContractAddress,
        initial_query_context: QueryContext<D>,
        final_query_context: QueryContext<D>,
        proof_data: ProofData<D>,
    ) -> Self {
        Self {
            circuit_id: circuit_id.into(),
            contract_address,
            initial_query_context,
            final_query_context,
            proof_data,
        }
    }
}

/// Ordered proof-data trace for calls made by a context.
#[derive(Clone, Default)]
pub struct CallProofDataTrace<D: DB = DefaultDB> {
    calls: Vec<CallProofData<D>>,
}

impl<D: DB> CallProofDataTrace<D> {
    pub fn new() -> Self {
        Self { calls: Vec::new() }
    }

    pub fn push(&mut self, call: CallProofData<D>) {
        self.calls.push(call);
    }

    pub fn len(&self) -> usize {
        self.calls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    pub fn as_slice(&self) -> &[CallProofData<D>] {
        &self.calls
    }

    pub fn into_vec(self) -> Vec<CallProofData<D>> {
        self.calls
    }
}

/// Stable, ledger-crate-free extraction result for the supported production
/// bridge slice: exactly one root Compact call in one contract.
pub struct SingleCallProof<'a, D: DB = DefaultDB> {
    pub circuit_id: &'a str,
    pub contract_address: &'a ContractAddress,
    pub initial_query_context: &'a QueryContext<D>,
    pub final_query_context: &'a QueryContext<D>,
    pub input: &'a AlignedValue,
    pub public_transcript: &'a [Op<ResultModeVerify, D>],
    pub private_transcript_outputs: &'a PrivateTranscriptOutputs,
    pub output: &'a AlignedValue,
}

impl<D: DB> CallProofDataTrace<D> {
    pub fn single_contract_call(&self) -> Result<SingleCallProof<'_, D>, CompactError> {
        match self.calls.as_slice() {
            [call] => Ok(SingleCallProof {
                circuit_id: &call.circuit_id,
                contract_address: &call.contract_address,
                initial_query_context: &call.initial_query_context,
                final_query_context: &call.final_query_context,
                input: &call.proof_data.input,
                public_transcript: &call.proof_data.public_transcript,
                private_transcript_outputs: call.proof_data.private_transcript_outputs(),
                output: &call.proof_data.output,
            }),
            [] => Err(CompactError::ProofData(
                "no Compact call proof data was recorded".into(),
            )),
            _ => Err(CompactError::ProofData(
                "cross-contract or multi-call proof traces are not supported by this extraction API".into(),
            )),
        }
    }
}

/// Concatenate already-aligned values in Compact argument/result order.
pub fn aligned_value_from_parts(parts: &[AlignedValue]) -> AlignedValue {
    AlignedValue::concat(parts.iter())
}

/// Exact proof-data encoding for values that already expose their Compact
/// alignment/type framing through the runtime `AlignedValue` conversion.
pub fn proof_aligned_value<T>(value: &T) -> AlignedValue
where
    T: Clone + crate::DynAligned,
    crate::Value: From<T>,
{
    AlignedValue::from(value.clone())
}

/// Exact Compact `Vector<N, T>` proof-data encoding. Compact vectors are the
/// concatenation of each element's aligned value, preserving each element's
/// alignment atom instead of flattening through field representation.
pub fn proof_aligned_array<T, const N: usize>(value: &[T; N]) -> AlignedValue
where
    T: Clone + crate::DynAligned,
    crate::Value: From<T>,
{
    let parts: Vec<AlignedValue> = value.iter().map(proof_aligned_value).collect();
    aligned_value_from_parts(&parts)
}

/// Exact Compact Merkle path encoding for the runtime MerklePath shape. The
/// path vector is emitted in order as the Compact `MerkleTreePath` entries
/// carried by the witness value.
pub fn proof_aligned_merkle_path<T>(path: &crate::MerklePath<T>) -> AlignedValue
where
    T: Clone + crate::DynAligned,
    crate::Value: From<T>,
{
    let mut parts = Vec::with_capacity(1 + path.path.len());
    parts.push(proof_aligned_value(&path.leaf));
    for entry in &path.path {
        parts.push(aligned_value_from_parts(&[
            proof_aligned_value::<crate::Fr>(&entry.sibling.0),
            proof_aligned_value::<bool>(&entry.goes_left),
        ]));
    }
    aligned_value_from_parts(&parts)
}

pub fn proof_aligned_maybe_merkle_path<T>(
    value: &crate::std_lib::Maybe<crate::MerklePath<T>>,
) -> AlignedValue
where
    T: Clone + crate::DynAligned,
    crate::Value: From<T>,
{
    aligned_value_from_parts(&[
        proof_aligned_value::<bool>(&value.is_some),
        proof_aligned_merkle_path(&value.value),
    ])
}

pub fn gather_ops_to_public_transcript<D: DB>(
    ops: &[Op<ResultModeGather, D>],
    reads: impl IntoIterator<Item = AlignedValue>,
) -> Result<Vec<Op<ResultModeVerify, D>>, CompactError> {
    let mut reads = reads.into_iter();
    let mut out = Vec::with_capacity(ops.len());
    for op in ops {
        out.push(match op {
            Op::Noop { n } => Op::Noop { n: *n },
            Op::Lt => Op::Lt,
            Op::Eq => Op::Eq,
            Op::Type => Op::Type,
            Op::Size => Op::Size,
            Op::New => Op::New,
            Op::And => Op::And,
            Op::Or => Op::Or,
            Op::Neg => Op::Neg,
            Op::Log => Op::Log,
            Op::Root => Op::Root,
            Op::Pop => Op::Pop,
            Op::Popeq { cached, result: () } => Op::Popeq {
                cached: *cached,
                result: reads.next().ok_or_else(|| {
                    CompactError::ProofData("popeq transcript is missing a read result".into())
                })?,
            },
            Op::Addi { immediate } => Op::Addi {
                immediate: *immediate,
            },
            Op::Subi { immediate } => Op::Subi {
                immediate: *immediate,
            },
            Op::Push { storage, value } => Op::Push {
                storage: *storage,
                value: value.clone(),
            },
            Op::Branch { skip } => Op::Branch { skip: *skip },
            Op::Jmp { skip } => Op::Jmp { skip: *skip },
            Op::Add => Op::Add,
            Op::Sub => Op::Sub,
            Op::Concat { cached, n } => Op::Concat {
                cached: *cached,
                n: *n,
            },
            Op::Member => Op::Member,
            Op::Rem { cached } => Op::Rem { cached: *cached },
            Op::Dup { n } => Op::Dup { n: *n },
            Op::Swap { n } => Op::Swap { n: *n },
            Op::Idx {
                cached,
                push_path,
                path,
            } => Op::Idx {
                cached: *cached,
                push_path: *push_path,
                path: path.clone(),
            },
            Op::Ins { cached, n } => Op::Ins {
                cached: *cached,
                n: *n,
            },
            Op::Ckpt => Op::Ckpt,
            _ => {
                return Err(CompactError::ProofData(
                    "unsupported non-exhaustive ledger op in public transcript".into(),
                ))
            }
        });
    }
    if reads.next().is_some() {
        return Err(CompactError::ProofData(
            "gather transcript produced more read results than popeq operations".into(),
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{new_cell, OpProgramGather};

    #[test]
    fn vector_proof_encoding_preserves_element_alignment() {
        let compact_vector = proof_aligned_array(&[1u8, 2u8]);
        let byte_array = AlignedValue::from([1u8, 2u8]);

        assert_eq!(compact_vector.value.0.len(), 2);
        assert_eq!(compact_vector.alignment.0.len(), 2);
        assert_eq!(byte_array.value.0.len(), 1);
        assert_ne!(compact_vector.alignment, byte_array.alignment);
    }

    #[test]
    fn gather_transcript_preserves_order_and_fills_reads() {
        let ops = OpProgramGather::<DefaultDB>::new()
            .dup(0)
            .idx_at_index(7, false)
            .popeq(true)
            .build();
        let read = AlignedValue::from(42u64);
        let public = gather_ops_to_public_transcript(&ops, [read.clone()]).unwrap();
        assert!(matches!(public[0], Op::Dup { n: 0 }));
        assert!(matches!(public[1], Op::Idx { .. }));
        assert!(matches!(
            &public[2],
            Op::Popeq {
                cached: true,
                result
            } if result == &read
        ));
    }

    #[test]
    fn private_outputs_are_owned_and_ordered() {
        let mut pd = PartialProofData::<DefaultDB>::new(aligned_value_from_parts(&[]));
        pd.push_private_output(AlignedValue::from(1u8));
        pd.push_private_output(AlignedValue::from(2u8));
        let out = pd.private_transcript_outputs().as_slice();
        assert_eq!(out, &[AlignedValue::from(1u8), AlignedValue::from(2u8)]);
    }

    #[test]
    fn single_call_rejects_empty_and_multi_call_traces() {
        let trace = CallProofDataTrace::<DefaultDB>::new();
        assert!(matches!(
            trace.single_contract_call(),
            Err(CompactError::ProofData(_))
        ));

        let qctx = QueryContext::new(
            crate::ChargedState::new(new_cell(0u8)),
            ContractAddress::default(),
        );
        let call = CallProofData::new(
            "circuit",
            ContractAddress::default(),
            qctx.clone(),
            qctx,
            PartialProofData::<DefaultDB>::new(aligned_value_from_parts(&[]))
                .finalize(aligned_value_from_parts(&[])),
        );
        let mut trace = CallProofDataTrace::new();
        trace.push(call.clone());
        trace.push(call);
        assert!(matches!(
            trace.single_contract_call(),
            Err(CompactError::ProofData(_))
        ));
    }

    #[test]
    fn private_outputs_do_not_derive_debug_or_serde() {
        let source = include_str!("proof_data.rs");
        let header = source
            .split("pub struct PrivateTranscriptOutputs")
            .next()
            .expect("private transcript type present");
        let derive_line = header
            .lines()
            .rev()
            .find(|line| line.trim_start().starts_with("#[derive"))
            .expect("derive line before private transcript outputs");
        assert!(!derive_line.contains("Debug"));
        assert!(!derive_line.contains("Serialize"));
        assert!(!derive_line.contains("Deserialize"));
        assert!(!source.contains(concat!(
            "impl std::fmt::Debug for ",
            "PrivateTranscriptOutputs"
        )));
        assert!(!source.contains(concat!(
            "impl core::fmt::Debug for ",
            "PrivateTranscriptOutputs"
        )));
        assert!(!source.contains(concat!(
            "impl serde::Serialize for ",
            "PrivateTranscriptOutputs"
        )));
        assert!(!source.contains(concat!(
            "impl serde::Deserialize for ",
            "PrivateTranscriptOutputs"
        )));
    }
}
