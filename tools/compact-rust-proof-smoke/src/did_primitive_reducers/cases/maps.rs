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

use super::*;
use compact_rust_did_enum_map_reducer_fixture::{ledger_contract as wide, types as wt};
use compact_rust_did_nested_map_reducer_fixture::{ledger_contract as nested, types as nt};
use compact_rust_did_point_map_reducer_fixture::{ledger_contract as points, types as pt};
use compact_rust_did_service_map_reducer_fixture::{ledger_contract as service, types as sv};
use r::context::ConstructorContext;
fn service_initial() -> Result<ConstructorResult<u64>, Failure> {
    Ok(service::initial_state(ConstructorContext::new(0))?)
}
fn point_initial() -> Result<ConstructorResult<u64>, Failure> {
    Ok(points::initial_state(ConstructorContext::new(0))?)
}
fn nested_initial() -> Result<ConstructorResult<u64>, Failure> {
    Ok(nested::initial_state(ConstructorContext::new(0))?)
}
fn wide_initial() -> Result<ConstructorResult<u64>, Failure> {
    Ok(wide::initial_state(ConstructorContext::new(0))?)
}
fn service_call(
    case: &Case,
    o: &ObservedContractState,
) -> Result<(Recorded, AlignedValue), Failure> {
    let id = text(ID);
    if case.operation == "remove_service" {
        return pair(
            service::remove_service(o.circuit_context(0), id.clone())?,
            service::recorded::remove_service(o.circuit_context(0), id.clone())?,
            AlignedValue::from((id,)),
        );
    }
    let (m, typ, endpoint) = match case.id {
        "insert-unicode" => (
            sv::Mutation::Insert,
            "LinkedDomains",
            "https://example.invalid/",
        ),
        "update-empty" => (sv::Mutation::Update, "", ""),
        _ => return Err("unknown Service case".into()),
    };
    let v = sv::Service {
        id,
        typ: text(typ),
        serviceEndpoint: text(endpoint),
    };
    pair(
        service::set_service(o.circuit_context(0), v.clone(), m)?,
        service::recorded::set_service(o.circuit_context(0), v.clone(), m)?,
        AlignedValue::from((v, m)),
    )
}
fn point_call(case: &Case, o: &ObservedContractState) -> Result<(Recorded, AlignedValue), Failure> {
    let id = text(ID);
    if case.operation == "remove" {
        return pair(
            points::remove(o.circuit_context(0), id.clone())?,
            points::recorded::remove(o.circuit_context(0), id.clone())?,
            AlignedValue::from((id,)),
        );
    }
    let (update, key) = match case.id {
        "insert-unicode" => (false, 3),
        "update-point" => (true, 4),
        _ => return Err("unknown Point Map case".into()),
    };
    let v = pt::PointMethod {
        id,
        publicKey: point(key)?,
    };
    pair(
        points::upsert(o.circuit_context(0), v.clone(), update)?,
        points::recorded::upsert(o.circuit_context(0), v.clone(), update)?,
        AlignedValue::from((v, update)),
    )
}
fn nested_call(
    case: &Case,
    o: &ObservedContractState,
) -> Result<(Recorded, AlignedValue), Failure> {
    let id = text(ID);
    if case.operation == "remove" {
        return pair(
            nested::remove(o.circuit_context(0), id.clone())?,
            nested::recorded::remove(o.circuit_context(0), id.clone())?,
            AlignedValue::from((id,)),
        );
    }
    let (update, x) = match case.id {
        "insert-unicode" => (false, "original"),
        "update-empty" => (true, ""),
        _ => return Err("unknown nested Map case".into()),
    };
    let v = nt::Method {
        id,
        jwk: nt::Jwk {
            kty: nt::KeyKind::A,
            x: text(x),
        },
    };
    pair(
        nested::upsert(o.circuit_context(0), v.clone(), update)?,
        nested::recorded::upsert(o.circuit_context(0), v.clone(), update)?,
        AlignedValue::from((v, update)),
    )
}
fn wide_call(case: &Case, o: &ObservedContractState) -> Result<(Recorded, AlignedValue), Failure> {
    let id = text(ID);
    if case.operation == "drop" {
        return pair(
            wide::drop(o.circuit_context(0), id.clone())?,
            wide::recorded::drop(o.circuit_context(0), id.clone())?,
            AlignedValue::from((id,)),
        );
    }
    let (curve, label, key) = match case.id {
        "insert-kind-a" => (wt::Kind::A, "label", 3),
        "replace-kind-b-outer-c" => (wt::Kind::B, "label", 3),
        "overwrite-empty" => (wt::Kind::A, "", 4),
        _ => return Err("unknown Enum Map case".into()),
    };
    let v = wt::Container {
        id,
        choice: wt::Kind::C,
        nested: wt::Leaf {
            curve,
            label: text(label),
            point: point(key)?,
        },
    };
    pair(
        wide::put(o.circuit_context(0), v.clone())?,
        wide::recorded::put(o.circuit_context(0), v.clone())?,
        AlignedValue::from((v,)),
    )
}
pub(super) fn fixture(kind: &str) -> Result<Fixture, Failure> {
    match kind {
        "service-map" => Ok(Fixture {
            operations: &["set_service", "remove_service"],
            cases: &[
                Case {
                    id: "insert-unicode",
                    operation: "set_service",
                },
                Case {
                    id: "update-empty",
                    operation: "set_service",
                },
                Case {
                    id: "remove",
                    operation: "remove_service",
                },
            ],
            initial: service_initial,
            call: service_call,
        }),
        "point-map" => Ok(Fixture {
            operations: &["upsert", "remove"],
            cases: &[
                Case {
                    id: "insert-unicode",
                    operation: "upsert",
                },
                Case {
                    id: "update-point",
                    operation: "upsert",
                },
                Case {
                    id: "remove",
                    operation: "remove",
                },
            ],
            initial: point_initial,
            call: point_call,
        }),
        "nested-map" => Ok(Fixture {
            operations: &["upsert", "remove"],
            cases: &[
                Case {
                    id: "insert-unicode",
                    operation: "upsert",
                },
                Case {
                    id: "update-empty",
                    operation: "upsert",
                },
                Case {
                    id: "remove",
                    operation: "remove",
                },
            ],
            initial: nested_initial,
            call: nested_call,
        }),
        "enum-map" => Ok(Fixture {
            operations: &["put", "drop"],
            cases: &[
                Case {
                    id: "insert-kind-a",
                    operation: "put",
                },
                Case {
                    id: "replace-kind-b-outer-c",
                    operation: "put",
                },
                Case {
                    id: "overwrite-empty",
                    operation: "put",
                },
                Case {
                    id: "drop",
                    operation: "drop",
                },
            ],
            initial: wide_initial,
            call: wide_call,
        }),
        _ => Err("unknown product Map reducer".into()),
    }
}
