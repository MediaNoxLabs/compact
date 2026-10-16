#!/usr/bin/env bash
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
set -e

# The compact installer symlinks this launcher into its bin directory. Resolve
# the link before locating sibling binaries, including relative link targets.
launcher="$0"
while [ -L "$launcher" ]; do
  directory="$(cd -P -- "$(dirname -- "$launcher")" && pwd)"
  launcher="$(readlink -- "$launcher")"
  case "$launcher" in
    /*) ;;
    *) launcher="$directory/$launcher" ;;
  esac
done
directory="$(cd -P -- "$(dirname -- "$launcher")" && pwd)"
export PATH="$directory/../lib:$directory:$PATH"
# A scoped local-test launcher may supply artifact preparation ahead of bundled
# tools. Ordinary compiler installations have no dependency on that launcher.
if [[ -n "${COMPACT_ZKIR_SHIM:-}" && "${COMPACT_ZKIR_MODE:-forge}" == forge ]]; then
  case "$COMPACT_ZKIR_SHIM" in
    /*/zkir) ;;
    *) echo "COMPACT_ZKIR_SHIM must be an absolute zkir executable" >&2; exit 2 ;;
  esac
  [[ -f "$COMPACT_ZKIR_SHIM" && -x "$COMPACT_ZKIR_SHIM" ]] || {
    echo "COMPACT_ZKIR_SHIM is not executable" >&2; exit 2;
  }
  export PATH="$(dirname -- "$COMPACT_ZKIR_SHIM"):$PATH"
fi
exec "$directory/compactc.bin" "$@"
