use crate::misc::{BasicUtf8Error, ExtUtf8Error, IncompleteUtf8Char, StdUtf8Error};

/// Internally uses `atoi` if the feature is active.
#[cfg(not(feature = "atoi"))]
#[inline]
pub fn atoi<T>(bytes: &[u8]) -> crate::Result<T>
where
  T: core::str::FromStr,
  T::Err: Into<crate::Error>,
{
  Ok(from_utf8_basic(bytes)?.parse().map_err(Into::into)?)
}
/// Internally uses `atoi` if the feature is active.
#[cfg(feature = "atoi")]
#[inline]
pub fn atoi<T>(bytes: &[u8]) -> crate::Result<T>
where
  T: atoi::FromRadix10SignedChecked,
{
  atoi::atoi(bytes).ok_or(crate::Error::AtoiInvalidBytes)
}

/// Internally uses `memchr` if the feature is active.
#[inline]
pub fn bytes_pos1<B>(bytes: B, elem: u8) -> Option<usize>
where
  B: AsRef<[u8]>,
{
  #[cfg(feature = "memchr")]
  return memchr::memchr(elem, bytes.as_ref());
  #[cfg(not(feature = "memchr"))]
  return bytes.as_ref().iter().position(|byte| *byte == elem);
}

/// Internally uses `memchr` if the feature is active.
#[inline]
pub fn bytes_rsplit1(bytes: &[u8], elem: u8) -> impl Iterator<Item = &[u8]> {
  #[cfg(feature = "memchr")]
  return memchr::memrchr_iter(elem, bytes)
    .map(|el| el.wrapping_add(1))
    .chain(core::iter::once(0))
    .scan(bytes.len(), move |end, begin| {
      let rslt = bytes.get(begin..*end);
      *end = begin.wrapping_sub(1);
      rslt
    });
  #[cfg(not(feature = "memchr"))]
  return bytes.rsplit(move |byte| *byte == elem);
}

/// Internally uses `memchr` if the feature is active.
#[inline]
pub fn bytes_split1(bytes: &[u8], elem: u8) -> impl Iterator<Item = &[u8]> {
  #[cfg(feature = "memchr")]
  return memchr::memchr_iter(elem, bytes).chain(core::iter::once(bytes.len())).scan(
    0,
    move |begin, end| {
      let rslt = bytes.get(*begin..end);
      *begin = end.wrapping_add(1);
      rslt
    },
  );
  #[cfg(not(feature = "memchr"))]
  return bytes.split(move |byte| *byte == elem);
}

/// Internally uses `simdutf8` if the feature is active.
#[inline]
pub fn from_utf8_basic(bytes: &[u8]) -> Result<&str, BasicUtf8Error> {
  #[cfg(feature = "simdutf8")]
  return simdutf8::basic::from_utf8(bytes).ok().ok_or(BasicUtf8Error {});
  #[cfg(not(feature = "simdutf8"))]
  return core::str::from_utf8(bytes).ok().ok_or(BasicUtf8Error {});
}

/// Internally uses `simdutf8` if the feature is active.
#[inline]
pub fn from_utf8_ext(bytes: &[u8]) -> Result<&str, ExtUtf8Error> {
  let err = match from_utf8_std(bytes) {
    Ok(elem) => return Ok(elem),
    Err(error) => error,
  };
  let (_valid_bytes, after_valid) = bytes.split_at(err.valid_up_to);
  match err.error_len {
    None => Err(ExtUtf8Error::Incomplete {
      incomplete_ending_char: {
        IncompleteUtf8Char::new(after_valid).ok_or(ExtUtf8Error::Invalid)?
      },
    }),
    Some(_) => Err(ExtUtf8Error::Invalid),
  }
}

/// Internally uses `simdutf8` if the feature is active.
#[inline]
pub fn from_utf8_std(bytes: &[u8]) -> Result<&str, StdUtf8Error> {
  #[cfg(feature = "simdutf8")]
  return simdutf8::compat::from_utf8(bytes).map_err(|element| StdUtf8Error {
    valid_up_to: element.valid_up_to(),
    error_len: element.error_len(),
  });
  #[cfg(not(feature = "simdutf8"))]
  return core::str::from_utf8(bytes).map_err(|element| StdUtf8Error {
    valid_up_to: element.valid_up_to(),
    error_len: element.error_len(),
  });
}

/// Internally uses `memchr` if the feature is active.
#[inline]
pub fn str_pos1(str: &str, elem: u8) -> Option<usize> {
  #[cfg(feature = "memchr")]
  return memchr::memchr(elem, str.as_bytes());
  #[cfg(not(feature = "memchr"))]
  return str.as_bytes().iter().position(|byte| *byte == elem);
}

/// Internally uses `memchr` if the feature is active.
#[inline]
pub fn str_rpos1(str: &str, elem: u8) -> Option<usize> {
  #[cfg(feature = "memchr")]
  return memchr::memrchr(elem, str.as_bytes());
  #[cfg(not(feature = "memchr"))]
  return str.as_bytes().iter().rev().position(|byte| *byte == elem);
}

/// Internally uses `memchr` if the feature is active.
#[inline]
pub fn str_rsplit_once1(str: &str, elem: u8) -> Option<(&str, &str)> {
  let idx = str_pos1(str, elem)?;
  Some((str.get(..idx)?, str.get(idx.wrapping_add(1)..)?))
}

/// Internally uses `memchr` if the feature is active.
#[inline]
pub fn str_split1(str: &str, elem: u8) -> impl Iterator<Item = &str> {
  #[cfg(feature = "memchr")]
  return memchr::memchr_iter(elem, str.as_bytes()).chain(core::iter::once(str.len())).scan(
    0,
    move |begin, end| {
      let rslt = str.get(*begin..end);
      *begin = end.wrapping_add(1);
      rslt
    },
  );
  #[cfg(not(feature = "memchr"))]
  return str
    .as_bytes()
    .split(move |el| *el == elem)
    .filter_map(|bytes| from_utf8_basic(bytes).ok());
}

/// Internally uses `memchr` if the feature is active.
#[inline]
pub fn str_split_once1(str: &str, elem: u8) -> Option<(&str, &str)> {
  let idx = str_pos1(str, elem)?;
  Some((str.get(..idx)?, str.get(idx.wrapping_add(1)..)?))
}
