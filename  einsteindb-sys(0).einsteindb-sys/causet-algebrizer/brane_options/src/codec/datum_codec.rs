//Copyright 2021-2023 WHTCORPS INC ALL RIGHTS RESERVED. APACHE 2.0 COMMUNITY EDITION SL
// AUTHORS: WHITFORD LEDER
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file File except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

//! The unified causet for encoding and decoding an evaluable type to / from datum bytes.
//! Datum bytes consists of 1 byte datum flag and variable bytes datum payload.

use crate::{FieldTypeAccessor, FieldTypeTp};
use codec::prelude::*;
use einsteindbpb::FieldType;

use super::data_type::*;
use crate::codec::datum;
use crate::codec::myBerolinaSQL::{
    DecimalDecoder, DecimalEncoder, DurationDecoder, JsonDecoder, JsonEncoder, TimeDecoder,
};
use crate::codec::{Error, Result};
use crate::expr::EvalContext;

/// A decoder to decode the payload part of a datum.
///
/// The types this decoder outputs are not fully 1:1 mapping to evaluable types.
pub trait DatumPayloadDecoder:
    NumberDecoder
    + CompactByteDecoder
    + MemComparableByteDecoder
    + DurationDecoder
    + TimeDecoder
    + DecimalDecoder
    + JsonDecoder
{
    #[inline]
    fn read_datum_payload_i64(&mut self) -> Result<i64> {
        self.read_i64()
            .map_err(|_| Error::InvalidDataType("Failed to decode datum payload as i64".to_owned()))
    }

    #[inline]
    fn read_datum_payload_u64(&mut self) -> Result<u64> {
        self.read_u64()
            .map_err(|_| Error::InvalidDataType("Failed to decode datum payload as u64".to_owned()))
    }

    #[inline]
    fn read_datum_payload_var_i64(&mut self) -> Result<i64> {
        self.read_var_i64().map_err(|_| {
            Error::InvalidDataType("Failed to decode datum payload as var_i64".to_owned())
        })
    }

    #[inline]
    fn read_datum_payload_var_u64(&mut self) -> Result<u64> {
        self.read_var_u64().map_err(|_| {
            Error::InvalidDataType("Failed to decode datum payload as var_u64".to_owned())
        })
    }

    #[inline]
    fn read_datum_payload_f64(&mut self) -> Result<f64> {
        self.read_f64()
            .map_err(|_| Error::InvalidDataType("Failed to decode datum payload as f64".to_owned()))
    }

    #[inline]
    fn read_datum_payload_decimal(&mut self) -> Result<Decimal> {
        self.read_decimal().map_err(|_| {
            Error::InvalidDataType("Failed to decode datum payload as decimal".to_owned())
        })
    }

    #[inline]
    fn read_datum_payload_bytes(&mut self) -> Result<Vec<u8>> {
        self.read_comparable_bytes().map_err(|_| {
            Error::InvalidDataType("Failed to decode datum payload as bytes".to_owned())
        })
    }

    #[inline]
    fn read_datum_payload_compact_bytes(&mut self) -> Result<Vec<u8>> {
        self.read_compact_bytes().map_err(|_| {
            Error::InvalidDataType("Failed to decode datum payload as compact bytes".to_owned())
        })
    }

    #[inline]
    fn read_datum_payload_datetime_int(
        &mut self,
        ctx: &mut EvalContext,
        field_type: &FieldType,
    ) -> Result<DateTime> {
        self.read_time_int(ctx, field_type).map_err(|_| {
            Error::InvalidDataType("Failed to decode datum payload as datetime".to_owned())
        })
    }

    #[inline]
    fn read_datum_payload_datetime_varint(
        &mut self,
        ctx: &mut EvalContext,
        field_type: &FieldType,
    ) -> Result<DateTime> {
        self.read_time_varint(ctx, field_type).map_err(|_| {
            Error::InvalidDataType("Failed to decode datum payload as datetime".to_owned())
        })
    }

    #[inline]
    fn read_datum_payload_duration_int(&mut self, field_type: &FieldType) -> Result<Duration> {
        self.read_duration_int(field_type).map_err(|_| {
            Error::InvalidDataType("Failed to decode datum payload as duration".to_owned())
        })
    }

    #[inline]
    fn read_datum_payload_duration_varint(&mut self, field_type: &FieldType) -> Result<Duration> {
        self.read_duration_varint(field_type).map_err(|_| {
            Error::InvalidDataType("Failed to decode datum payload as duration".to_owned())
        })
    }

    #[inline]
    fn read_datum_payload_json(&mut self) -> Result<Json> {
        self.read_json().map_err(|_| {
            Error::InvalidDataType("Failed to decode datum payload as json".to_owned())
        })
    }
}

