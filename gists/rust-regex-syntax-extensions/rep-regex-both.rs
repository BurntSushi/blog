#![allow(dead_code)] fn main() {} struct Captures;
struct Regex {
    prog: MaybeNative,
}
enum MaybeNative {
    Dynamic(Vec<Inst>),
    Native(fn(&str) -> Captures),
}
enum Inst { Match, OneChar, /* ... */ }
