#![allow(dead_code)] fn main() {} struct Captures;
struct Regex {
    prog: MaybeNative,
}
enum MaybeNative {
    Dynamic(Vec<Inst>),
    Native(fn(&str) -> Captures),
}
enum Inst { Match, OneChar, /* ... */ }
fn run_vm_dynamic(_: &[Inst], _: &str) -> Captures { Captures }

fn run_vm(re: &Regex, search: &str) -> Captures {
    match re.prog {
        Dynamic(ref insts) => run_vm_dynamic(insts.as_slice(), search),
        Native(run_vm_native) => run_vm_native(search),
    }
}
