#!/usr/bin/env python3
"""Compare checked-in Rust fixtures with fresh Compact compiler output.

Set COMPACTC to a ledger-8 compactc executable. Pass --update to replace
stale generated fixture libraries after reviewing the backend changes.
"""

import argparse
from pathlib import Path
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]
SOURCES = ROOT / "examples" / "rust_backend"
FIXTURES = ROOT / "tests-rust-backend"
BACKEND = ROOT / "target" / "debug" / "compact-rustc"


def run(command: list[str], *, cwd: Path = ROOT) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, capture_output=True, text=True, check=False)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--update", action="store_true", help="write fresh generated fixtures")
    args = parser.parse_args()

    build = run(["cargo", "build", "-q", "-p", "compact-rust-backend", "--bin", "compact-rustc"])
    if build.returncode:
        sys.stderr.write(build.stderr)
        return build.returncode

    checked = 0
    changed = []
    failures = []
    for source in sorted(SOURCES.glob("*.compact")):
        fixture = FIXTURES / source.stem.replace("_", "-") / "lib.rs"
        if not fixture.exists():
            failures.append(f"{source.name}: missing {fixture.relative_to(ROOT)}")
            continue
        with tempfile.TemporaryDirectory(prefix="compact-rust-fixture-") as output:
            compile_result = run([str(BACKEND), str(source), output])
            if compile_result.returncode:
                failures.append(f"{source.name}: {compile_result.stderr.strip()}")
                continue
            generated = Path(output) / "contract" / "lib.rs"
            format_result = run(["rustfmt", "--edition", "2024", str(generated)])
            if format_result.returncode:
                failures.append(f"{source.name}: {format_result.stderr.strip()}")
                continue
            checked += 1
            fresh = generated.read_bytes()
            if fresh != fixture.read_bytes():
                changed.append(str(fixture.relative_to(ROOT)))
                if args.update:
                    fixture.write_bytes(fresh)

    for failure in failures:
        print(f"FAIL {failure}", file=sys.stderr)
    for path in changed:
        print(f"{'UPDATED' if args.update else 'STALE'} {path}")
    print(f"Checked {checked} fixtures; {len(changed)} {'updated' if args.update else 'stale'}; {len(failures)} failed")
    return 1 if failures or (changed and not args.update) else 0


if __name__ == "__main__":
    raise SystemExit(main())
