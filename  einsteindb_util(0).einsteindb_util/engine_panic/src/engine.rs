// Copyright 2019 EinsteinDB Project Authors. Licensed under Apache-2.0.

use crate::db_vector::PanicCauset;
use crate::lightlike_persistence::PanicLightlikePersistence;
use crate::write_batch::PanicWriteBatch;
use fdb_traits::{
    IterOptions, Iterable, Iterator, KV, Peekable, ReadOptions, Result, SeekKey, SyncMutable,
    WriteOptions,
};

#[derive(Clone, Debug)]
pub struct Paniceinstein_merkle_tree;

impl KV for Paniceinstein_merkle_tree {
    type LightlikePersistence = PanicLightlikePersistence;

    fn lightlike_persistence(&self) -> Self::LightlikePersistence {
        panic!()
    }
    fn sync(&self) -> Result<()> {
        panic!()
    }
    fn bad_downcast<T: 'static>(&self) -> &T {
        panic!()
    }
}

impl Peekable for Paniceinstein_merkle_tree {
    type Causet = PanicCauset;

    fn get_value_opt(&self, opts: &ReadOptions, key: &[u8]) -> Result<Option<Self::Causet>> {
        panic!()
    }
    fn get_value_namespaced_opt(
        &self,
        opts: &ReadOptions,
        namespaced: &str,
        key: &[u8],
    ) -> Result<Option<Self::Causet>> {
        panic!()
    }
}

impl SyncMutable for Paniceinstein_merkle_tree {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        panic!()
    }
    fn put_namespaced(&self, namespaced: &str, key: &[u8], value: &[u8]) -> Result<()> {
        panic!()
    }

    fn delete(&self, key: &[u8]) -> Result<()> {
        panic!()
    }
    fn delete_namespaced(&self, namespaced: &str, key: &[u8]) -> Result<()> {
        panic!()
    }
    fn delete_range(&self, begin_key: &[u8], end_key: &[u8]) -> Result<()> {
        panic!()
    }
    fn delete_range_namespaced(&self, namespaced: &str, begin_key: &[u8], end_key: &[u8]) -> Result<()> {
        panic!()
    }
}

impl Iterable for Paniceinstein_merkle_tree {
    type Iterator = Paniceinstein_merkle_treeIterator;

    fn iterator_opt(&self, opts: IterOptions) -> Result<Self::Iterator> {
        panic!()
    }
    fn iterator_namespaced_opt(&self, namespaced: &str, opts: IterOptions) -> Result<Self::Iterator> {
        panic!()
    }
}

pub struct Paniceinstein_merkle_treeIterator;

impl Iterator for Paniceinstein_merkle_treeIterator {
    fn seek(&mut self, key: SeekKey<'_>) -> Result<bool> {
        panic!()
    }
    fn seek_for_prev(&mut self, key: SeekKey<'_>) -> Result<bool> {
        panic!()
    }

    fn prev(&mut self) -> Result<bool> {
        panic!()
    }
    fn next(&mut self) -> Result<bool> {
        panic!()
    }

    fn key(&self) -> &[u8] {
        panic!()
    }
    fn value(&self) -> &[u8] {
        panic!()
    }

    fn valid(&self) -> Result<bool> {
        panic!()
    }
}