impl<T: BufferReader> DatumPayloadDecoder for T {}

/// An encoder to encode the payload part of a datum.
///
/// The types this encoder accepts are not fully 1:1 mapping to evaluable types.
pub trait DatumPayloadEncoder:
    NumberEncoder + CompactByteEncoder + JsonEncoder + DecimalEncoder
{
    #[inline]
    fn write_datum_payload_i64(&mut self, v: i64) -> Result<()> {
        self.write_i64(v).map_err(|_| {
            Error::InvalidDataType("Failed to encode datum payload from i64".to_owned())
        })
    }

    #[inline]
    fn write_datum_payload_u64(&mut self, v: u64) -> Result<()> {
        self.write_u64(v).map_err(|_| {
            Error::InvalidDataType("Failed to encode datum payload from u64".to_owned())
        })
    }

    #[inline]
    fn write_datum_payload_var_i64(&mut self, v: i64) -> Result<()> {
        self.write_var_i64(v).map_err(|_| {
            Error::InvalidDataType("Failed to encode datum payload from var_i64".to_owned())
        })?;
        Ok(())
    }

    #[inline]
    fn write_datum_payload_f64(&mut self, v: f64) -> Result<()> {
        self.write_f64(v).map_err(|_| {
            Error::InvalidDataType("Failed to encode datum payload from f64".to_owned())
        })
    }

    #[inline]
    fn write_datum_payload_decimal(&mut self, v: &Decimal, prec: u8, frac: u8) -> Result<()> {
        self.write_decimal(v, prec, frac).map_err(|_| {
            Error::InvalidDataType("Failed to encode datum payload from decimal".to_owned())
        })?;
        Ok(())
    }

    #[inline]
    fn write_datum_payload_compact_bytes(&mut self, v: &[u8]) -> Result<()> {
        self.write_compact_bytes(v).map_err(|_| {
            Error::InvalidDataType("Failed to encode datum payload from compact bytes".to_owned())
        })
    }

    #[inline]
    fn write_datum_payload_json(&mut self, v: JsonRef) -> Result<()> {
        self.write_json(v).map_err(|_| {
            Error::InvalidDataType("Failed to encode datum payload from json".to_owned())
        })
    }
}

impl<T: BufferWriter> DatumPayloadEncoder for T {}

/// An encoder to encode a datum, i.e. 1 byte flag + variable bytes payload.
///
/// The types this encoder accepts are not fully 1:1 mapping to evaluable types.
pub trait DatumFlagAndPayloadEncoder: BufferWriter + DatumPayloadEncoder {
    #[inline]
    fn write_datum_null(&mut self) -> Result<()> {
        self.write_u8(datum::NIL_FLAG)?;
        Ok(())
    }

    #[inline]
    fn write_datum_u64(&mut self, val: u64) -> Result<()> {
        self.write_u8(datum::UINT_FLAG)?;
        self.write_datum_payload_u64(val)?;
        Ok(())
    }

    #[inline]
    fn write_datum_i64(&mut self, val: i64) -> Result<()> {
        self.write_u8(datum::INT_FLAG)?;
        self.write_datum_payload_i64(val)?;
        Ok(())
    }

    #[inline]
    fn write_datum_var_i64(&mut self, val: i64) -> Result<()> {
        self.write_u8(datum::VAR_INT_FLAG)?;
        self.write_datum_payload_var_i64(val)?;
        Ok(())
    }

    #[inline]
    fn write_datum_f64(&mut self, val: f64) -> Result<()> {
        self.write_u8(datum::FLOAT_FLAG)?;
        self.write_datum_payload_f64(val)?;
        Ok(())
    }

