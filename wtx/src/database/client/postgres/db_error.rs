use crate::{
  database::client::postgres::{PostgresError, SqlState},
  misc::{FromRadix10, Usize, _usize_range_from_u32_range, into_rslt, str_split1},
};
use alloc::boxed::Box;
use core::{
  fmt::{Debug, Formatter},
  ops::Range,
};

/// Position of an error in a query.
#[derive(Debug, Eq, PartialEq)]
pub enum ErrorPosition {
  /// Position in an internally generated query.
  Internal {
    /// Byte position.
    position: u32,
    /// Query generated by the server.
    query: Range<u32>,
  },
  /// Position in the original query.
  Original(u32),
}

_create_enum! {
  /// The severity of a Postgres error or notice.
  #[derive(Clone, Copy, Debug, Eq, PartialEq)]
  pub enum Severity<u8> {
    /// Debug
    Debug = (0, "DEBUG"),
    /// Error
    Error = (1, "ERROR"),
    /// Fatal
    Fatal = (2, "FATAL"),
    /// Info
    Info = (3, "INFO"),
    /// Log
    Log = (4, "LOG"),
    /// Notice
    Notice = (5, "NOTICE"),
    /// Panic
    Panic = (6, "PANIC"),
    /// Warning
    Warning = (7, "WARNING"),
  }
}

/// A Postgres error or notice.
#[derive(Eq, PartialEq)]
pub struct DbError {
  buffer: Box<str>,
  code: SqlState,
  column: Option<Range<u32>>,
  constraint: Option<Range<u32>>,
  datatype: Option<Range<u32>>,
  detail: Option<Range<u32>>,
  file: Option<Range<u32>>,
  hint: Option<Range<u32>>,
  line: Option<u32>,
  message: Range<u32>,
  position: Option<ErrorPosition>,
  routine: Option<Range<u32>>,
  scheme: Option<Range<u32>>,
  severity_localized: Range<u32>,
  severity_nonlocalized: Option<Severity>,
  table: Option<Range<u32>>,
  r#where: Option<Range<u32>>,
}

impl DbError {
  /// The SQLSTATE code for the error
  #[inline]
  pub fn code(&self) -> &SqlState {
    &self.code
  }

  /// If the error was associated with a specific table column, the name of the column.
  #[inline]
  pub fn column(&self) -> Option<&str> {
    self
      .column
      .as_ref()
      .and_then(|range| self.buffer.get(_usize_range_from_u32_range(range.clone())))
  }

  /// If the error was associated with a specific constraint, the name of the constraint.
  #[inline]
  pub fn constraint(&self) -> Option<&str> {
    self
      .constraint
      .as_ref()
      .and_then(|range| self.buffer.get(_usize_range_from_u32_range(range.clone())))
  }

  /// If the error was associated with a specific data type, the name of the data type.
  #[inline]
  pub fn datatype(&self) -> Option<&str> {
    self
      .datatype
      .as_ref()
      .and_then(|range| self.buffer.get(_usize_range_from_u32_range(range.clone())))
  }

  /// An optional secondary error message carrying more detail about the problem. Might run to
  /// multiple lines.
  #[inline]
  pub fn detail(&self) -> Option<&str> {
    self
      .detail
      .as_ref()
      .and_then(|range| self.buffer.get(_usize_range_from_u32_range(range.clone())))
  }

  /// The file name of the source-code location where the error was reported.
  #[inline]
  pub fn file(&self) -> Option<&str> {
    self.file.as_ref().and_then(|range| self.buffer.get(_usize_range_from_u32_range(range.clone())))
  }

  /// An optional suggestion what to do about the problem.
  #[inline]
  pub fn hint(&self) -> Option<&str> {
    self.hint.as_ref().and_then(|range| self.buffer.get(_usize_range_from_u32_range(range.clone())))
  }

  /// The line number of the source-code location where the error was reported.
  #[inline]
  pub fn line(&self) -> Option<u32> {
    self.line
  }

  /// The primary human-readable error message.
  #[inline]
  pub fn message(&self) -> &str {
    self.buffer.get(_usize_range_from_u32_range(self.message.clone())).unwrap_or_default()
  }

  /// The field value is a decimal ASCII integer, indicating an error cursor position as an index
  /// into the original query string.
  #[inline]
  pub fn position(&self) -> Option<&ErrorPosition> {
    self.position.as_ref()
  }

  /// The name of the source-code routine reporting the error.
  #[inline]
  pub fn routine(&self) -> Option<&str> {
    self
      .routine
      .as_ref()
      .and_then(|range| self.buffer.get(_usize_range_from_u32_range(range.clone())))
  }

  /// If the error was associated with a specific database object, the name of the schema
  /// containing that object, if any.
  #[inline]
  pub fn scheme(&self) -> Option<&str> {
    self
      .scheme
      .as_ref()
      .and_then(|range| self.buffer.get(_usize_range_from_u32_range(range.clone())))
  }

