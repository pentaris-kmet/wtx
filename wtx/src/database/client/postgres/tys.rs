macro_rules! proptest {
  ($name:ident, $ty:ty) => {
    #[cfg(feature = "_proptest")]
    #[cfg(test)]
    #[test_strategy::proptest]
    fn $name(instance: $ty) {
      let mut vec = &mut crate::misc::Vector::new();
      let mut fbw = crate::misc::FilledBufferWriter::new(0, &mut vec);
      let mut ev = EncodeValue::new(&mut fbw);
      Encode::<Postgres<crate::Error>>::encode(&instance, &mut ev).unwrap();
      let decoded: $ty = Decode::<Postgres<crate::Error>>::decode(&DecodeValue::new(
        ev.fbw()._curr_bytes(),
        &crate::database::client::postgres::Ty::Any,
      ))
      .unwrap();
      assert_eq!(instance, decoded);
      vec.clear();
    }
  };
}

macro_rules! test {
  ($name:ident, $ty:ty, $instance:expr) => {
    #[cfg(test)]
    #[test]
    fn $name() {
      let mut vec = &mut crate::misc::Vector::new();
      let mut fbw = crate::misc::FilledBufferWriter::new(0, &mut vec);
      let mut ev = EncodeValue::new(&mut fbw);
      let instance: $ty = $instance;
      Encode::<Postgres<crate::Error>>::encode(&instance, &mut ev).unwrap();
      let decoded: $ty = Decode::<Postgres<crate::Error>>::decode(&DecodeValue::new(
        ev.fbw()._curr_bytes(),
        &crate::database::client::postgres::Ty::Any,
      ))
      .unwrap();
      assert_eq!(instance, decoded);
    }
  };
}

mod array {
  use crate::{
    database::{
      client::postgres::{DecodeValue, EncodeValue, Postgres, Ty},
      Decode, Encode, Typed,
    },
    misc::{from_utf8_basic, ArrayString},
  };

  impl<E, const N: usize> Decode<'_, Postgres<E>> for ArrayString<N>
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn decode(input: &DecodeValue<'_>) -> Result<Self, E> {
      Ok(from_utf8_basic(input.bytes()).map_err(Into::into)?.try_into().map_err(Into::into)?)
    }
  }
  impl<E, const N: usize> Encode<Postgres<E>> for ArrayString<N>
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn encode(&self, ev: &mut EncodeValue<'_, '_>) -> Result<(), E> {
      ev.fbw()._extend_from_slice(self.as_str().as_bytes()).map_err(Into::into)?;
      Ok(())
    }
  }
  impl<E, const N: usize> Typed<Postgres<E>> for ArrayString<N>
  where
    E: From<crate::Error>,
  {
    const TY: Ty = Ty::Text;
  }

  test!(array_string, ArrayString<4>, ArrayString::try_from("123").unwrap());
}

