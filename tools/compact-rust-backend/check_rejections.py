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
quietly emits a plausible Rust library. This gate pins two source-level
refusals and verifies that no generated Cargo library survives. It also
checks that a later packaging failure cannot publish partial Rust output
or replace a previously complete directory.
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

    def run_compiler(environment: dict[str, str]) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [compactc, "--target", "rust", "--skip-zk", str(source), str(output)],
            capture_output=True, text=True, env=environment, check=False,
        )

    def leaked_stage() -> bool:
        return any(directory.glob(f".{output.name}.compactc-stage-*"))

    rejected = run_compiler(invalid_environment)
    if rejected.returncode == 0 or "runtime source directory is invalid" not in rejected.stderr:
        failures.append(f"post-render failure was not reported:\n{rejected.stderr}")
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
    rejected = run_compiler(invalid_environment)
    if (
        rejected.returncode == 0
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
    source.write_text(
        "import CompactStandardLibrary;\n"
        "export ledger value: Field;\n"
        "export pure circuit square(x: Field): Field { return x * x; }\n"
        "export circuit write(input: Field): [] { value = disclose(square(input)); }\n"
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
        assert report["schema_version"] == 1
        assert report["circuits"] == [{
            "name": "write",
            "source": {"file": "pure-call.compact", "line": 4, "column": 1},
            "recorded": False,
            "observed_call": False,
        }]
        assert manifest["contract"]["rust-capabilities.json"]["hash"] == hashlib.sha256(report_bytes).hexdigest()
    except (AssertionError, FileNotFoundError, KeyError, ValueError) as error:
        failures.append(f"native-only pure-call report or manifest is wrong: {error}")
    before = snapshot(output)
    rejected = compile_source(source, output, True)
    if rejected.returncode == 0 or "pure-call.compact line 4 char 1" not in rejected.stderr:
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
        before = snapshot(collision_output)
        rejected = compile_source(collision_source, collision_output, True)
        if rejected.returncode == 0 or "call-collision.compact line 3 char 1" not in rejected.stderr:
            failures.append(f"strict call-name collision did not reject at source:\n{rejected.stderr}")
        if snapshot(collision_output) != before:
            failures.append("strict call-name collision changed the prior output")
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

    for failure in failures:
        print(f"FAIL {failure}", file=sys.stderr)
    print(f"Checked {len(CASES)} source rejections, output publication and proof capabilities; {len(failures)} failed")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
