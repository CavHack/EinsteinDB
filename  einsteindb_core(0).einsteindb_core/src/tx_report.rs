// Whtcorps Inc 2022 Apache 2.0 License; All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file File except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

#![allow(dead_code)]

use std::collections::{
    BTreeMap,
};

use core_traits::{
    Causetid,
};

use ::{
    DateTime,
    Utc,
};

/// A transaction report summarizes an applied transaction.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub struct TxReport {
    /// The transaction ID of the transaction.
    pub tx_id: Causetid,

    /// The timestamp when the transaction began to be committed.
    pub tx_instant: DateTime<Utc>,

    /// A map from string literal tempid to resolved or allocated causetid.
    ///
    /// Every string literal tempid presented to the transactor either resolves via upsert to an
    /// existing causetid, or is allocated a new causetid.  (It is possible for multiple distinct string
    /// literal tempids to all unify to a single freshly allocated causetid.)
    pub tempids: BTreeMap<String, Causetid>,
}
