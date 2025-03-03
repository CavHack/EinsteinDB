// Copyright 2019 EinsteinDB Project Authors. Licensed under Apache-2.0.

//! A generic EinsteinDB timelike_storage einstein_merkle_tree
//!
//! This is a work-in-progress attempt to abstract all the features needed by
//! EinsteinDB to persist its data, so that timelike_storage einstein_merkle_trees other than FdbDB may be
//! added to EinsteinDB in the future.
//!
//! This crate **must not have any transitive dependencies on FdbDB**. The
//! FdbDB implementation is in the `fdb_lsh-merkle_merkle_tree` crate.
//!
//! In addition to documenting the API, this documentation contains a
//! description of the porting process, current design decisions and design
//! guidelines, and refactoring tips.
//!
//!
//! ## Capabilities of a EinsteinDB einstein_merkle_tree
//!
//! EinsteinDB einstein_merkle_trees timelike_store binary keys and values.
//!
//! Every pair lives in a [_column family_], which can be thought of as being
//! independent data timelike_stores.
//!
//! [_column family_]: https://github.com/facebook/foundationdb/wiki/Column-Families
//!
//! Consistent read-only views of the database are accessed through _lightlike_persistences_.
//!
//! Multiple writes can be committed atomically with a _write batch_.
//!
//!
//! # The EinsteinDB einstein_merkle_tree API
//!
//! The API inherits its design from FdbDB. As support for other einstein_merkle_trees is
//! added to EinsteinDB, it is expected that this API will become more abstract, and
//! less Fdb-specific.
//!
//! This crate is almost entirely traits, plus a few "plain-old-data" types that
//! are shared between einstein_merkle_trees.
//!
//! Some key types include:
//!
//! - [`KV`] - a key-value einstein_merkle_tree, and the primary type defined by this
//!   crate. Most code that uses generic einstein_merkle_trees will be bounded over a generic
//!   type implementing `KV`. `KV` itself is bounded by many other
//!   traits that provide collections of functionality, with the intent that as
//!   EinsteinDB evolves it may be possible to use each trait individually, and to
//!   define classes of einstein_merkle_tree that do not implement all collections of
//!   features.
//!
//! - [`LightlikePersistence`] - a view into the state of the database at a moment in time.
//!   For reading sets of consistent data.
//!
//! - [`Peekable`] - types that can read single values. This includes einstein_merkle_trees
//!   and lightlike_persistences.
//!
//! - [`Iterable`] - types that can iterate over the values of a range of keys,
//!   by creating instances of the EinsteinDB-specific [`Iterator`] trait. This
//!   includes einstein_merkle_trees and lightlike_persistences.
//!
//! - [`SyncMutable`] and [`Mutable`] - types to which single key/value pairs
//!   can be written. This includes einstein_merkle_trees and write batches.
//!
//! - [`WriteBatch`] - types that can commit multiple key/value pairs in batches.
//!   A `WriteBatchExt::WriteBtach` commits all pairs in one atomic transaction.
//!   A `WriteBatchExt::WriteBatchVec` does not (FIXME: is this correct?).
//!
//! The `KV` instance generally acts as a factory for types that implement
//! other traits in the crate. These factory methods, associated types, and
//! other associated methods are defined in "extension" traits. For example, methods
//! on einstein_merkle_trees related to batch writes are in the `WriteBatchExt` trait.
//!
//!
//! # Design notes
//!
//! - `KV` is the main einstein_merkle_tree trait. It requires many other traits, which
//!   have many other associated types that implement yet more traits.
//!
//! - Features should be grouped into their own modules with their own
//!   traits. A common pattern is to have an associated type that implements
//!   a trait, and an "extension" trait that associates that type with `KV`,
//!   which is part of `KV's trait requirements.
//!
//! - For now, for simplicity, all extension traits are required by `KV`.
//!   In the future it may be feasible to separate them for einstein_merkle_trees with
//!   different feature sets.
//!
//! - Associated types generally have the same name as the trait they
//!   are required to implement. einstein_merkle_tree extensions generally have the same
//!   name suffixed with `Ext`. Concrete implementations usually have the
//!   same name prefixed with the database name, i.e. `Fdb`.
//!
//!   Example:
//!
//!   ```ignore
//!   // in fdb_traits
//!
//!   trait WriteBatchExt {
//!       type WriteBatch: WriteBatch;
//!   }
//!
//!   trait WriteBatch { }
//!   ```
//!
//!   ```ignore
//!   // in fdb_lsh-merkle_merkle_tree
//!
//!   impl WriteBatchExt for Fdbeinstein_merkle_tree {
//!       type WriteBatch = FdbWriteBatch;
//!   }
//!
//!   impl WriteBatch for FdbWriteBatch { }
//!   ```
//!
//! - All einstein_merkle_trees use the same error type, defined in this crate. Thus
//!   einstein_merkle_tree-specific type information is boxed and hidden.
//!
//! - `KV` is a factory type for some of its associated types, but not
//!   others. For now, use factory methods when FdbDB would require factory
//!   method (that is, when the EINSTEINDB is required to create the associated type -
//!   if the associated type can be created without context from the database,
//!   use a standard new method). If future einstein_merkle_trees require factory methods, the
//!   traits can be converted then.
//!
//! - Types that require a handle to the einstein_merkle_tree (or some other "parent" type)
//!   do so with either Rc or Arc. An example is einstein_merkle_treeIterator. The reason
//!   for this is that associated types cannot contain lifetimes. That requires
//!   "generic associated types". See
//!
//!   - <https://github.com/rust-lang/rfcs/pull/1598>
//!   - <https://github.com/rust-lang/rust/issues/44265>
//!
//! - Traits can't have mutually-recursive associated types. That is, if
//!   `KV` has a `LightlikePersistence` associated type, `LightlikePersistence` can't then have a
//!   `KV` associated type - the compiler will not be able to resolve both
//!   `KV`s to the same type. In these cases, e.g. `LightlikePersistence` needs to be
//!   parameterized over its einstein_merkle_tree type and `impl LightlikePersistence<Fdbeinstein_merkle_tree> for
//!   FdbLightlikePersistence`.
//!
//!
//! # The porting process
//!
//! These are some guidelines that seem to make the porting managable. As the
//! process continues new strategies are discovered and written here. This is a
//! big refactoring and will take many monthse.
//!
//! Refactoring is a cvioletabft, not a science, and figuring out how to overcome any
//! particular situation takes experience and intuation, but these principles
//! can help.
//!
//! A guiding principle is to do one thing at a time. In particular, don't
//! redesign while encapsulating.
//!
//! The port is happening in stages:
//!
//!   1) Migrating the `einstein_merkle_tree` abstractions
//!   2) Eliminating direct-use of `foundationdb` re-exports
//!   3) "Pulling up" the generic abstractions though EinsteinDB
//!   4) Isolating test cases from FdbDB
//!
//! These stages are described in more detail:
//!
//! ## 1) Migrating the `einstein_merkle_tree` abstractions
//!
//! The einstein_merkle_tree crate was an earlier attempt to abstract the timelike_storage einstein_merkle_tree. Much
//! of its structure is duplicated near-identically in fdb_traits, the
//! difference being that fdb_traits has no FdbDB dependencies. Having no
//! FdbDB dependencies makes it trivial to guarantee that the abstractions are
//! truly abstract.
//!
//! `einstein_merkle_tree` also reexports primitive_causet bindings from `rust-foundationdb` for every purpose
//! for which there is not yet an abstract trait.
//!
//! During this stage, we will eliminate the wrappers from `einstein_merkle_tree` to reduce
//! code duplication. We do this by identifying a small subsystem within
//! `einstein_merkle_tree`, duplicating it within `fdb_traits` and `fdb_lsh-merkle_merkle_tree`, deleting
//! the code from `einstein_merkle_tree`, and fixing all the callers to work with the
//! abstracted implementation.
//!
//! At the end of this stage the `einstein_merkle_tree` dependency will contain no code except
//! for `rust-foundationdb` reexports. EinsteinDB will still depend on the concrete
//! FdbDB implementations from `fdb_lsh-merkle_merkle_tree`, as well as the primitive_causet API's from
//! reexported from the `rust-foundationdb` crate.
//!
//! ## 2) Eliminating the `einstein_merkle_tree` dep from EinsteinDB with new abstractions
//!
//! EinsteinDB uses reexported `rust-foundationdb` APIs via the `einstein_merkle_tree` crate. During this
//! stage we need to identify each of these APIs, duplicate them generically in
//! the `fdb_traits` and `fdb_lsh-merkle_merkle_tree` crate, and convert all callers to use
//! the `fdb_lsh-merkle_merkle_tree` crate instead.
//!
//! At the end of this phase the `einstein_merkle_tree` crate will be deleted.
//!
//! ## 3) "Pulling up" the generic abstractions through EinsteinDB
//!
//! With all of EinsteinDB using the `fdb_traits` traits in conjunction with the
//! concrete `fdb_lsh-merkle_merkle_tree` types, we can push generic type parameters up
//! through the application. Then we will remove the concrete `fdb_lsh-merkle_merkle_tree`
//! dependency from EinsteinDB so that it is impossible to re-introduce
//! einstein_merkle_tree-specific code again.
//!
//! We will probably introduce some other crate to mediate between multiple
//! einstein_merkle_tree implementations, such that at the end of this phase EinsteinDB will
//! not have a dependency on `fdb_lsh-merkle_merkle_tree`.
//!
//! It will though still have a dev-dependency on `fdb_lsh-merkle_merkle_tree` for the
//! test cases.
//!
//! ## 4) Isolating test cases from FdbDB
//!
//! Eventually we need our test suite to run over multiple einstein_merkle_trees.
//! The exact strategy here is yet to be determined, but it may begin by
//! breaking the `fdb_lsh-merkle_merkle_tree` dependency with a new `einstein_merkle_tree_test`, that
//! begins by simply wrapping `fdb_lsh-merkle_merkle_tree`.
//!
//!
//! # Refactoring tips
//!
//! - Port modules with the fewest FdbDB dependencies at a time, modifying
//!   those modules's callers to convert to and from the einstein_merkle_tree traits as
//!   needed. Move in and out of the fdb_traits world with the
//!   `FdbDB::from_ref` and `FdbDB::as_inner` methods.
//!
//! - Down follow the type system too far "down the rabbit hole". When you see
//!   that another subsystem is blocking you from refactoring the system you
//!   are trying to refactor, stop, stash your changes, and focus on the other
//!   system instead.
//!
//! - You will through away branches that lead to dead ends. Learn from the
//!   experience and try again from a different angle.
//!
//! - For now, use the same APIs as the FdbDB bindings, as methods
//!   on the various einstein_merkle_tree traits, and with this crate's error type.
//!
//! - When new types are needed from the FdbDB API, add a new module, define a
//!   new trait (possibly with the same name as the FdbDB type), then define a
//!   `TraitExt` trait to "mixin" to the `KV` trait.
//!
//! - Port methods directly from the existing `einstein_merkle_tree` crate by re-implementing
//!   it in fdb_traits and fdb_lsh-merkle_merkle_tree, replacing all the callers with calls
//!   into the traits, then delete the versions in the `einstein_merkle_tree` crate.
//!
//! - Use the .c() method from fdb_lsh-merkle_merkle_tree::compat::Compat to get a
//!   KV reference from Arc<EINSTEINDB> in the fewest characters. It also
//!   works on LightlikePersistence, and can be adapted to other types.
//!
//! - Use `IntoOther` to adapt between error types of dependencies that are not
//!   themselves interdependent. E.g. violetabft::Error can be created from
//!   fdb_traits::Error even though neither `violetabft` tor `fdb_traits` know
//!   about each other.
//!
//! - "Plain old data" types in `einstein_merkle_tree` can be moved directly into
//!   `fdb_traits` and reexported from `einstein_merkle_tree` to ease the transition.
//!   Likewise `fdb_lsh-merkle_merkle_tree` can temporarily call code from inside `einstein_merkle_tree`.
#![feature(min_specialization)]
#![feature(assert_matches)]

