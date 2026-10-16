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

"""Explicit Compact source registrations and their actual generated consumers.

``fixture_map`` retains the checked-in flat-library API. ``source_registry``
also accounts for build-script outputs and accepted compiler probes; missing
flat libraries never decide whether a source participates in the full gate.
"""

from dataclasses import dataclass
from enum import Enum
import hashlib
import json
from pathlib import Path
import re
import tomllib

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


class SourceRole(str, Enum):
    FLAT_LIBRARY = "flat_library"
    GENERATED_CONSUMER = "generated_consumer"
    COMPILER_PROBE = "compiler_probe"


@dataclass(frozen=True)
class SourceRegistration:
    source: Path
    role: SourceRole
    source_sha256: str
    manifest: Path | None
    package: str | None
    library: Path | None
    generated_library: Path | None
    retained_ir: Path | None
    retained_ir_sha256: str | None
    build_script: Path | None
    generated_artifact: str | None
    consumer: Path | None
    namespace: str | None
    compiler_tests: tuple[Path, ...]
    input_fingerprints: tuple[tuple[Path, str], ...]
    frontend_flags: tuple[str, ...]
    qualification: str

    @property
    def flat_fixture(self) -> Path | None:
        return self.generated_library


def _path(root: Path, relative: str) -> Path:
    if (not isinstance(relative, str) or not relative
            or Path(relative).is_absolute() or ".." in Path(relative).parts):
        raise ValueError(f"unsafe registry path: {relative!r}")
    path = (root / relative).resolve()
    if not path.is_relative_to(root):
        raise ValueError(f"registry path escapes root: {relative}")
    if not path.is_file():
        raise ValueError(f"missing registry input: {relative}")
    return path


def _fingerprint(path: Path, expected: str) -> None:
    if (not isinstance(expected, str) or not re.fullmatch(r"[0-9a-f]{64}", expected)
            or hashlib.sha256(path.read_bytes()).hexdigest() != expected):
        raise ValueError(f"registry source binding changed: {path}")


def _has_out_dir_include(path: Path) -> bool:
    """Recognize the actual generated-consumer include form, not a flat output."""
    return re.search(
        r'include!\s*\(\s*concat!\s*\(\s*env!\s*\(\s*"OUT_DIR"\s*\)'
        r'\s*,\s*"/[A-Za-z_][A-Za-z0-9_]*\.rs"\s*\)\s*\)',
        path.read_text()) is not None


def _build_binding(root: Path, row: dict, manifest: Path, library: Path):
    build = _path(root, row["build_script"])
    consumer = _path(root, row["consumer"])
    artifact, namespace = row["generated_artifact"], row["namespace"]
    if not isinstance(artifact, str) or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*\.rs", artifact):
        raise ValueError("invalid OUT_DIR artifact")
    if namespace is not None and not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", namespace):
        raise ValueError("invalid generated namespace")
    inputs = row["consumer_input_sha256"]
    expected = {str(path.relative_to(root)) for path in (manifest, library, build, consumer)}
    supplemental = row.get("supplemental_build_inputs", [])
    if (not isinstance(supplemental, list)
            or ("supplemental_build_inputs" in row and not supplemental)
            or any(not isinstance(relative, str) for relative in supplemental)
            or len(supplemental) != len(set(supplemental))):
        raise ValueError("invalid supplemental build inputs")
    supplemental_paths = {_path(root, relative) for relative in supplemental}
    additional = {str(path.relative_to(root)) for path in supplemental_paths}
    if additional & expected or len(additional) != len(supplemental):
        raise ValueError("duplicate supplemental build inputs")
    expected |= additional
    if set(inputs) != expected:
        raise ValueError("incomplete consumer input authority")
    pins = []
    for relative, pin in inputs.items():
        path = _path(root, relative)
        _fingerprint(path, pin)
        pins.append((path, pin))
    return build, consumer, artifact, namespace, pins


