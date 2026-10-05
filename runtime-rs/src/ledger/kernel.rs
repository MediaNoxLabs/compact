// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Kernel effects use their own ledger VM frame, never a contract ledger slot.
use super::{
    CoinCommitment, CoinNullifier, CompactError, HashOutput, QueryContext, QueryResults,
    StateValue, constructor_cell,
};
use midnight_base_crypto::{cost_model::RunningCost, fab::AlignedValue};
use midnight_onchain_vm::{
    cost_model::CostModel,
    ops::{Key, Op},
    result_mode::{ResultMode, ResultModeVerify},
};
use midnight_storage::db::DB;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelClaim {
    Nullifier(CoinNullifier),
    CoinSpend(CoinCommitment),
    CoinReceive(CoinCommitment),
}

pub fn query_kernel_claim<D: DB>(
    context: &QueryContext<D>,
    claim: KernelClaim,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, CompactError> {
    context
        .query(&kernel_claim_program(claim), gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))
}
pub fn query_kernel_mint_shielded<D: DB>(
    context: &QueryContext<D>,
    domain: HashOutput,
    amount: u64,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, CompactError> {
    context
        .query(
            &kernel_mint_shielded_program(domain, amount),
            gas_limit,
            cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))
}

pub(crate) fn kernel_claim_program<M: ResultMode<D>, D: DB>(claim: KernelClaim) -> Vec<Op<M, D>> {
    let (index, value) = match claim {
        KernelClaim::Nullifier(value) => (0_u8, AlignedValue::from(value)),
        KernelClaim::CoinSpend(value) => (2, AlignedValue::from(value)),
        KernelClaim::CoinReceive(value) => (1, AlignedValue::from(value)),
    };
    vec![
        Op::Swap { n: 0 },
        Op::Idx {
            cached: true,
            push_path: true,
            path: vec![Key::Value(AlignedValue::from(index))].into(),
        },
        Op::Push {
            storage: false,
            value: StateValue::from(value),
        },
        Op::Push {
            storage: false,
            value: StateValue::Null,
        },
        Op::Ins { cached: true, n: 2 },
        Op::Swap { n: 0 },
    ]
}
pub(crate) fn kernel_mint_shielded_program<M: ResultMode<D>, D: DB>(
    domain: HashOutput,
    amount: u64,
) -> Vec<Op<M, D>> {
    vec![
        Op::Swap { n: 0 },
        Op::Idx {
            cached: true,
            push_path: true,
            path: vec![Key::Value(AlignedValue::from(4_u8))].into(),
        },
        Op::Push {
            storage: false,
            value: StateValue::from(AlignedValue::from(domain)),
        },
        Op::Dup { n: 1 },
        Op::Dup { n: 1 },
        Op::Member,
        Op::Push {
            storage: false,
            value: constructor_cell(amount),
        },
        Op::Swap { n: 0 },
        Op::Neg,
        Op::Branch { skip: 4 },
        Op::Dup { n: 2 },
        Op::Dup { n: 2 },
        Op::Idx {
            cached: true,
            push_path: false,
            path: vec![Key::Stack].into(),
        },
        Op::Add,
        Op::Ins { cached: true, n: 2 },
        Op::Swap { n: 0 },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{ChargedState, ContractAddress, DefaultDB};
    #[test]
    fn canonical_kernel_programs_match_independent_typescript() {
        let rows: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/kernel-shielded-effects-oracle.json"
        ))
        .unwrap();
        for row in rows.as_array().unwrap().iter().filter(|r| {
            ["zero", "seven", "max", "nullifier", "spend", "receive"]
                .contains(&r["label"].as_str().unwrap())
        }) {
            let bytes: [u8; 32] = serde_json::from_value(row["args"][0].clone()).unwrap();
            let value = HashOutput(bytes);
            let program = match row["name"].as_str().unwrap() {
                "mint" => kernel_mint_shielded_program::<ResultModeVerify, DefaultDB>(
                    value,
                    row["args"][1].as_str().unwrap().parse().unwrap(),
                ),
                "nullifier" => kernel_claim_program(KernelClaim::Nullifier(CoinNullifier(value))),
                "spend" => kernel_claim_program(KernelClaim::CoinSpend(CoinCommitment(value))),
                "claim_receive" => {
                    kernel_claim_program(KernelClaim::CoinReceive(CoinCommitment(value)))
                }
                _ => unreachable!(),
            };
            assert_eq!(
                serde_json::to_value(program).unwrap(),
                row["queries"][0]["ops"],
                "{}",
                row["label"]
            );
        }
    }
    #[test]
    fn mint_overflow_rejects_the_second_query_without_replacing_prior_effects() {
        let query = QueryContext::new(
            ChargedState::new(StateValue::<DefaultDB>::Array(
                vec![constructor_cell(false)].into(),
            )),
            ContractAddress::default(),
        );
        let domain = HashOutput([1; 32]);
        let model = &midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;
        let first = query_kernel_mint_shielded(&query, domain, u64::MAX, None, model).unwrap();
        let error = query_kernel_mint_shielded(&first.context, domain, 1, None, model)
            .err()
            .unwrap();
        assert!(matches!(error, CompactError::LedgerQueryRejected(_)));
        assert!(error.to_string().to_lowercase().contains("overflow"));
        assert_eq!(
            *first.context.effects.shielded_mints.get(&domain).unwrap(),
            u64::MAX
        );
        assert_eq!(first.context.state, query.state);
        assert!(query.effects.shielded_mints.is_empty());
    }

    #[test]
    fn upstream_query_rejects_a_malformed_effects_frame() {
        let query = QueryContext::new(
            ChargedState::new(StateValue::<DefaultDB>::Array(
                vec![constructor_cell(false)].into(),
            )),
            ContractAddress::default(),
        );
        let corrupt_effects: Vec<Op<ResultModeVerify, DefaultDB>> = vec![
            Op::Swap { n: 0 },
            Op::Pop,
            Op::Push {
                storage: false,
                value: constructor_cell(false),
            },
            Op::Swap { n: 0 },
        ];
        assert!(matches!(
            query.query(
                &corrupt_effects,
                None,
                &midnight_onchain_vm::cost_model::INITIAL_COST_MODEL
            ),
            Err(super::super::TranscriptRejected::EffectDecodeError)
        ));
    }
}
