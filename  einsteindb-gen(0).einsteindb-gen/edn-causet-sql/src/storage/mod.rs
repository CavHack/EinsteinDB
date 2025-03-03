//Copyright 2021-2023 WHTCORPS INC ALL RIGHTS RESERVED. APACHE 2.0 COMMUNITY EDITION SL
// AUTHORS: WHITFORD LEDER
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file File except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

mod range;
pub mod ranges_iter;
pub mod mutant_searchner;
pub mod test_fixture;

pub use self::range::*;

pub type Result<T> = std::result::Result<T, crate::error::StorageError>;

pub type OwnedHikvPair = (Vec<u8>, Vec<u8>);

/// The abstract storage interface. The table mutant_search and index mutant_search executor relies on a `Storage`
/// implementation to provide source data.
pub trait Storage: Send {
    type Statistics;

    fn begin_mutant_search(
        &mut self,
        is_spacelike_completion_mutant_search: bool,
        is_key_only: bool,
        range: IntervalRange,
    ) -> Result<()>;

    fn mutant_search_next(&mut self) -> Result<Option<OwnedHikvPair>>;

    // TODO: Use const generics.
    // TODO: Use reference is better.
    fn get(&mut self, is_key_only: bool, range: PointRange) -> Result<Option<OwnedHikvPair>>;

    fn met_uncacheable_data(&self) -> Option<bool>;

    fn collect_statistics(&mut self, dest: &mut Self::Statistics);
}

impl<T: Storage + ?Sized> Storage for Box<T> {
    type Statistics = T::Statistics;

    fn begin_mutant_search(
        &mut self,
        is_spacelike_completion_mutant_search: bool,
        is_key_only: bool,
        range: IntervalRange,
    ) -> Result<()> {
        (**self).begin_mutant_search(is_spacelike_completion_mutant_search, is_key_only, range)
    }

    fn mutant_search_next(&mut self) -> Result<Option<OwnedHikvPair>> {
        (**self).mutant_search_next()
    }

    fn get(&mut self, is_key_only: bool, range: PointRange) -> Result<Option<OwnedHikvPair>> {
        (**self).get(is_key_only, range)
    }

    fn met_uncacheable_data(&self) -> Option<bool> {
        (**self).met_uncacheable_data()
    }

    fn collect_statistics(&mut self, dest: &mut Self::Statistics) {
        (**self).collect_statistics(dest);
    }
}
