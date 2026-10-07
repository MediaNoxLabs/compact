#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#  	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Shared maintained Compact source to generated Rust fixture inventory."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPECIAL_FIXTURES = {
    'tools/compact-rust-backend/tests/point-composition/point_digest.compact':
        'tests-rust-backend/did-point-digest-reducer/lib.rs',
    'tools/compact-rust-backend/tests/point-composition/point_guard.compact':
        'tests-rust-backend/did-point-guard-reducer/lib.rs',
    'tools/compact-rust-backend/tests/set-composition/set_string.compact':
        'tests-rust-backend/did-alias-set-reducer/lib.rs',
    'tools/compact-rust-backend/tests/set-composition/opaque_digest.compact':
        'tests-rust-backend/did-alias-digest-reducer/lib.rs',
    'tools/compact-rust-backend/tests/set-composition/pure_unit_guard.compact':
        'tests-rust-backend/did-alias-guard-reducer/lib.rs',
    'tools/compact-rust-backend/tests/map-composition/service_mutation.compact':
        'tests-rust-backend/did-service-map-reducer/lib.rs',
    'tools/compact-rust-backend/tests/map-point-composition/point_nested.compact':
        'tests-rust-backend/did-point-map-reducer/lib.rs',
    'tools/compact-rust-backend/tests/map-nested-product/nested-enum.compact':
        'tests-rust-backend/did-nested-map-reducer/lib.rs',
    'tools/compact-rust-backend/tests/map-nested-product/nested-enum-wide.compact':
        'tests-rust-backend/did-enum-map-reducer/lib.rs',
    'examples/rust_backend/did_digest_read_reducer/contract.compact':
        'tests-rust-backend/did-digest-read-reducer/lib.rs',
    'examples/rust_backend/did_adoption/packages/contract/src/did.compact':
        'tests-rust-backend/did-adoption/lib.rs',
    'examples/rust_backend/vc_passport_adoption/src/digital-passport-credential.compact':
        'tests-rust-backend/vc-passport-adoption/lib.rs',
    'test-center/test-contracts/coracle.compact':
        'tests-rust-backend/test-center-coracle/lib.rs',
    'test-center/test-contracts/micro-dao.compact':
        'tests-rust-backend/test-center-micro-dao/lib.rs',
    'examples/rust_backend/digital-passport-credential/src/digital-passport-credential.compact':
        'tests-rust-backend/passport-dogfood/lib.rs',
    'examples/bugs/pm-19252/example_ten.compact':
        'tests-rust-backend/pm-19252-own-public-key/lib.rs',
    'examples/adt/tests/set_enum.compact':
        'tests-rust-backend/adt-set-enum/lib.rs',
    'examples/adt/tests/set_vector.compact':
        'tests-rust-backend/adt-set-vector/lib.rs',
    'examples/adt/tests/set_qualified_coin_info.compact':
        'tests-rust-backend/adt-set-qualified-coin-info/lib.rs',
    'examples/adt/tests/list_field.compact':
        'tests-rust-backend/adt-list-field/lib.rs',
    'examples/adt/tests/list_enum.compact':
        'tests-rust-backend/adt-list-enum/lib.rs',
    'test-center/test-contracts/counter.compact':
        'tests-rust-backend/test-center-counter/lib.rs',
    'test-center/test-contracts/bboard.compact':
        'tests-rust-backend/test-center-bboard/lib.rs',
    'test-center/test-contracts/welcome.compact':
        'tests-rust-backend/test-center-welcome/lib.rs',
    'examples/adt/tests/list_vector_field_4.compact':
        'tests-rust-backend/adt-list-vector-field-4/lib.rs',
    'examples/adt/tests/list_bytes.compact':
        'tests-rust-backend/adt-list-bytes/lib.rs',
    'examples/bugs/pm-19252/example_seven.compact':
        'tests-rust-backend/pm-19252-unused-read-seven/lib.rs',
    'examples/bugs/pm-19252/example_eight_a.compact':
        'tests-rust-backend/pm-19252-unused-read-eight-a/lib.rs',
    'examples/bugs/pm-19252/example_eight_b.compact':
        'tests-rust-backend/pm-19252-unused-read-eight-b/lib.rs',
}


def extra_sources(root: Path = ROOT) -> dict[Path, Path]:
    return {root / source: root / fixture for source, fixture in SPECIAL_FIXTURES.items()}


def fixture_map(root: Path = ROOT) -> dict[Path, Path]:
    root = root.resolve()
    mapping = {source.resolve(): root / "tests-rust-backend" / source.stem.replace("_", "-") / "lib.rs"
               for source in (root / "examples/rust_backend").glob("*.compact")}
    extra = extra_sources(root)
    if mapping.keys() & extra.keys():
        raise ValueError("duplicate registered Compact source")
    mapping.update(extra)
    if len(set(mapping.values())) != len(mapping):
        raise ValueError("duplicate generated fixture destination")
    for source, fixture in mapping.items():
        if not source.is_file() or not fixture.is_file():
            raise ValueError(f"missing registered source or fixture: {source} -> {fixture}")
    return mapping
