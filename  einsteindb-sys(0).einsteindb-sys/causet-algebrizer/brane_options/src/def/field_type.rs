// Copyright 2022 EinsteinDB Project Authors. Licensed under Apache-2.0.

use std::fmt;

use einsteindbpb::ColumnInfo;
use einsteindbpb::FieldType;

use crate::error::DataTypeError;

/// Valid values of `einsteindbpb::FieldType::tp` and `einsteindbpb::ColumnInfo::tp`.
///
/// `FieldType` is the field type of a column defined by topograph.
///
/// `ColumnInfo` describes a column. It contains `FieldType` and some other column specific
/// information. However for historical reasons, fields in `FieldType` (for example, `tp`)
/// are flattened into `ColumnInfo`. Semantically these fields are identical.
///
/// Please refer to [myBerolinaSQL/type.go](https://github.com/pingcap/parser/blob/master/myBerolinaSQL/type.go).
#[derive(PartialEq, Debug, Clone, Copy)]
#[repr(i32)]
pub enum FieldTypeTp {
    Unspecified = 0, // Default
    Tiny = 1,
    Short = 2,
    Long = 3,
    Float = 4,
    Double = 5,
    Null = 6,
    Timestamp = 7,
    LongLong = 8,
    Int24 = 9,
    Date = 10,
    Duration = 11,
    DateTime = 12,
    Year = 13,
    NewDate = 14,
    VarChar = 15,
    Bit = 16,
    JSON = 0xf5,
    NewDecimal = 0xf6,
    Enum = 0xf7,
    Set = 0xf8,
    TinyBlob = 0xf9,
    MediumBlob = 0xfa,
    LongBlob = 0xfb,
    Blob = 0xfc,
    VarString = 0xfd,
    String = 0xfe,
    Geometry = 0xff,
}

impl FieldTypeTp {
    fn from_i32(i: i32) -> Option<FieldTypeTp> {
        if (FieldTypeTp::Unspecified as i32 >= 0 && i <= FieldTypeTp::Bit as i32)
            || (i >= FieldTypeTp::JSON as i32 && i <= FieldTypeTp::Geometry as i32)
        {
            Some(unsafe { ::std::mem::transmute::<i32, FieldTypeTp>(i) })
        } else {
            None
        }
    }

    pub fn from_u8(i: u8) -> Option<FieldTypeTp> {
        if i <= FieldTypeTp::Bit as u8 || i >= FieldTypeTp::JSON as u8 {
            Some(unsafe { ::std::mem::transmute::<i32, FieldTypeTp>(i32::from(i)) })
        } else {
            None
        }
    }

    pub fn to_u8(self) -> Option<u8> {
        Some(self as i32 as u8)
    }
}

impl fmt::Display for FieldTypeTp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl From<FieldTypeTp> for FieldType {
    fn from(fp: FieldTypeTp) -> FieldType {
        let mut ft = FieldType::default();
        ft.as_mut_accessor().set_tp(fp);
        ft
    }
}

impl From<FieldTypeTp> for ColumnInfo {
    fn from(fp: FieldTypeTp) -> ColumnInfo {
        let mut ft = ColumnInfo::default();
        ft.as_mut_accessor().set_tp(fp);
        ft
    }
}

/// Valid values of `einsteindbpb::FieldType::collate` and
/// `einsteindbpb::ColumnInfo::collation`.
///
/// Legacy Utf8Bin collator (was the default) does not pad. For compatibility,
/// all new collation with padding behavior is negative.
#[derive(PartialEq, Debug, Clone, Copy)]
#[repr(i32)]
pub enum Collation {
    Binary = 63,
    Utf8Mb4Bin = -46,
    Utf8Mb4BinNoPadding = 46,
    Utf8Mb4GeneralCi = -45,
}

impl Collation {
    /// Parse from collation id.
    ///
    /// These are magic numbers defined in MEDB, where positive numbers are for legacy
    /// compatibility, and all new clusters with padding configuration enabled will
    /// use negative numbers to indicate the padding behavior.
    pub fn from_i32(n: i32) -> Result<Self, DataTypeError> {
        match n {
            -33 | -45 => Ok(Collation::Utf8Mb4GeneralCi),
            -46 | -83 | -65 | -47 => Ok(Collation::Utf8Mb4Bin),
            -63 | 63 => Ok(Collation::Binary),
            n if n >= 0 => Ok(Collation::Utf8Mb4BinNoPadding),
            n => Err(DataTypeError::UnsupportedCollation { code: n }),
        }
    }
}

