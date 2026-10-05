// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use midnight_coin_structure::transfer::SenderEvidence;
use midnight_compact_runtime::{FixedBytes, ledger};

fn bytes(first: u8) -> FixedBytes<32> {
    let mut bytes = [0; 32];
    bytes[0] = first;
    FixedBytes::new(bytes)
}

#[test]
fn pinned_ledger_commitments_match_independent_typescript_pure_helper_across_u128() {
    // Generated TypeScript _coinCommitment_0 from the unchanged standard
    // library; the two wide values hash purely but TS output creation rejects
    // them until MediaNoxLabs/compact#300 is resolved.
    let vectors = [
        (
            1,
            2,
            0,
            9,
            "7613989076c9d67a6b316676f41102858ba2346aa13619a251c0dbf9fe1c34fd",
        ),
        (
            3,
            2,
            42,
            9,
            "b0cad399066d92125226d188a1e0532d4e19a51437b64fe07c2c009bb232003a",
        ),
        (
            4,
            5,
            u64::MAX as u128,
            11,
            "31b26ba1811b7b76ea6dd2c7e71dc72967b1b0d3ea4d2d5bb77439e4e1fea6e5",
        ),
        (
            4,
            5,
            1u128 << 64,
            11,
            "026a66903340037725dd58247abc09202f73d091c213b9f193d11a11b2caa3d1",
        ),
        (
            4,
            5,
            u128::MAX,
            11,
            "dc0c62f951b7a2ec113278b41075c9380d1f99f3b917ef8a2f949aa5021118da",
        ),
    ];
    for (nonce, color, value, address, expected) in vectors {
        let info = ledger::coin_info_from_compact(bytes(nonce), bytes(color), value);
        let recipient = ledger::coin_recipient_from_compact(false, bytes(0), bytes(address));
        assert_eq!(hex::encode(info.commitment(&recipient).0.0), expected);
    }
}

#[test]
fn generated_typescript_nullifiers_and_both_recipient_domains_match_pinned_ledger() {
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/shielded-receive-oracle.json")).unwrap();
    for row in capture["identityVectors"].as_array().unwrap() {
        let first = |name: &str| row[name].as_u64().unwrap() as u8;
        let info = ledger::coin_info_from_compact(
            bytes(first("nonce")),
            bytes(first("color")),
            row["value"].as_str().unwrap().parse().unwrap(),
        );
        let address =
            ledger::ContractAddress(ledger::HashOutput(bytes(first("addressByte")).into_array()));
        let contract = ledger::CoinRecipient::Contract(address);
        let user = ledger::CoinRecipient::User(ledger::CoinPublicKey(ledger::HashOutput(
            bytes(7).into_array(),
        )));
        let nullifier = info.nullifier(&SenderEvidence::Contract(address));
        assert_eq!(
            hex::encode(info.commitment(&contract).0.0),
            row["contractCommitmentHex"]
        );
        assert_eq!(
            hex::encode(info.commitment(&user).0.0),
            row["userCommitmentHex"]
        );
        assert_eq!(hex::encode(nullifier.0.0), row["contractNullifierHex"]);
        assert_ne!(nullifier.0, info.commitment(&contract).0);
        assert_ne!(info.commitment(&contract), info.commitment(&user));
    }
}
