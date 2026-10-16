# This file is part of Compact.
# Copyright (C) 2026 contributors to Minokawa Compact
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# You may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# These two ledger 8.1.3 Git outputs use an unhyphenated npm scope.
# Adapt only their metadata at this boundary, retaining every payload byte.
{pkgs, upstream, profile}: let
  versions = {
    onchain-runtime-v3 = "3.1.2";
    zkir-v2 = "2.1.1";
  };
  version = versions.${profile};
  package = pkgs.runCommand "${profile}-legacy-npm-scope-${version}" {
    nativeBuildInputs = [pkgs.python3];
  } ''
    python3 ${./npm_scope_compat.py} \
      --profile ${profile} \
      --source ${upstream}/lib/node_modules/@midnightntwrk/${profile} \
      --output "$out"
  '';
in
  assert builtins.hasAttr profile versions;
  assert upstream.version == version; {
    inherit package;
    libPath = "${package}/lib/node_modules/@midnight-ntwrk/${profile}";
    tarPath = "${package}/lib/midnight-${profile}-${version}.tgz";
  }
