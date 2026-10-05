#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#   http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Inventory Compact declarations and, optionally, compiler-reported Rust APIs.

This is a source inventory, not a Compact parser or an assertion of semantic
TypeScript/Rust parity. A baseline checks declaration membership and signatures;
the compiler capability report checks recorded/observed API availability and
compiler proof applicability, not proof execution or behavioral parity.
"""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import tomllib


ROOT = Path(__file__).resolve().parents[2]
ORACLE_MANIFEST = Path(__file__).with_name("oracle_acceptance.json")
POSITIVE_SOURCE_MANIFEST = Path(__file__).with_name("parity_positive_sources.json")
ADT_SET_SOURCE_MANIFEST = Path(__file__).with_name("parity_positive_adt_set_sources.json")
ADT_LIST_SOURCE_MANIFEST = Path(__file__).with_name("parity_positive_adt_list_sources.json")
TOP_LEVEL_SOURCE_MANIFEST = Path(__file__).with_name("parity_positive_top_level_sources.json")
TEST_CENTER_COUNTER_SOURCE_MANIFEST = Path(__file__).with_name(
    "parity_positive_test_center_counter_sources.json")
POSITIVE_SOURCE_MANIFESTS = (
    POSITIVE_SOURCE_MANIFEST, ADT_SET_SOURCE_MANIFEST,
    ADT_LIST_SOURCE_MANIFEST, TOP_LEVEL_SOURCE_MANIFEST,
    TEST_CENTER_COUNTER_SOURCE_MANIFEST)
DEFAULT_BASELINE = Path(__file__).with_name("parity_baseline.json")
COMPILED_PACKAGE_ROOTS = {
    "examples/rust_backend/digital-passport-credential/src/digital-passport-credential.compact",
}
DECLARATION = re.compile(
    r"(?m)^[ \t]*(?P<export>export[ \t]+)?(?P<pure>pure[ \t]+)?"
    r"(?P<kind>circuit|witness|constructor|module)\b"
)
IMPORT = re.compile(r"(?m)^[ \t]*(include|import)\s+(.+?);", re.DOTALL)
EXPORT_LIST = re.compile(r"(?m)^[ \t]*export[ \t]*\{([^}]*)\}")
NAME = re.compile(r"[A-Za-z_$][A-Za-z0-9_$]*")
PURE_DECLARATION = re.compile(r"(?m)^[ \t]*(export[ \t]+)?pure[ \t]+circuit\b")


def positive_scope(root: Path) -> dict | None:
    if root.resolve() != ROOT.resolve():
        return None
    cohorts = [json.loads(path.read_text()) for path in POSITIVE_SOURCE_MANIFESTS]
    positive = [entry for cohort in cohorts for entry in cohort["positive_sources"]]
    rejections = [entry for cohort in cohorts for entry in cohort["expected_rejections"]]
    sources = [entry["source"] for entry in positive + rejections]
    if len(sources) != len(set(sources)):
        raise ValueError("positive source manifests contain duplicate paths")
    return {"cohorts": cohorts, "positive_sources": positive,
            "expected_rejections": rejections}


def without_comments(source: str) -> str:
    """Hide comments, preserving offsets, newlines and string literals."""
    out = list(source)
    index = 0
    quote = None
    while index < len(source):
        char = source[index]
        next_char = source[index + 1] if index + 1 < len(source) else ""
        if quote:
            if char == "\\":
                index += 2
                continue
            if char == quote:
                quote = None
        elif char in ('"', "'"):
            quote = char
        elif char == "/" and next_char in ("/", "*"):
            line = next_char == "/"
            end = source.find("\n" if line else "*/", index + 2)
            end = len(source) if end < 0 else end + (0 if line else 2)
            for offset in range(index, end):
                if source[offset] != "\n":
                    out[offset] = " "
            index = end
            continue
        index += 1
    return "".join(out)


def declaration_end(source: str, start: int) -> tuple[int, str]:
    """Find a declaration's body opener or semicolon outside type delimiters."""
    depths = {"(": 0, "[": 0, "<": 0}
    closes = {")": "(", "]": "[", ">": "<"}
    quote = None
    index = start
    while index < len(source):
        char = source[index]
        if quote:
            if char == "\\":
                index += 2
                continue
            if char == quote:
                quote = None
        elif char in ('"', "'"):
            quote = char
        elif char in depths:
            depths[char] += 1
        elif char in closes:
            opener = closes[char]
            depths[opener] = max(0, depths[opener] - 1)
        elif char in "{;" and not any(depths.values()):
            return index, char
        index += 1
    raise ValueError("unterminated declaration")


