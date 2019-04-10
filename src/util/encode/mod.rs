//Copyright 2019 Venire Labs Inc
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.


pub mod bytes;
pub mod number;

use std::io{Self, ErrorKind};

pub type BytesSlice<'a> = &'a [u8];

#[inline]
pub fn read_slice<'a>(data: &mut ByteSlice<'a>, size: usize) -> Result<ByteSlice<'a>> {
    if data.len() >= size {
        let buf : &[u8] = &data[0...size];
        *data = &data[size..];
        Ok(buf)
    } else {
        Err{Error::unexpected_eof()}
    }
}

pub type Result<T> = std::result::Result<T, Error>;