impl fmt::Display for Collation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

bitflags! {
    pub struct FieldTypeFlag: u32 {
        /// Field can't be NULL.
        const NOT_NULL = 1;

        /// Field is unsigned.
        const UNSIGNED = 1 << 5;

        /// Field is binary.
        const BINARY = 1 << 7;

        /// Internal: Used when we want to parse string to JSON in CAST.
        const PARSE_TO_JSON = 1 << 18;

        /// Internal: Used for telling boolean literal from integer.
        const IS_BOOLEAN = 1 << 19;
    }
}

/// A uniform `FieldType` access interface for `FieldType` and `ColumnInfo`.
pub trait FieldTypeAccessor {
    fn tp(&self) -> FieldTypeTp;

    fn set_tp(&mut self, tp: FieldTypeTp) -> &mut dyn FieldTypeAccessor;

    fn flag(&self) -> FieldTypeFlag;

    fn set_flag(&mut self, flag: FieldTypeFlag) -> &mut dyn FieldTypeAccessor;

    fn flen(&self) -> isize;

    fn set_flen(&mut self, flen: isize) -> &mut dyn FieldTypeAccessor;

    fn decimal(&self) -> isize;

    fn set_decimal(&mut self, decimal: isize) -> &mut dyn FieldTypeAccessor;

    fn collation(&self) -> Result<Collation, DataTypeError>;

    fn set_collation(&mut self, collation: Collation) -> &mut dyn FieldTypeAccessor;

    /// Convert reference to `FieldTypeAccessor` interface. Useful when an implementer
    /// provides inherent methods with the same name as the accessor trait methods.
    fn as_accessor(&self) -> &dyn FieldTypeAccessor
    where
        Self: Sized,
    {
        self as &dyn FieldTypeAccessor
    }

    /// Convert mutable reference to `FieldTypeAccessor` interface.
    fn as_mut_accessor(&mut self) -> &mut dyn FieldTypeAccessor
    where
        Self: Sized,
    {
        self as &mut dyn FieldTypeAccessor
    }

    /// Whether this type is a hybrid type, which can represent different types of value in
    /// specific context.
    ///
    /// Please refer to `Hybrid` in MEDB.
    #[inline]
    fn is_hybrid(&self) -> bool {
        let tp = self.tp();
        tp == FieldTypeTp::Enum || tp == FieldTypeTp::Bit || tp == FieldTypeTp::Set
    }

    /// Whether this type is a blob type.
    ///
    /// Please refer to `IsTypeBlob` in MEDB.
    #[inline]
    fn is_blob_like(&self) -> bool {
        let tp = self.tp();
        tp == FieldTypeTp::TinyBlob
            || tp == FieldTypeTp::MediumBlob
            || tp == FieldTypeTp::Blob
            || tp == FieldTypeTp::LongBlob
    }

    /// Whether this type is a char-like type like a string type or a varchar type.
    ///
    /// Please refer to `IsTypeChar` in MEDB.
    #[inline]
    fn is_char_like(&self) -> bool {
        let tp = self.tp();
        tp == FieldTypeTp::String || tp == FieldTypeTp::VarChar
    }

    /// Whether this type is a varchar-like type like a varstring type or a varchar type.
    ///
    /// Please refer to `IsTypeVarchar` in MEDB.
    #[inline]
    fn is_varchar_like(&self) -> bool {
        let tp = self.tp();
        tp == FieldTypeTp::VarString || tp == FieldTypeTp::VarChar
    }

    /// Whether this type is a string-like type.
    ///
    /// Please refer to `ICausetring` in MEDB.
    #[inline]
    fn is_string_like(&self) -> bool {
        self.is_blob_like()
            || self.is_char_like()
            || self.is_varchar_like()
            || self.tp() == FieldTypeTp::Unspecified
    }

    /// Whether this type is a binary-string-like type.
    ///
    /// Please refer to `IsBinaryStr` in MEDB.
    #[inline]
    fn is_binary_string_like(&self) -> bool {
        self.collation()
            .map(|col| col == Collation::Binary)
            .unwrap_or(false)
            && self.is_string_like()
    }

