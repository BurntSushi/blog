#![allow(dead_code, unused_imports, unused_variables)]
fn main() {
  use std::io;
  use std::num;
  
  // We derive `Debug` because all types should probably derive `Debug`.
  // This gives us a reasonable human readable description of `CliError` values.
  #[derive(Debug)]
  enum CliError {
      Io(io::Error),
      Parse(num::ParseIntError),
  }
  
  use std::error;
  use std::fmt;
  
  impl fmt::Display for CliError {
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
          match *self {
              // Both underlying errors already impl `Display`, so we defer to
              // their implementations.
              CliError::Io(ref err) => write!(f, "IO error: {}", err),
              CliError::Parse(ref err) => write!(f, "Parse error: {}", err),
          }
      }
  }
  
  impl error::Error for CliError {
      fn description(&self) -> &str {
          // This method returns a borrowed string with a lifetime attached to
          // the error value. This means we cannot heap allocate a new string
          // in this method using safe code, so we are forced to keep the
          // description short and sweet.
          match *self {
              CliError::Io(_) => "IO error",
              CliError::Parse(_) => "error converting string to number",
          }
      }
  
      fn cause(&self) -> Option<&error::Error> {
          match *self {
              // N.B. Both of these implicitly cast `err` from their concrete
              // types (either `&io::Error` or `&num::ParseIntError`)
              // to a trait object `&Error`. This works because both error types
              // implement `Error`.
              CliError::Io(ref err) => Some(err),
              CliError::Parse(ref err) => Some(err),
          }
      }
  }
}