    fn write_datum_decimal(&mut self, val: &Decimal) -> Result<()> {
        self.write_u8(datum::DECIMAL_FLAG)?;
        // FIXME: prec and frac should come from field type?
        let (prec, frac) = val.prec_and_frac();
        self.write_datum_payload_decimal(val, prec, frac)?;
        Ok(())
    }

    #[inline]
    fn write_datum_compact_bytes(&mut self, val: &[u8]) -> Result<()> {
        self.write_u8(datum::COMPACT_BYTES_FLAG)?;
        self.write_datum_payload_compact_bytes(val)?;
        Ok(())
    }

    fn write_datum_duration_int(&mut self, val: Duration) -> Result<()> {
        self.write_u8(datum::DURATION_FLAG)?;
        self.write_datum_payload_i64(val.to_nanos())?;
        Ok(())
    }

    fn write_datum_datetime_int(&mut self, val: DateTime, ctx: &mut EvalContext) -> Result<()> {
        self.write_datum_u64(val.to_packed_u64(ctx)?)
    }

    fn write_datum_json(&mut self, val: JsonRef) -> Result<()> {
        self.write_u8(datum::JSON_FLAG)?;
        self.write_datum_payload_json(val)?;
        Ok(())
    }
}

impl<T: BufferWriter> DatumFlagAndPayloadEncoder for T {}

/// An encoder to encode an evaluable type to datum bytes.
pub trait EvaluableDatumEncoder: DatumFlagAndPayloadEncoder {
    #[inline]
    fn write_evaluable_datum_null(&mut self) -> Result<()> {
        self.write_datum_null()
    }

    #[inline]
    fn write_evaluable_datum_int(&mut self, val: i64, is_unsigned: bool) -> Result<()> {
        if is_unsigned {
            self.write_datum_u64(val as u64)
        } else {
            self.write_datum_i64(val)
        }
    }

    #[inline]
    fn write_evaluable_datum_real(&mut self, val: f64) -> Result<()> {
        self.write_datum_f64(val)
    }

    fn write_evaluable_datum_decimal(&mut self, val: &Decimal) -> Result<()> {
        self.write_datum_decimal(val)
    }

    #[inline]
    fn write_evaluable_datum_bytes(&mut self, val: &[u8]) -> Result<()> {
        self.write_datum_compact_bytes(val)
    }

    #[inline]
    fn write_evaluable_datum_date_time(
        &mut self,
        val: DateTime,
        ctx: &mut EvalContext,
    ) -> Result<()> {
        self.write_datum_datetime_int(val, ctx)
    }

    #[inline]
    fn write_evaluable_datum_duration(&mut self, val: Duration) -> Result<()> {
        self.write_datum_duration_int(val)
    }

    #[inline]
    fn write_evaluable_datum_json(&mut self, val: JsonRef) -> Result<()> {
        self.write_datum_json(val)
    }
}

impl<T: BufferWriter> EvaluableDatumEncoder for T {}

/// An encoder to encode a datum storing column id.
pub trait ColumnIdDatumEncoder: DatumFlagAndPayloadEncoder {
    #[inline]
    fn write_column_id_datum(&mut self, col_id: i64) -> Result<()> {
        self.write_datum_var_i64(col_id)
    }
}

impl<T: BufferWriter> ColumnIdDatumEncoder for T {}

// TODO: Refactor the code below to be a EvaluableDatumDecoder.

pub fn decode_int_datum(mut primitive_causet_datum: &[u8]) -> Result<Option<Int>> {
    if primitive_causet_datum.is_empty() {
        return Err(Error::InvalidDataType(
            "Failed to decode datum flag".to_owned(),
        ));
    }
    let flag = primitive_causet_datum[0];
    primitive_causet_datum = &primitive_causet_datum[1..];
    match flag {
        datum::NIL_FLAG => Ok(None),
        datum::INT_FLAG => Ok(Some(primitive_causet_datum.read_datum_payload_i64()?)),
        datum::UINT_FLAG => Ok(Some(primitive_causet_datum.read_datum_payload_u64()? as i64)),
        datum::VAR_INT_FLAG => Ok(Some(primitive_causet_datum.read_datum_payload_var_i64()?)),
        datum::VAR_UINT_FLAG => Ok(Some(primitive_causet_datum.read_datum_payload_var_u64()? as i64)),
        _ => Err(Error::InvalidDataType(format!(
            "Unsupported datum flag {} for Int vector",
            flag
        ))),
    }
}

