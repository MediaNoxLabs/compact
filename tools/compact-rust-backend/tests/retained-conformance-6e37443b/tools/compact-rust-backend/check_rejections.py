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

"""Ensure the public Rust compiler target rejects unsupported programs.

The positive fixture checker cannot detect an unsupported construct that
quietly emits a plausible Rust library. This gate pins source-level
refusals and verifies that no generated Cargo library survives. It also
checks that runtime validation and failed rebuilds preserve output, including
recovery of a previously complete directory after an interrupted replacement.
"""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time


CASES = {
    "unknown-opaque": (
        'import CompactStandardLibrary;\n'
        'export ledger thing: Opaque<"NotAThing">;\n'
        'constructor() {}\n',
        "Rust backend does not yet support this opaque type",
        None,
    ),
    "field-to-uint": (
        'export ledger n: Uint<64>;\n'
        'export circuit narrow(f: Field): Uint<64> { return f as Uint<64>; }\n',
        "Rust backend does not yet support Field-to-Uint downcasts",
        None,
    ),
    "vector-spread": (
        'export pure circuit spread(v: Vector<2, Field>): Vector<2, Field> {\n'
        '  return [...v];\n'
        '}\n',
        "Rust backend does not yet support vector spreads",
        (2, 10),
    ),
    "tuple-spread": (
        'export pure circuit pair(v: [Field, Field]): [Field, Field] {\n'
        '  return [...v];\n'
        '}\n',
        "Rust backend does not yet support tuple spreads",
        (2, 10),
    ),
}

ROOT = Path(__file__).resolve().parents[2]


# Cast cases are valid TypeScript programs. Unknown opaque types deliberately
# exercise a shared target refusal; they are not Rust-only language gaps.
CONTEXT_CASES = {
    "nested": (
        'export pure circuit narrow(f: Field): Uint<64> {\n'
        '  return (true ? f : 1) as Uint<64>;\n}\n',
        "nested.compact", (2, 10), False,
    ),
    "constructor": (
        'export ledger n: Uint<64>;\nconstructor(f: Field) {\n'
        '  n = disclose(f as Uint<64>);\n}\n',
        "constructor.compact", (3, 16), False,
    ),
    "imported": (
        'import "broken";\n'
        'export pure circuit value(f: Field): Uint<64> { return narrow(f); }\n',
        "broken.compact", (3, 10), False,
    ),
    "witness": (
        'witness wrong(): Opaque<"NotAThing">;\n'
        'export circuit value(): Opaque<"NotAThing"> { return disclose(wrong()); }\n',
        "witness.compact", (1, 18), True,
    ),
    "struct": (
        'export struct Broken {\n  value: Opaque<"NotAThing">;\n}\n',
        "struct.compact", (2, 10), True,
    ),
    "ledger": (
        'import CompactStandardLibrary;\n'
        'export ledger bad: Map<Field, Opaque<"NotAThing">>;\nconstructor() {}\n',
        "ledger.compact", (2, 31), True,
    ),
}


