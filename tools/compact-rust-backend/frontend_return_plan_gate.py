#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Source-to-IR return-plan regressions and optional fresh-source compatibility census.

Both frontends use the exact same supplied Rust CLI/backend. --source-manifest
accepts a retained census with rows[].source; it never reuses the retained IR.
This gate measures compilation and reported capabilities, not runtime/proof parity.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = "tools/compact-rust-backend/tests/frontend-return-plans"


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def nodes(value):
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from nodes(child)
    elif isinstance(value, list):
        for child in value:
            yield from nodes(child)


def compile_source(compiler, scheme, source, output):
    output.mkdir(parents=True, exist_ok=True)
    command = [str(compiler), "--target", "rust", "--skip-zk",
               "--rust-runtime-root", str(ROOT), source, str(output)]
    result = subprocess.run(command, cwd=ROOT, env={**os.environ, "COMPACTC_SCHEME": str(scheme)},
                            capture_output=True, text=True)
    (output / "compiler.log").write_text(result.stdout + result.stderr)
    row = {"command": command, "scheme": str(scheme), "exit_code": result.returncode,
           "source_sha256": sha(ROOT / source)}
    if result.returncode == 0:
        for name, filename in [("ir", "compact-rust-ir.json"),
                               ("capabilities", "rust-capabilities.json")]:
            row[name] = json.loads((output / "contract" / filename).read_text())
        row["rust_sha256"] = sha(output / "contract/lib.rs")
    return row


def signature(row):
    return {c["name"]: {key: c.get(key) for key in ("recorded", "observed_call", "proof_required")}
            for c in row.get("capabilities", {}).get("circuits", [])}


def compare(source, baseline, candidate):
    errors = []
    if baseline["exit_code"] == 0:
        if candidate["exit_code"] != 0:
            errors.append("previously accepted source failed")
        else:
            before, after = signature(baseline), signature(candidate)
            for name, capabilities in before.items():
                if name not in after:
                    errors.append(f"missing circuit {name}")
                    continue
                for key in ("recorded", "observed_call"):
                    if capabilities[key] and not after[name][key]:
                        errors.append(f"{name}: lost {key}")
                if capabilities["proof_required"] != after[name]["proof_required"]:
                    errors.append(f"{name}: changed proof requirement")
    return {"source": source, "baseline_exit": baseline["exit_code"],
            "candidate_exit": candidate["exit_code"], "errors": errors,
            "ir_identical": baseline.get("ir") == candidate.get("ir"),
            "rust_identical": baseline.get("rust_sha256") == candidate.get("rust_sha256"),
            "baseline_capabilities": signature(baseline), "candidate_capabilities": signature(candidate)}