#[cfg(feature = "chrono")]
mod chrono {
  use crate::database::{
    client::postgres::{DecodeValue, EncodeValue, Postgres, PostgresError, Ty},
    Decode, Encode, Typed,
  };
  use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeDelta, TimeZone, Utc};

  const MIN_PG_ND: Option<NaiveDate> = NaiveDate::from_ymd_opt(-4713, 1, 1);
  const MAX_CHRONO_ND: Option<NaiveDate> = NaiveDate::from_ymd_opt(262142, 1, 1);

  impl<E> Decode<'_, Postgres<E>> for DateTime<Utc>
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn decode(dv: &DecodeValue<'_>) -> Result<Self, E> {
      let naive = <NaiveDateTime as Decode<Postgres<E>>>::decode(dv)?;
      Ok(Utc.from_utc_datetime(&naive))
    }
  }
  impl<E, TZ> Encode<Postgres<E>> for DateTime<TZ>
  where
    E: From<crate::Error>,
    TZ: TimeZone,
  {
    #[inline]
    fn encode(&self, ev: &mut EncodeValue<'_, '_>) -> Result<(), E> {
      Encode::<Postgres<E>>::encode(&self.naive_utc(), ev)
    }
  }
  impl<E, TZ> Typed<Postgres<E>> for DateTime<TZ>
  where
    E: From<crate::Error>,
    TZ: TimeZone,
  {
    const TY: Ty = Ty::Timestamptz;
  }

  impl<E> Decode<'_, Postgres<E>> for NaiveDate
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn decode(dv: &DecodeValue<'_>) -> Result<Self, E> {
      let days: i32 = Decode::<Postgres<E>>::decode(dv)?;
      pg_epoch_nd()
        .and_then(|el| el.checked_add_signed(TimeDelta::try_days(days.into())?))
        .ok_or_else(|| {
          E::from(PostgresError::UnexpectedValueFromBytes { expected: "timestamp" }.into())
        })
    }
  }
  impl<E> Encode<Postgres<E>> for NaiveDate
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn encode(&self, ev: &mut EncodeValue<'_, '_>) -> Result<(), E> {
      Encode::<Postgres<E>>::encode(
        &match pg_epoch_nd().and_then(|epoch| {
          if self < &MIN_PG_ND? || self > &MAX_CHRONO_ND? {
            return None;
          }
          i32::try_from(self.signed_duration_since(epoch).num_days()).ok()
        }) {
          Some(time) => time,
          None => {
            return Err(E::from(
              PostgresError::UnexpectedValueFromBytes { expected: "date" }.into(),
            ))
          }
        },
        ev,
      )
    }
  }
  impl<E> Typed<Postgres<E>> for NaiveDate
  where
    E: From<crate::Error>,
  {
    const TY: Ty = Ty::Date;
  }

  impl<E> Decode<'_, Postgres<E>> for NaiveDateTime
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn decode(input: &DecodeValue<'_>) -> Result<Self, E> {
      let timestamp = Decode::<Postgres<E>>::decode(input)?;
      pg_epoch_ndt()
        .and_then(|el| el.checked_add_signed(Duration::microseconds(timestamp)))
        .ok_or_else(|| {
          E::from(PostgresError::UnexpectedValueFromBytes { expected: "timestamp" }.into())
        })
    }
  }
  impl<E> Encode<Postgres<E>> for NaiveDateTime
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn encode(&self, ev: &mut EncodeValue<'_, '_>) -> Result<(), E> {
      Encode::<Postgres<E>>::encode(
        &match pg_epoch_ndt().and_then(|epoch| {
          if self < &MIN_PG_ND?.and_hms_opt(0, 0, 0)?
            || self > &MAX_CHRONO_ND?.and_hms_opt(0, 0, 0)?
          {
            return None;
          }
          self.signed_duration_since(epoch).num_microseconds()
        }) {
          Some(time) => time,
          None => {
            return Err(E::from(
              PostgresError::UnexpectedValueFromBytes { expected: "timestamp" }.into(),
            ))
          }
        },
        ev,
      )
    }
  }
  impl<E> Typed<Postgres<E>> for NaiveDateTime
  where
    E: From<crate::Error>,
  {
    const TY: Ty = Ty::Timestamp;
  }

  fn pg_epoch_nd() -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(2000, 1, 1)
  }

  fn pg_epoch_ndt() -> Option<NaiveDateTime> {
    pg_epoch_nd()?.and_hms_opt(0, 0, 0)
  }

  test!(datetime_utc, DateTime<Utc>, Utc.from_utc_datetime(&pg_epoch_ndt().unwrap()));
}

mod collections {
  use crate::{
    database::{
      client::postgres::{DecodeValue, EncodeValue, Postgres, Ty},
      Decode, Encode, Typed,
    },
    misc::from_utf8_basic,
  };
  use alloc::string::String;

  // &[u8]

  impl<'exec, E> Decode<'exec, Postgres<E>> for &'exec [u8]
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn decode(input: &DecodeValue<'exec>) -> Result<Self, E> {
      Ok(input.bytes())
    }
  }
  impl<E> Encode<Postgres<E>> for &[u8]
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn encode(&self, ev: &mut EncodeValue<'_, '_>) -> Result<(), E> {
      ev.fbw()._extend_from_slice(self).map_err(Into::into)?;
      Ok(())
    }
  }
  impl<E> Typed<Postgres<E>> for &[u8]
  where
    E: From<crate::Error>,
  {
    const TY: Ty = Ty::ByteaArray;
  }
  test!(bytes, &[u8], &[1, 2, 3, 4]);

  // &str

  impl<'exec, E> Decode<'exec, Postgres<E>> for &'exec str
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn decode(input: &DecodeValue<'exec>) -> Result<Self, E> {
      Ok(from_utf8_basic(input.bytes()).map_err(crate::Error::from)?)
    }
  }
  impl<E> Encode<Postgres<E>> for &str
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn encode(&self, ev: &mut EncodeValue<'_, '_>) -> Result<(), E> {
      ev.fbw()._extend_from_slice(self.as_bytes()).map_err(Into::into)?;
      Ok(())
    }
  }
  impl<E> Typed<Postgres<E>> for &str
  where
    E: From<crate::Error>,
  {
    const TY: Ty = Ty::Text;
  }
  test!(str, &str, "1234");

  // String

  impl<E> Decode<'_, Postgres<E>> for String
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn decode(input: &DecodeValue<'_>) -> Result<Self, E> {
      Ok(from_utf8_basic(input.bytes()).map_err(crate::Error::from)?.into())
    }
  }
  impl<E> Encode<Postgres<E>> for String
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn encode(&self, ev: &mut EncodeValue<'_, '_>) -> Result<(), E> {
      ev.fbw()._extend_from_slice(self.as_bytes()).map_err(Into::into)?;
      Ok(())
    }
  }
  impl<E> Typed<Postgres<E>> for String
  where
    E: From<crate::Error>,
  {
    const TY: Ty = Ty::Text;
  }
  proptest!(string, String);
}