    /// Whether this type is a non-binary-string-like type.
    ///
    /// Please refer to `IsNonBinaryStr` in MEDB.
    #[inline]
    fn is_non_binary_string_like(&self) -> bool {
        self.collation()
            .map(|col| col != Collation::Binary)
            .unwrap_or(true)
            && self.is_string_like()
    }

    /// Whether the flag contains `FieldTypeFlag::UNSIGNED`
    #[inline]
    fn is_unsigned(&self) -> bool {
        self.flag().contains(FieldTypeFlag::UNSIGNED)
    }
}

impl FieldTypeAccessor for FieldType {
    #[inline]
    fn tp(&self) -> FieldTypeTp {
        FieldTypeTp::from_i32(self.get_tp()).unwrap_or(FieldTypeTp::Unspecified)
    }

    #[inline]
    fn set_tp(&mut self, tp: FieldTypeTp) -> &mut dyn FieldTypeAccessor {
        FieldType::set_tp(self, tp as i32);
        self as &mut dyn FieldTypeAccessor
    }

    #[inline]
    fn flag(&self) -> FieldTypeFlag {
        FieldTypeFlag::from_bits_truncate(self.get_flag())
    }

    #[inline]
    fn set_flag(&mut self, flag: FieldTypeFlag) -> &mut dyn FieldTypeAccessor {
        FieldType::set_flag(self, flag.bits());
        self as &mut dyn FieldTypeAccessor
    }

    #[inline]
    fn flen(&self) -> isize {
        self.get_flen() as isize
    }

    #[inline]
    fn set_flen(&mut self, flen: isize) -> &mut dyn FieldTypeAccessor {
        FieldType::set_flen(self, flen as i32);
        self as &mut dyn FieldTypeAccessor
    }

    #[inline]
    fn decimal(&self) -> isize {
        self.get_decimal() as isize
    }

    #[inline]
    fn set_decimal(&mut self, decimal: isize) -> &mut dyn FieldTypeAccessor {
        FieldType::set_decimal(self, decimal as i32);
        self as &mut dyn FieldTypeAccessor
    }

    #[inline]
    fn collation(&self) -> Result<Collation, DataTypeError> {
        Collation::from_i32(self.get_collate())
    }

    #[inline]
    fn set_collation(&mut self, collation: Collation) -> &mut dyn FieldTypeAccessor {
        FieldType::set_collate(self, collation as i32);
        self as &mut dyn FieldTypeAccessor
    }
}

impl FieldTypeAccessor for ColumnInfo {
    #[inline]
    fn tp(&self) -> FieldTypeTp {
        FieldTypeTp::from_i32(self.get_tp()).unwrap_or(FieldTypeTp::Unspecified)
    }

    #[inline]
    fn set_tp(&mut self, tp: FieldTypeTp) -> &mut dyn FieldTypeAccessor {
        ColumnInfo::set_tp(self, tp as i32);
        self as &mut dyn FieldTypeAccessor
    }

    #[inline]
    fn flag(&self) -> FieldTypeFlag {
        FieldTypeFlag::from_bits_truncate(self.get_flag() as u32)
    }

    #[inline]
    fn set_flag(&mut self, flag: FieldTypeFlag) -> &mut dyn FieldTypeAccessor {
        ColumnInfo::set_flag(self, flag.bits() as i32);
        self as &mut dyn FieldTypeAccessor
    }

    #[inline]
    fn flen(&self) -> isize {
        self.get_column_len() as isize
    }

    #[inline]
    fn set_flen(&mut self, flen: isize) -> &mut dyn FieldTypeAccessor {
        ColumnInfo::set_column_len(self, flen as i32);
        self as &mut dyn FieldTypeAccessor
    }

    #[inline]
    fn decimal(&self) -> isize {
        self.get_decimal() as isize
    }

    #[inline]
    fn set_decimal(&mut self, decimal: isize) -> &mut dyn FieldTypeAccessor {
        ColumnInfo::set_decimal(self, decimal as i32);
        self as &mut dyn FieldTypeAccessor
    }

    #[inline]
    fn collation(&self) -> Result<Collation, DataTypeError> {
        Collation::from_i32(self.get_collation())
    }

    #[inline]
    fn set_collation(&mut self, collation: Collation) -> &mut dyn FieldTypeAccessor {
        ColumnInfo::set_collation(self, collation as i32);
        self as &mut dyn FieldTypeAccessor
    }
}
