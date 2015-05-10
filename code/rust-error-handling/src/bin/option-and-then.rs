#![allow(dead_code, unused_imports, unused_variables)]
fn main() {
  fn and_then<F, T, A>(option: Option<T>, f: F) -> Option<A>
          where F: FnOnce(T) -> Option<A> {
      match option {
          None => None,
          Some(value) => f(value),
      }
  }
}
