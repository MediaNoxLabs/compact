// This file is part of midnightntwrk/midnight-verifiable-credential-digital-passport.
// Copyright (C) Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Root public surface: family codecs and the generated/runtime contract module.
// Fixtures live behind the first-class `./testing` subpath (design D3) and are
// intentionally NOT re-exported here, so importing the root entry never pulls
// testing-only dependencies into wallet/verifier code.
export * from './codecs.js';
export * from './managed/digital-passport-credential/contract/index.js';
