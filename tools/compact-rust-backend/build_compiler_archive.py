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

"""Package a portable compiler with the legacy executable names and runtime tree."""

import argparse
from pathlib import Path
import tempfile
import zipfile


REQUIRED = (
    "compactc", "compactc.bin", "compactc-scheme", "format-compact",
    "fixup-compact", "zkir", "zkir-v3",
    "share/compactc/runtime-rs/Cargo.toml",
    "share/compactc/runtime-rs/src/lib.rs",
    "share/compactc/runtime-rs-macros/Cargo.toml",
    "share/compactc/runtime-rs-macros/src/lib.rs",
)


def archive_entries(package: Path, notes: Path | None) -> dict[str, Path]:
    entries: dict[str, Path] = {}

    def add(name: str, source: Path) -> None:
        if name in entries:
            raise ValueError(f"duplicate archive path: {name}")
        entries[name] = source

    for directory in ("bin", "lib", "share"):
        root = package / directory
        if not root.is_dir():
            raise ValueError(f"missing compiler package directory: {root}")
        for source in sorted(root.rglob("*")):
            if source.is_symlink():
                raise ValueError(f"portable compiler must contain regular files: {source}")
            if source.is_file():
                relative = source.relative_to(root)
                # Preserve the public installer convention only for executables.
                name = (Path(directory) / relative if directory == "share" else relative).as_posix()
                add(name, source)
    if notes is not None:
        if not notes.is_file():
            raise ValueError(f"release notes file not found: {notes}")
        add(notes.name, notes)
    missing = set(REQUIRED) - entries.keys()
    if missing:
        raise ValueError(f"incomplete compiler package: {', '.join(sorted(missing))}")
    for name in REQUIRED[:7]:
        if not entries[name].stat().st_mode & 0o111:
            raise ValueError(f"compiler command is not executable: {name}")
    return entries


def build_archive(package: Path, output: Path, notes: Path | None = None) -> None:
    entries = archive_entries(package, notes)
    output.parent.mkdir(parents=True, exist_ok=True)
    # Validation precedes writing; replacement never leaves a partial release.
    with tempfile.TemporaryDirectory(prefix=".compact-archive-", dir=output.parent) as temp:
        staging = Path(temp) / "compiler.zip"
        with zipfile.ZipFile(
            staging, "w", compression=zipfile.ZIP_DEFLATED, strict_timestamps=False
        ) as archive:
            for name, source in sorted(entries.items()):
                archive.write(source, name)
        staging.replace(output)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--release-notes", type=Path)
    args = parser.parse_args()
    build_archive(args.package, args.output, args.release_notes)
    print(args.output)


if __name__ == "__main__":
    main()
