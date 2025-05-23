/// Database Error
#[derive(Debug)]
pub enum DatabaseError {
  /// A "null" field received from the database was decoded as a non-nullable type or value.
  MissingFieldDataInDecoding(&'static str),
  /// Expected one record but got none.
  MissingRecord,
  /// Received size differs from expected size.
  UnexpectedBufferSize {
    /// Expected
    expected: u32,
    /// Received
    received: u32,
  },
  /// Bytes don't represent expected type
  UnexpectedValueFromBytes {
    /// Expected
    expected: &'static str,
  },
  /// Received a statement ID that is not present in the local cache.
  UnknownStatementId,
}