def check_context_locations(compactc: str, directory: Path) -> list[str]:
    """Pin definition locations and atomic refusal across six source contexts."""
    failures = []
    directory = directory / "contexts"
    directory.mkdir()
    (directory / "broken.compact").write_text(
        "module broken {\nexport pure circuit narrow(f: Field): Uint<64> {\n"
        "  return f as Uint<64>;\n}\n}\n"
    )

    def compile_source(source: Path, output: Path, target: str):
        return subprocess.run(
            [compactc, "--target", target, "--skip-zk", str(source), str(output)],
            capture_output=True, text=True, check=False,
        )

    preserved = directory / "preserved"
    baseline = compile_source(ROOT / "examples/rust_backend/counter.compact", preserved, "rust")
    if baseline.returncode or not (preserved / "contract/lib.rs").is_file():
        return [f"context preservation baseline failed:\n{baseline.stderr}"]
    before = snapshot(preserved)
    for name, (source, defining_file, (line, column), opaque) in CONTEXT_CASES.items():
        source_path = directory / f"{name}.compact"
        source_path.write_text(source)
        position = f"{defining_file} line {line} char {column}"
        control = compile_source(source_path, directory / f"{name}-ts", "ts")
        if opaque:
            if (control.returncode == 0 or position not in control.stderr
                    or "opaque type NotAThing is not supported" not in control.stderr):
                failures.append(f"{name}: shared type refusal control changed:\n{control.stderr}")
        elif control.returncode or not (directory / f"{name}-ts/contract/index.js").is_file():
            failures.append(f"{name}: valid TypeScript control failed:\n{control.stderr}")
        diagnostic = "Rust backend does not yet support " + (
            "this opaque type" if opaque else "Field-to-Uint downcasts"
        )
        for output in (directory / name, preserved):
            rejected = compile_source(source_path, output, "rust")
            if (rejected.returncode == 0 or position not in rejected.stderr
                    or diagnostic not in rejected.stderr):
                failures.append(f"{name}: missing exact Rust source diagnostic:\n{rejected.stderr}")
            if any(directory.glob(f".{output.name}.compactc-stage-*")):
                failures.append(f"{name}: refusal left staging debris")
            if output == preserved:
                if snapshot(preserved) != before:
                    failures.append(f"{name}: refusal changed previously complete output")
            elif output.exists():
                failures.append(f"{name}: fresh refusal published output")
        print(f"Context {name}: {position}; TS {'shared refusal' if opaque else 'accepted'}")
    return failures



# These source constructs are checked by the shared frontend before either
# backend runs. Field is a finite non-Boolean control, not all possible types.
BOOLEAN_TYPE_REJECTIONS = {
    "and-left": ("f && b", "expected test to have type Boolean, received Field", 10),
    "and-right": ("b && f", "mismatch between type Field and type Boolean of condition branches", 10),
    "or-left": ("f || b", "expected test to have type Boolean, received Field", 10),
    "or-right": ("b || f", "mismatch between type Boolean and type Field of condition branches", 10),
    "not-field": ("!f", "expected test to have type Boolean, received Field", 10),
    "conditional-field-guard": ("f ? b : false", "expected test to have type Boolean, received Field", 10),
    "and-literal-skip": ("false && f", "mismatch between type Field and type Boolean of condition branches", 10),
    "or-literal-skip": ("true || f", "mismatch between type Boolean and type Field of condition branches", 10),
    "conditional-invalid-alternate": ("true ? b : f", "mismatch between type Boolean and type Field of condition branches", 10),
    "conditional-invalid-consequent": ("false ? f : b", "mismatch between type Field and type Boolean of condition branches", 10),
    "conditional-bad-unselected-alternate": ("true ? b : !f", "expected test to have type Boolean, received Field", 21),
    "conditional-bad-unselected-consequent": ("false ? !f : b", "expected test to have type Boolean, received Field", 18),
}
BOOLEAN_TYPE_CONTROLS = {
    "logical_and": "a && b", "logical_or": "a || b", "logical_not": "!a",
    "conditional": "a ? b : false", "and_literal_skip": "false && b",
    "or_literal_skip": "true || b", "valid_alternate": "true ? a : !b",
    "valid_consequent": "false ? !a : b",
}


