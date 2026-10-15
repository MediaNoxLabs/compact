#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0

"""Bind fresh frontend emission to the artifact Cargo actually consumes.

Cargo supplies OUT_DIR identities; directory guesses and retained IR alone do
not establish that a generated source was compiled by a maintained consumer.
"""

import hashlib
import json
from pathlib import Path


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cargo_messages(messages: Path):
    for line in messages.read_text().splitlines():
        if not line.startswith("{"):
            continue  # Cargo test harness text accompanies machine build messages.
        value = json.loads(line)
        if isinstance(value, dict) and "reason" in value:
            yield value


def cargo_output_directories(messages: Path, metadata: dict) -> dict[str, Path]:
    packages = {package["id"]: package["name"] for package in metadata["packages"]}
    outputs = {}
    for value in cargo_messages(messages):
        if value.get("reason") != "build-script-executed":
            continue
        identity = value["package_id"]
        if identity not in packages:
            continue  # --no-deps metadata intentionally selects workspace consumers.
        name = packages[identity]
        output = Path(value["out_dir"]).resolve()
        if name in outputs and outputs[name] != output:
            raise ValueError(f"ambiguous Cargo output for {name}")
        outputs[name] = output
    return outputs


def depfile_inputs(path: Path, root: Path) -> set[Path]:
    """Read rustc's first Make dependency rule, including escaped path spaces."""
    rule = path.read_text().replace("\\\n", "").splitlines()[0]
    _, separator, dependencies = rule.partition(": ")
    if not separator:
        raise ValueError(f"invalid rustc dependency rule: {path}")
    words, current, escaped = [], [], False
    for character in dependencies:
        if escaped:
            current.append(character)
            escaped = False
        elif character == "\\":
            escaped = True
        elif character.isspace():
            if current:
                words.append("".join(current))
                current = []
        else:
            current.append(character)
    if escaped:
        raise ValueError(f"unterminated rustc dependency escape: {path}")
    if current:
        words.append("".join(current))
    return {(root / word).resolve() for word in words}


def consumed_artifact_bindings(registration, messages: Path, metadata: dict, root: Path) -> list[dict]:
    """Require rustc dependency evidence for the declared include owner and output."""
    directories = cargo_output_directories(messages, metadata)
    output = consumer_artifact(registration, directories).resolve()
    package_ids = {package["id"] for package in metadata["packages"]
                   if package["name"] == registration.package}
    bindings = []
    for value in cargo_messages(messages):
        if value.get("reason") != "compiler-artifact" or value.get("package_id") not in package_ids:
            continue
        if Path(value["target"]["src_path"]).resolve() != registration.consumer:
            continue
        for filename in value["filenames"]:
            artifact = Path(filename)
            if artifact.suffix in {".rlib", ".rmeta"} and artifact.name.startswith("lib"):
                depfile = artifact.with_name(artifact.name[3:]).with_suffix(".d")
            elif value.get("executable") == filename:
                depfile = artifact.with_suffix(".d")
            else:
                continue
            if not depfile.is_file():
                continue
            inputs = depfile_inputs(depfile, root)
            if registration.consumer not in inputs or output not in inputs:
                continue
            bindings.append({"consumer": str(registration.consumer), "target": value["target"]["name"],
                             "kind": value["target"]["kind"], "profile": value["profile"],
                             "artifact": str(artifact), "artifact_sha256": sha256(artifact),
                             "depfile": str(depfile), "depfile_sha256": sha256(depfile),
                             "generated_artifact": str(output), "generated_sha256": sha256(output)})
    if not bindings:
        raise ValueError(f"no compiled target consumes declared output: {registration.source}")
    return bindings


def consumer_artifact(registration, directories: dict[str, Path]) -> Path:
    if registration.package not in directories:
        raise ValueError(f"missing Cargo build output for {registration.package}")
    directory = directories[registration.package]
    path = directory / registration.generated_artifact
    if not path.is_file() or not path.resolve().is_relative_to(directory):
        raise ValueError(f"missing or escaped generated consumer artifact: {path}")
    return path


def place_in_host(source: bytes, namespace: str | None) -> bytes:
    """Apply only the registry's explicit multi-contract test-host placement."""
    if namespace is None:
        return source
    return source.replace(b"crate::", f"crate::{namespace}::".encode())


def check_input_fingerprints(registrations) -> list[dict]:
    pins = {}
    for registration in registrations:
        for path, expected in registration.input_fingerprints:
            if path in pins and pins[path] != expected:
                raise ValueError(f"conflicting registered input authority: {path}")
            pins[path] = expected
    rows = []
    for path, expected in sorted(pins.items(), key=lambda pair: str(pair[0])):
        actual = sha256(path)
        if actual != expected:
            raise ValueError(f"registered consumer input changed: {path}")
        rows.append({"path": str(path), "sha256": actual})
    return rows


def compare_consumer_artifact(registration, generated: Path, formatted: Path,
                              original: Path, expected: Path) -> dict:
    if formatted.read_bytes() != expected.read_bytes():
        raise ValueError(f"fresh emission differs from compiled consumer: {registration.source}")
    return {"source": str(registration.source), "package": registration.package,
            "consumer": str(registration.consumer), "artifact": str(original),
            "artifact_sha256": sha256(original),
            "formatted_artifact": str(formatted),
            "formatted_artifact_sha256": sha256(formatted),
            "generated": str(generated), "generated_sha256": sha256(generated),
            "host_placed_generated": str(expected), "host_placed_sha256": sha256(expected),
            "namespace": registration.namespace,
            "scope": "Artifact identity; execution and proof receipts are separate gates."}