mod pg_numeric {
  use crate::{
    database::{
      client::postgres::{DecodeValue, EncodeValue, Postgres, PostgresError},
      Decode, Encode,
    },
    misc::{ArrayVector, Usize},
  };

  const _DIGITS_CAP: usize = 64;
  const SIGN_NAN: u16 = 0xC000;
  const SIGN_NEG: u16 = 0x4000;
  const SIGN_POS: u16 = 0x0000;

  pub(crate) enum _PgNumeric {
    NaN,
    Number { digits: ArrayVector<i16, _DIGITS_CAP>, scale: u16, sign: Sign, weight: i16 },
  }

  impl<E> Decode<'_, Postgres<E>> for _PgNumeric
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn decode(input: &DecodeValue<'_>) -> Result<Self, E> {
      let [a, b, c, d, e, f, g, h, rest @ ..] = input.bytes() else {
        return Err(E::from(
          PostgresError::UnexpectedBufferSize {
            expected: 8,
            received: Usize::from(input.bytes().len()).into(),
          }
          .into(),
        ));
      };
      let digits = u16::from_be_bytes([*a, *b]);
      let digits_usize = usize::from(digits);
      let weight = i16::from_be_bytes([*c, *d]);
      let sign = u16::from_be_bytes([*e, *f]);
      let scale = u16::from_be_bytes([*g, *h]);
      let mut curr_slice = rest;
      Ok(if sign == SIGN_NAN {
        _PgNumeric::NaN
      } else {
        if digits_usize > _DIGITS_CAP || digits_usize > 0x7FFF {
          return Err(E::from(PostgresError::VeryLargeDecimal.into()));
        }
        let mut array = [0i16; _DIGITS_CAP];
        for elem in array.iter_mut().take(digits_usize) {
          let [i, j, local_rest @ ..] = curr_slice else {
            break;
          };
          *elem = i16::from_be_bytes([*i, *j]);
          curr_slice = local_rest;
        }
        _PgNumeric::Number {
          digits: ArrayVector::from_parts(array, digits.into()),
          scale,
          sign: Sign::try_from(sign)?,
          weight,
        }
      })
    }
  }
  impl<E> Encode<Postgres<E>> for _PgNumeric
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn encode(&self, ev: &mut EncodeValue<'_, '_>) -> Result<(), E> {
      match self {
        _PgNumeric::NaN => {
          ev.fbw()._extend_from_slice(&0i16.to_be_bytes()).map_err(Into::into)?;
          ev.fbw()._extend_from_slice(&0i16.to_be_bytes()).map_err(Into::into)?;
          ev.fbw()._extend_from_slice(&SIGN_NAN.to_be_bytes()).map_err(Into::into)?;
          ev.fbw()._extend_from_slice(&0u16.to_be_bytes()).map_err(Into::into)?;
        }
        _PgNumeric::Number { digits, scale, sign, weight } => {
          let len: i16 = digits.len().try_into().map_err(Into::into)?;
          ev.fbw()._extend_from_slice(&len.to_be_bytes()).map_err(Into::into)?;
          ev.fbw()._extend_from_slice(&weight.to_be_bytes()).map_err(Into::into)?;
          ev.fbw()._extend_from_slice(&u16::from(*sign).to_be_bytes()).map_err(Into::into)?;
          ev.fbw()._extend_from_slice(&scale.to_be_bytes()).map_err(Into::into)?;
          for digit in digits {
            ev.fbw()._extend_from_slice(&digit.to_be_bytes()).map_err(Into::into)?;
          }
        }
      }
      Ok(())
    }
  }

  #[derive(Clone, Copy, Debug, Eq, PartialEq)]
  pub(crate) enum Sign {
    Negative,
    Positive,
  }

  impl From<Sign> for u16 {
    #[inline]
    fn from(from: Sign) -> Self {
      match from {
        Sign::Negative => SIGN_NEG,
        Sign::Positive => SIGN_POS,
      }
    }
  }

  impl TryFrom<u16> for Sign {
    type Error = crate::Error;

    #[inline]
    fn try_from(from: u16) -> Result<Self, Self::Error> {
      Ok(match from {
        SIGN_NAN => return Err(PostgresError::DecimalCanNotBeConvertedFromNaN.into()),
        SIGN_NEG => Self::Negative,
        SIGN_POS => Self::Positive,
        _ => return Err(crate::Error::MISC_UnexpectedUint { received: from.into() }),
      })
    }
  }
}

