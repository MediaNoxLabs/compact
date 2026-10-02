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

use midnight_base_crypto::fab::{AlignmentSegment, Value, ValueAtom};
use midnight_compact_runtime::fab::{Aligned, AlignedValue, AlignmentAtom};
use midnight_compact_runtime::ledger::CellValue;
use midnight_compact_runtime::{
    BinaryHashRepr, CompactCellValue, CompactEnum, Field, FieldRepr, FromFieldRepr,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, CompactCellValue, CompactEnum)]
#[allow(non_camel_case_types)]
enum Choice {
    yes,
    no,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, CompactCellValue, CompactEnum)]
enum Only {
    Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, CompactCellValue, CompactEnum)]
#[allow(dead_code)]
enum Wide {
    V0,
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
    V10,
    V11,
    V12,
    V13,
    V14,
    V15,
    V16,
    V17,
    V18,
    V19,
    V20,
    V21,
    V22,
    V23,
    V24,
    V25,
    V26,
    V27,
    V28,
    V29,
    V30,
    V31,
    V32,
    V33,
    V34,
    V35,
    V36,
    V37,
    V38,
    V39,
    V40,
    V41,
    V42,
    V43,
    V44,
    V45,
    V46,
    V47,
    V48,
    V49,
    V50,
    V51,
    V52,
    V53,
    V54,
    V55,
    V56,
    V57,
    V58,
    V59,
    V60,
    V61,
    V62,
    V63,
    V64,
    V65,
    V66,
    V67,
    V68,
    V69,
    V70,
    V71,
    V72,
    V73,
    V74,
    V75,
    V76,
    V77,
    V78,
    V79,
    V80,
    V81,
    V82,
    V83,
    V84,
    V85,
    V86,
    V87,
    V88,
    V89,
    V90,
    V91,
    V92,
    V93,
    V94,
    V95,
    V96,
    V97,
    V98,
    V99,
    V100,
    V101,
    V102,
    V103,
    V104,
    V105,
    V106,
    V107,
    V108,
    V109,
    V110,
    V111,
    V112,
    V113,
    V114,
    V115,
    V116,
    V117,
    V118,
    V119,
    V120,
    V121,
    V122,
    V123,
    V124,
    V125,
    V126,
    V127,
    V128,
    V129,
    V130,
    V131,
    V132,
    V133,
    V134,
    V135,
    V136,
    V137,
    V138,
    V139,
    V140,
    V141,
    V142,
    V143,
    V144,
    V145,
    V146,
    V147,
    V148,
    V149,
    V150,
    V151,
    V152,
    V153,
    V154,
    V155,
    V156,
    V157,
    V158,
    V159,
    V160,
    V161,
    V162,
    V163,
    V164,
    V165,
    V166,
    V167,
    V168,
    V169,
    V170,
    V171,
    V172,
    V173,
    V174,
    V175,
    V176,
    V177,
    V178,
    V179,
    V180,
    V181,
    V182,
    V183,
    V184,
    V185,
    V186,
    V187,
    V188,
    V189,
    V190,
    V191,
    V192,
    V193,
    V194,
    V195,
    V196,
    V197,
    V198,
    V199,
    V200,
    V201,
    V202,
    V203,
    V204,
    V205,
    V206,
    V207,
    V208,
    V209,
    V210,
    V211,
    V212,
    V213,
    V214,
    V215,
    V216,
    V217,
    V218,
    V219,
    V220,
    V221,
    V222,
    V223,
    V224,
    V225,
    V226,
    V227,
    V228,
    V229,
    V230,
    V231,
    V232,
    V233,
    V234,
    V235,
    V236,
    V237,
    V238,
    V239,
    V240,
    V241,
    V242,
    V243,
    V244,
    V245,
    V246,
    V247,
    V248,
    V249,
    V250,
    V251,
    V252,
    V253,
    V254,
    V255,
    V256,
}

#[test]
fn singleton_enum_keeps_the_zero_byte_binary_width() {
    assert_eq!(Only::default(), Only::Value);
    assert_eq!(Only::Value.field_vec(), vec![Field::from(0_u64)]);
    assert_eq!(Only::Value.binary_vec(), Vec::<u8>::new());
    assert_eq!(Only::Value.binary_len(), 0);
    assert_eq!(Only::from_field_repr(&[Field::from(1_u64)]), None);
    assert_eq!(
        Only::alignment().0,
        vec![AlignmentSegment::Atom(AlignmentAtom::Bytes { length: 0 })]
    );
    let encoded = AlignedValue::from(Only::Value);
    assert_eq!(encoded.value, Value(vec![ValueAtom(vec![])]));
    assert_eq!(
        Only::decode_cell_value(&encoded.as_slice()),
        Ok(Only::Value)
    );
}

#[test]
fn enum_derive_preserves_one_byte_repr_and_rejects_invalid_ordinals() {
    assert_eq!(Choice::default(), Choice::yes);
    assert_eq!(Choice::no.field_vec(), vec![Field::from(1_u64)]);
    assert_eq!(Choice::no.binary_vec(), vec![1]);
    assert_eq!(Choice::no.binary_len(), 1);
    assert_eq!(
        Choice::from_field_repr(&Choice::no.field_vec()),
        Some(Choice::no)
    );
    assert_eq!(Choice::from_field_repr(&[Field::from(2_u64)]), None);
    assert_eq!(
        Choice::alignment().0,
        vec![AlignmentSegment::Atom(AlignmentAtom::Bytes { length: 1 })]
    );
    let encoded = AlignedValue::from(Choice::no);
    assert_eq!(encoded.value, Value(vec![ValueAtom(vec![1])]));
    assert_eq!(
        Choice::decode_cell_value(&encoded.as_slice()),
        Ok(Choice::no)
    );
    let invalid = Value(vec![ValueAtom(vec![2])]);
    assert!(Choice::decode_cell_value(&invalid).is_err());
}

#[test]
fn enum_derive_widens_at_257_variants() {
    assert_eq!(Wide::default(), Wide::V0);
    assert_eq!(Wide::V256.field_vec(), vec![Field::from(256_u64)]);
    assert_eq!(Wide::V256.binary_vec(), vec![0, 1]);
    assert_eq!(Wide::V256.binary_len(), 2);
    assert_eq!(
        Wide::from_field_repr(&Wide::V256.field_vec()),
        Some(Wide::V256)
    );
    assert_eq!(Wide::from_field_repr(&[Field::from(257_u64)]), None);
    assert_eq!(
        Wide::alignment().0,
        vec![AlignmentSegment::Atom(AlignmentAtom::Bytes { length: 2 })]
    );
    let encoded = AlignedValue::from(Wide::V256);
    assert_eq!(encoded.value, Value(vec![ValueAtom(vec![0, 1])]));
    assert_eq!(Wide::decode_cell_value(&encoded.as_slice()), Ok(Wide::V256));
    let invalid = Value(vec![ValueAtom(vec![1, 1])]);
    assert!(Wide::decode_cell_value(&invalid).is_err());
}
