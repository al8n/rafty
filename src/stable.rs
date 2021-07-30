/* Copyright 2021 Al Liu (https://github.com/al8n). Licensed under Apache-2.0.
 *
 * Copyright 2017 The Hashicorp's Raft repository authors(https://github.com/hashicorp/raft) authors. Licensed under MPL-2.0.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate::errors::Error;
use crate::Bytes;

/// `StableStore` is used to provide stable storage
/// of key configurations to ensure safety.
pub trait StableStore {
    fn set(&mut self, key: Bytes, val: Bytes) -> Result<(), Error>;

    /// `get` returns the value for key, or an empty byte slice if key was not found
    fn get(&self, key: Bytes) -> Result<Bytes, Error>;

    fn set_u64(&mut self, key: Bytes, val: u64) -> Result<(), Error>;

    /// `get_u64` returns the u64 value for key, or 0 if key was not found.
    fn get_u64(&self, key: Bytes) -> Result<u64, Error>;
}
