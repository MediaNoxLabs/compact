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

"""Select the checked proof executable profile without changing other Cargo work."""

PROOF_PACKAGE = "compact-rust-proof-smoke"
DEFAULT_PROFILE = "proof-checks"
PROFILES = frozenset({"dev", DEFAULT_PROFILE})


def command(arguments, environment):
    """Only route explicit proof-package runs, before the executable's `--`."""
    args = list(arguments)
    if not args or args[0] != "cargo":
        return args
    verb = 2 if len(args) > 1 and args[1].startswith("+") else 1
    if len(args) <= verb or args[verb] != "run":
        return args
    boundary = args.index("--") if "--" in args else len(args)
    options = args[verb + 1:boundary]
    packages = [options[i + 1] for i, item in enumerate(options[:-1])
                if item in ("-p", "--package")]
    if packages != [PROOF_PACKAGE]:
        return args
    selected = environment.get("COMPACT_RUST_PROOF_PROFILE", DEFAULT_PROFILE)
    if selected not in PROFILES:
        raise ValueError("COMPACT_RUST_PROOF_PROFILE must be dev or proof-checks")
    # The harness owns these invocations; avoid ambiguous profile precedence.
    if any(item in ("--release", "-r", "--profile") or item.startswith("--profile=")
           for item in options):
        raise ValueError("proof commands select their profile with COMPACT_RUST_PROOF_PROFILE")
    return args[:boundary] + ["--profile", selected] + args[boundary:]
