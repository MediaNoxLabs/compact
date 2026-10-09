#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
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
        re.search(r"11effect_plan(?:6render(?:5block|6render)|8validate(?:5block|8validate|5check|5calls8validate)|5check|16check_with_calls|5lower(?:5lower|16lower_with_calls))$", symbol)
        or re.search(r"7Lowerer(?:4call|12circuit_body|6action|6locals|8returned|10expression)$", symbol)
        or ("7Lowerer11conditional" in symbol and symbol.startswith("__RINvM"))
        or re.search(r"11CheckedPlan8validate$", symbol)
    )


def static_reservation(disassembly):
    """Decode qualified ARM64 entry allocation, never return a partial total.

    #530: LLVM emits both unrolled page probes and a counted x9 probe loop.
    This bounded decoder accepts those exact forms. Missing/truncated or unknown
    allocation sequences yield None; the retained raw prologue remains evidence.
    """
    instructions = []
    for line in disassembly.splitlines():
        match = re.match(r"\s*([0-9a-f]+):\s+[0-9a-f]+\s+(.+)$", line)
        if match:
            asm = match[2].split(';', 1)[0].strip()
            instructions.append((int(match[1], 16), asm))
    immediate = r"(0x[0-9a-f]+|[0-9]+)"
    subtract = re.compile(r"sub\s+sp,\s*sp,\s*#" + immediate + r"(?:,\s*lsl\s*#(\d+))?$")
    loop_start = re.compile(r"sub\s+(x\d+),\s*sp,\s*#" + immediate + r"(?:,\s*lsl\s*#(\d+))?$")
    probe = re.compile(r"(?:ldr|str)\s+xzr,\s*\[sp\]$")
    total, recognized, index = 0, False, 0
    while index < len(instructions):
        _, asm = instructions[index]
        writeback = re.fullmatch(r"stp\s+\w+,\s*\w+,\s*\[sp,\s*#-" + immediate + r"\]!", asm)
        allocation = subtract.fullmatch(asm)
        loop = loop_start.fullmatch(asm)
        if writeback:
            total += int(writeback[1], 0)
            recognized = True
        elif loop:
            # Four following instructions must probe exactly one page, compare
            # against the precomputed bound, then branch to the page subtraction.
            if index + 4 >= len(instructions):
                return None
            register, amount, shift = loop.groups()
            size = int(amount, 0) << int(shift or 0)
            page_address, page_asm = instructions[index + 1]
            page = subtract.fullmatch(page_asm)
            compare = re.fullmatch(r"cmp\s+sp,\s*" + register, instructions[index + 3][1])
            branch = re.fullmatch(r"b\.ne\s+(0x[0-9a-f]+)(?:\s+<[^>]+>)?", instructions[index + 4][1])
            if (not page or (int(page[1], 0) << int(page[2] or 0)) != 4096
                    or not probe.fullmatch(instructions[index + 2][1])
                    or not compare or not branch or int(branch[1], 16) != page_address
                    or size <= 0 or size % 4096):
                return None
            total += size
            recognized = True
            index += 4
        elif allocation:
            total += int(allocation[1], 0) << int(allocation[2] or 0)
            recognized = True
        elif (re.fullmatch(r"stp\s+\w+,\s*\w+,\s*\[sp(?:,\s*#" + immediate + r")?\]", asm)
              or re.fullmatch(r"add\s+x29,\s*sp,\s*#" + immediate, asm)
              or re.fullmatch(r"mov\s+x29,\s*sp", asm)
              or probe.fullmatch(asm)):
            pass
        else:
            # A body instruction marks the end only if no unresolved allocation
            # or probe sequence remains in the bounded window. Conservative None
            # is preferable to misreporting a partial frame on new LLVM codegen.
            remainder = [instruction for _, instruction in instructions[index:]]
            if any(re.match(r"(?:sub|mov|and|bic)\s+sp,|(?:stp|str)\s+.*\[sp.*\]!|sub\s+x\d+,\s*sp,", instruction)
                   or re.search(r"(?:chkstk|probe_stack)", instruction)
                   for instruction in remainder):
                return None
            return total if recognized else None
        index += 1
    # The window ended inside a recognized prologue, so completion is unknown.
    return None


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
        reservation = static_reservation(disassembly)
        frames.append({"symbol": parts[-1], "address": hex(address),
                       "static_entry_reservation_bytes": reservation,
                       "reservation_status": "decoded" if reservation is not None else "unknown-or-incomplete",
                       "prologue": disassembly})
    if not frames:
        parser.error("no experimental effect-plan symbols; check build feature/debug symbols")
    source = Path(__file__).resolve().parent / "src" / "recorded"
    files = [source / "effect_plan.rs", *sorted((source / "effect_plan").rglob("*.rs"))]
    print(json.dumps({
        "interpretation": "Qualified static entry reservations only; null means unknown/incomplete. Not measured live stack bytes or a new support limit.",
        "binary": str(binary), "binary_sha256": digest(binary),
        "platform": platform.platform(), "active_rustc": output("rustc", "-vV"),
        "objdump": objdump, "objdump_version": output(objdump, "--version"),
        "current_source_sha256": {str(p.relative_to(source)): digest(p) for p in files},
        "frames": frames,
    }, indent=2))


if __name__ == "__main__":
    main()
