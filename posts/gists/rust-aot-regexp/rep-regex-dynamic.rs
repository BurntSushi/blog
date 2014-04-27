#![allow(dead_code)] fn main() {}
struct Regex {
    insts: Vec<Inst>,
    // (N.B. The real representation has a few more things, like the names of
    // capture groups or pre-computed data for optimizations, but they are
    // elided here to keep things simple.)
}
enum Inst { Match, OneChar, /* ... */ }
