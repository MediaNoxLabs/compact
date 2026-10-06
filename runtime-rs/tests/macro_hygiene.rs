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

//! Compile actual macro consumers with hostile names in the invocation scope.
//! Only project-owned derives are exercised; upstream derives have separate contracts.

#[allow(dead_code, non_camel_case_types, unused_macros)]
mod shadowed {
    use midnight_compact_runtime as rt;
    use rt::fab::{Aligned, AlignedValue, Value};
    use rt::ledger::CellValue;
    use rt::{BinaryHashRepr, FieldRepr, FromFieldRepr};

    // Deliberately occupy both type and value namespaces used by expansions.
    pub struct Result;
    pub struct Option;
    pub struct From;
    pub struct Into;
    pub struct TryFrom;
    pub struct Default;
    pub struct Some;
    pub struct None;
    pub struct Ok;
    pub struct Err;
    pub struct u128;
    pub struct usize;
    pub struct u8;
    // The witness expansion must not rely on a caller's `runtime` module alias.
    pub struct runtime;
    macro_rules! vec {
        ($($token:tt)*) => {
            compile_error!("consumer vec macro was invoked")
        };
    }

    #[derive(Debug, PartialEq, Eq, rt::CompactCellValue)]
    pub struct Vec {
        left: bool,
        right: ::core::primitive::u64,
    }

    #[derive(Debug, PartialEq, Eq, rt::CompactCellValue, rt::CompactEnum)]
    pub enum Choice {
        First,
        Second,
    }

    #[derive(Debug, PartialEq, Eq, rt::CompactMerkleTreeDigest)]
    pub struct Digest {
        field: rt::Field,
    }

    #[derive(Debug, PartialEq, Eq, rt::CompactMerklePathEntry)]
    pub struct Entry {
        sibling: Digest,
        goes_left: bool,
    }

    #[derive(Debug, PartialEq, Eq, rt::CompactMerklePath)]
    pub struct Path {
        leaf: rt::Field,
        path: rt::FixedVector<Entry, 2>,
    }

    #[rt::compact_witness_bridge]
    pub trait Witnesses<Private> {
        fn read(
            &self,
            context: Private,
            seed: ::core::primitive::u64,
        ) -> (Private, ::core::primitive::u64);
    }

    struct Infallible;
    impl Witnesses<::core::primitive::u64> for Infallible {
        fn read(
            &self,
            context: ::core::primitive::u64,
            seed: ::core::primitive::u64,
        ) -> (::core::primitive::u64, ::core::primitive::u64) {
            (context + 1, seed + 2)
        }
    }
    struct Fallible;
    impl TryWitnesses<()> for Fallible {
        fn read(
            &self,
            _: (),
            _: ::core::primitive::u64,
        ) -> ::core::result::Result<((), ::core::primitive::u64), rt::CompactError> {
            ::core::result::Result::Err(rt::CompactError::AssertionFailed("witness refused".into()))
        }
    }

    #[test]
    fn cell_record_preserves_layout_and_checked_decoding() {
        let encoded = AlignedValue::from(Vec {
            left: true,
            right: 17,
        });
        assert_eq!(encoded.value, Value::from((true, 17_u64)));
        assert_eq!(
            encoded.alignment,
            <(bool, ::core::primitive::u64)>::alignment()
        );
        assert_eq!(
            Vec::decode_cell_value(&encoded.as_slice()).unwrap(),
            Vec {
                left: true,
                right: 17
            }
        );
        assert!(Vec::decode_cell_value(&Value::from(true)).is_err());
        assert!(Vec::decode_cell_value(&Value::from((true, 17_u64, false))).is_err());
    }

    #[test]
    fn enum_preserves_default_ordinals_and_invalid_value_rejection() {
        assert_eq!(
            <Choice as ::core::default::Default>::default(),
            Choice::First
        );
        let value = Choice::Second;
        assert_eq!(value.binary_vec(), ::std::vec![1]);
        assert_eq!(value.field_vec(), ::std::vec![rt::Field::from(1_u64)]);
        assert_eq!(
            Choice::from_field_repr(&value.field_vec()),
            ::core::option::Option::Some(Choice::Second)
        );
        assert_eq!(
            Choice::from_field_repr(&[rt::Field::from(2_u64)]),
            ::core::option::Option::None
        );
        assert_eq!(
            Choice::decode_cell_value(&Value::from(1_u128)).unwrap(),
            Choice::Second
        );
        assert!(Choice::decode_cell_value(&Value::from(2_u128)).is_err());
    }

    fn ledger_path(depth: ::core::primitive::usize) -> rt::ledger::MerklePath<rt::Field> {
        rt::ledger::MerklePath {
            leaf: rt::Field::from(3_u64),
            path: (0..depth)
                .map(|i| rt::ledger::MerklePathEntry {
                    sibling: rt::ledger::MerkleTreeDigest(rt::Field::from(
                        i as ::core::primitive::u64,
                    )),
                    goes_left: i == 0,
                })
                .collect(),
        }
    }

    #[test]
    fn merkle_conversions_preserve_entries_and_refuse_wrong_depth() {
        let source = ledger_path(2);
        let converted = Path::from_ledger_path(source.clone()).unwrap();
        assert_eq!(converted.leaf, source.leaf);
        let round_trip = converted.into_ledger_path();
        assert_eq!(round_trip.leaf, source.leaf);
        assert_eq!(round_trip.path.len(), source.path.len());
        for (actual, expected) in round_trip.path.iter().zip(&source.path) {
            assert_eq!(actual.sibling, expected.sibling);
            assert_eq!(actual.goes_left, expected.goes_left);
        }
        assert!(Path::from_ledger_path(ledger_path(1)).is_err());
        assert!(Path::from_ledger_path(ledger_path(3)).is_err());
    }

    #[test]
    fn witness_bridge_preserves_outputs_and_fallible_error() {
        assert_eq!(TryWitnesses::read(&Infallible, 5, 7).unwrap(), (6, 9));
        assert_eq!(
            TryWitnesses::read(&Fallible, (), 7),
            ::core::result::Result::Err(rt::CompactError::AssertionFailed(
                "witness refused".into()
            ))
        );
    }
}