def regression_errors(source, baseline, candidate):
    if candidate["exit_code"]:
        return ["regression fixture did not compile"]
    circuits = {c["name"]: c for c in candidate["ir"]["stateful_circuits"]}
    errors = []
    if source.endswith("value_effects.compact"):
        if baseline["exit_code"] or baseline.get("ir") != candidate["ir"]:
            errors.append("expression-capable effects changed representation")
    else:
        for name, circuit in circuits.items():
            if circuit["return_value"]["kind"] != "effectful" or circuit["actions"]:
                errors.append(f"{name}: storage mutation not retained in ordered return plan")
    if source.endswith("storage_mutations.compact"):
        expected = {"nested": "counter_increment", "insert": "set_insert",
                    "remove": "set_remove", "write_coin": "cell_write_coin"}
        for name, kind in expected.items():
            if not any(n.get("kind") == kind for n in nodes(circuits[name])):
                errors.append(f"{name}: missing action {kind}")
    if source.endswith("checked_subtraction.compact"):
        expected_subtraction = {"kind": "unsigned_subtract", "max": "65535",
                                "left": {"kind": "parameter", "name": "value"},
                                "right": {"kind": "unsigned_literal", "value": "1", "max": "65535"}}
        for name, explicit in (("mutate_then_subtract", False), ("explicit_guard_then_subtract", True)):
            circuit = circuits.get(name, {})
            if circuit.get("result") != {"kind": "unsigned", "max": "65535"}:
                errors.append(f"{name}: missing Uint16 result")
            body = circuit.get("return_value", {}).get("body", {})
            if body.get("kind") != "conditional":
                errors.append(f"{name}: missing selected return plan")
            selected = body.get("then", {})
            arithmetic = [n for n in nodes(selected) if n.get("kind") == "unsigned_subtract"]
            if arithmetic != [expected_subtraction]:
                errors.append(f"{name}: checked Uint16 subtraction or operands changed")
            events = [n["kind"] for n in nodes(selected)
                      if n.get("kind") in ("assert", "counter_increment", "unsigned_subtract")]
            expected_events = (["assert"] if explicit else []) + ["counter_increment", "unsigned_subtract"]
            if events != expected_events:
                errors.append(f"{name}: assertion/mutation/subtraction order changed")
            assertions = [n for n in nodes(selected) if n.get("kind") == "assert"]
            if explicit and (len(assertions) != 1 or assertions[0].get("message") != "explicit guard before mutation"):
                errors.append(f"{name}: explicit user assertion lost or replaced")
    if source.endswith("effect_plan_counter_only.compact"):
        if len(candidate["ir"]["ledger_fields"]) != 2:
            errors.append("Counter-only source gained artificial ledger fields")
        if not signature(candidate).get("adjust", {}).get("recorded"):
            errors.append("Counter-only adjust lost recording")
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", type=Path, required=True)
    parser.add_argument("--scheme", type=Path, required=True)
    parser.add_argument("--baseline-scheme", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--source-manifest", type=Path)
    args = parser.parse_args()
    for name in ("compiler", "scheme", "baseline_scheme", "output"):
        setattr(args, name, getattr(args, name).resolve())
    args.output.mkdir(parents=True, exist_ok=True)
    regressions = [f"{FIXTURES}/{name}.compact" for name in
                   ("storage_mutations", "value_effects", "checked_subtraction")]
    regressions.append("examples/rust_backend/effect_plan_counter_only.compact")
    census = []
    if args.source_manifest:
        census = [r["source"] for r in json.loads(args.source_manifest.read_text())["rows"]]
        if not census or len(set(census)) != len(census):
            raise ValueError("source census must contain unique nonempty rows")
    sources = list(dict.fromkeys(regressions + census))
    rows = []
    started = time.monotonic()
    for index, source in enumerate(sources):
        if Path(source).is_absolute() or ".." in Path(source).parts:
            raise ValueError("source must be a repository-relative path")
        directory = args.output / f"{index:03d}"
        baseline = compile_source(args.compiler, args.baseline_scheme, source, directory / "baseline")
        candidate = compile_source(args.compiler, args.scheme, source, directory / "candidate")
        row = compare(source, baseline, candidate)
        if source in regressions:
            row["errors"] += regression_errors(source, baseline, candidate)
        if source in census and baseline["exit_code"]:
            row["errors"].append("census baseline source failed; comparison is incomplete")
        for label, data in (("baseline", baseline), ("candidate", candidate)):
            (directory / f"{label}.json").write_text(json.dumps(data, indent=2) + "\n")
        rows.append(row)
    report = {"claim": "fresh source compilation and capability preservation, not runtime/proof coverage",
              "compiler_sha256": sha(args.compiler), "scheme_sha256": sha(args.scheme),
              "baseline_scheme_sha256": sha(args.baseline_scheme), "census_count": len(census),
              "regression_count": len(regressions), "elapsed_seconds": round(time.monotonic()-started, 2),
              "passed": all(not r["errors"] for r in rows), "rows": rows}
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({k: v for k, v in report.items() if k != "rows"}, indent=2))
    for row in rows:
        if row["errors"]:
            print(row["source"], row["errors"])
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
