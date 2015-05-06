#![feature(macro_rules)]

macro_rules! try(
    ($e:expr) => (match $e { Ok(e) => e, Err(e) => return Err(e) })
)

fn test_try() -> Result<uint, &str> {
    let num = try!(Err("error!"));
    Ok(num) // never reached
}

fn main() {
    println!("{}", test_try());
    // Output:
    // Err(error!)
}