def check_boolean_typing(compactc: str, directory: Path) -> list[str]:
    """Check finite source Boolean typing and both static arms for TS and Rust."""
    directory = directory / "boolean-typing"
    directory.mkdir()
    control_source = directory / "valid.compact"
    control_source.write_text("".join(
        f"export pure circuit {name}(a: Boolean, b: Boolean): Boolean {{\n"
        f"  return {expression};\n}}\n"
        for name, expression in BOOLEAN_TYPE_CONTROLS.items()
    ))
    failures = []
    for target in ("ts", "rust"):
        output = directory / f"valid-{target}"
        control = subprocess.run(
            [compactc, "--target", target, "--skip-zk", str(control_source), str(output)],
            capture_output=True, text=True, check=False,
        )
        artifacts = ("contract/index.js",) if target == "ts" else (
            "contract/lib.rs", "contract/Cargo.toml"
        )
        if control.returncode or not all((output / path).is_file() for path in artifacts):
            failures.append(f"Boolean {target}: valid controls failed:\n{control.stderr}")
        for name, (expression, diagnostic, column) in BOOLEAN_TYPE_REJECTIONS.items():
            source = directory / f"{name}.compact"
            source.write_text(
                "export pure circuit value(b: Boolean, f: Field): Boolean {\n"
                f"  return {expression};\n}}\n"
            )
            output = directory / f"{name}-{target}"
            rejected = subprocess.run(
                [compactc, "--target", target, "--skip-zk", str(source), str(output)],
                capture_output=True, text=True, check=False,
            )
            expected = f"Exception: {name}.compact line 2 char {column}:\n  {diagnostic}"
            if rejected.returncode == 0 or rejected.stderr.strip() != expected:
                failures.append(f"Boolean {name}/{target}: expected exact type refusal:\n"
                                f"{expected}\nreceived:\n{rejected.stderr}")
            if output.exists() or any(directory.glob(f".{output.name}.compactc-stage-*")):
                failures.append(f"Boolean {name}/{target}: refusal published output or staging debris")
        print(f"Boolean typing {target}: {len(BOOLEAN_TYPE_CONTROLS)} valid expressions, "
              f"{len(BOOLEAN_TYPE_REJECTIONS)} exact refusal expectations checked")
    return failures


def snapshot(path: Path) -> dict[str, str]:
    return {
        item.relative_to(path).as_posix(): hashlib.sha256(item.read_bytes()).hexdigest()
        for item in path.rglob("*") if item.is_file()
    }


def check_output_publication(compactc: str, directory: Path) -> list[str]:
    failures = []
    source = ROOT / "examples/rust_backend/counter.compact"
    output = directory / "publication"
    invalid_environment = os.environ.copy()
    invalid_environment["COMPACT_RUST_RUNTIME_DIR"] = str(directory / "missing-runtime")

    def run_compiler(
        environment: dict[str, str], input_source: Path = source,
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [compactc, "--target", "rust", "--skip-zk", str(input_source), str(output)],
            capture_output=True, text=True, env=environment, check=False,
        )

    def leaked_stage() -> bool:
        return any(directory.glob(f".{output.name}.compactc-stage-*"))

    rejected = run_compiler(invalid_environment)
    if rejected.returncode == 0 or "runtime source directory is invalid" not in rejected.stderr:
        failures.append(f"invalid runtime source was not rejected:\n{rejected.stderr}")
    if output.exists() or leaked_stage():
        failures.append("failed fresh compile published partial output or left staging debris")

    valid = run_compiler(os.environ.copy())
    if valid.returncode or not (output / "contract/lib.rs").is_file() or not (
        output / "contract/Cargo.toml"
    ).is_file():
        failures.append(f"valid output for replacement test was not complete:\n{valid.stderr}")
        return failures
    before = snapshot(output)
    rejected = run_compiler(invalid_environment)
    if rejected.returncode == 0 or snapshot(output) != before or leaked_stage():
        failures.append("failed rebuild changed previously complete output or left staging debris")

    (output / "stale-marker").write_text("previous generated directory")
    valid = run_compiler(os.environ.copy())
    if valid.returncode or (output / "stale-marker").exists() or leaked_stage():
        failures.append(f"successful rebuild did not replace output cleanly:\n{valid.stderr}")

    preserved = snapshot(output)
    interrupted_backup = directory / f".{output.name}.compactc-stage-42-123-0-previous"
    output.rename(interrupted_backup)
    interrupted_files = snapshot(directory)
    interrupted_entries = set(directory.iterdir())
    rejected = run_compiler(invalid_environment)
    if (
        rejected.returncode == 0
        or "runtime source directory is invalid" not in rejected.stderr
        or output.exists()
        or not interrupted_backup.is_dir()
        or snapshot(directory) != interrupted_files
        or set(directory.iterdir()) != interrupted_entries
    ):
        failures.append("invalid runtime selection mutated interrupted output before recovery")

    # Runtime selection precedes recovery; a frontend refusal with valid runtime
    # sources reaches recovery and then exercises failed staging cleanup.
    invalid_source = directory / "publication-invalid.compact"
    invalid_source.write_text(
        "export pure circuit value(f: Field): Boolean {\n"
        "  return !f;\n}\n"
    )
    rejected = run_compiler(os.environ.copy(), invalid_source)
    expected = (
        "Exception: publication-invalid.compact line 2 char 10:\n"
        "  expected test to have type Boolean, received Field"
    )
    if (
        rejected.returncode == 0
        or rejected.stderr.strip() != expected
        or not output.is_dir()
        or snapshot(output) != preserved
        or interrupted_backup.exists()
        or leaked_stage()
    ):
        failures.append("interrupted replacement was not recovered before failed rebuild")

    file_output = directory / "output-file"
    file_output.write_text("keep this file")
    link_output = directory / "output-link"
    link_output.symlink_to(output, target_is_directory=True)
    for candidate in (file_output, link_output):
        result = subprocess.run(
            [compactc, "--target", "rust", "--skip-zk", str(source), str(candidate)],
            capture_output=True, text=True, env=os.environ.copy(), check=False,
        )
        if result.returncode == 0 or "file or symlink" not in result.stderr:
            failures.append(f"{candidate.name}: unsafe output root was accepted:\n{result.stderr}")
    if file_output.read_text() != "keep this file" or not link_output.is_symlink():
        failures.append("unsafe output root check changed an existing file or symlink")
    if snapshot(output) != preserved:
        failures.append("unsafe output root check changed the valid generated directory")
    return failures