def closing_brace(source: str, opener: int) -> int:
    depth = 0
    quote = None
    for index in range(opener, len(source)):
        char = source[index]
        if quote:
            if char == quote and (index == 0 or source[index - 1] != "\\"):
                quote = None
        elif char in ('"', "'"):
            quote = char
        elif char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return index
    raise ValueError("unclosed module body")


def normalized(source: str) -> str:
    return " ".join(source.split())


def relative_source(path: Path, root: Path) -> str:
    resolved = path.resolve()
    if not resolved.is_relative_to(root.resolve()) or not resolved.is_file():
        raise ValueError(f"invalid repository source: {path}")
    return resolved.relative_to(root.resolve()).as_posix()


def source_paths(root: Path, extra_dirs: list[Path] | None = None) -> list[Path]:
    paths = set((root / "examples/rust_backend").glob("*.compact"))
    paths.update((root / "examples/rust_backend/digital-passport-credential").rglob("*.compact"))
    paths.update((root / "examples").glob("*.compact"))
    paths.update((root / "test-center/test-contracts").glob("*.compact"))
    if ORACLE_MANIFEST.is_file() and root.resolve() == ROOT.resolve():
        manifest = json.loads(ORACLE_MANIFEST.read_text())
        paths.update(root / item["source"] for item in manifest["fixtures"])
    scope = positive_scope(root)
    if scope:
        paths.update(root / item["source"] for item in scope["positive_sources"])
    for directory in extra_dirs or []:
        directory = directory if directory.is_absolute() else root / directory
        if not directory.is_dir() or not directory.resolve().is_relative_to(root.resolve()):
            raise ValueError(f"invalid source directory: {directory}")
        paths.update(directory.rglob("*.compact"))
    return [root / name for name in sorted(relative_source(path, root) for path in paths)]


def parse_source(path: Path, root: Path) -> dict:
    source = without_comments(path.read_text())
    relative = relative_source(path, root)
    modules = []
    declarations = []
    for match in DECLARATION.finditer(source):
        kind = match.group("kind")
        end, terminator = declaration_end(source, match.end())
        tail = source[match.end():end].strip()
        name_match = NAME.match(tail)
        if kind != "constructor" and not name_match:
            raise ValueError(f"{relative}:{source.count(chr(10), 0, match.start()) + 1}: missing name")
        name = "constructor" if kind == "constructor" else name_match.group()
        signature = normalized(source[match.start():end])
        declarations.append({
            "kind": kind,
            "name": name,
            "signature": signature,
            "explicit_export": bool(match.group("export")),
            "declared_pure": bool(match.group("pure")),
            "line": source.count("\n", 0, match.start()) + 1,
            "offset": match.start(),
        })
        if kind == "module" and terminator == "{":
            modules.append((match.start(), closing_brace(source, end), name))
    modules.sort()
    named_exports = {name.strip() for match in EXPORT_LIST.finditer(source)
                     if not any(module[0] < match.start() < module[1] for module in modules)
                     for name in match.group(1).split(",")}
    for item in declarations:
        enclosing = [module for module in modules if module[0] < item["offset"] < module[1]]
        item["module_path"] = [module[2] for module in enclosing]
        if item["name"] in named_exports or (item["explicit_export"] and not enclosing):
            item["visibility"] = "export"
        elif item["explicit_export"]:
            item["visibility"] = "module_export"
        else:
            item["visibility"] = "internal"
        del item["explicit_export"]
        del item["offset"]
    imports = [{"kind": match.group(1), "expression": normalized(match.group(2))}
               for match in IMPORT.finditer(source)]
    return {
        "source": relative,
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "named_exports": sorted(named_exports),
        "imports": imports,
        "declarations": declarations,
    }


