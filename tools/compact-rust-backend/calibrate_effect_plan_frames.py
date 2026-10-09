#!/usr/bin/env python3
"""Retain ARM64 debug prologues for ADR-0355's experimental compiler owner.

Usage: python3 calibrate_effect_plan_frames.py PATH_TO_FEATURE_LIB_TEST_BINARY

This read-only probe does not build, change resource limits, or infer live stack
high-water marks. Run the separate resource_tests on that same build. The reported
rustc is the active tool, not independently verified binary build provenance.
"""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import re
import shutil
import subprocess


def output(*args):
    return subprocess.check_output(args, text=True).strip()


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def selected(symbol):
    if "11effect_plan" not in symbol or symbol.startswith("__RNC"):
        return False
    return bool(
        re.search(r"11effect_plan(?:6render(?:5block|6render)|8validate(?:5block|8validate|5check)|5check|5lower5lower)$", symbol)
        or re.search(r"7Lowerer(?:6action|6locals|8returned|10expression)$", symbol)
        or ("7Lowerer11conditional" in symbol and symbol.startswith("__RINvM"))
        or re.search(r"11CheckedPlan8validate$", symbol)
    )


def static_reservation(disassembly):
    """Sum supported entry-prologue adjustments only; retain the raw evidence."""
    total = 0
    recognized = False
    for line in disassembly.splitlines():
        instruction = re.match(r"\s*[0-9a-f]+:\s+[0-9a-f]+\s+(.+)$", line)
        if not instruction:
            continue
        asm = instruction[1]
        writeback = re.match(r"stp\s+\w+,\s*\w+,\s*\[sp,\s*#-(0x[0-9a-f]+|[0-9]+)\]!", asm)
        subtract = re.match(r"sub\s+sp,\s*sp,\s*#(0x[0-9a-f]+|[0-9]+)(?:,\s*lsl\s*#(\d+))?", asm)
        if writeback:
            total += int(writeback[1], 0)
            recognized = True
        elif subtract:
            total += int(subtract[1], 0) << int(subtract[2] or 0)
            recognized = True
        elif (re.match(r"stp\s+\w+,\s*\w+,\s*\[sp", asm)
              or re.match(r"(?:add|mov)\s+x29,\s*sp", asm)
              or re.match(r"str\s+xzr,\s*\[sp\]", asm)):
            continue
        else:
            break
    return total if recognized else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--objdump", help="Path to llvm-objdump; defaults to xcrun on Darwin")
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        parser.error("this prologue decoder is qualified only for Darwin arm64")
    objdump = args.objdump or output("xcrun", "--find", "llvm-objdump")
    nm = shutil.which("nm")
    if nm is None:
        parser.error("nm is required")
    frames = []
    for line in output(nm, "-nm", str(binary)).splitlines():
        parts = line.split()
        if not parts or not selected(parts[-1]):
            continue
        address = int(parts[0], 16)
        disassembly = output(objdump, "-d", f"--start-address={address:#x}",
                             f"--stop-address={address + 96:#x}", str(binary))
        if "file format mach-o arm64" not in disassembly:
            parser.error("expected a Mach-O ARM64 binary")
        frames.append({"symbol": parts[-1], "address": hex(address),
                       "static_entry_reservation_bytes": static_reservation(disassembly),
                       "prologue": disassembly})
    if not frames:
        parser.error("no experimental effect-plan symbols; check build feature/debug symbols")
    source = Path(__file__).resolve().parent / "src" / "recorded"
    files = [source / "effect_plan.rs", *sorted((source / "effect_plan").rglob("*.rs"))]
    print(json.dumps({
        "interpretation": "Static entry reservations, not measured live stack bytes or a new support limit.",
        "binary": str(binary), "binary_sha256": digest(binary),
        "platform": platform.platform(), "active_rustc": output("rustc", "-vV"),
        "objdump": objdump, "objdump_version": output(objdump, "--version"),
        "current_source_sha256": {str(p.relative_to(source)): digest(p) for p in files},
        "frames": frames,
    }, indent=2))


if __name__ == "__main__":
    main()
