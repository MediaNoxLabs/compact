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
refusals and verifies that no generated Cargo library survives.
"""

import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


CASES = {
    "unknown-opaque": (
        'import CompactStandardLibrary;\n'
        'export ledger thing: Opaque<"NotAThing">;\n'
        'constructor() {}\n',
        "Rust backend does not yet support this opaque type",
    ),
    "field-to-uint": (
        'export ledger n: Uint<64>;\n'
        'export circuit narrow(f: Field): Uint<64> { return f as Uint<64>; }\n',
        "Rust backend does not yet support Field-to-Uint downcasts",
    ),
}


def main() -> int:
    compactc = os.environ.get("COMPACTC")
    if not compactc or shutil.which(compactc) is None:
        print("Set COMPACTC to a ledger-8 compiler before running this gate", file=sys.stderr)
        return 1
    failures = []
    with tempfile.TemporaryDirectory(prefix="compact-rust-rejections-") as temp:
        directory = Path(temp)
        for name, (source, diagnostic) in CASES.items():
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
            elif diagnostic not in message or f"{name}.compact line " not in message:
                failures.append(f"{name}: missing source diagnostic:\n{message}")
            if (output_path / "contract" / "lib.rs").exists():
                failures.append(f"{name}: rejected source left a generated Rust library")
            if (output_path / "contract" / "Cargo.toml").exists():
                failures.append(f"{name}: rejected source left a generated Cargo manifest")

    for failure in failures:
        print(f"FAIL {failure}", file=sys.stderr)
    print(f"Checked {len(CASES)} rejection probes; {len(failures)} failed")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
