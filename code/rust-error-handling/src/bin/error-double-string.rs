#![allow(dead_code, unused_imports, unused_variables)]

use std::env;

fn double_arg(mut argv: env::Args) -> Result<i32, String> {
    argv.nth(1)
        .ok_or("Please give at least one argument".into())
        .and_then(|arg| arg.parse::<i32>().map_err(|err| err.to_string()))
}

fn main() {
    match double_arg(env::args()) {
        Ok(n) => println!("{}", n),
        Err(err) => println!("Error: {}", err),
    }
}
