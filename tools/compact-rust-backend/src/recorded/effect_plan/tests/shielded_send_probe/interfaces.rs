// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Representative public encoding edges and local owned interfaces.
use super::*;
#[test]
fn whole_old_send_public_interfaces_cover_encodable_owned_products() {
    for (label, ty) in [
        (
            "public-tuple11",
            Type::Tuple {
                elements: vec![Type::OpaqueBytes; 11],
            },
        ),
        (
            "public-vector-tuple9",
            vector(
                Type::Tuple {
                    elements: vec![Type::OpaqueBytes; 9],
                },
                2,
            ),
        ),
        (
            "public-string-and-wide",
            Type::Struct {
                name: "OwnedPublic".into(),
                fields: vec![
                    StructField {
                        name: "text".into(),
                        ty: Type::OpaqueString,
                    },
                    StructField {
                        name: "wide".into(),
                        ty: validate::uint("340282366920938463463374607431768211456"),
                    },
                ],
            },
        ),
    ] {
        let mut contract = source();
        root_mut(&mut contract, "send_to_self")
            .parameters
            .push(parameter("unused_owned", ty));
        probe(&contract, "send_to_self", label, true);
    }
}
