#![allow(dead_code, unused_imports, unused_variables)]

use std::num::ParseIntError;

// We derive `Debug` because all types should probably derive `Debug`.
// Moreover, it is a prerequisite for implementing `Error`.
#[derive(Debug)]
enum CliError {
    NoArguments,
    InvalidNumber(ParseIntError),
}

use std::error;
use std::fmt;

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CliError::NoArguments => write!(f, "No arguments were given. \
                                                Please provide one argument."),
            // `std::num::ParseIntError` implements `Error`, which means
            // it already implements `Display`. We defer to its implementation.
            CliError::InvalidNumber(ref err) => err.fmt(f),
        }
    }
}

impl error::Error for CliError {
    fn description(&self) -> &str {
        match *self {
            CliError::NoArguments => "no arguments given",
            CliError::InvalidNumber(_) => "invalid number",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            CliError::NoArguments => None,
            // N.B. This implicitly casts `err` from `&ParseIntError`
            // to a trait object `&Error`. This works because `ParseIntError`
            // implements `Error`.
            CliError::InvalidNumber(ref err) => Some(err),
        }
    }
}

use std::env;

fn double_arg(mut argv: env::Args) -> Result<i32, CliError> {
    argv.nth(1)
        .ok_or(CliError::NoArguments)
        .and_then(|arg| arg.parse().map_err(CliError::InvalidNumber))
}

fn main() {
    match double_arg(env::args()) {
        Ok(n) => println!("{}", n),
        Err(err) => println!("Error: {}", err),
    }
}