def source_registry(root: Path = ROOT, *, registry_path: Path | None = None
                    ) -> tuple[SourceRegistration, ...]:
    """Validate every current source's explicit role, origin and consumer inputs.

    Registration asserts the declared source/consumer relationship, not language
    acceptance, execution coverage or proof readiness. The gate supplies fresh
    frontend compilation, output identity and actual consumer execution.
    """
    root = root.resolve()
    path = registry_path or root / "tools/compact-rust-backend/fixture_registry.json"
    data = json.loads(path.read_text())
    if data.get("schema_version") != 1 or not isinstance(data.get("sources"), list):
        raise ValueError("invalid source registry schema")
    registrations = []
    identities, flat_destinations, artifact_destinations = set(), set(), set()
    for row in data["sources"]:
        source = _path(root, row["source"])
        if source.suffix != ".compact" or source in identities:
            raise ValueError(f"duplicate or invalid Compact source registration: {source}")
        identities.add(source)
        try:
            role = SourceRole(row["role"])
        except ValueError as error:
            raise ValueError(f"unreviewed source role: {row.get('role')}") from error
        if row.get("frontend_outcome") != "accepted":
            raise ValueError("current registry contains only reviewed accepted source roles")
        _fingerprint(source, row["source_sha256"])
        flags = row.get("frontend_flags", [])
        if flags not in ([], ["--feature-zkir-v3"]):
            raise ValueError(f"unreviewed frontend flags: {source}")
        qualification = row.get("qualification")
        if not isinstance(qualification, str) or not qualification.strip():
            raise ValueError(f"source qualification required: {source}")
        manifest = library = generated_library = ir = build = consumer = None
        package = artifact = namespace = ir_sha = None
        compiler_tests = ()
        fingerprints = [(source, row["source_sha256"])]
        if role is SourceRole.FLAT_LIBRARY:
            fields = {"source", "source_sha256", "role", "frontend_outcome",
                            "frontend_flags", "qualification", "manifest", "library",
                            "generated_library"}
            if set(row) not in (fields, fields | {"build_binding"}):
                raise ValueError(f"invalid flat-library fields: {source}")
            library = _path(root, row["library"])
            generated_library = _path(root, row["generated_library"])
            if _has_out_dir_include(generated_library):
                raise ValueError(f"OUT_DIR wrapper is not a flat golden output: {source}")
            if generated_library in flat_destinations:
                raise ValueError(f"duplicate generated fixture destination: {generated_library}")
            flat_destinations.add(generated_library)
            manifest = _path(root, row["manifest"])
            binding = row.get("build_binding")
            if library != generated_library and binding is None:
                raise ValueError(f"flat fixture differs from Cargo entry without a build binding: {source}")
            if binding is not None:
                binding_fields = {"retained_ir", "retained_ir_sha256", "build_script",
                                  "generated_artifact", "consumer", "namespace", "consumer_input_sha256"}
                if set(binding) not in (binding_fields, binding_fields | {"supplemental_build_inputs"}):
                    raise ValueError(f"invalid flat build binding: {source}")
                ir = _path(root, binding["retained_ir"])
                ir_sha = binding["retained_ir_sha256"]
                _fingerprint(ir, ir_sha)
                fingerprints.append((ir, ir_sha))
                build, consumer, artifact, namespace, pins = _build_binding(root, binding, manifest, library)
                fingerprints.extend(pins)
        else:
            ir = _path(root, row["retained_ir"])
            ir_sha = row["retained_ir_sha256"]
            _fingerprint(ir, ir_sha)
            fingerprints.append((ir, ir_sha))
            if type(json.loads(ir.read_text()).get("schema_version")) is not int:
                raise ValueError(f"retained IR has no version: {ir}")
            if role is SourceRole.GENERATED_CONSUMER:
                consumer_fields = {"source", "source_sha256", "role", "frontend_outcome",
                                   "frontend_flags", "qualification", "retained_ir",
                                   "retained_ir_sha256", "manifest", "library", "build_script",
                                   "generated_artifact", "consumer", "namespace",
                                   "consumer_input_sha256"}
                if set(row) not in (consumer_fields, consumer_fields | {"supplemental_build_inputs"}):
                    raise ValueError(f"invalid generated-consumer fields: {source}")
                manifest = _path(root, row["manifest"])
                library = _path(root, row["library"])
                build, consumer, artifact, namespace, pins = _build_binding(root, row, manifest, library)
                fingerprints.extend(pins)
            elif set(row) != {"source", "source_sha256", "role", "frontend_outcome",
                              "frontend_flags", "qualification", "retained_ir",
                              "retained_ir_sha256", "compiler_tests", "compiler_test_sha256"}:
                raise ValueError(f"invalid compiler-probe fields: {source}")
            else:
                tests = row["compiler_tests"]
                if not tests or set(tests) != set(row["compiler_test_sha256"]):
                    raise ValueError(f"compiler-probe authority required: {source}")
                compiler_tests = tuple(_path(root, relative) for relative in tests)
                for relative in tests:
                    input_path = _path(root, relative)
                    pin = row["compiler_test_sha256"][relative]
                    _fingerprint(input_path, pin)
                    fingerprints.append((input_path, pin))
        if manifest is not None:
            spec = tomllib.loads(manifest.read_text())
            package = spec["package"]["name"]
            declared_library = (manifest.parent / spec.get("lib", {}).get("path", "src/lib.rs")).resolve()
            if library != declared_library:
                raise ValueError(f"consumer library disagrees with Cargo manifest: {source}")
            if build is not None:
                declared_build = (manifest.parent / spec["package"].get("build", "build.rs")).resolve()
                if build != declared_build:
                    raise ValueError(f"consumer build script disagrees with Cargo manifest: {source}")
                destination = (manifest, artifact)
                if destination in artifact_destinations:
                    raise ValueError(f"duplicate generated consumer artifact: {destination}")
                artifact_destinations.add(destination)
        registrations.append(SourceRegistration(
            source, role, row["source_sha256"], manifest, package, library, generated_library, ir, ir_sha,
            build, artifact, consumer, namespace, compiler_tests,
            tuple(sorted(fingerprints, key=lambda pair: str(pair[0]))), tuple(flags), qualification))
    top_level = {path.resolve() for path in (root / "examples/rust_backend").glob("*.compact")}
    extras = {path.resolve(): target.resolve() for path, target in extra_sources(root).items()}
    if top_level & extras.keys():
        raise ValueError("duplicate registered Compact source")
    expected = top_level | extras.keys()
    if identities != expected:
        missing = sorted(str(path.relative_to(root)) for path in expected - identities)
        unknown = sorted(str(path.relative_to(root)) for path in identities - expected)
        raise ValueError(f"unclassified/missing source registrations: {missing}; unexpected: {unknown}")
    by_source = {row.source: row for row in registrations}
    for source, target in extras.items():
        registration = by_source[source]
        if not ((registration.role is SourceRole.FLAT_LIBRARY
                 and registration.flat_fixture == target)
                or (registration.role is SourceRole.GENERATED_CONSUMER
                    and registration.library == target)):
            raise ValueError(f"special fixture consumer target changed: {source}")
    return tuple(sorted(registrations, key=lambda row: str(row.source)))


def emission_fixture(registration: SourceRegistration, root: Path = ROOT) -> Path | None:
    """Retained independent output, separate from current-emitter Cargo bindings.

    Flat libraries remain their existing oracle. Build-time consumers retain an
    unhosted, rustfmt-normalized snapshot; accepted compiler probes have no golden.
    """
    if registration.flat_fixture is not None:
        return registration.flat_fixture
    if registration.role is SourceRole.GENERATED_CONSUMER:
        return (root / "tools/compact-rust-backend/tests/generated-snapshots"
                / registration.source.relative_to(root).with_suffix(".rs"))
    return None


def fixture_map(root: Path = ROOT) -> dict[Path, Path]:
    """Checked-in flat libraries only; source_registry carries all other roles."""
    return {row.source: row.flat_fixture for row in source_registry(root)
            if row.flat_fixture is not None}