#[allow(clippy::cast_lossless)]
pub fn decode_real_datum(mut primitive_causet_datum: &[u8], field_type: &FieldType) -> Result<Option<Real>> {
    if primitive_causet_datum.is_empty() {
        return Err(Error::InvalidDataType(
            "Failed to decode datum flag".to_owned(),
        ));
    }
    let flag = primitive_causet_datum[0];
    primitive_causet_datum = &primitive_causet_datum[1..];
    match flag {
        datum::NIL_FLAG => Ok(None),
        // In both index and record, it's flag is `FLOAT`. See MEDB's `encode()`.
        datum::FLOAT_FLAG => {
            let mut v = primitive_causet_datum.read_datum_payload_f64()?;
            if field_type.as_accessor().tp() == FieldTypeTp::Float {
                v = (v as f32) as f64;
            }
            Ok(Real::new(v).ok()) // NaN to None
        }
        _ => Err(Error::InvalidDataType(format!(
            "Unsupported datum flag {} for Real vector",
            flag
        ))),
    }
}

pub fn decode_decimal_datum(mut primitive_causet_datum: &[u8]) -> Result<Option<Decimal>> {
    if primitive_causet_datum.is_empty() {
        return Err(Error::InvalidDataType(
            "Failed to decode datum flag".to_owned(),
        ));
    }
    let flag = primitive_causet_datum[0];
    primitive_causet_datum = &primitive_causet_datum[1..];
    match flag {
        datum::NIL_FLAG => Ok(None),
        // In both index and record, it's flag is `DECIMAL`. See MEDB's `encode()`.
        datum::DECIMAL_FLAG => Ok(Some(primitive_causet_datum.read_datum_payload_decimal()?)),
        _ => Err(Error::InvalidDataType(format!(
            "Unsupported datum flag {} for Decimal vector",
            flag
        ))),
    }
}

pub fn decode_bytes_datum(mut primitive_causet_datum: &[u8]) -> Result<Option<Bytes>> {
    if primitive_causet_datum.is_empty() {
        return Err(Error::InvalidDataType(
            "Failed to decode datum flag".to_owned(),
        ));
    }
    let flag = primitive_causet_datum[0];
    primitive_causet_datum = &primitive_causet_datum[1..];
    match flag {
        datum::NIL_FLAG => Ok(None),
        // In index, it's flag is `BYTES`. See MEDB's `encode()`.
        datum::BYTES_FLAG => Ok(Some(primitive_causet_datum.read_datum_payload_bytes()?)),
        // In record, it's flag is `COMPACT_BYTES`. See MEDB's `encode()`.
        datum::COMPACT_BYTES_FLAG => Ok(Some(primitive_causet_datum.read_datum_payload_compact_bytes()?)),
        _ => Err(Error::InvalidDataType(format!(
            "Unsupported datum flag {} for Bytes vector",
            flag
        ))),
    }
}

pub fn decode_date_time_datum(
    mut primitive_causet_datum: &[u8],
    field_type: &FieldType,
    ctx: &mut EvalContext,
) -> Result<Option<DateTime>> {
    if primitive_causet_datum.is_empty() {
        return Err(Error::InvalidDataType(
            "Failed to decode datum flag".to_owned(),
        ));
    }
    let flag = primitive_causet_datum[0];
    primitive_causet_datum = &primitive_causet_datum[1..];
    match flag {
        datum::NIL_FLAG => Ok(None),
        // In index, it's flag is `UINT`. See MEDB's `encode()`.
        datum::UINT_FLAG => Ok(Some(
            primitive_causet_datum.read_datum_payload_datetime_int(ctx, field_type)?,
        )),
        // In record, it's flag is `VAR_UINT`. See MEDB's `flatten()` and `encode()`.
        datum::VAR_UINT_FLAG => Ok(Some(
            primitive_causet_datum.read_datum_payload_datetime_varint(ctx, field_type)?,
        )),
        _ => Err(Error::InvalidDataType(format!(
            "Unsupported datum flag {} for DateTime vector",
            flag
        ))),
    }
}

