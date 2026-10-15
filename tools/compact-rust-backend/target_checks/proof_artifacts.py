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

"""Completeness checks for one circuit's compiler-produced proof artifacts.

The caller owns compilation, manifests, circuit selection, and proof execution.
Keep the lookup order stable so the first missing artifact stops the gate.
"""

from pathlib import Path


def require_circuit_files(output: Path, circuit: str) -> None:
    for extension in ("prover", "verifier"):
        assert (output / "keys" / f"{circuit}.{extension}").is_file()
    for extension in ("zkir", "bzkir"):
        assert (output / "zkir" / f"{circuit}.{extension}").is_file()