def row_identity(row: dict) -> dict:
    return {key: row[key] for key in ("source", "module_path", "kind", "name", "signature", "visibility")}


def identity_key(row: dict) -> str:
    return json.dumps(row_identity(row), sort_keys=True, separators=(",", ":"))


def baseline_rows(rows: list[dict]) -> list[dict]:
    return [row_identity(row) for row in sorted(rows, key=identity_key)]


def sha256_file(path: Path) -> str | None:
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None


def constant_from_source(path: Path, name: str) -> int | None:
    if not path.is_file():
        return None
    match = re.search(rf"\b{name}\s*:\s*u32\s*=\s*(\d+)", path.read_text())
    return int(match.group(1)) if match else None


def git_head(root: Path) -> str | None:
    result = subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD"],
                            capture_output=True, text=True, check=False)
    return result.stdout.strip() if result.returncode == 0 else None


def upstream_packages(lock_path: Path) -> dict:
    if not lock_path.is_file():
        return {}
    selected = {"midnight-ledger", "midnight-zk-stdlib", "midnight-zkir"}
    lock = tomllib.loads(lock_path.read_text())
    return {package["name"]: {key: package[key] for key in ("version", "source", "checksum")
                              if key in package}
            for package in lock.get("package", []) if package.get("name") in selected}