mod primitives {
  use crate::{
    database::{
      client::postgres::{DecodeValue, EncodeValue, Postgres, PostgresError, Ty},
      Decode, Encode, Typed,
    },
    misc::Usize,
  };

  // bool

  impl<E> Decode<'_, Postgres<E>> for bool
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn decode(input: &DecodeValue<'_>) -> Result<Self, E> {
      let &[byte] = input.bytes() else {
        return Err(E::from(
          PostgresError::UnexpectedBufferSize {
            expected: 1,
            received: Usize::from(input.bytes().len()).into(),
          }
          .into(),
        ));
      };
      Ok(byte != 0)
    }
  }
  impl<E> Encode<Postgres<E>> for bool
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn encode(&self, ev: &mut EncodeValue<'_, '_>) -> Result<(), E> {
      ev.fbw()._extend_from_byte((*self).into()).map_err(Into::into)?;
      Ok(())
    }
  }
  impl<E> Typed<Postgres<E>> for bool
  where
    E: From<crate::Error>,
  {
    const TY: Ty = Ty::Bool;
  }

  proptest!(bool_true, bool);
  proptest!(bool_false, bool);

  macro_rules! impl_integer_from_array {
    ($instance:expr, [$($elem:ident),+], ($signed:ident, $signed_pg_ty:expr), ($unsigned:ident, $unsigned_pg_ty:expr)) => {
      impl_primitive_from_array!($instance, [$($elem),+], $signed, $signed_pg_ty);

      impl<E> Decode<'_, Postgres<E>> for $unsigned
      where
        E: From<crate::Error>,
      {
        #[inline]
        fn decode(input: &DecodeValue<'_>) -> Result<Self, E> {
          <$signed as Decode::<Postgres<E>>>::decode(input)?
            .try_into()
            .map_err(|_err| E::from(PostgresError::InvalidPostgresUint.into()))
        }
      }
      impl<E> Encode<Postgres<E>> for $unsigned
      where
        E: From<crate::Error>,
      {
        #[inline]
        fn encode(&self, ev: &mut EncodeValue<'_, '_>) -> Result<(), E> {
          if *self >> const { $unsigned::BITS - 1 } == 1 {
            return Err(E::from(PostgresError::InvalidPostgresUint.into()));
          }
          ev.fbw()._extend_from_slice(&self.to_be_bytes()).map_err(Into::into)?;
          Ok(())
        }
      }
      impl<E> Typed<Postgres<E>> for $unsigned
      where
        E: From<crate::Error>
      {
        const TY: Ty = $unsigned_pg_ty;
      }

      test!($unsigned, $unsigned, $instance);
    };
  }

  macro_rules! impl_primitive_from_array {
    ($instance:expr, [$($elem:ident),+], $ty:ident, $pg_ty:expr) => {
      impl<E> Decode<'_, Postgres<E>> for $ty
      where
        E: From<crate::Error>,
      {
        #[inline]
        fn decode(input: &DecodeValue<'_>) -> Result<Self, E> {
          if let &[$($elem,)+] = input.bytes() {
            return Ok(<$ty>::from_be_bytes([$($elem),+]));
          }
          Err(E::from(PostgresError::UnexpectedBufferSize {
            expected: Usize::from(size_of::<$ty>()).into(),
            received: Usize::from(input.bytes().len()).into()
          }.into()))
        }
      }

      impl<E> Encode<Postgres<E>> for $ty
      where
        E: From<crate::Error>,
      {
        #[inline]
        fn encode(&self, ev: &mut EncodeValue<'_, '_>) -> Result<(), E> {
          ev.fbw()._extend_from_slice(&self.to_be_bytes()).map_err(Into::into)?;
          Ok(())
        }
      }

      impl<E> Typed<Postgres<E>> for $ty
      where
        E: From<crate::Error>
      {
        const TY: Ty = $pg_ty;
      }

      test!($ty, $ty, $instance);
    }
  }

  impl_integer_from_array!(37, [a], (i8, Ty::Char), (u8, Ty::Bytea));
  impl_integer_from_array!(37, [a, b], (i16, Ty::Int2), (u16, Ty::Int2));
  impl_integer_from_array!(37, [a, b, c, d], (i32, Ty::Int4), (u32, Ty::Int4));
  impl_integer_from_array!(37, [a, b, c, d, e, f, g, h], (i64, Ty::Int8), (u64, Ty::Int8));

  impl_primitive_from_array!(37.0, [a, b, c, d], f32, Ty::Float4);
  impl_primitive_from_array!(37.0, [a, b, c, d, e, f, g, h], f64, Ty::Float8);
}

