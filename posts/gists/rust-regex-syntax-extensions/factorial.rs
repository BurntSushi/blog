// Currently, the `phase` feature must be enabled in order to import a crate
// that defines a syntax extension.
#![feature(phase)]

// The `phase` attribute is used here to indicate that the `factorial` crate
// provides a syntax extension.
// It's also possible for `factorial` to provide things other than a syntax
// extension, in which case, `#[phase(plugin, link)]` is required.
#[phase(plugin)] extern crate factorial;

fn main() {
    println!("{}", factorial!());
    // Output:
    // 120
}
