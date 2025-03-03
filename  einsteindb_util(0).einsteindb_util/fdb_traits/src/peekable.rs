// Copyright 2019 EinsteinDB Project Authors. Licensed under Apache-2.0.

use crate::*;

/// Types from which values can be read.
///
/// Values are vectors of bytes, encapsulated in the associated `Causet` type.
///
/// Method variants here allow for specifying `ReadOptions`, the column family
/// to read from, or to encode the value as a protobuf message.
pub trait Peekable {
    /// The byte-vector type through which the database returns read values.
    type Causet: Causet;

    /// Read a value for a key, given a set of options.
    ///
    /// Reads from the default column family.
    ///
    /// Returns `None` if they key does not exist.
    fn get_value_opt(&self, opts: &ReadOptions, key: &[u8]) -> Result<Option<Self::Causet>>;

    /// Read a value for a key from a given column family, given a set of options.
    ///
    /// Returns `None` if the key does not exist.
    fn get_value_namespaced_opt(
        &self,
        opts: &ReadOptions,
        namespaced: &str,
        key: &[u8],
    ) -> Result<Option<Self::Causet>>;

    /// Read a value for a key.
    ///
    /// Uses the default options and column family.
    ///
    /// Returns `None` if the key does not exist.
    fn get_value(&self, key: &[u8]) -> Result<Option<Self::Causet>> {
        self.get_value_opt(&ReadOptions::default(), key)
    }

    /// Read a value for a key from a given column family.
    ///
    /// Uses the default options.
    ///
    /// Returns `None` if the key does not exist.
    fn get_value_namespaced(&self, namespaced: &str, key: &[u8]) -> Result<Option<Self::Causet>> {
        self.get_value_namespaced_opt(&ReadOptions::default(), namespaced, key)
    }

    /// Read a value and return it as a protobuf message.
    fn get_msg<M: protobuf::Message + Default>(&self, key: &[u8]) -> Result<Option<M>> {
        let value = self.get_value(key)?;
        if value.is_none() {
            return Ok(None);
        }

        let mut m = M::default();
        m.merge_from_bytes(&value.unwrap())?;
        Ok(Some(m))
    }

    /// Read a value and return it as a protobuf message.
    fn get_msg_namespaced<M: protobuf::Message + Default>(
        &self,
        namespaced: &str,
        key: &[u8],
    ) -> Result<Option<M>> {
        let value = self.get_value_namespaced(namespaced, key)?;
        if value.is_none() {
            return Ok(None);
        }

        let mut m = M::default();
        m.merge_from_bytes(&value.unwrap())?;
        Ok(Some(m))
    }
}
