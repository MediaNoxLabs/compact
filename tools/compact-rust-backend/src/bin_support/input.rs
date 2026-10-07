// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Shared byte ingress policy for all three compiler executables. This module
//! is compiled into each binary, keeping the helper out of the library API.
use std::io::{self, Read};

const INPUT_BYTES: usize = 4 * 1024 * 1024;

pub(crate) fn read(reader: impl Read) -> io::Result<Vec<u8>> {
    read_with_limit(reader, INPUT_BYTES)
}

fn read_with_limit(reader: impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let bound = limit.checked_add(1).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "invalid compiler input limit")
    })?;
    let mut bytes = Vec::new();
    reader.take(bound as u64).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "compiler resource input_bytes exceeds {limit} (observed at least {})",
                bytes.len()
            ),
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_short_utf8_and_one_over() {
        struct Short<'a>(&'a [u8]);
        impl Read for Short<'_> {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                if self.0.is_empty() || out.is_empty() {
                    return Ok(0);
                }
                out[0] = self.0[0];
                self.0 = &self.0[1..];
                Ok(1)
            }
        }
        let bytes = "λ".as_bytes();
        assert_eq!(read_with_limit(Short(bytes), 2).unwrap(), bytes);
        assert_eq!(
            read_with_limit(Short(bytes), 1).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert!(read_with_limit(&[][..], 0).unwrap().is_empty());
        assert_eq!(
            read_with_limit(&[][..], usize::MAX).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
    #[test]
    fn oversized_stream_is_not_fully_consumed() {
        let mut stream = io::Cursor::new([0_u8; 32]);
        assert_eq!(
            read_with_limit(&mut stream, 8).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(stream.position(), 9);
    }
}