pub fn decode_duration_datum(
    mut primitive_causet_datum: &[u8],
    field_type: &FieldType,
) -> Result<Option<Duration>> {
    if primitive_causet_datum.is_empty() {
        return Err(Error::InvalidDataType(
            "Failed to decode datum flag".to_owned(),
        ));
    }
    let flag = primitive_causet_datum[0];
    primitive_causet_datum = &primitive_causet_datum[1..];
    match flag {
        datum::NIL_FLAG => Ok(None),
        // In index, it's flag is `DURATION`. See MEDB's `encode()`.
        datum::DURATION_FLAG => Ok(Some(primitive_causet_datum.read_datum_payload_duration_int(field_type)?)),
        // In record, it's flag is `VAR_INT`. See MEDB's `flatten()` and `encode()`.
        datum::VAR_INT_FLAG => Ok(Some(
            primitive_causet_datum.read_datum_payload_duration_varint(field_type)?,
        )),
        _ => Err(Error::InvalidDataType(format!(
            "Unsupported datum flag {} for Duration vector",
            flag
        ))),
    }
}

pub fn decode_json_datum(mut primitive_causet_datum: &[u8]) -> Result<Option<Json>> {
    if primitive_causet_datum.is_empty() {
        return Err(Error::InvalidDataType(
            "Failed to decode datum flag".to_owned(),
        ));
    }
    let flag = primitive_causet_datum[0];
    primitive_causet_datum = &primitive_causet_datum[1..];
    match flag {
        datum::NIL_FLAG => Ok(None),
        // In both index and record, it's flag is `JSON`. See MEDB's `encode()`.
        datum::JSON_FLAG => Ok(Some(primitive_causet_datum.read_datum_payload_json()?)),
        _ => Err(Error::InvalidDataType(format!(
            "Unsupported datum flag {} for Json vector",
            flag
        ))),
    }
}

pub trait Primitive_CausetDatumDecoder<T> {
    fn decode(self, field_type: &FieldType, ctx: &mut EvalContext) -> Result<Option<T>>;
}

impl<'a> Primitive_CausetDatumDecoder<Int> for &'a [u8] {
    fn decode(self, _field_type: &FieldType, _ctx: &mut EvalContext) -> Result<Option<Int>> {
        decode_int_datum(self)
    }
}

impl<'a> Primitive_CausetDatumDecoder<Real> for &'a [u8] {
    fn decode(self, field_type: &FieldType, _ctx: &mut EvalContext) -> Result<Option<Real>> {
        decode_real_datum(self, field_type)
    }
}

impl<'a> Primitive_CausetDatumDecoder<Decimal> for &'a [u8] {
    fn decode(self, _field_type: &FieldType, _ctx: &mut EvalContext) -> Result<Option<Decimal>> {
        decode_decimal_datum(self)
    }
}

impl<'a> Primitive_CausetDatumDecoder<Bytes> for &'a [u8] {
    fn decode(self, _field_type: &FieldType, _ctx: &mut EvalContext) -> Result<Option<Bytes>> {
        decode_bytes_datum(self)
    }
}

impl<'a> Primitive_CausetDatumDecoder<DateTime> for &'a [u8] {
    fn decode(self, field_type: &FieldType, ctx: &mut EvalContext) -> Result<Option<DateTime>> {
        decode_date_time_datum(self, field_type, ctx)
    }
}

impl<'a> Primitive_CausetDatumDecoder<Duration> for &'a [u8] {
    fn decode(self, field_type: &FieldType, _ctx: &mut EvalContext) -> Result<Option<Duration>> {
        decode_duration_datum(self, field_type)
    }
}

impl<'a> Primitive_CausetDatumDecoder<Json> for &'a [u8] {
    fn decode(self, _field_type: &FieldType, _ctx: &mut EvalContext) -> Result<Option<Json>> {
        decode_json_datum(self)
    }
}