def receipt_metadata(root: Path, compiler: Path | None, contracts: list[dict]) -> dict:
    ir_file = root / "tools/compact-rust-backend/src/ir.rs"
    runtime_file = root / "runtime-rs/src/lib.rs"
    lock_file = root / "Cargo.lock"
    source_hashes = [{"source": contract["source"], "sha256": contract["sha256"]}
                     for contract in contracts]
    return {
        "git_head": git_head(root),
        "compiler": {"path": str(compiler), "sha256": sha256_file(compiler)} if compiler else None,
        "rust_ir_schema": constant_from_source(ir_file, "SCHEMA_VERSION"),
        "rust_runtime_abi": constant_from_source(runtime_file, "RUST_RUNTIME_ABI"),
        "ir_file_sha256": sha256_file(ir_file),
        "runtime_file_sha256": sha256_file(runtime_file),
        "cargo_lock_sha256": sha256_file(lock_file),
        "source_manifest_sha256": hashlib.sha256(
            json.dumps(source_hashes, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest(),
        "positive_scope_sha256": sha256_file(POSITIVE_SOURCE_MANIFEST)
        if root.resolve() == ROOT.resolve() else None,
        "positive_cohort_manifest_sha256s": {
            path.name: sha256_file(path) for path in POSITIVE_SOURCE_MANIFESTS
        } if root.resolve() == ROOT.resolve() else None,
        "upstream_packages": upstream_packages(lock_file),
    }


def compile_capabilities(compiler: Path, source: Path, root: Path) -> tuple[dict, dict]:
    with tempfile.TemporaryDirectory(prefix="compact-parity-inventory-") as output:
        command = [str(compiler), "--target", "rust", "--skip-zk", "--rust-runtime-root",
                   str(root), str(source), output]
        result = subprocess.run(command, cwd=root, capture_output=True, text=True, check=False)
        if result.returncode:
            raise RuntimeError(f"{source.relative_to(root)}: compiler failed: {result.stderr.strip()}")
        report = Path(output) / "contract/rust-capabilities.json"
        contract_info = Path(output) / "compiler/contract-info.json"
        if not report.is_file():
            raise RuntimeError(f"{source.relative_to(root)}: missing Rust capability report")
        if not contract_info.is_file():
            raise RuntimeError(f"{source.relative_to(root)}: missing compiler contract-info")
        return json.loads(report.read_text()), json.loads(contract_info.read_text())


def included_sources(source: str, by_source: dict[str, dict], root: Path) -> set[str]:
    """Resolve the scanned local include closure for a compiled package root."""
    seen = set()
    pending = [source]
    while pending:
        current = pending.pop()
        if current in seen:
            continue
        seen.add(current)
        for item in by_source[current]["imports"]:
            if item["kind"] != "include":
                continue
            path = re.fullmatch(r'"([^"\n]+)"', item["expression"])
            if path is None:
                raise ValueError(f"{current}: include path is not a quoted local path")
            included = Path(path.group(1))
            if included.is_absolute() or included.suffix not in ("", ".compact"):
                raise ValueError(f"{current}: invalid local include path: {included}")
            if not included.suffix:
                included = included.with_suffix(".compact")
            target = relative_source(root / Path(current).parent / included, root)
            if target not in by_source:
                raise ValueError(f"{current}: included source is outside the inventory: {target}")
            pending.append(target)
    return seen


def make_inventory(root: Path, extra_dirs: list[Path], compiler: Path | None,
                   only: set[str] | None = None) -> dict:
    paths = source_paths(root, extra_dirs)
    if only is not None:
        selected = set(only)
        package_roots = selected & COMPILED_PACKAGE_ROOTS
        if package_roots:
            all_sources = {relative_source(path, root): parse_source(path, root) for path in paths}
            for package in package_roots:
                selected.update(included_sources(package, all_sources, root))
        paths = [path for path in paths if relative_source(path, root) in selected]
    contracts = [parse_source(path, root) for path in paths]
    scope = positive_scope(root)
    expected_proof = {(entry["source"], circuit["name"]): circuit["proof"]
                      for entry in scope["positive_sources"] for circuit in entry["proof_circuits"]} if scope else {}
    rows = [{"source": contract["source"], **item,
             "ts_source_declaration": True, "rust_recorded": None, "rust_observed_call": None,
             "proof_required": None, "compiler_pure": None, "rust_recording_status": None,
             "compiler_metadata_source": None,
             "ts_expected_proof": expected_proof.get((contract["source"], item["name"]))}
            for contract in contracts for item in contract["declarations"]]
    by_source = {contract["source"]: contract for contract in contracts}
    unmatched = []
    missing_compiler_proof_rows = []
    compiled = []
    if compiler:
        acceptance_rust_success = {entry["source"] for entry in scope["positive_sources"]
                                   if entry["expected_rust"] == "success"} if scope else set()
        roots = [path for path in paths if path.parent == root / "examples/rust_backend"
                 or relative_source(path, root) in COMPILED_PACKAGE_ROOTS
                 or relative_source(path, root) in acceptance_rust_success]
        for path in roots:
            source = relative_source(path, root)
            report, contract_info = compile_capabilities(compiler, path, root)
            if report.get("schema_version") != 3:
                raise ValueError(f"{source}: expected Rust capability schema 3")
            compiled.append(source)
            by_source[source]["rust_capability_schema"] = report.get("schema_version")
            compiler_circuits = contract_info.get("circuits")
            if not isinstance(compiler_circuits, list):
                raise ValueError(f"{source}: compiler contract-info has no circuits array")
            metadata_by_name = {}
            for item in compiler_circuits:
                name = item.get("name")
                if not isinstance(name, str) or not name:
                    raise ValueError(f"{source}: compiler proof row has no name")
                if name in metadata_by_name:
                    raise ValueError(f"{source}: ambiguous compiler proof rows for {name}")
                metadata_by_name[name] = item
            provenance = included_sources(source, by_source, root) \
                if source in COMPILED_PACKAGE_ROOTS else {source}
            candidates = [row for row in rows if row["kind"] == "circuit"
                          and row["visibility"] == "export"
                          and (row["source"] == source
                               or (row["source"] in provenance and row["declared_pure"]))]
            candidates_by_name = {}
            for row in candidates:
                if row["name"] in candidates_by_name:
                    raise ValueError(f"{source}: ambiguous included declaration for {row['name']}")
                candidates_by_name[row["name"]] = row
            if source in COMPILED_PACKAGE_ROOTS:
                for name in sorted(metadata_by_name.keys() - candidates_by_name.keys()):
                    raise ValueError(f"{source}: compiler proof row has no included declaration: {name}")
            for row in candidates:
                metadata = metadata_by_name.get(row["name"])
                if metadata is None:
                    if source in COMPILED_PACKAGE_ROOTS:
                        raise ValueError(f"{source}: included declaration has no compiler proof row: {row['name']}")
                    missing_compiler_proof_rows.append({
                        "source": row["source"], "name": row["name"],
                        "module_path": row["module_path"],
                        "declared_pure": row["declared_pure"],
                    })
                    continue
                if type(metadata.get("proof")) is not bool or type(metadata.get("pure")) is not bool:
                    raise ValueError(f"{source}: invalid compiler proof/pure metadata for {row['name']}")
                if source in COMPILED_PACKAGE_ROOTS and (
                    row["declared_pure"] is not metadata["pure"]
                    or (row["source"] != source and metadata["proof"])
                ):
                    raise ValueError(f"{source}: included declaration disagrees with compiler metadata for {row['name']}")
                if row["compiler_metadata_source"] is not None:
                    raise ValueError(f"{source}: declaration already attributed by another compiler root: {row['name']}")
                row["proof_required"] = metadata["proof"]
                row["compiler_pure"] = metadata["pure"]
                row["compiler_metadata_source"] = source
                if not metadata["proof"]:
                    row["rust_recording_status"] = "not_applicable"
            for capability in report["circuits"]:
                matches = [row for row in rows if row["source"] == source
                           and row["kind"] == "circuit" and row["name"] == capability["name"]
                           and row["visibility"] == "export"]
                if len(matches) == 1:
                    if not isinstance(capability.get("proof_required"), bool):
                        raise ValueError(f"{source}: missing Boolean proof applicability for {capability['name']}")
                    expected_status = ("not_applicable" if not capability["proof_required"] else
                                       "available" if capability["recorded"] and capability["observed_call"] else
                                       "unavailable")
                    if capability.get("recording_status") != expected_status:
                        raise ValueError(f"{source}: invalid recording status for {capability['name']}")
                    if matches[0]["proof_required"] is not capability["proof_required"]:
                        raise ValueError(f"{source}: capability proof flag disagrees with contract-info for {capability['name']}")
                    matches[0]["rust_recorded"] = capability["recorded"]
                    matches[0]["rust_observed_call"] = capability["observed_call"]
                    matches[0]["rust_recording_status"] = expected_status
                else:
                    unmatched.append({"source": source, "name": capability["name"]})
    rows.sort(key=lambda row: (row["source"], row["line"], row["kind"], row["name"]))
    missing = Counter(row["source"] for row in rows if row["kind"] == "circuit"
                      and row["visibility"] == "export" and row["proof_required"] is True
                      and (row["rust_recorded"] is not True or row["rust_observed_call"] is not True))
    pure_detected = [match for path in paths
                     for match in PURE_DECLARATION.finditer(without_comments(path.read_text()))]
    pure_rows = [row for row in rows if row["kind"] == "circuit" and row["declared_pure"]]
    if len(pure_detected) != len(pure_rows):
        raise ValueError("pure circuit declarations were not all inventoried")
    return {
        "format_version": 1,
        "scope": "lexical repository declarations including pure and module exports; contract proof applicability requires compiler metadata; TS behavior parity requires executing tests",
        "contracts": contracts,
        "rows": rows,
        "positive_acceptance_scope": scope,
        "summary": {
            "sources": len(contracts),
            "declarations": len(rows),
            "exported_circuits": sum(row["kind"] == "circuit" and row["visibility"] == "export" for row in rows),
            "module_exported_circuits": sum(row["kind"] == "circuit" and row["visibility"] == "module_export"
                                            for row in rows),
            "compiled_rust_sources": len(compiled),
            "recorded_available": sum(row["rust_recorded"] is True for row in rows),
            "recorded_missing": sum(row["rust_recorded"] is False for row in rows),
            "proof_required": sum(row["proof_required"] is True for row in rows),
            "proof_available": sum(row["proof_required"] is True and row["rust_recorded"] is True
                                   and row["rust_observed_call"] is True for row in rows),
            "proof_missing": sum(row["proof_required"] is True and
                                 (row["rust_recorded"] is not True or row["rust_observed_call"] is not True)
                                 for row in rows),
            "nonproof": sum(row["proof_required"] is False for row in rows),
            "unassessed_exported_circuits": sum(row["kind"] == "circuit" and row["visibility"] == "export"
                                                and row["proof_required"] is None for row in rows),
            "declared_pure_circuits": len(pure_rows),
            "exported_declared_pure_circuits": sum(row["visibility"] == "export" for row in pure_rows),
            "module_exported_declared_pure_circuits": sum(row["visibility"] == "module_export"
                                                          for row in pure_rows),
            "known_lexical_pure_omissions": 0,
            "known_exported_pure_omissions": 0,
            "ranked_missing_sources": [{"source": source, "count": count}
                                       for source, count in sorted(missing.items(), key=lambda pair: (-pair[1], pair[0]))],
            "unmatched_compiler_circuits": unmatched,
            "missing_compiler_proof_rows": missing_compiler_proof_rows,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--source-dir", type=Path, action="append", default=[],
                        help="additional repository source tree (repeatable)")
    parser.add_argument("--compiler", type=Path, help="compactc binary for Rust capability reports")
    parser.add_argument("--only", action="append", help="repository-relative source filter; compiled package roots include their local source closure (repeatable)")
    parser.add_argument("--baseline", type=Path, help="compare identities to a baseline (defaults to checked-in baseline for full repository inventory)")
    parser.add_argument("--no-baseline", action="store_true", help="skip the default checked-in baseline")
    parser.add_argument("--write-baseline", type=Path, help="write identity rows for review/check-in")
    parser.add_argument("--output", type=Path, help="write JSON receipt instead of stdout")
    parser.add_argument("--receipt-metadata", action="store_true",
                        help="attach exact local compiler/source/ABI/upstream package identifiers")
    parser.add_argument("--require-full", action="store_true",
                        help="fail if a proof-required exported circuit lacks recording or any exported circuit is unassessed")
    args = parser.parse_args()
    try:
        root = args.root.resolve()
        compiler = args.compiler.resolve() if args.compiler else None
        if compiler and not compiler.is_file():
            raise ValueError(f"compiler does not exist: {compiler}")
        inventory = make_inventory(root, args.source_dir, compiler, set(args.only) if args.only else None)
        identities = baseline_rows(inventory["rows"])
        if args.receipt_metadata:
            inventory["receipt_metadata"] = receipt_metadata(root, compiler, inventory["contracts"])
        if args.write_baseline:
            args.write_baseline.write_text(json.dumps(identities, indent=2) + "\n")
        failure = False
        baseline = args.baseline
        if baseline is None and not args.no_baseline and not args.write_baseline \
                and root == ROOT and not args.source_dir and not args.only and DEFAULT_BASELINE.is_file():
            baseline = DEFAULT_BASELINE
        if baseline:
            previous = json.loads(baseline.read_text())
            if len(previous) != len({identity_key(row) for row in previous}):
                raise ValueError(f"duplicate identities in baseline: {baseline}")
            before = {identity_key(row): row for row in previous}
            after = {identity_key(row): row for row in identities}
            inventory["baseline_diff"] = {
                "added": [after[key] for key in sorted(after.keys() - before.keys())],
                "removed": [before[key] for key in sorted(before.keys() - after.keys())],
            }
            failure = bool(inventory["baseline_diff"]["added"] or inventory["baseline_diff"]["removed"])
        if args.require_full:
            failure |= bool(inventory["summary"]["proof_missing"]
                            or inventory["summary"]["unassessed_exported_circuits"]
                            or inventory["summary"]["unmatched_compiler_circuits"]
                            or inventory["summary"]["missing_compiler_proof_rows"])
            if not compiler:
                raise ValueError("--require-full requires --compiler")
            failure |= any(row["kind"] == "circuit" and row["visibility"] == "export"
                           and (row["proof_required"] is None
                                or row["proof_required"] is True
                                and (row["rust_recorded"] is not True or row["rust_observed_call"] is not True))
                           for row in inventory["rows"])
        receipt = json.dumps(inventory, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.write_text(receipt)
        else:
            sys.stdout.write(receipt)
        return 1 if failure else 0
    except (ValueError, RuntimeError, OSError, KeyError, json.JSONDecodeError) as error:
        print(f"parity inventory: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