  /// Localized severity.
  #[inline]
  pub fn severity_localized(&self) -> &str {
    self
      .buffer
      .get(_usize_range_from_u32_range(self.severity_localized.clone()))
      .unwrap_or_default()
  }

  /// Nonlocalized `severity`.
  #[inline]
  pub fn severity_nonlocalized(&self) -> Option<Severity> {
    self.severity_nonlocalized
  }

  /// If the error was associated with a specific table, the name of the table.
  #[inline]
  pub fn table(&self) -> Option<&str> {
    self
      .table
      .as_ref()
      .and_then(|range| self.buffer.get(_usize_range_from_u32_range(range.clone())))
  }

  /// An indication of the context in which the error occurred. Presently this includes a call
  /// stack traceback of active procedural language functions and internally-generated queries.
  #[inline]
  pub fn r#where(&self) -> Option<&str> {
    self
      .r#where
      .as_ref()
      .and_then(|range| self.buffer.get(_usize_range_from_u32_range(range.clone())))
  }
}

impl Debug for DbError {
  #[inline]
  fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("DbError")
      .field("code", &self.code())
      .field("column", &self.column())
      .field("constraint", &self.constraint())
      .field("datatype", &self.datatype())
      .field("detail", &self.detail())
      .field("file", &self.file())
      .field("hint", &self.hint())
      .field("line", &self.line())
      .field("message", &self.message())
      .field("position", &self.position())
      .field("routine", &self.routine())
      .field("schema", &self.scheme())
      .field("severity_localized", &self.severity_localized())
      .field("severity_nonlocalized", &self.severity_nonlocalized())
      .field("table", &self.table())
      .field("where", &self.r#where())
      .finish()
  }
}

impl TryFrom<&str> for DbError {
  type Error = crate::Error;

  #[inline]
  fn try_from(from: &str) -> Result<Self, Self::Error> {
    let mut code = None;
    let mut column = None;
    let mut constraint = None;
    let mut datatype = None;
    let mut detail = None;
    let mut file = None;
    let mut hint = None;
    let mut internal_position = None;
    let mut internal_query = None;
    let mut line = None;
    let mut message = None;
    let mut normal_position = None;
    let mut routine = None;
    let mut schema = None;
    let mut severity_localized = None;
    let mut severity_nonlocalized = None;
    let mut table = None;
    let mut r#where = None;

    let mut idx: u32 = 0;
    loop {
      let Some(curr) = from.get(*Usize::from(idx)..) else {
        break;
      };
      let Some((ty, rest)) = curr.split_at_checked(1) else {
        break;
      };
      idx = idx.wrapping_add(1);
      if ty == "\0" {
        if rest.is_empty() {
          break;
        }
        return Err(crate::Error::UnexpectedString { length: rest.len() });
      }
      let Some(data) = str_split1(rest, b'\0').next() else {
        return Err(PostgresError::InsufficientDbErrorBytes.into());
      };
      let begin = idx;
      let (end, new_idx) = u32::try_from(data.len())
        .ok()
        .and_then(|data_len_u32| {
          let end = idx.checked_add(data_len_u32)?;
          let new_idx = end.checked_add(1)?;
          Some((end, new_idx))
        })
        .unwrap_or((u32::MAX, u32::MAX));
      let range = begin..end;
      idx = new_idx;
      match ty {
        "C" => code = Some(SqlState::try_from(data)?),
        "D" => detail = Some(range),
        "H" => hint = Some(range),
        "L" => line = Some(u32::from_radix_10(data.as_bytes())?),
        "M" => message = Some(range),
        "P" => normal_position = Some(u32::from_radix_10(data.as_bytes())?),
        "R" => routine = Some(range),
        "S" => severity_localized = Some(range),
        "V" => severity_nonlocalized = Some(Severity::try_from(data)?),
        "W" => r#where = Some(range),
        "c" => column = Some(range),
        "d" => datatype = Some(range),
        "F" => file = Some(range),
        "n" => constraint = Some(range),
        "p" => internal_position = Some(u32::from_radix_10(data.as_bytes())?),
        "q" => internal_query = Some(range),
        "s" => schema = Some(range),
        "t" => table = Some(range),
        _ => {
          return Err(crate::Error::UnexpectedUint {
            received: u32::from_radix_10(ty.as_bytes())?,
          });
        }
      }
    }

    Ok(Self {
      buffer: from.get(..*Usize::from(idx)).unwrap_or_default().into(),
      code: into_rslt(code)?,
      column,
      constraint,
      datatype,
      detail,
      file,
      hint,
      line,
      message: into_rslt(message)?,
      severity_localized: into_rslt(severity_localized)?,
      severity_nonlocalized,
      position: match normal_position {
        None => match internal_position {
          Some(position) => {
            Some(ErrorPosition::Internal { position, query: into_rslt(internal_query)? })
          }
          None => None,
        },
        Some(position) => Some(ErrorPosition::Original(position)),
      },
      routine,
      scheme: schema,
      table,
      r#where,
    })
  }
}
