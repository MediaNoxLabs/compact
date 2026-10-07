#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
# 	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Focused typed consumer controls, shared by standalone and archive acceptance."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

FIXTURES = Path(__file__).resolve().parent / "consumers/safety"
POSITIVES = {
    "thread_returned_context": "returned_context_threads_two_real_calls",
    "checked_bounded_uint": "checked_constructor_enforces_both_boundaries",
}
NEGATIVES = {"reuse_consumed_context": "E0382", "unchecked_bounded_uint": "E0423"}


def error_diagnostics(stdout: str) -> list[dict]:
    messages = []
    for line in stdout.splitlines():
        try:
            entry = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(entry, dict) and entry.get("reason") == "compiler-message":
            diagnostic = entry["message"]
            if diagnostic.get("level") == "error":
                messages.append(diagnostic)
    return messages


def require_expected_rejection(result: subprocess.CompletedProcess, code: str, target: str) -> None:
    errors = error_diagnostics(result.stdout)
    if result.returncode == 0 or not errors:
        raise RuntimeError(f"{target}: expected compiler error {code}, not success or infrastructure failure")
    for error in errors:
        actual = (error.get("code") or {}).get("code")
        at_target = any(
            span.get("is_primary") and Path(span.get("file_name", "")).name == f"{target}.rs"
            for span in error.get("spans", [])
        )
        if actual != code or not at_target:
            raise RuntimeError(f"{target}: unexpected compiler diagnostic {actual!r}; required {code} at consumer")


def check_consumer_safety(
    consumer: Path, environment: dict[str, str], cargo=("cargo",), *, logs: Path | None = None
) -> dict:
    """Run real positive consumers first; never count a general build failure as safety."""
    consumer = consumer.resolve()
    tests = consumer / "tests"
    tests.mkdir(exist_ok=True)
    created = []
    commands = []
    if logs is not None:
        logs.mkdir(parents=True, exist_ok=False)

    def execute(arguments):
        command = [*cargo, *arguments]
        result = subprocess.run(command, cwd=consumer, env=environment, capture_output=True, text=True)
        if logs is not None:
            for stream in ("stdout", "stderr"):
                (logs / f"{len(commands):02d}.{stream}").write_text(getattr(result, stream))
        commands.append({
            "command": command,
            "exit_code": result.returncode,
            "stdout_sha256": hashlib.sha256(result.stdout.encode()).hexdigest(),
            "stderr_sha256": hashlib.sha256(result.stderr.encode()).hexdigest(),
        })
        return result

    def install(name):
        target = f"typed_safety_{name}"
        path = tests / f"{target}.rs"
        with path.open("x") as output:
            created.append(path)
            output.write((FIXTURES / f"{name}.rs").read_text())
        return target

    try:
        for name, test_name in POSITIVES.items():
            target = install(name)
            checked = execute(["check", "--locked", "--offline", "--test", target, "--message-format=json"])
            if checked.returncode:
                diagnostics = [(e.get("code"), e.get("message")) for e in error_diagnostics(checked.stdout)]
                raise RuntimeError(f"{target}: valid consumer failed to compile: {diagnostics}; {checked.stderr[-2000:]}")
            tested = execute(["test", "--locked", "--offline", "--test", target, "--", "--exact", test_name])
            if tested.returncode or not re.search(
                r"^test result: ok\. 1 passed; 0 failed; 0 ignored;", tested.stdout, re.MULTILINE
            ):
                raise RuntimeError(f"{target}: positive runtime assertion did not pass exactly once: {tested.stderr[-2000:]}")
        for name, code in NEGATIVES.items():
            target = install(name)
            rejected = execute(["check", "--locked", "--offline", "--test", target, "--message-format=json"])
            require_expected_rejection(rejected, code, target)
            created.pop().unlink()
        return {
            "format": "compact-typed-consumer-safety/v1",
            "positive_tests": list(POSITIVES.values()),
            "negative_codes": NEGATIVES,
            "diagnostic_scope": "Every reported error has the expected code and a primary span in the installed consumer target.",
            "commands": commands,
            "fixture_sha256": {
                name: hashlib.sha256((FIXTURES / f"{name}.rs").read_bytes()).hexdigest()
                for name in (*POSITIVES, *NEGATIVES)
            },
        }
    finally:
        for path in reversed(created):
            path.unlink()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--consumer", required=True, type=Path, help="existing Counter consumer with Cargo.lock")
    parser.add_argument("--toolchain", help="explicit rustup Cargo toolchain")
    parser.add_argument("--receipt", required=True, type=Path)
    parser.add_argument("--logs", type=Path, help="new directory retaining exact Cargo output")
    args = parser.parse_args()
    cargo = ("cargo", f"+{args.toolchain}") if args.toolchain else ("cargo",)
    result = check_consumer_safety(args.consumer, os.environ.copy(), cargo, logs=args.logs)
    args.receipt.write_text(json.dumps(result, indent=2) + "\n")
    print("typed consumer safety: two positive tests and two intended compiler refusals passed")


if __name__ == "__main__":
    main()