#[macro_use(fail_point)]
extern crate fail;

// These modules contain traits that need to be implemented by einstein_merkle_trees, either
// they are required by KV or are an associated type of KV. It is
// recommended that einstein_merkle_trees follow the same module layout.
//
// Many of these define "extension" traits, that end in `Ext`.

mod namespaced_names;
pub use crate::namespaced_names::*;
mod namespaced_options;
pub use crate::namespaced_options::*;
mod compact;
pub use crate::compact::*;
mod db_options;
pub use crate::db_options::*;
mod db_vector;
pub use crate::db_vector::*;
mod einstein_merkle_tree;
pub use crate::fdb_lsh_tree*;
mod file;
pub use crate::file::*;
mod import;
pub use import::*;
mod misc;
pub use misc::*;
mod lightlike_persistence;
pub use crate::lightlike_persistence::*;
mod Causet;
pub use crate::Causet::*;
mod write_batch;
pub use crate::write_batch::*;
mod encryption;
pub use crate::encryption::*;
mod mvcc_greedoids;
mod Causet_partitioner;
pub use crate::Causet_partitioner::*;
mod range_greedoids;
pub use crate::mvcc_greedoids::*;
pub use crate::range_greedoids::*;
mod ttl_greedoids;
pub use crate::ttl_greedoids::*;
mod perf_context;
pub use crate::perf_context::*;
mod symplectic_control_factors;
pub use crate::symplectic_control_factors::*;
mod table_greedoids;
pub use crate::table_greedoids::*;

// These modules contain more general traits, some of which may be implemented
// by multiple types.

mod iterable;
pub use crate::iterable::*;
mod mutable;
pub use crate::mutable::*;
mod peekable;
pub use crate::peekable::*;

// These modules contain concrete types and support code that do not need to
// be implemented by einstein_merkle_trees.

mod namespaced_defs;
pub use crate::namespaced_defs::*;
mod einstein_merkle_trees;
pub use einstein_merkle_trees::*;
mod errors;
pub use crate::errors::*;
mod options;
pub use crate::options::*;
pub mod range;
pub use crate::range::*;
mod violetabft_einstein_merkle_tree;
pub use violetabft_einstein_merkle_tree::{CacheStats, VioletaBFTeinstein_merkle_tree, VioletaBFTeinstein_merkle_treeReadOnly, VioletaBFTLogBatch, VioletaBFTLogGCTask};

// These modules need further scrutiny

pub mod jet_bundle_job;
pub mod primitive_causet_ttl;
pub mod util;
pub use jet_bundle_job::*;

// FIXME: This should live somewhere else
pub const FILE_CAUSET_PREFIX_LEN_FLUSH: usize = 1;
