#![allow(dead_code, unused_imports, unused_variables)]
fn main() {
  enum Option<T> {
      None,
      Some(T),
  }
  
  impl<T> Option<T> {
      fn unwrap(self) -> T {
          match self {
              Option::Some(val) => val,
              Option::None =>
                panic!("called `Option::unwrap()` on a `None` value"),
          }
      }
  }
}
