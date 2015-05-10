#![allow(dead_code, unused_imports, unused_variables)]
fn main() {
  fn ok_or<T, E>(option: Option<T>, err: E) -> Result<T, E> {
      match option {
          Some(val) => Ok(val),
          None => Err(err),
      }
  }
}