def check_output_serialization(compactc: str, directory: Path) -> list[str]:
    failures = []
    source = ROOT / "examples/rust_backend/counter.compact"
    output = directory / "serialized-output"
    trace = directory / "scheme-starts.txt"
    release = directory / "release-first-scheme"
    scheme = directory / "blocking-scheme.py"
    scheme.write_text(
        "#!/usr/bin/env python3\n"
        "import os, sys, time\n"
        "from pathlib import Path\n"
        "trace = Path(os.environ['COMPACT_LOCK_TRACE'])\n"
        "with trace.open('a') as output:\n"
        "    output.write(str(os.getpid()) + '\\n')\n"
        "if len(trace.read_text().splitlines()) == 1:\n"
        "    release = Path(os.environ['COMPACT_LOCK_RELEASE'])\n"
        "    while not release.exists():\n"
        "        time.sleep(0.02)\n"
        "sys.exit(7)\n"
    )
    scheme.chmod(0o755)
    environment = os.environ.copy()
    environment.update({
        "COMPACTC_SCHEME": str(scheme),
        "COMPACT_LOCK_TRACE": str(trace),
        "COMPACT_LOCK_RELEASE": str(release),
    })
    command = [compactc, "--target", "rust", "--skip-zk", str(source), str(output)]
    first = subprocess.Popen(command, env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    second = None
    try:
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline and not trace.exists():
            if first.poll() is not None:
                break
            time.sleep(0.05)
        if not trace.exists():
            failures.append("first compiler did not enter the Scheme phase")
            return failures
        second = subprocess.Popen(command, env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline and second.poll() is None:
            if len(trace.read_text().splitlines()) > 1:
                failures.append("second compiler entered Scheme before the first released its output lock")
                break
            time.sleep(0.05)
        if second.poll() is not None:
            failures.append("second compiler exited before the first released its output lock")
        release.touch()
        first.communicate(timeout=30)
        if first.returncode != 7:
            failures.append("first compiler did not report the injected Scheme failure")
        second.communicate(timeout=30)
        if second.returncode != 7:
            failures.append("second compiler did not run after the first released its output lock")
        if len(trace.read_text().splitlines()) != 2 or output.exists() or any(
            directory.glob(f".{output.name}.compactc-stage-*")
        ):
            failures.append("serialized compiler runs left output/staging debris or skipped the second run")
    except subprocess.TimeoutExpired:
        failures.append("serialized compiler runs timed out")
    finally:
        release.touch()
        for process in (first, second):
            if process is not None and process.poll() is None:
                process.kill()
                process.communicate()
    return failures


def check_proof_capabilities(compactc: str, directory: Path) -> list[str]:
    failures = []
    source = directory / "pure-call.compact"
    # The recorder admits only a typed two-Field pair hash. A three-Field
    # vector remains a valid native call and a strict recording rejection.
    source.write_text(
        "import CompactStandardLibrary;\n"
        "export ledger value: Field;\n"
        "pure circuit sumVec(v: Vector<3, Field>): Field { "
        "return transientHash<Vector<3, Field>>(v); }\n"
        "export circuit write(input: Vector<3, Field>): [] { value = disclose(sumVec(input)); }\n"
    )
    output = directory / "pure-call-output"

    def compile_source(path: Path, destination: Path, strict: bool) -> subprocess.CompletedProcess[str]:
        command = [compactc, "--target", "rust", "--skip-zk"]
        if strict:
            command.append("--rust-require-recording")
        return subprocess.run(
            [*command, str(path), str(destination)],
            capture_output=True, text=True, check=False,
        )

    default = compile_source(source, output, False)
    if default.returncode:
        return [f"native-only pure call did not compile normally:\n{default.stderr}"]
    report_path = output / "contract/rust-capabilities.json"
    manifest_path = output / "compiler/contract-manifest.json"
    try:
        report_bytes = report_path.read_bytes()
        report = json.loads(report_bytes)
        manifest = json.loads(manifest_path.read_bytes())
        assert report["schema_version"] == 3
        assert len(report["circuits"]) == 1
        capability = report["circuits"][0]
        assert {key: capability[key] for key in ("name", "source", "recorded", "observed_call")} == {
            "name": "write",
            "source": {"file": "pure-call.compact", "line": 4, "column": 1},
            "recorded": False,
            "observed_call": False,
        }
        assert capability["recording_unavailable"]["code"] == "unsupported_expression"
        assert capability["recording_unavailable"]["path"] == "actions[0].bindings[0].value"
        assert capability["observed_call_unavailable"]["code"] == "recording_unavailable"
        assert manifest["contract"]["rust-capabilities.json"]["hash"] == hashlib.sha256(report_bytes).hexdigest()
    except (AssertionError, FileNotFoundError, KeyError, ValueError) as error:
        failures.append(f"native-only pure-call report or manifest is wrong: {error}")
    before = snapshot(output)
    rejected = compile_source(source, output, True)
    if (rejected.returncode == 0
            or "pure-call.compact line 4 char 1" not in rejected.stderr
            or "unsupported_expression at actions[0].bindings[0].value" not in rejected.stderr):
        failures.append(f"strict rebuild did not reject at source:\n{rejected.stderr}")
    if snapshot(output) != before:
        failures.append("strict rebuild changed the prior native-only output")
    fresh = directory / "pure-call-strict-output"
    rejected = compile_source(source, fresh, True)
    if rejected.returncode == 0 or fresh.exists() or any(directory.glob(f".{fresh.name}.compactc-stage-*")):
        failures.append("strict fresh rejection published output or left staging debris")

    for name in ("counter", "cell_boolean"):
        path = ROOT / f"examples/rust_backend/{name}.compact"
        destination = directory / f"{name}-strict-output"
        accepted = compile_source(path, destination, True)
        if accepted.returncode:
            failures.append(f"strict {name} compile failed:\n{accepted.stderr}")
            continue
        report = json.loads((destination / "contract/rust-capabilities.json").read_text())
        if not report["circuits"] or any(
            not item["recorded"] or not item["observed_call"] for item in report["circuits"]
        ):
            failures.append(f"strict {name} report overstates proving support")

    collision_source = directory / "call-collision.compact"
    collision_source.write_text(
        "import CompactStandardLibrary;\n"
        "export ledger round: Counter;\n"
        "export circuit bump(): [] { round.increment(1); }\n"
        "export circuit bump_call(): [] { round.increment(1); }\n"
    )
    collision_output = directory / "call-collision-output"
    accepted = compile_source(collision_source, collision_output, False)
    if accepted.returncode:
        failures.append(f"call-name collision did not compile normally:\n{accepted.stderr}")
    else:
        report = json.loads((collision_output / "contract/rust-capabilities.json").read_text())
        actual = [(item["name"], item["recorded"], item["observed_call"]) for item in report["circuits"]]
        if actual != [("bump", True, False), ("bump_call", True, True)]:
            failures.append(f"call-name collision capability report is wrong: {actual}")
        if report["circuits"][0]["observed_call_unavailable"]["code"] != "name_collision":
            failures.append("call-name collision has no typed reason")
        before = snapshot(collision_output)
        rejected = compile_source(collision_source, collision_output, True)
        if (rejected.returncode == 0
                or "call-collision.compact line 3 char 1" not in rejected.stderr
                or "name_collision at name" not in rejected.stderr):
            failures.append(f"strict call-name collision did not reject at source:\n{rejected.stderr}")
        if snapshot(collision_output) != before:
            failures.append("strict call-name collision changed the prior output")
    for name in ("boolean_logic", "witness_ledger_cell"):
        source = ROOT / "examples/rust_backend" / f"{name}.compact"
        accepted = compile_source(source, directory / f"{name}-proof-aware", True)
        if accepted.returncode:
            failures.append(f"proof-false {name} failed strict recording:\n{accepted.stderr}")
    checked_value_only = directory / "checked-value-only.compact"
    checked_value_only.write_text(
        "import CompactStandardLibrary;\n"
        "witness echo(flag: Boolean): Boolean;\n"
        "export circuit checked_value(first: Boolean, second: Boolean, value: Field): Field {\n"
        '  assert(disclose(echo(first)), "first witness failed");\n'
        '  assert(disclose(echo(second)), "second witness failed");\n'
        "  return value;\n}"
    )
    accepted = compile_source(checked_value_only, directory / "checked-value-proof-aware", True)
    if accepted.returncode:
        failures.append(f"proof-false checked_value failed strict recording:\n{accepted.stderr}")
    else:
        report = json.loads((directory / "checked-value-proof-aware/contract/rust-capabilities.json").read_text())
        if [(item["name"], item["proof_required"], item["recording_status"])
                for item in report["circuits"]] != [("checked_value", False, "not_applicable")]:
            failures.append("proof-false checked_value has wrong schema-3 applicability")
    merkle = ROOT / "examples/rust_backend/merkle_path_verify.compact"
    accepted = compile_source(merkle, directory / "merkle-proof-aware", True)
    if accepted.returncode:
        failures.append(f"Merkle strict recording rejected supported circuits:\n{accepted.stderr}")
    emitted = compile_source(merkle, directory / "merkle-proof-report", False)
    if emitted.returncode:
        failures.append(f"Merkle proof capability report failed:\n{emitted.stderr}")
    else:
        report = json.loads((directory / "merkle-proof-report/contract/rust-capabilities.json").read_text())
        statuses = {item["name"]: (item["proof_required"], item["recording_status"])
                    for item in report["circuits"]}
        if statuses != {name: (True, "available") for name in ("append", "replace", "verify")}:
            failures.append(f"Merkle proof capability statuses are wrong: {statuses}")
    witness_assert = ROOT / "examples/rust_backend/assert_witness.compact"
    accepted = compile_source(witness_assert, directory / "assert-proof-aware", True)
    if accepted.returncode:
        failures.append(f"recorded Boolean witness Assert failed strict recording:\n{accepted.stderr}")
    else:
        report = json.loads((directory / "assert-proof-aware/contract/rust-capabilities.json").read_text())
        statuses = {item["name"]: (item["proof_required"], item["recording_status"])
                    for item in report["circuits"]}
        if statuses.get("checked_value") != (False, "not_applicable") \
                or statuses.get("checked_write") != (True, "available"):
            failures.append(f"witness Assert proof applicability changed unexpectedly: {statuses}")
    return failures


def main() -> int:
    compactc = os.environ.get("COMPACTC")
    if not compactc or shutil.which(compactc) is None:
        print("Set COMPACTC to a ledger-8 compiler before running this gate", file=sys.stderr)
        return 1
    failures = []
    with tempfile.TemporaryDirectory(prefix="compact-rust-rejections-") as temp:
        directory = Path(temp)
        for name, (source, diagnostic, position) in CASES.items():
            source_path = directory / f"{name}.compact"
            output_path = directory / name
            source_path.write_text(source)
            result = subprocess.run(
                [compactc, "--target", "rust", "--skip-zk", str(source_path), str(output_path)],
                capture_output=True,
                text=True,
                env=os.environ.copy(),
                check=False,
            )
            message = result.stderr + result.stdout
            if result.returncode == 0:
                failures.append(f"{name}: compiler accepted an unsupported construct")
            elif (
                diagnostic not in message
                or f"{name}.compact line " not in message
                or " char " not in message
                or (position is not None and f"{name}.compact line {position[0]} char {position[1]}" not in message)
            ):
                failures.append(f"{name}: missing source diagnostic:\n{message}")
            if (output_path / "contract" / "lib.rs").exists():
                failures.append(f"{name}: rejected source left a generated Rust library")
            if (output_path / "contract" / "Cargo.toml").exists():
                failures.append(f"{name}: rejected source left a generated Cargo manifest")
            if output_path.exists() or any(directory.glob(f".{name}.compactc-stage-*")):
                failures.append(f"{name}: rejected source left output or staging debris")
        for name, source in (
            ("vector", 'export pure circuit values(): Vector<2, Field> { return [1, 2]; }\n'),
            ("tuple", 'export pure circuit values(): [Field, Field] { return [1, 2]; }\n'),
        ):
            control_source = directory / f"{name}-without-spread.compact"
            control_output = directory / f"{name}-without-spread"
            control_source.write_text(source)
            control = subprocess.run(
                [compactc, "--target", "rust", "--skip-zk", str(control_source), str(control_output)],
                capture_output=True, text=True, env=os.environ.copy(), check=False,
            )
            if control.returncode or not (control_output / "contract/lib.rs").is_file():
                failures.append(f"no-spread {name} control did not build:\n{control.stderr}")
        existing_output = directory / "vector-without-spread"
        if (existing_output / "contract/lib.rs").is_file():
            before = snapshot(existing_output)
            rejected = subprocess.run(
                [compactc, "--target", "rust", "--skip-zk",
                 str(directory / "vector-spread.compact"), str(existing_output)],
                capture_output=True, text=True, env=os.environ.copy(), check=False,
            )
            if rejected.returncode == 0 or snapshot(existing_output) != before or any(
                directory.glob(f".{existing_output.name}.compactc-stage-*")
            ):
                failures.append("spread rejection changed a complete existing output")
        failures.extend(check_output_publication(compactc, directory))
        failures.extend(check_output_serialization(compactc, directory))
        failures.extend(check_proof_capabilities(compactc, directory))
        failures.extend(check_context_locations(compactc, directory))
        failures.extend(check_boolean_typing(compactc, directory))

    for failure in failures:
        print(f"FAIL {failure}", file=sys.stderr)
    print(f"Checked {len(CASES)} source rejections, {len(CONTEXT_CASES)} source contexts, "
          f"output publication, proof capabilities and shared Boolean typing; {len(failures)} failed")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