#[cfg(feature = "rust_decimal")]
mod rust_decimal {
  use crate::{
    database::{
      client::postgres::{
        tys::pg_numeric::{Sign, _PgNumeric},
        DecodeValue, EncodeValue, Postgres, PostgresError, Ty,
      },
      Decode, Encode, Typed,
    },
    misc::ArrayVector,
  };
  use rust_decimal::{Decimal, MathematicalOps};

  impl<E> Decode<'_, Postgres<E>> for Decimal
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn decode(input: &DecodeValue<'_>) -> Result<Self, E> {
      let pg_numeric = _PgNumeric::decode(input)?;
      let (digits, sign, mut weight, scale) = match pg_numeric {
        _PgNumeric::NaN => {
          return Err(E::from(PostgresError::DecimalCanNotBeConvertedFromNaN.into()));
        }
        _PgNumeric::Number { digits, sign, weight, scale } => (digits, sign, weight, scale),
      };
      if digits.is_empty() {
        return Ok(0u64.into());
      }
      let mut value = Decimal::ZERO;
      for digit in digits.into_iter() {
        let mut operations = || {
          let mul = Decimal::from(10_000u16).checked_powi(weight.into())?;
          let part = Decimal::from(digit).checked_mul(mul)?;
          value = value.checked_add(part)?;
          weight = weight.checked_sub(1)?;
          Some(())
        };
        operations().ok_or_else(|| crate::Error::MISC_OutOfBoundsArithmetic)?;
      }
      match sign {
        Sign::Positive => value.set_sign_positive(true),
        Sign::Negative => value.set_sign_negative(true),
      }
      value.rescale(scale.into());
      Ok(value)
    }
  }
  impl<E> Encode<Postgres<E>> for Decimal
  where
    E: From<crate::Error>,
  {
    #[inline]
    fn encode(&self, ev: &mut EncodeValue<'_, '_>) -> Result<(), E> {
      if self.is_zero() {
        let rslt = _PgNumeric::Number {
          digits: ArrayVector::new(),
          scale: 0,
          sign: Sign::Positive,
          weight: 0,
        };
        rslt.encode(ev)?;
        return Ok(());
      }

      let scale = self.scale() as u16;

      let mut mantissa = u128::from_le_bytes(self.serialize());
      mantissa >>= 32;
      let diff = scale % 4;
      if diff > 0 {
        let remainder = 4u32.wrapping_sub(u32::from(diff));
        mantissa = mantissa.wrapping_mul(u128::from(10u32.pow(remainder)));
      }

      let mut digits = ArrayVector::new();
      while mantissa != 0 {
        digits.push((mantissa % 10_000) as i16).map_err(From::from)?;
        mantissa /= 10_000;
      }
      digits.reverse();

      let after_decimal = usize::from(scale.wrapping_add(3) / 4);
      let weight = digits.len().wrapping_sub(after_decimal).wrapping_sub(1) as i16;

      while let Some(&0) = digits.last() {
        let _ = digits.pop();
      }

      let rslt = _PgNumeric::Number {
        digits,
        scale,
        sign: match self.is_sign_negative() {
          false => Sign::Positive,
          true => Sign::Negative,
        },
        weight,
      };
      rslt.encode(ev)?;
      Ok(())
    }
  }
  impl<E> Typed<Postgres<E>> for Decimal
  where
    E: From<crate::Error>,
  {
    const TY: Ty = Ty::Numeric;
  }

  proptest!(rust_decimal, Decimal);
}
