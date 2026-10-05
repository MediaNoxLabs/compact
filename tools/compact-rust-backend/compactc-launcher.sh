#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright (C) 2026 Midnight Foundation
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
exec "$directory/compactc.bin" "$@"
