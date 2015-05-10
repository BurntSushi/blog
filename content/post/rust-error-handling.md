+++
date = "2015-05-06T18:22:00-04:00"
draft = true
title = "Error Handling in Rust"
author = "Andrew Gallant"
url = "rust-error-handling"

[blackfriday]
plainIdAnchors = true
+++

Like most programming languages, Rust encourages the programmer to handle
errors in a particular way. Generally speaking, error handling is divided into
two broad categories: exceptions and return values. Rust opts for return
values.

In this article, I intend to provide a comprehensive treatment of how to deal
with errors in Rust. I will cover error handling in libraries, where it is the
library's responsibility to pass errors on to the caller. I will also cover
error handling in command line programs, where it is the program's
responsibility to present errors to the user in a clean and predictable manner.

When done naively, error handling in Rust can be verbose and annoying. This
article will explore those stumbling blocks and demonstrate how to use the
standard library to make error handling concise and ergonomic.

**Target audience**: Those new to Rust that don't know its error handling
idioms yet. Some familiarity with Rust is helpful (e.g., this article will make
use of traits and closures).

<!--more-->


## Brief notes

All code samples in this post compile with Rust `1.0.0-beta.4`. They should
continue to work as Rust 1.0 stable is released.

All code can be found and compiled in
[my blog's repository](https://github.com/BurntSushi/blog/tree/master/code/rust-error-handling).

The [Rust Book](http://doc.rust-lang.org/1.0.0-beta.4/book/)
has a [section on error
handling](http://doc.rust-lang.org/1.0.0-beta.4/book/error-handling.html).
It gives a very brief overview, but doesn't (yet) go into enough detail,
particularly when working with some of the more recent additions to the
standard library.


## Run the code!

If you'd like to run any of the code samples below, then the following should
work:

{{< high sh >}}
$ git clone git://github.com/BurntSushi/blog
$ cd blog/code/rust-error-handling
$ cargo run --bin NAME-OF-CODE-SAMPLE [ args ... ]
{{< /high >}}

Each code sample is labeled with its name.


## Table of Contents

This article has three major sections (plus some caveats and closing remarks).
The first section, "The Basics," can be skipped if you are already familiar
with algebraic data types and combinators (even if you don't know Rust). If
you don't know Rust, it will still be a good idea to at least skim the code
examples so you can get familiar with the syntax.

If you're already familiar with Rust and just want to learn more about using
the `From` and `Error` traits with the `try!` macro, then please skip the "The
Basics" entirely.

* [The Basics](#the-basics)
    * [Unwrapping explained](#unwrapping-explained)
    * [The `Option` type](#the-option-type)
        * [Composing `Option<T>` values](#composing-option<t>-values)
    * [The `Result` type](#the-result-type)
        * [Parsing integers](#parsing-integers)
        * [The `Result` type alias idiom](#the-result-type-alias-idiom)
    * [A brief interlude: unwrapping isn't evil](#a-brief-interlude-unwrapping-isn-t-evil)
* [Working with multiple error types](#working-with-multiple-error-types)
    * [Composing `Option` and `Result`](#composing-option-and-result)
    * [The limits of combinators](#the-limits-of-combinators)
    * [Early returns](#early-returns)
    * [The `try!` macro](#the-try-macro)
* [The `Error` trait](#the-error-trait)
* [The `From` trait and the `try!` macro](#the-from-trait-and-the-try-macro)


## The Basics

I like to think of error handling as using *case analysis* to determine whether
a computation was successful or not. As we will see, the key to ergnomic error
handling is reducing the amount of explicit case analysis the programmer has to
do while keeping code composable.

Keeping code composable is important, because without that requirement, we
could [`panic`](http://doc.rust-lang.org/std/macro.panic!.html) whenever we
come across something unexpected. (`panic` causes the current task to unwind,
and in most cases, the entire program aborts.) Here's an example:

{{< code-rust "panic-simple" >}}
// Guess a number between 1 and 10.
// If it matches the number I had in mind, return true. Else, return false.
fn guess(n: i32) -> bool {
    if n < 1 || n > 10 {
        panic!("Invalid number: {}", n);
    }
    n == 5
}

fn main() {
    guess(11);
}
{{< /code-rust >}}

(If you like, it's easy to [run this code](#run-the-code).)

If you try running this code, the program will crash with a message like this:

{{< high sh  >}}
thread '<main>' panicked at 'Invalid number: 11', src/bin/panic-simple.rs:5
{{< /high >}}

Here's another example that is slightly less contrived. A program that accepts
an integer as an argument, doubles it and prints it.

{{< code-rust "unwrap-double" >}}
use std::env;

fn main() {
    let mut argv = env::args();
    let arg: String = argv.nth(1).unwrap(); // error 1
    let n: i32 = arg.parse().unwrap(); // error 2
    println!("{}", 2 * n);
}

// $ cargo run --bin unwrap-double 5
// 10
{{< /code-rust >}}

If you give this program zero arguments (error 1) or if the first argument
isn't an integer (error 2), the program will panic just like in the first
example.

I like to think of this style of error handling as similar to a bull running
through a china shop. The bull will get to where it wants to go, but it will
trample everything in the process.


### Unwrapping explained

In the previous example ([`unwrap-double`](#code-unwrap-double)), I claimed
that the program would simply panic if it reached one of the two error
conditions, yet, the program does not include an explicit call to `panic` like
the first example ([`panic-simple`](#code-panic-simple)). This is because the
panic is embedded in the calls to `unwrap`.

To "unwrap" something in Rust is to say, "Give me the result of the
computation, and if there was an error, just panic and stop the program."
It would be better if I just showed the code for unwrapping because it is so
simple, but to do that, we will first need to explore the `Option` and `Result`
types. Both of these types have a method called `unwrap` defined on them.


### The `Option` type

The `Option` type is
[defined in the standard library](http://doc.rust-lang.org/std/option/enum.Option.html):

{{< code-rust "option-def" >}}
enum Option<T> {
    None,
    Some(T),
}
{{< /code-rust >}}

The `Option` type is a way to use Rust's type system to express the
*possibility of absence*. Encoding the possibility of absence into the type
system is an important concept because it will cause the compiler to force the
programmer to handle that absence. Let's take a look at an example that tries
to find a character in a string:

{{< code-rust "option-ex-string-find" "1" >}}
// Searches `haystack` for the Unicode character `needle`. If one is found, the
// byte offset of the character is returned. Otherwise, `None` is returned.
fn find(haystack: &str, needle: char) -> Option<usize> {
    for (offset, c) in haystack.char_indices() {
        if c == needle {
            return Some(offset);
        }
    }
    None
}
{{< /code-rust >}}

(Pro-tip: don't use this code. Instead, use the
[`find`](http://doc.rust-lang.org/std/primitive.str.html#method.find)
method from the standard library.)

Notice that when this function finds a matching character, it doen't just
return the `offset`. Instead, it returns `Some(offset)`. `Some` is a variant or
a *value constructor* for the `Option` type. You can think of it as a function
with the type `fn<T>(value: T) -> Option<T>`. Correspondingly, `None` is also a
value constructor, except it has no arguments. You can think of `None` as a
function with the type `fn<T>() -> Option<T>`.

This might seem like much ado about nothing, but this is only half of the
story. The other half is *using* the `find` function we've written:

{{< code-rust "option-ex-string-find" "2" >}}
fn main_find() {
    let file_name = "foobar.rs";
    match find(file_name, '.') {
        None => println!("No file extension found."),
        Some(i) => println!("File extension: {}", &file_name[i+1..]),
    }
}
{{< /code-rust >}}

This code uses [pattern
matching](http://doc.rust-lang.org/1.0.0-beta.4/book/patterns.html) to do *case
analysis* on the `Option<usize>` returned by the `find` function. In fact, case
analysis is the only way to get at the value stored inside an `Option<T>`. This
means that you, as the programmer, must handle the case when an `Option<T>` is
`None` instead of `Some(t)`.

But wait, what about `unwrap` used in [`unwrap-double`](#code-unwrap-double)?
There was no case analysis there! Instead, the case analysis was put inside the
`unwrap` method for you. You could define it yourself if you want:

{{< code-rust "option-def-unwrap" >}}
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
{{< /code-rust >}}

The `unwrap` method *abstracts the case analysis*. This is precisely the thing
that makes `unwrap` ergonomic to use. Unfortunately, that `panic!` means that
`unwrap` is not composable: it is the bull in the china shop.


#### Composing `Option<T>` values

In
[`option-ex-string-find`](#code-option-ex-string-find-2)
we saw how to use `find` to discover the extension in a file name. Of course,
not all file names have a `.` in them, so it's possible that the file name has
no extension. This *possibility of absence* is encoded into the types using
`Option<T>`. In other words, the compiler will force us to address the
possibility that an extension does not exist. In our case, we just print out a
message saying as such.

Getting the extension of a file name is a pretty common operation, so it makes
sense to put it into a function:

{{< code-rust "option-ex-string-find" "3" >}}
// Returns the extension of the given file name, where the extension is defined
// as all characters proceding the first `.`.
// If `file_name` has no `.`, then `None` is returned.
fn extension_explicit(file_name: &str) -> Option<&str> {
    match find(file_name, '.') {
        None => None,
        Some(i) => Some(&file_name[i+1..]),
    }
}
{{< /code-rust >}}

(Pro-tip: don't use this code. Use the
[`extension`](http://doc.rust-lang.org/std/path/struct.Path.html#method.extension)
method in the standard library instead.)

The code stays simple, but the important thing to notice is that the type of
`find` forces us to consider the possibility of absence. This is a good thing
because it means the compiler won't let us accidentally forget about the case
where a file name doesn't have an extension. On the other hand, doing explicit
case analysis like we've done in `extension_explicit` every time can get a bit
tiresome.

In fact, the case analysis in `extension_explicit` follows a very common
pattern: *map* a function on to the value inside of an `Option<T>`, unless the
option is `None`, in which case, just return `None`.

Rust has parametric polymorphism, so it is very easy to define a combinator
that abstracts this pattern:

{{< code-rust "option-map" >}}
fn map<F, T, A>(option: Option<T>, f: F) -> Option<A> where F: FnOnce(T) -> A {
    match option {
        None => None,
        Some(value) => Some(f(value)),
    }
}
{{< /code-rust >}}

Indeed, `map` is
[defined as a
method](http://doc.rust-lang.org/std/option/enum.Option.html#method.map)
on `Option<T>` in the standard library.

Armed with our new combinator, we can rewrite our `extension_explicit` method
to get rid of the case analysis:

{{< code-rust "option-ex-string-find" "4" >}}
// Returns the extension of the given file name, where the extension is defined
// as all characters proceding the first `.`.
// If `file_name` has no `.`, then `None` is returned.
fn extension(file_name: &str) -> Option<&str> {
    find(file_name, '.').map(|i| &file_name[i+1..])
}
{{< /code-rust >}}

One other pattern that I find is very common is assigning a default value to
the case when an `Option` value is `None`. For example, maybe your program
assumes that the extension of a file is `rs` even if none is present. As you
might imagine, the case analysis for this is not specific to file
extensions---it can work with any `Option<T>`:

{{< code-rust "option-unwrap-or" >}}
fn unwrap_or<T>(option: Option<T>, default: T) -> T {
    match option {
        None => default,
        Some(value) => value,
    }
}
{{< /code-rust >}}

The trick here is that the default value must have the same type as the value
that might be inside the `Option<T>`. Using it is dead simple in our case:

{{< code-rust "option-ex-string-find" "5" >}}
fn main() {
    assert_eq!(extension("foobar.csv").unwrap_or("rs"), "csv");
    assert_eq!(extension("foobar").unwrap_or("rs"), "rs");
}
{{< /code-rust >}}

(Note that `unwrap_or` is
[defined as a
method](http://doc.rust-lang.org/std/option/enum.Option.html#method.unwrap_or)
on `Option<T>` in the standard library, so we use that here instead of the
free-standing function we defined above. Don't forget to check out the more
general
[`unwrap_or_else`](http://doc.rust-lang.org/std/option/enum.Option.html#method.unwrap_or_else)
method.)

There is one more combinator that I think is worth paying special attention to:
`and_then`. It makes it easy to compose distinct computations that admit the
*possibility of absence*. For example, much of the code in this section is
about finding an extension given a file name. In order to do this, you first
need the file name which is typically extracted from a file *path*. While most
file paths have a file name, not *all* of them do. For example, `.`, `..` or
`/`.

So, we are tasked with the challenge of finding an extension given a file
*path*. Let's start with explicit case analysis:

{{< code-rust "option-ex-string-find" "6" >}}
fn file_path_ext_explicit(file_path: &str) -> Option<&str> {
    match file_name(file_path) {
        None => None,
        Some(name) => match extension(name) {
            None => None,
            Some(ext) => Some(ext),
        }
    }
}

fn file_name(file_path: &str) -> Option<&str> {
  // implementation elided
  unimplemented!()
}
{{< /code-rust >}}

You might think that we could just use the `map` combinator to reduce the case
analysis, but its type doesn't quite fit. Namely, `map` takes a function that
does something only with the inner value. The result of that function is then
*always* [rewrapped with `Some`](#code-option-map). Instead, we need something
like `map`, but which allows the caller to return another `Option`. Its generic
implementation is even simpler than `map`:

{{< code-rust "option-and-then" >}}
fn and_then<F, T, A>(option: Option<T>, f: F) -> Option<A>
        where F: FnOnce(T) -> Option<A> {
    match option {
        None => None,
        Some(value) => f(value),
    }
}
{{< /code-rust >}}

Now we can rewrite our `file_path_ext` function without explicit case analysis:

{{< code-rust "option-ex-string-find" "6" >}}
fn file_path_ext(file_path: &str) -> Option<&str> {
    file_name(file_path).and_then(extension)
}
{{< /code-rust >}}

The `Option` type has many other combinators
[defined in the standard
library](http://doc.rust-lang.org/std/option/enum.Option.html). It is a good
idea to skim this list and familiarize yourself with what's available---they
can often reduce case analysis for you. Familiarizing yourself with these
combinators will pay dividends because many of them are also defined (with
similar semantics) for `Result`, which we will talk about next.

Combinators make using types like `Option` ergonomic because they reduce
explicit case analysis. They are also composable because they permit the caller
to handle the possibility of absence in their own way. Methods like `unwrap`
remove choices because they will panic if `Option<T>` is `None`.


### The `Result` type

The `Result` type is also
[defined in the standard library](http://doc.rust-lang.org/std/result/):

{{< code-rust "result-def" "1" >}}
enum Result<T, E> {
    Ok(T),
    Err(E),
}
{{< /code-rust >}}

The `Result` type is richer version of `Option`. Instead of expressing the
possibility of *absence* like `Option` does, `Result` expresses the possibility
of *error*. Usually, the *error* is used to explain why the result of some
computation failed. This is a strictly more general form of `Option`. Consider
the following type alias, which is semantically equivalent to the real
`Option<T>` in every way:

{{< code-rust "option-as-result" >}}
type Option<T> = Result<T, ()>;
{{< /code-rust >}}

This fixes the second type parameter of `Result` to always be `()` (pronounced
"unit"). Exactly one value inhabits the `()` type: `()`. (Yup, the type and
value level terms have the same notation!)

The `Result` type is a way of representing one of two possible outcomes in a
computation. By convention, one outcome is meant to be expected or "`Ok`" while
the other outcome is meant to be unexpected or "`Err`".

Just like `Option`, the `Result` type also has an
[`unwrap` method
defined](http://doc.rust-lang.org/std/result/enum.Result.html#method.unwrap)
in the standard library. Let's define it:

{{< code-rust "result-def" "2" >}}
impl<T, E: ::std::fmt::Debug> Result<T, E> {
    fn unwrap(self) -> T {
        match self {
            Result::Ok(val) => val,
            Result::Err(err) =>
              panic!("called `Result::unwrap()` on an `Err` value: {:?}", err),
        }
    }
}
{{< /code-rust >}}

This is effectively the same as our
[definition for `Option::unwrap`](#code-option-def-unwrap),
except it includes the error value in the `panic!` message. This makes
debugging easier, but it also requires us to add a
[`Debug`](http://doc.rust-lang.org/std/fmt/trait.Debug.html)
constraint on the `E` type parameter (which represents our error type). Since
the vast majority of types should satisfy the `Debug` constraint, this tends to
work out in practice. (`Debug` on a type simply means that there's a reasonable
way to print a human readable description of values with that type.)

OK, let's move on to an example.


#### Parsing integers

The Rust standard library makes converting strings to integers dead simple.
It's so easy in fact, that it is very tempting to write something like the
following:

{{< code-rust "result-num-unwrap" >}}
fn double_number(number_str: &str) -> i32 {
    2 * number_str.parse::<i32>().unwrap()
}

fn main() {
    let n: i32 = double_number("10");
    assert_eq!(n, 20);
}
{{< /code-rust >}}

At this point, you should be skeptical of calling `unwrap`. For example, if
the string doesn't parse as a number, you'll get a panic:

{{< high text >}}
thread '<main>' panicked at 'called `Result::unwrap()` on an `Err` value: ParseIntError { kind: InvalidDigit }', /home/rustbuild/src/rust-buildbot/slave/beta-dist-rustc-linux/build/src/libcore/result.rs:729
{{< /high >}}

This is rather unsightly, and if this happened inside a library you're using,
you might be understandably annoyed. Instead, we should try to handle the error
in our function and let the caller decide what to do. This means changing the
return type of `double_number`. But to what? Well, that requires looking at the
signature of the
[`parse` method](http://doc.rust-lang.org/std/primitive.str.html#method.parse)
in the standard library:

{{< high "rust" >}}
impl str {
    fn parse<F: FromStr>(&self) -> Result<F, F::Err>;
}
{{< /high >}}

Hmm. So we at least know that we need to use a `Result`. Certainly, it's
possible that this could have returned an `Option`. After all, a string either
parses as a number or it doesn't, right? That's certainly a reasonable way to
go, but the implementation internally distinguishes *why* the string didn't
parse as an integer. (Whether it's an empty string, an invalid digit, too big
or too small.) Therefore, using a `Result` makes sense because we want to
provide more information than simply "absence." We want to say *why* the
parsing failed. You should try to emulate this line of reasoning when faced
with a choice between `Option` and `Result`. If you can provide detailed error
information, then you probably should. (We'll see more on this later.)

OK, but how do we write our return type? The `parse` method as defined above is
generic over all the different number types defined in the standard library. We
could (and probably should) also make our function generic, but let's favor
explicitness for the moment. We only care about `i32`, so we need to
[find its implementation of
`FromStr`](http://doc.rust-lang.org/std/primitive.i32.html)
(do a `CTRL-F` in your browser for "FromStr")
and look at its [associated
type](http://doc.rust-lang.org/1.0.0-beta.4/book/associated-types.html) `Err`.
We did this so we can find the concrete error type. In this case, it's
[`std::num::ParseIntError`](http://doc.rust-lang.org/std/num/struct.ParseIntError.html).
Finally, we can rewrite our function:

{{< code-rust "result-num-no-unwrap" >}}
use std::num::ParseIntError;

fn double_number(number_str: &str) -> Result<i32, ParseIntError> {
    match number_str.parse::<i32>() {
        Ok(n) => Ok(2 * n),
        Err(err) => Err(err),
    }
}

fn main() {
    match double_number("10") {
        Ok(n) => assert_eq!(n, 20),
        Err(err) => println!("Error: {:?}", err),
    }
}
{{< /code-rust >}}

This is a little better, but now we've written a lot more code! The case
analysis has once again bitten us.

Combinators to the rescue! Just like `Option`, `Result` has lots of combinators
defined as methods. There is a large intersection of common combinators between
`Result` and `Option`. In particular, `map` is part of that intersection:

{{< code-rust "result-num-no-unwrap-map" >}}
use std::num::ParseIntError;

fn double_number(number_str: &str) -> Result<i32, ParseIntError> {
    number_str.parse::<i32>().map(|n| 2 * n)
}

fn main() {
    match double_number("10") {
        Ok(n) => assert_eq!(n, 20),
        Err(err) => println!("Error: {:?}", err),
    }
}
{{< /code-rust >}}

The usual suspects are all there for `Result`, including
[`unwrap_or`](http://doc.rust-lang.org/std/result/enum.Result.html#method.unwrap_or)
and
[`and_then`](http://doc.rust-lang.org/std/result/enum.Result.html#method.and_then).
Additionally, since `Result` has a second type parameter, there are combinators
that affect only the error type, such as
[`map_err`](http://doc.rust-lang.org/std/result/enum.Result.html#method.map_err)
(instead of `map`) and
[`or_else`](http://doc.rust-lang.org/std/result/enum.Result.html#method.or_else)
(instead of `and_then`).


#### The `Result` type alias idiom

In the standard library, you may frequently see types like `Result<i32>`. But
wait,
[we defined `Result`](#code-result-def-1)
to have two type parameters. How can we get away with only specifying one? The
key is to define a `Result` type alias that *fixes* one of the type parameters
to a particular type. Usually this is the error. For example, our previous
example parsing integers could be rewritten like this:

{{< code-rust "result-num-no-unwrap-map-alias" >}}
use std::num::ParseIntError;
use std::result;

type Result<T> = result::Result<T, ParseIntError>;

fn double_number(number_str: &str) -> Result<i32> {
    unimplemented!();
}
{{< /code-rust >}}

Why would we do this? Well, if we have a lot of functions that could return
`ParseIntError`, then it's much more convenient to define an alias that always
uses `ParseIntError` so that we don't have to write it out all the time.

The most prominent place this idiom is used in the standard library is with
[`io::Result`](http://doc.rust-lang.org/std/io/type.Result.html). Typically,
one writes `io::Result<T>`, which makes it clear that you're using the `io`
module's type alias instead of the plain definition from `std::result`.
(This idiom is also used for
[`fmt::Result`](http://doc.rust-lang.org/std/fmt/type.Result.html).)


### A brief interlude: unwrapping isn't evil

If you've been following along, you might have noticed that I've taken a pretty
hard line against calling methods like `unwrap` that could `panic` and abort
your program. *Generally speaking*, this is good advice.

However, `unwrap` can still be used judiciously. What exactly justifies use of
`unwrap` is somewhat of a grey area and reasonable people can disagree. I'll
summarize some of my *opinions* on the matter.

* **In examples and quick 'n' dirty code.** Sometimes you're writing examples
  or a quick program, and error handling simply isn't important. Beating the
  convenience of `unwrap` can be hard in such scenarios, so it is very
  appealing.
* **When panicing indicates a bug in the program.** When the invariants of your
  code should prevent a certain case from happening (like, say, popping from an
  empty stack), then panicing can be permissible. This is because it exposes a
  bug in your program. This can be explicit as a result from an `assert!`
  failing, or it could be because your index into an array was out of bounds.

This is probably not an exhaustive list. Moreover, when using an `Option`, it
is often better to use its
[`expect`](http://doc.rust-lang.org/std/option/enum.Option.html#method.expect)
method. `expect` does exactly the same thing as `unwrap`, except it prints a
message you give to `expect`. This makes the resulting panic a bit nicer to
deal with, since it will show your message instead of "called unwrap on a
`None` value."

My advice boils down to this: use good judgment. There's a reason why the words
"never do X" or "Y is considered harmful" don't appear in my writing. There are
trade offs to all things, and it is up to you as the programmer to determine
what is acceptable for your use cases. My goal is only to help you evaluate
trade offs as accurately as possible.

Now that we've covered the basics of error handling in Rust, and I've said my
piece about unwrapping, let's start exploring more of the standard library.


## Working with multiple error types

Thus far, we've looked at error handling where everything was either an
`Option<T>` or a `Result<T, SomeError>`. But what happens when you have both an
`Option` and a `Result`? Or what if you have a `Result<T, Error1>` and a
`Result<T, Error2>`? Handling *composition of distinct error types* is the next
challenge in front of us, and it will be the major theme throughout the rest of
this article.


### Composing `Option` and `Result`

So far, I've talked about combinators defined for `Option` and combinators
defined for `Result`. We can use these combinators to compose results of
different computations without doing explicit case analysis.

Of course, in real code, things aren't always as clean. Sometimes you have a
mix of `Option` and `Result` types. Must we resort to explicit case analysis,
or can we continue using combinators?

For now, let's revisit one of the first examples in this article:

{{< high "rust" >}}
use std::env;

fn main() {
    let mut argv = env::args();
    let arg: String = argv.nth(1).unwrap(); // error 1
    let n: i32 = arg.parse().unwrap(); // error 2
    println!("{}", 2 * n);
}

// $ cargo run --bin unwrap-double 5
// 10
{{< /high >}}

Given our new found knowledge of `Option`, `Result` and their various
combinators, we should try to rewrite this so that errors are handled properly
and the program doesn't panic if there's an error.

The tricky aspect here is that `argv.nth(1)` produces an `Option` while
`arg.parse()` produces a `Result`. These aren't directly composable. When faced
with both an `Option` and a `Result`, the solution is *usually* to convert the
`Option` to a `Result`. In our case, the absence of a command line parameter
(from `env::args()`) means the user didn't invoke the program correctly. We
could just use a `String` to describe the error. Let's try:

{{< code-rust "error-double-string" >}}
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
{{< /code-rust >}}

There are a couple new things in this example. The first is the use of the
[`Option::ok_or`](http://doc.rust-lang.org/std/option/enum.Option.html#method.ok_or)
combinator. This is one way to convert an `Option` into a `Result`. The
conversion requires you to specify what error to use if `Option` is `None`.
Like the other combinators we've seen, its definition is very simple:

{{< code-rust "option-ok-or-def" >}}
fn ok_or<T, E>(option: Option<T>, err: E) -> Result<T, E> {
    match option {
        Some(val) => Ok(val),
        None => Err(err),
    }
}
{{< /code-rust >}}

The other new combinator used here is
[`Result::map_err`](http://doc.rust-lang.org/std/result/enum.Result.html#method.map_err).
This is just like `Result::map`, except it maps a function on to the *error*
portion of a `Result` value. If the `Result` is an `Ok(...)` value, then it is
returned unmodified.

We use `map_err` here because it is necessary for the error types to remain
the same (because of our use of `and_then`). Since we chose to convert the
`Option<String>` (from `argv.nth(1)`) to a `Result<String, String>`, we must
also convert the `ParseIntError` from `arg.parse()` to a `String`.


### The limits of combinators

Doing IO and parsing input is a very common task, and it's one that I
personally have done a lot of in Rust. Therefore, we will use (and continue to
use) IO and various parsing routines to exemplify error handling.

Let's start simple. We are tasked with opening a file, reading all of its
contents and converting its contents to a number. Then we multiply it by `2`
and print the output.

Although I've tried to convince you not to use `unwrap`, it can be useful
to first write your code using `unwrap`. It allows you to focus on your problem
instead of the error handling, and it exposes the points where proper error
handling need to occur. Let's start there so we can get a handle on the code,
and then refactor it to use better error handling.

{{< code-rust "io-basic-unwrap" >}}
use std::fs::File;
use std::io::Read;
use std::path::Path;

fn file_double<P: AsRef<Path>>(file_path: P) -> i32 {
    let mut file = File::open(file_path).unwrap(); // error 1
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap(); // error 2
    let n: i32 = contents.trim().parse().unwrap(); // error 3
    2 * n
}

fn main() {
    let doubled = file_double("foobar");
    println!("{}", doubled);
}
{{< /code-rust >}}

(N.B. The `AsRef<Path>` is used because those are the
[same bounds used on
`std::fs::File::open`](http://doc.rust-lang.org/std/fs/struct.File.html#method.open).
This makes it ergnomic to use any kind of string as a file path.)

There are three different errors that can occur here:

1. A problem opening the file.
2. A problem reading data from the file.
3. A problem parsing the data as a number.

The first two problems are described via the
[`std::io::Error`](http://doc.rust-lang.org/std/io/struct.Error.html) type.
We know this because of the return types of
[`std::fs::File::open`](http://doc.rust-lang.org/std/fs/struct.File.html#method.open)
and
[`std::io::Read::read_to_string`](http://doc.rust-lang.org/std/io/trait.Read.html#method.read_to_string).
(Note that they both use the
[`Result` type alias idiom](#the-result-type-alias-idiom)
described previously. If you click on the `Result` type, you'll
[see the type alias](http://doc.rust-lang.org/std/io/type.Result.html), and
consequently, the underlying `io::Error` type.)
The third problem is described by the
[`std::num::ParseIntError`](http://doc.rust-lang.org/std/num/struct.ParseIntError.html)
type. The `io::Error` type in particular is *pervasive* throughout the standard
library. You will see it again and again.

Let's start the process of refactoring the `file_double` function. To make this
function composable with other components of the program, it should *not* panic
if any of the above error conditions are met. Effectively, this means that the
function should *return an error* if any of its operations fail. Our problem is
that the return type of `file_double` is `i32`, which does not give us any
useful way of reporting an error. Thus, we must start by changing the return
type from `i32` to something else.

The first thing we need to decide: should we use `Option` or `Result`? We
certainly could use `Option` very easily. If any of the three errors occur, we
could simply return `None`. This will work *and it is better than panicing*,
but we can do a lot better. Instead, we should pass some detail about the error
that occurred. This means we should use `Result<i32, E> But what should `E` be?
Since two *different* types of errors can occur, we need to convert them to a
common type. One such type is `String`. Let's see how that impacts our code:

{{< code-rust "io-basic-error-string" >}}
use std::fs::File;
use std::io::Read;
use std::path::Path;

fn file_double<P: AsRef<Path>>(file_path: P) -> Result<i32, String> {
    File::open(file_path)
         .map_err(|err| err.to_string())
         .and_then(|mut file| {
              let mut contents = String::new();
              file.read_to_string(&mut contents)
                  .map_err(|err| err.to_string())
                  .map(|_| contents)
         })
         .and_then(|contents| {
              contents.trim().parse::<i32>()
                      .map_err(|err| err.to_string())
         })
         .map(|n| 2 * n)
}

fn main() {
    match file_double("foobar") {
        Ok(n) => println!("{}", n),
        Err(err) => println!("Error: {}", err),
    }
}
{{< /code-rust >}}

This code looks a bit hairy. It can take quite a bit of practice before code
like this becomes easy to write. The way I write it is by *following the
types*. As soon as I changed the return type of `file_double` to
`Result<i32, String>`, I had to start looking for the right combinators. In
this case, we only used three different combinators: `and_then`, `map` and
`map_err`.

`and_then` is used to chain multiple computations where each computation could
return an error. After opening the file, there are two more computations that
could fail: reading from the file and parsing the contents as a number.
Correspondingly, there are two calls to `and_then`.

`map` is used to apply a function to the `Ok(...)` value of a `Result`. For
example, the very last call to `map` multiplies the `Ok(...)` value (which is
an `i32`) by `2`. If an error had occurred before that point, this operation
would have been skipped because of how `map` is defined.

`map_err` is the trick the makes all of this work. `map_err` is just like
`map`, except it applies a function to the `Err(...)` value of a `Result`. In
this case, we want to convert all of our errors to one type: `String`. Since
both `io::Error` and `num::ParseIntError` implement `ToString`, we can call the
`to_string()` method to convert them.

With all of that said, the code is still hairy. Mastering use of combinators is
important, but they have their limits. Let's try a different approach: early
returns.


### Early returns

I'd like to take the code from the previous section and rewrite it using *early
returns*. Early returns let you exit the function early. We can't return early
in `file_double` from inside another closure, so we'll need to revert back to
explicit case analysis.

{{< code-rust "io-basic-error-string-early-return" >}}
use std::fs::File;
use std::io::Read;
use std::path::Path;

fn file_double<P: AsRef<Path>>(file_path: P) -> Result<i32, String> {
    let mut file = match File::open(file_path) {
        Ok(file) => file,
        Err(err) => return Err(err.to_string()),
    };
    let mut contents = String::new();
    if let Err(err) = file.read_to_string(&mut contents) {
        return Err(err.to_string());
    }
    let n: i32 = match contents.trim().parse() {
        Ok(n) => n,
        Err(err) => return Err(err.to_string()),
    };
    Ok(2 * n)
}

fn main() {
    match file_double("foobar") {
        Ok(n) => println!("{}", n),
        Err(err) => println!("Error: {}", err),
    }
}
{{< /code-rust >}}

Reasonable people can disagree over whether this code is better that the code
that uses combinators, but if you aren't familiar with the combinator approach,
this code looks simpler to read to me. It uses explicit case analysis with
`match` and `if let`. If an error occurs, it simply stops executing the
function and returns the error (by converting it to a string).

Isn't this a step backwards though? Previously, I said that the key to
ergonomic error handling is reducing explicit case analysis, yet we've reverted
back to explicit case analysis here. It turns out, there are *multiple* ways to
reduce explicit case analysis. Combinators aren't the only way.


### The `try!` macro

A cornerstone of error handling in Rust is the `try!` macro. The `try!` macro
abstracts case analysis just like combinators, but unlike combinators, it also
abstracts *control flow*. Namely, it can abstract the *early return* pattern
seen above.

Here is a simplified definition of a `try!` macro:

{{< code-rust "try-def-simple" >}}
macro_rules! try {
    ($e:expr) => (match $e {
        Ok(val) => val,
        Err(err) => return Err(err),
    });
}
{{< /code-rust >}}

(The
[real definition](http://doc.rust-lang.org/std/macro.try!.html)
is a bit more sophisticated. We will address that later.)


## The `Error` trait

The `Error` trait is
[defined in the standard
library](http://doc.rust-lang.org/std/error/trait.Error.html):

{{< code-rust "error-def" >}}
use std::fmt::{Debug, Display};

trait Error: Debug + Display {
  /// A short description of the error.
  fn description(&self) -> &str;

  /// The lower level cause of this error, if any.
  fn cause(&self) -> Option<&Error> { None }
}
{{< /code-rust >}}

This trait is super generic because it is meant to be implemented for *all*
types that represent errors. This will prove useful for writing composable code
as we'll see later. Otherwise, the trait allows you to do at least the
following things:

* Obtain a `Debug` representation of the error.
* Obtain a user-facing `Display` representation of the error.
* Obtain a short description of the error (via the `description` method).
* Inspect the causal chain of an error, if one exists (via the `cause` method).

Let's define our first custom error type of the article and implement `Error`
for it. We can start with our
[last example](#code-error-double-string)
where we used `String` for our error type. `String` is useful in a pinch, but
it suffers from a couple draw backs:

* `String`s are mostly opaque to the caller. The only thing a caller can
  reasonably do with a `String` is show it to the user and hope it's
  descriptive enough.
* Unless you're unusually disciplined, using `String`s as errors tends to
  devolve into embedding error message strings inside your code. Reasonable
  people can disagree on whether this is bad or not, but I tend to view it as
  clutter.

An alternative to using `String`s for errors is to define an `enum` that
represents all possible error cases as *structured data* rather opaque strings.
In the case of our example, we had two primary error cases:

1. The user didn't provide an argument to the program.
2. The user provided an argument, but it couldn't be converted to an integer.

Both of these error cases could be collapsed into one: "invalid user input."
But we'd like our program to be descriptive about what went wrong. We achieved
this previously with `String`s as errors, but as I've discussed, they aren't
robust. So let's define our error type:

{{< code-rust "error-double" "1" >}}
use std::num::ParseIntError;

// We derive `Debug` because all types should probably derive `Debug`.
// Moreover, it is a prerequisite for implementing `Error`.
#[derive(Debug)]
enum CliError {
    NoArguments,
    InvalidNumber(ParseIntError),
}
{{< /code-rust >}}

This type encodes the two error cases described above. The `NoArguments` value
constructor has no additional information associated with it because there
isn't any. The `InvalidNumber` value constructor includes the *evidence* that
determined the number given by the user was invalid. This means that when we
pass our error on to the caller, they can inspect it for as much detail as they
like. Moreover, the caller can be confident that we didn't arbitrarily leave
out any error information.

Let's use this new error type instead of `String`s. First, we must implement
`Error`. Implementing `Error` also requires implementing `Debug` and `Display`.
We've already derived an automatic implementation of `Debug` (with
`#[derive(Debug)]`), so we only need to provide implementations for `Display`
and `Error`:

{{< code-rust "error-double" "2" >}}
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
{{< /code-rust >}}

Now that we've implemented `Error`, we can re-write our code to use our new
error type:

{{< code-rust "error-double" "3" >}}
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
{{< /code-rust >}}

The code itself doesn't change too much, but in my opinion, the code is a bit
cleaner without the error messages invading the implementation of `double_arg`.
Moreover, all of the error messages are lifted out into other places in the
code. (This is to my taste.)

Given only this example, it might be difficult to see the justification for
using the `Error` trait. The code is a bit cleaner, but there's also a lot more
of it. There are two important things to consider:

1. The proportion of code implementing `Error` in this example is lopsided
   because we have only one function producing errors. In real code, you'll
   likely have many functions producing errors.
2. The use of a generic `Error` trait makes automatically composing errors *of
   different types* very easy. We'll learn more about this in the next section.


## The `From` trait and the `try!` macro
