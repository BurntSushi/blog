+++
date = "2022-09-07T14:00:00-04:00"
title = "A byte string library for Rust"
author = "Andrew Gallant"
url = "bstr"

[blackfriday]
plainIdAnchors = true
+++

[`bstr`][bstr] is a byte string library for Rust and [its 1.0 version has
just been released][bstr1-release]! It provides string oriented operations on
arbitrary sequences of bytes, but is most useful when those bytes are UTF-8. In
other words, it provides a string type that is UTF-8 by _convention_, where as
Rust's built-in string types are _guaranteed_ to be UTF-8.

This blog will briefly describe the API, do a deep dive on the motivation for
creating `bstr`, show some short example programs using `bstr` and conclude
with a few thoughts.

**Target audience**: Rust programmers with some background knowledge of UTF-8.

<!--more-->

## Table of Contents

* [Brief API overview](#brief-api-overview)
* [Quick exampes](#quick-examples)
* [Motivation based on concepts](#motivation-based-on-concepts)
* [Motivation based on performance](#motivation-based-on-performance)
* [Example: counting characters, words and lines](#example-counting-characters-words-and-lines)
* [Example: windowing grep](#example-windowing-grep)
* [Example: detecting invalid UTF-8](#example-detecting-invalid-utf-8)
* [Other crates that support byte strings](#other-crates-that-support-byte-strings)
* [Should byte strings be added to std?](#should-byte-strings-be-added-to-std)
* [Acknowledgments](#acknowledgments)

## Brief API overview

The `bstr` crate works primarily by defining two extension traits,
[`ByteSlice`][byteslice] and [`ByteVec`][bytevec], that add string oriented
methods to the standard library `&[u8]` and `Vec<u8>` types.

Since methods are added to existing types, `bstr` does not require you to use
any new string types to get access to its APIs. However, `bstr` does provide
its own [`BStr`][bstr-slice] and [`BString`][bstr-owned] string types that
mirror the standard library `&str` and `String` string types. The main purpose
of these types is for use in public APIs, for communicating intent, to gain
access to string oriented `Debug` impls and for optional integration with
[Serde][serde].

That's pretty much it. The extension traits are where most of the APIs live.
Most of those APIs look very similar if not identical to the APIs provided
by the standard library `&str` and `String` types. The main difference is that
`bstr`'s APIs don't require valid UTF-8. For some APIs like substring search,
UTF-8 validity isn't a concern at all. For other APIs like iterators over
[`char`][char]s, invalid UTF-8 is handled by substituting the Unicode
replacement codepoint (`U+FFFD`): `�`.

One last thing to mention is the [`B` function][B], which I'll use occasionally
in this blog. See the API docs for a complete explanation, but it makes it
slightly more convenient to write byte slices. Namely, while `"foo"` has type
`&'static str`, the corresponding byte string `b"foo"` has type `&'static [u8;
3]`. In some cases, the use of an array leads to annoyances, for example,
`vec!["a", "ab"]` compiles but `vec![b"a", b"ab"]` does not.

## Quick examples

If you want to follow along at home with the examples, then a simple binary
Rust program is sufficient:

```
$ mkdir bstrblog
$ cd bstrblog
$ touch main.rs
$ cargo init --bin
$ cargo add bstr
```

Then open `main.rs` in your favorite editor and paste examples in there. Run
your program with `cargo run --release`.

First up is an example demonstrating substring search. Both the needle and
the haystack can be arbitrary bytes, just like classic C [`memmem`][memmem]
routine:

```rust
use bstr::ByteSlice;

fn main() {
    let haystack = b"foo bar foo\xFF\xFFfoo quux foo";

    let mut matches = vec![];
    for start in haystack.find_iter("foo") {
        matches.push(start);
    }
    assert_eq!(matches, [0, 8, 13, 22]);
}
```

This makes use of the [`ByteSlice::find_iter`][find_iter] method. Unlike the
standard library, `bstr` doesn't define polymorphic substring search APIs and
instead keeps things a little more concrete. For example, to search for a
`char`, you can use the [`ByteSlice::find_char`][find_char] method.

Note that if all you need is substring search on arbitrary bytes, or even just
a SIMD accelerated substring search which the standard library doesn't yet
have, then you can avoid bringing in all of `bstr` and just use the [`memmem`
sub-module of the `memchr` crate][memchr-memmem] instead. It's the same
substring search that powers [ripgrep].

Here's another example that demonstrates Unicode-aware uppercasing, but on text
that is not valid UTF-8.

```rust
use bstr::{B, ByteSlice};

fn main() {
    // \xCE\xB2 is the UTF-8 encoding of β.
    let lower = b"\xFF hello \xCE\xB2";
    let upper = lower.to_uppercase();
    // \xCE\x92 is the UTF-8 encoding of Β
    assert_eq!(B(b"\xFF HELLO \xCE\x92"), upper);
    // Why use 'B' here? Because otherwise its type is &[u8; N] and
    // there is no PartialEq impl for it and Vec<u8>. Another way to
    // write it would have been &b"\xFF HEL..."[..].
}
```

The above example demonstrates that invalid UTF-8 doesn't actually prevent one
from applying Unicode-aware algorithms on the parts of the string that are
valid UTF-8. The parts that are invalid UTF-8 are simply ignored.

Iterating over `char`s also works just fine even when the byte string is not
entirely valid UTF-8. The parts that are invalid UTF-8 simply get subtituted
with the Unicode replacement codepoint:

```rust
use bstr::ByteSlice;

fn main() {
    // We write out the raw encoding here because it isn't possible to
    // write a string literal in Rust that has both Unicode literals
    // and invalid UTF-8.
    let bytes = b"\xE2\x98\x83\xFF\xF0\x9D\x9E\x83\xE2\x98\x61";
    let chars: Vec<char> = bytes.chars().collect();
    assert_eq!(vec!['☃', '\u{FFFD}', '𝞃', '\u{FFFD}', 'a'], chars);
}
```

This next example shows one of the most useful aspects of `bstr`: the ability
to get nice `Debug` representations of byte strings. The downside is that you
need to convert your byte string to a `BStr` first, because there is no way to
override the standard library `Debug` impl for `&[u8]`. (Which just prints each
byte as a decimal number.)

```rust
use bstr::ByteSlice;

fn main() {
    // \xCE\xB2 is the UTF-8 encoding of β.
    let bytes = b"\xFF hello \xCE\xB2";

    println!("{:?}", bytes);
    // Output: [255, 32, 104, 101, 108, 108, 111, 32, 206, 178]
    println!("{:?}", bytes.as_bstr());
    // Output: "\xFF hello β"
}
```

## Motivation based on concepts

Rust's primary string types (`&str`/`String`) are perfectly fine for nearly
everything. And having the property that strings are guaranteed to be valid
UTF-8 can be quite useful in many contexts. That is, it lets you worry about
whether your data is malformed or not at the edges, and everything downstream
from it can know it's clean UTF-8 without ever having to worry about what to do
when it's not valid UTF-8.

In other words, if Rust's primary string types work for your use case, then you
should probably ignore `bstr` altogether and continue using them.

So why have a byte string library? The simplest way to explain it is to point
at the [`std::io::Read` trait][read-read]. How does it work? Well, it says
"anything implementing `std::io::Read` can take a writable slice of bytes, read
from its underlying source and put the bytes from the source to the writable
slice given." Do you see anything missing? There's no guarantee whatsoever
about what those bytes are. They can be anything. They might be an image. They
might be a video. Or a PDF. Or a plain text file.

In other words, the fundamental API we use to interact with data streams
doesn't make any guarantees about the nature of that stream. This is by design
and it isn't a Rust problem. On most mainstream operating systems, this is
how files themselves are represented. They are just sequences of bytes. The
*format* of those bytes usually exists at some other layer or is determined
through some additional context.

Let's try to make this concrete by considering how a `grep` program works: it
reads lines from stdin and prints lines that match a literal (so no regex
support). We'll write it using strings, because that's what you're supposed to
do... right?

```rust
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let needle = "Affiliate";
    for result in std::io::stdin().lines() {
        let line = result?;
        if line.contains(needle) {
            writeln!(std::io::stdout(), "{}", line)?;
        }
    }
    Ok(())
}
```

The question is, what do you expect the behavior of this program to be if
`stdin` doesn't contain valid UTF-8? And does that expectation line up with
what you _want_ the behavior of the program to be?

Well, let's try it out. And there's no need to make up our own data either.
Because invalid UTF-8 is more common than you might think. Whether it's because
of test data, or just not using UTF-8 (perhaps latin-1 instead) or just
outright errors. The Linux kernel as of a few years ago used to have plenty of
C source files that weren't valid UTF-8. That has since been fixed. But
[`gecko-dev` (the source code for Firefox)][gecko-dev] has plenty of files
that aren't valid UTF-8. Let's try running our program above on one:

```
$ path/to/bstrblog < ./third_party/aom/PATENTS
1.3. Defensive Termination. If any Licensee, its Affiliates, or its agents
Error: Error { kind: InvalidData, message: "stream did not contain valid UTF-8" }
```

(**Tip**: Check out my little [`find-invalid-utf8` utility][find-invalid-utf8]
for how I quickly discover files that contain invalid UTF-8. It also doubles
as a nice example usage of `bstr` APIs that would be pretty annoying to write
using Rust's standard library string types.)

Now, to be clear, it is perfectly reasonable for you to say, "I'm okay with
this." Indeed, plain text files that are also not valid UTF-8 are pretty rare.
And if you're building a purpose driven tool for data you control, it's
probably pretty likely that you don't care about your tool barfing on invalid
UTF-8.

On the other hand, if you're building a general purpose tool, invalid UTF-8 is
not quite rare enough to declare you don't care about it. Rust's own file path
handling, for example, goes to great pains to ensure it can handle all possible
file paths, even when they don't conform to any UTF encoding.

So... what do you? There are a few choices I can think of:

1. Skip any line that contains invalid UTF-8 and possibly print a warning
message to stderr.
2. If a line isn't valid UTF-8, lossily decode it and search that. This does
mean you won't be able to search for a needle that itself contains invalid
UTF-8, but it will handle a lot of cases. This is maybe not so bad for our
little literal searching tool, but is probably less ideal if your `grep`
program supported regexes. Do you or don't you want something like `.+` to
match through invalid UTF-8?
3. Use byte strings pretty much like you'd use `&str`/`String`, but have it
work automatically even when there's invalid UTF-8. And it will work even if
the needle contains invalid UTF-8.

The first two options can be done with Rust's standard library pretty easily,
but the third option cannot be. So if the third option is the choice you want
to make, then `bstr` is probably exactly what you're looking for. While the
standard library provides the `&[u8]`/`Vec<u8>` types, there is effectively
almost no support for treating them as byte strings. For example, the _only_
substring search that the standard library provides is defined on `&str`. You
can't use the standard library to do a substring search on `&[u8]`. So unless
you write your own substring search implementation (you probably shouldn't),
you're probably going to be looking for some kind of crate to do (3).

So what does this program look like with `bstr`? (It's about the same size, but
I've added some comments explaining a few things in the likely event that
you're unfamiliar with `bstr`.)

```rust
use std::io::Write;

// BufReadExt is an extension trait to std::io::BufRead
// that defines a number of byte string oriented methods,
// primarily for iterating over lines. These are useful
// because most of the line iterators in the standard
// library require valid UTF-8!
use bstr::{io::BufReadExt, ByteSlice};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let needle = "Affiliate";
    for result in std::io::BufReader::new(std::io::stdin()).byte_lines() {
        let line = result?;
        // '[T]::contains' is already defined in std and is not
        // substring search, so bstr defines it under a new name and
        // makes it generic so that it accepts anything that implements
        // AsRef<[u8]>.
        if line.contains_str(needle) {
            // We can't use 'writeln!' any more because we
            // want to output exactly what we read, and the
            // 'writeln!' family of macros can only write
            // valid UTF-8.
            std::io::stdout().write_all(&line)?;
            std::io::stdout().write_all(b"\n")?;
        }
    }
    Ok(())
}
```

Running it gives:

```
$ bstrblog < ./third_party/aom/PATENTS
1.3. Defensive Termination. If any Licensee, its Affiliates, or its agents
2.1. Affiliate.  Affiliate means an entity that directly or indirectly
```

In summary, when a program is given data, unless there is some other mechanism
for describing what that data is, the program simply does not know how to
interpret it. In _most_ cases, this is a bad thing. You really want to have
some kind of expected format and barf when the data does not conform.

But for general purpose Unix-like tooling on plain text files, what is your
expected format? It's probably something like "valid UTF-8 with a reasonably
small number of bytes between newline characters." (And an honorable mention
for UTF-16 on Windows.) But when there are so many files in practice that just
aren't valid UTF-8 but are still mostly plain text, it winds up being important
for your general purpose tool to handle them by simply skipping over those
invalid UTF-8 bytes. But crucially, when it comes time for your tool to print
its output, like a `grep`, it's important for it to print exactly what was
read. Doing this with string types that are guaranteed to be valid UTF-8 is
often difficult and sometimes just impossible.

(**Note**: a `grep` tool doesn't just search plain text. It can also search
binary data. But most `grep` tools have heuristics for detecting binary data.
In those cases, the data is still searched but output is often suppressed
unless a special flag is given.)

But this is only half of the story. The other reason why a byte string library
is useful is performance.

## Motivation based on performance

The high level idea for why byte strings might be faster than strings that
are guaranteed to be valid UTF-8 is relatively simple. Namely, when you're
expecting to see plain text, in most cases and in most contexts, that plain
text is going to be valid UTF-8. If you're using a byte string library, how
much does it cost to build the string in memory? It costs exactly as much as
it takes to load the data from the file and into memory. But how much does it
cost if your string types are guaranteed to be valid UTF-8? Well, the relative
cost from byte strings is UTF-8 validation, which requires a full scan over the
string.

That's pretty much it. Byte strings optimistically assume your strings are
UTF-8 and deal with invalid UTF-8 by defining some reasonable behavior on all
of its APIs for when invalid UTF-8 is encountered. In contrast, strings that
are guaranteed to be valid UTF-8 have to pessimistically assume the data might
be invalid UTF-8. Thus, UTF-8 validation must run during construction. Strings
that are guaranteed to be valid UTF-8 do have some performance upsides on
usage, for example, iterating over `char`s in a string _can_ be faster because
code that knows and can rely on valid UTF-8 is likely to be faster than code
that needs to deal with error conditions.

(**Musing**: it is perhaps possible to build a string type between these two
design points, but Rust's `&str` API as it is certainly requires validation
to be run before permitting the construction of a `&str`. Otherwise it's not
really possible to call it a type that guarantees valid UTF-8.)

(**Fun fact**: `str` types merely have a _safety_ invariant that they are
always valid UTF-8. They used to have a language level _validity_ invariant,
but [this was relaxed some time ago][validity-to-safety]. This doesn't have
much practical impact, but the short story is that building a `str` that isn't
valid UTF-8 _isn't_ instantly undefined behavior, but using almost any API on
`str` will probably result in undefined behavior.)

Let's continue with our `grep` example in the previous section. We'll start
with the `grep` program that uses `&str`/`String` for string types with a
couple tweaks:

* We use a needle that we know occurs a small number of times in the haystack.
This simplifies our benchmarking model somewhat by declaring that all we care
about is raw throughput. (Remember, all models are wrong, but some are useful.
This is not the *only* model, but it's a decent one that balances real world
use cases with simplicity.) We could instead choose a needle that never
matches, but it's good sense to pick one that matches sometimes to know that
we're actually achieving the problem we set out to solve: to print matching
lines. When it comes to throughput of a grep program, there is no practical
difference between printing a small number of matching lines and printing no
matching lines.
* Despite chiding one against implementing a substring search algorithm above,
we do exactly that here to ensure we are comparing apples to apples. In
particular, `bstr`'s substring search (which just uses the `memchr::memmem`
implementation) is _oodles_ faster than the standard library's due to its SIMD
acceleration. We'll do a quick bonus round at the end of this section to show
the difference.
* We do our best to eliminate "obvious" performance problems by amortizing
allocations. That is, we don't create a whole new allocation for every line.
However, we resist the urge to "write a fast grep." I [cover that
elsewhere][ripgrep], and instead stick to a decently simple program.
* We change our haystack to something a bit bigger. If we use a haystack that's
too small, then our benchmarking model likely needs to become more complicated
to account for noise. But if the haystack is big enough, noise is unlikely to
meaningfully impact our measurements.

To generate the haystack, clone the Rust repo and concatenate all
Rust files into a single file. For this blog post, I used commit
`b44197abb0b3ffe4908892e1e08ab1cd721ff3b9`. Note also that the files are sorted
before concatenating, so that the result is guaranteed to be deterministic.
I've also duplicated the result 5 times to make it just a little bigger. Here
are the precise commands to generate the haystack:

```
$ git clone https://github.com/rust-lang/rust
$ cd rust
$ git checkout b44197abb0b3ffe4908892e1e08ab1cd721ff3b9
$ find ./ -regex '[^ ]+\.rs' | sort | xargs cat > /tmp/rust.rs
$ for ((i=0; i<5; i++)); do cat /tmp/rust.rs; done > /tmp/rust.5x.rs
```

So with that, here's our revised `grep` program:

```rust
use std::io::{BufRead, Write};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let shiftor = ShiftOr::new("Sushi")?;
    let mut rdr = std::io::BufReader::new(std::io::stdin().lock());
    let mut line = String::new();
    loop {
        line.clear();
        let nread = rdr.read_line(&mut line)?;
        if nread == 0 {
            break;
        }
        if shiftor.find(&line).is_some() {
            // 'read_line' doesn't strip the line
            // terminator, so no need to write our own.
            std::io::stdout().write_all(line.as_bytes())?;
        }
    }
    Ok(())
}

#[derive(Debug)]
struct ShiftOr {
    masks: [u8; 256],
    needle_len: usize,
}

impl ShiftOr {
    fn new<T: AsRef<[u8]>>(needle: T) -> Result<ShiftOr> {
        let needle = needle.as_ref();
        let needle_len = needle.len();
        if needle_len > 7 {
            // A match is found when bit 7 is set in 'result' in the search
            // routine below. So our needle can't be bigger than 7. We could
            // permit bigger needles by using u16, u32 or u64 for our mask
            // entries. But this is all we need for this example.
            return Err("needle exceeds 7 bytes, too long".into());
        }
        let mut searcher = ShiftOr { masks: [!0; 256], needle_len };
        for (i, &byte) in needle.iter().enumerate() {
            searcher.masks[usize::from(byte)] &= !(1 << i);
        }
        Ok(searcher)
    }

    fn find<T: AsRef<[u8]>>(&self, haystack: T) -> Option<usize> {
        let haystack = haystack.as_ref();
        let mut result = !1u8;
        for (i, &byte) in haystack.iter().enumerate() {
            result |= self.masks[usize::from(byte)];
            result <<= 1;
            if result & (1 << self.needle_len) == 0 {
                return Some(i - self.needle_len + 1);
            }
        }
        None
    }
}
```

(**Note**: I chose to use the [Shift-Or][shiftor] algorithm because it's simple
and visits each byte in the haystack at most once. It's not going to come close
to something that is SIMD accelerated in terms of performance, but it keeps
our benchmark model simple without making it totally naive. The other thing
to notice here is that our substring search implementation works on `&[u8]`.
There is absolutely nothing about it that requires `&str`. And this is indeed
true for just about all substring search algorithms. It's just dealing with
bytes and doesn't care about UTF-8 at all. The key thing to remember is that if
it weren't for trying to do an apples-to-apples comparison here, we would of
course be using the standard library [`str::contains`][str-contains] method.
We have to kind of hold our nose a little bit here and acknowledge that the
Shift-Or code is purely a property of our measurement model. We'll explore what
"real world" code performance looks like later.)

Now let's build our code and run it. Since we chose a needle that occurs
exactly three times, we should see three lines of output on the smaller
haystack (which will be duplicated 5 times in the bigger haystack):

```
$ cargo build --release
$ cp ./target/release/bstrblog grep-str
$ ./grep-str < /tmp/rust.rs
        repo: "https://github.com/BurntSushi/ripgrep",
        repo: "https://github.com/BurntSushi/xsv",
//! @BurntSushi.
```

Now let's write the byte string version. If we did everything right, the only
operational difference between this program and the previous one is that this
program doesn't do UTF-8 validation. But everything else should be the same.
Note that we omit the `ShiftOr` type from this code listing since it remains
unchanged.

```rust
use std::io::{BufRead, Write};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let shiftor = ShiftOr::new("Sushi")?;
    let mut rdr = std::io::BufReader::new(std::io::stdin().lock());
    let mut line = Vec::new();
    loop {
        line.clear();
        let nread = rdr.read_until(b'\n', &mut line)?;
        if nread == 0 {
            break;
        }
        if shiftor.find(&line).is_some() {
            // 'read_line' doesn't strip the line
            // terminator, so no need to write our own.
            std::io::stdout().write_all(&line)?;
        }
    }
    Ok(())
}
```

Now build the new program and save the binary to a different name like we did
above. Also do our quick test to ensure it's doing the same work:

```
$ cp ./target/release/bstrblog ./grep-bytes
$ ./grep-bytes < /tmp/rust.rs
        repo: "https://github.com/BurntSushi/ripgrep",
        repo: "https://github.com/BurntSushi/xsv",
//! @BurntSushi.
```

Now let's bake them off with [Hyperfine][hyperfine]

```
$ hyperfine --warmup 5 "./grep-str < /tmp/rust.5x.rs" "./grep-bytes < /tmp/rust.5x.rs"
Benchmark #1: ./grep-str < /tmp/rust.5x.rs
  Time (mean ± σ):     573.0 ms ±   5.1 ms    [User: 531.1 ms, System: 41.3 ms]
  Range (min … max):   567.1 ms … 583.5 ms    10 runs

Benchmark #2: ./grep-bytes < /tmp/rust.5x.rs
  Time (mean ± σ):     449.2 ms ±   2.0 ms    [User: 407.5 ms, System: 41.2 ms]
  Range (min … max):   446.6 ms … 452.6 ms    10 runs

Summary
  './grep-bytes < /tmp/rust.5x.rs' ran
    1.28 ± 0.01 times faster than './grep-str < /tmp/rust.5x.rs'
```

There we go. The byte string version of the program is 1.28 times faster. It's
not Earth shattering by any means, but it's also nothing to sneeze at either.

Before popping up a level to discuss our findings, I think it would be fun to
compare the performance of similar programs, but without our Shift-Or
implementation. At the very least, it should tell us what kind of mistake we
would have made if we had just assumed that the standard library's substring
search had the same performance characteristics as the one found in `bstr`.

So here's the program using standard library routines:

```rust
use std::io::{BufRead, Write};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let needle = "Sushi";
    let mut rdr = std::io::BufReader::new(std::io::stdin().lock());
    let mut line = String::new();
    loop {
        line.clear();
        let nread = rdr.read_line(&mut line)?;
        if nread == 0 {
            break;
        }
        if line.contains(needle) {
            std::io::stdout().write_all(line.as_bytes())?;
        }
    }
    Ok(())
}
```

And now the program using `bstr`:

```rust
use std::io::{BufRead, Write};

use bstr::ByteSlice;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let needle = "Sushi";
    let mut rdr = std::io::BufReader::new(std::io::stdin().lock());
    let mut line = Vec::new();
    loop {
        line.clear();
        let nread = rdr.read_until(b'\n', &mut line)?;
        if nread == 0 {
            break;
        }
        if line.contains_str(needle) {
            std::io::stdout().write_all(line.as_bytes())?;
        }
    }
    Ok(())
}
```

And now let's bake them off with Hyperfine again:

```
$ hyperfine --warmup 5 "./grep-simple-str < /tmp/rust.5x.rs" "./grep-simple-bytes < /tmp/rust.5x.rs"
Benchmark #1: ./grep-simple-str < /tmp/rust.5x.rs
  Time (mean ± σ):     710.2 ms ±   7.7 ms    [User: 667.6 ms, System: 41.9 ms]
  Range (min … max):   704.1 ms … 731.1 ms    10 runs

Benchmark #2: ./grep-simple-bytes < /tmp/rust.5x.rs
  Time (mean ± σ):     549.6 ms ±   2.9 ms    [User: 509.5 ms, System: 39.6 ms]
  Range (min … max):   546.5 ms … 556.1 ms    10 runs

Summary
  './grep-simple-bytes < /tmp/rust.5x.rs' ran
    1.29 ± 0.02 times faster than './grep-simple-str < /tmp/rust.5x.rs'
```

Errrmmm... Wait, the byte string version is 1.29 times faster, which is almost
identical to our apples-to-apples version above. And on top of that, both
programs are _slower_ than our Shift-Or programs despite supposedly both using
much fancier and faster substring search algorithms. Indeed, it turns out that
a significant portion of the runtime of our program (~25% from a quick glance
at a profile) is the _construction of the substring searcher_. Owch! This means
we're really not measuring just throughput, but some combination of search
throughput added on to searcher construction.

Indeed, if you look back to our apples-to-apples comparison above, you'll
notice that the `ShiftOr` searcher is constructed _once_ at the start of the
program because our needle is invariant throughout the lifetime of the program.
Rebuilding it for every line doesn't make sense.

So now what? Well, the standard library doesn't really give us any options. You
can't build a substring searcher once like we did for `ShiftOr`. You just have
to call `contains` every time and eat the overhead.

`bstr` does provide a way to build the searcher once through, via its [`Finder`
API][finder]:

```rust
use std::io::{BufRead, Write};

use bstr::ByteSlice;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let searcher = bstr::Finder::new("Sushi");
    let mut rdr = std::io::BufReader::new(std::io::stdin().lock());
    let mut line = Vec::new();
    loop {
        line.clear();
        let nread = rdr.read_until(b'\n', &mut line)?;
        if nread == 0 {
            break;
        }
        if searcher.find(&line).is_some() {
            std::io::stdout().write_all(line.as_bytes())?;
        }
    }
    Ok(())
}
```

And now let's bake this one off against our standard library routine:

```
$ hyperfine --warmup 5 "./grep-simple-str < /tmp/rust.5x.rs" "./grep-opt1-bytes < /tmp/rust.5x.rs"
Benchmark #1: ./grep-simple-str < /tmp/rust.5x.rs
  Time (mean ± σ):     708.1 ms ±   2.4 ms    [User: 663.6 ms, System: 43.9 ms]
  Range (min … max):   704.5 ms … 710.8 ms    10 runs

Benchmark #2: ./grep-opt1-bytes < /tmp/rust.5x.rs
  Time (mean ± σ):     329.6 ms ±   2.9 ms    [User: 290.8 ms, System: 38.5 ms]
  Range (min … max):   326.7 ms … 335.1 ms    10 runs

Summary
  './grep-opt1-bytes < /tmp/rust.5x.rs' ran
    2.15 ± 0.02 times faster than './grep-simple-str < /tmp/rust.5x.rs'
```

This is now the fastest `grep` program we've written. There is simply not an
apples-to-apples comparison we can do with the standard library here, because
the API is not available.

There is one more `bstr` API that helps things: its [`BufReadExt` extension
trait][bufreadext]. It provides internal iterators over lines in a buffered
reader. In effect, it lets one avoid an additional copy of the bytes into our
caller provided `line` buffer in the code above. In exchange, we have to
provide a closure and invent our own protocol for stopping iteration early:

```rust
use std::io::Write;

use bstr::{io::BufReadExt, ByteSlice};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let searcher = bstr::Finder::new("Sushi");
    let mut rdr = std::io::BufReader::new(std::io::stdin().lock());
    rdr.for_byte_line_with_terminator(|line| {
        if searcher.find(line).is_some() {
            std::io::stdout().write_all(line.as_bytes())?;
        }
        Ok(true)
    })?;
    Ok(())
}
```

Baking this one off against our standard library routine gives us our best
run yet:

```
$ hyperfine --warmup 5 "./grep-simple-str < /tmp/rust.5x.rs" "./grep-opt2-bytes < /tmp/rust.5x.rs"
Benchmark #1: ./grep-simple-str < /tmp/rust.5x.rs
  Time (mean ± σ):     707.7 ms ±   3.0 ms    [User: 665.3 ms, System: 41.7 ms]
  Range (min … max):   702.4 ms … 712.7 ms    10 runs

Benchmark #2: ./grep-opt2-bytes < /tmp/rust.5x.rs
  Time (mean ± σ):     252.9 ms ±   1.1 ms    [User: 211.2 ms, System: 41.3 ms]
  Range (min … max):   251.3 ms … 254.5 ms    11 runs

Summary
  './grep-opt2-bytes < /tmp/rust.5x.rs' ran
    2.80 ± 0.02 times faster than './grep-simple-str < /tmp/rust.5x.rs'
```

So where does this leave us? Arguably, this section was just as much about
benchmarking methodology than it was about byte strings versus Rust's default
strings. But the benchmarking methodology is critical because it's important to
know what we're measuring. It also lets us examine some of the strengths and
weaknesses of our model.

One strength is that it lets us precisely characterize the performance we're
leaving on the table by doing UTF-8 validation for every line we iterate over.
This isn't a micro-benchmark either. It's a real program or part of a program
that someone might conceivably write, with perhaps a few alterations.
Iterating over lines and doing something with each matching line is a common
task.

But, this does expose a weakness: the model is _simplistic_. By being
simplistic, we are inherently leaving some performance on the table. For
example, we _could_ rearchitect our program to decrease the granularity with
which we run UTF-8 validation. Instead of doing it once per line, we might try
to do it once per 64KB buffer. Since it's likely that UTF-8 validation might
have some non-zero overhead, that could begin to add up when it's called for
every line. And it's likely that 64KB is _a lot_ of lines. So it would
effectively eliminate that overhead cost.

This not only results in a more complex program, and while it might eliminate
the _overhead_ of UTF-8 validation, it does not eliminate the cost of UTF-8
validation itself. That is, regardless of how you architect your program to
make UTF-8 validation faster, it will always have the relative disadvantage to
a program that uses byte strings that might not ever need to care about UTF-8
at all. It is perhaps [conceivable that UTF-8 validation could be made to run
so fast][simdutf8] that it is a nearly unnoticeable given the other work the
program is doing. But you'll still need to architect your program around it to
ensure you aren't getting bitten by overhead.

Program rearchitecture can actually make a very significant difference in the
`grep` problem domain. Consider baking off our fastest variant so far with
ripgrep (GNU grep achieves a similar speed up):

```
$ hyperfine --warmup 5 "./grep-opt2-bytes < /tmp/rust.5x.rs" "rg Sushi < /tmp/rust.5x.rs"
Benchmark #1: ./grep-opt2-bytes < /tmp/rust.5x.rs
  Time (mean ± σ):     253.6 ms ±   2.2 ms    [User: 210.1 ms, System: 43.2 ms]
  Range (min … max):   250.6 ms … 257.8 ms    11 runs

Benchmark #2: rg Sushi < /tmp/rust.5x.rs
  Time (mean ± σ):      13.3 ms ±   0.9 ms    [User: 4.6 ms, System: 8.9 ms]
  Range (min … max):    11.3 ms …  16.9 ms    178 runs

Summary
  'rg Sushi < /tmp/rust.5x.rs' ran
   19.01 ± 1.35 times faster than './grep-opt2-bytes < /tmp/rust.5x.rs'
```

As I said, I'm not going to do a deep dive on how to write a fast `grep` in
this blog, but it's likely the main reason why ripgrep is so much faster here
is because it doesn't actually iterate over the lines of the input. While line
iteration itself can be quite fast, what isn't fast is calling the substring
search implementation over and over again for every line. The substring search
implementation has overhead and that overhead adds up. Since the number of
matches are rare, ripgrep spends the vast majority of its time in substring
search, where as `grep-opt2-bytes` spends its time ping-ponging between line
iteration and substring search.

I leave it as an exercise to the reader to compare these programs when matches
are more frequent.

## Example: counting characters, words and lines

In this example, we're going to write a stripped down version of `wc`, which
counts things like lines, words and characters. We aren't going to try to be
POSIX compliant or even match GNU's `wc` behavior (or performance) precisely,
but we will make use of Unicode's grapheme and word segmentation algorithms.
Moreover, our program will work even when the input contains invalid UTF-8.

Let's initialize our project and setup some dependencies:

```
$ mkdir wc
$ cd wc
$ touch main.rs
$ cargo init --bin
$ cargo add anyhow bstr lexopt
```

(**Shout** out to the [`lexopt`][lexopt] crate, which provides what I think is
the best minimalist argument parser in Rust. Note that, by design, it doesn't
do `--help` generation. But it gets all the corner cases correct and is just
perfect for short little programs like these.)

Speaking of `lexopt`, let's start with the argument parsing part of this
program:

```rust
/// A configuration that says what we should count.
///
/// If no options are selected via arg parsing, then all options are
/// enabled.
#[derive(Clone, Debug, Default)]
struct Config {
    chars: bool,
    words: bool,
    lines: bool,
}

impl Config {
    /// Parse the given OS string args into a `wc` configuration.
    fn parse<I>(args: I) -> anyhow::Result<Config>
    where
        I: IntoIterator<Item = std::ffi::OsString> + 'static,
    {
        use lexopt::Arg::*;

        let mut config = Config::default();
        // lexopt is just the bee's knees for small little
        // programs like this!
        let mut parser = lexopt::Parser::from_iter(args);
        while let Some(arg) = parser.next()? {
            match arg {
                Short('m') | Long("chars") => config.chars = true,
                Short('l') | Long("lines") => config.lines = true,
                Short('w') | Long("words") => config.words = true,
                Short('h') | Long("help") => {
                    anyhow::bail!(
                        "Usage: wc [-m/--chars -l/--lines -w/--words]"
                    );
                }
                _ => return Err(arg.unexpected().into()),
            }
        }
        // If nothing is asked for, we do them all.
        if !config.chars && !config.words && !config.lines {
            config.chars = true;
            config.words = true;
            config.lines = true;
        }
        Ok(config)
    }
}
```

There isn't too much to see here. We set the fields on `Config` based on the
flags we see. If we don't see any flags, then we enable all of them.

And now for the main part of our program:

```rust
use std::io::{self, Write};

use bstr::{io::BufReadExt, ByteSlice};

/// Usage:
///   wc [options] < stdin
///   foo ... | wc [options]
///
/// Where 'options' is zero or more flags:
///   -m, --chars   Counts grapheme clusters.
///   -l, --lines   Counts lines, terminated by \n.
///   -w, --words   Counts words, using Unicode Word Segmentation.
fn main() -> anyhow::Result<()> {
    let config = Config::parse(std::env::args_os())?;
    let (mut chars, mut words, mut lines) = (0, 0, 0);
    let mut bufrdr = io::BufReader::new(io::stdin().lock());
    bufrdr.for_byte_line_with_terminator(|line| {
        lines += 1;
        if config.chars {
            chars += line.graphemes().count();
        }
        if config.words {
            words += line.words().count();
        }
        Ok(true)
    })?;
    let mut toprint = vec![];
    if config.lines {
        toprint.push(lines.to_string());
    }
    if config.words {
        toprint.push(words.to_string());
    }
    if config.chars {
        toprint.push(chars.to_string());
    }
    writeln!(io::stdout(), "{}", toprint.join("\t"))?;
    Ok(())
}
```

The parts of this program that actually make use of `bstr` are quite brief, but
this simplicity comes in part because of how byte strings optimistically assume
valid UTF-8. Those parts are:

* [`BufReadExt::for_byte_line_with_terminator`][buf_lines_term] gives us a
  super fast way of iterating over lines. We do have to provide a closure, but
  in exchange, we don't need to amortize allocations ourselves and there is no
  extra copying.
* [`ByteSlice::graphemes`][bstr-graphemes] gives us an iterator over all
  [grapheme clusters][grapheme-cluster] in a byte string. Grapheme clusters are
  Unicode's answer to how to _approximate_ what an end user might think of as
  a character. When invalid UTF-8 is encountered, it is substituted with the
  Unicode replacement codepoint and yielded as its own grapheme.
* [`ByteSlice:::words`][bstr-words] gives us an iterator over all
  [Unicode words][unicode-word] in a byte string. Like with graphemes, when
  invalid UTF-8 is encountered, it is subtituted with the Unicode replacement
  codepoint and yielded as its own word.

This program _could_ be written using `&str`/`String` with the
[`unicode-segmentation` crate][unicode-segmentation]. But, in order to use that
crate, you need a `&str`. This runs into similar issues we faced when writing
our `grep` program above. You could error out completely if invalid UTF-8 is
seen, skip lines that are invalid UTF-8, or try to lossily decode lines that
contain invalid UTF-8. Depending on your requirements, any of these options are
workable, but they come with extra code complexity and probably additional
runtime overhead. (I say "probably" because the grapheme and word counting
might dwarf UTF-8 validation, but this is very hand-wavy. Optimizing a
Unicode-aware `wc` is a deep rabbit hole that is beyond the scope of this
blog.)


## Example: windowing grep

In this example, we're going to adapt our `grep` program above so that it
prints a window of grapheme clusters around each match on a line. This is
especially useful when searching files with large lines (like minified
Javascript). This way, you can still see a bit of context for each match, but
without dumping a bunch of jibberish to your terminal.

We'll get started similarly as the previous example, except we'll add
`termcolor` so that we can colorize our matches.

```
$ mkdir window-grep
$ cd window-grep
$ touch main.rs
$ cargo init --bin
$ cargo add anyhow bstr lexopt termcolor
```

Let's again start with the argument parsing aspect of the program:

```rust
use bstr::ByteVec;

/// A configuration that says what we should look for and big the
/// window to print around each match.
#[derive(Clone, Debug)]
struct Config {
    /// The needle we want to search for.
    needle: Vec<u8>,
    /// A window size bigger than 255 kind of defeats the purpose.
    window: u8,
}

impl Config {
    /// Parse the given OS string args into a `window-grep`
    /// configuration.
    fn parse<I>(args: I) -> anyhow::Result<Config>
    where
        I: IntoIterator<Item = std::ffi::OsString> + 'static,
    {
        use lexopt::{Arg::*, ValueExt};

        const USAGE: &str = "Usage: window-grep [-w/--window SIZE] <needle>";

        let mut config = Config { needle: vec![], window: 10 };
        let mut saw_needle = false;
        let mut parser = lexopt::Parser::from_iter(args);
        while let Some(arg) = parser.next()? {
            match arg {
                Short('w') | Long("window") => {
                    config.window = parser.value()?.parse()?;
                }
                Short('h') | Long("help") => {
                    anyhow::bail!(USAGE);
                }
                Value(v) => {
                    anyhow::ensure!(!saw_needle, USAGE);
                    saw_needle = true;
                    // This is a bstr API that is a no-op on Unix and
                    // returns an error on Windows if the OS string
                    // wasn't originally valid UTF-16. Such things are
                    // rare on Windows and we don't care to support
                    // them.
                    config.needle = Vec::from_os_string(v).map_err(|_| {
                        anyhow::anyhow!(
                            "needle is not valid UTF-16 on Windows",
                        )
                    })?;
                }
                _ => return Err(arg.unexpected().into()),
            }
        }
        anyhow::ensure!(saw_needle, USAGE);
        anyhow::ensure!(!config.needle.is_empty(), "needle must be non-empty");
        Ok(config)
    }
}
```

Now let's write a couple routines to extract the leading and trailing grapheme
clusters from an arbitrary byte string:

```rust
use bstr::ByteSlice;

/// Return a slice of the `size` leading grapheme clusters from `slice`.
fn leading_graphemes(slice: &[u8], size: u8) -> &[u8] {
    slice
        .grapheme_indices()
        .take(usize::from(size))
        .last()
        .map_or(&[], |(_, end, _)| &slice[..end])
}

/// Return a slice of the `size` trailing grapheme clusters from `slice`.
fn trailing_graphemes(slice: &[u8], size: u8) -> &[u8] {
    slice
        .grapheme_indices()
        .rev()
        .take(usize::from(size))
        .last()
        .map_or(&[], |(start, _, _)| &slice[start..])
}
```

These routines make use of the
[`ByteSlice::grapheme_indices`][bstr-grapheme-indices] API in `bstr`, which not
only provides the grapheme cluster itself, but also the byte offsets at which
the cluster started and ended in the original byte string. (It's similar to the
standard library [`str::char_indices`][std-char-indices] API, but for grapheme
clusters.)

(**Note**: A similar approach is [used in ripgrep][ripgrep-grapheme] to
implement a similar windowing feature.)

Consider how one might implement something like this without an API
to decode graphemes from byte strings. Even if you had a substring
search implementation that works on byte strings, how would you go about
finding the surrounding grapheme cluster window? Since crates like
[`unicode-segmentation`][unicode-segmentation] require a `&str`, you'd have
to do some kind of UTF-8 validation. In that case, you would in turn have to
either validate the entire line, or find some way to write an incremental
UTF-8 validator (which is difficult/annoying to do with just standard library
routines). But even if you had that, how would you know when to stop? The key
problem here is that you don't know when to stop decoding grapheme clusters
without actually decoding them. The `unicode-segmentation` crate could
potentially help you by exposing an API that works on an `Iterator<Item=char>`,
but it's not totally clear if that's a good idea, and it could require big
internal refactoring.

Okay, let's move on to a function that prints and colorizes the match:

```rust
/// Write the given slice as a colored match.
fn write_match<W: WriteColor>(mut wtr: W, slice: &[u8]) -> io::Result<()> {
    use termcolor::{Color, ColorSpec};

    let mut color = ColorSpec::new();
    color.set_fg(Some(Color::Red)).set_bold(true);
    wtr.set_color(&color)?;
    wtr.write_all(slice)?;
    wtr.reset()?;
    Ok(())
}
```

There's nothing much interesting to note here. So now finally, let's look at
the meat of the program that looks for matches and does the printing:

```rust
use std::io::{self, Write};

use bstr::io::BufReadExt;

/// Usage:
///   window-grep [options] <needle> < stdin
///   foo ... | window-grep [options] <needle>
///
/// Where 'options' is zero or more flags:
///   -w SIZE, --window SIZE   The window size in graphemes. Default is 10.
fn main() -> anyhow::Result<()> {
    let config = Config::parse(std::env::args_os())?;
    let searcher = bstr::Finder::new(&config.needle);
    let mut bufrdr = io::BufReader::new(io::stdin().lock());
    let mut wtr = termcolor::StandardStream::stdout(ColorChoice::Auto);
    let mut lineno = 0;
    bufrdr.for_byte_line(|line| {
        lineno += 1;
        // Contains the offset of the last printed byte. This ensures
        // we don't print overlapping windows if the span between
        // matches is less than our window size.
        let mut printed = 0;
        let (mut start, mut end) = match searcher.find(line) {
            None => return Ok(true),
            Some(i) => (i, i + config.needle.len()),
        };
        write!(wtr, "{}:", lineno)?;
        loop {
            let before = &line[printed..start];
            wtr.write_all(trailing_graphemes(before, config.window))?;
            write_match(&mut wtr, &config.needle)?;
            printed = end;
            match searcher.find(&line[end..]) {
                None => break,
                Some(i) => {
                    start = end + i;
                    end = start + config.needle.len();
                }
            }
        }
        wtr.write_all(leading_graphemes(&line[end..], config.window))?;
        write!(wtr, "\n")?;
        Ok(true)
    })?;
    Ok(())
}
```

The offset management is a bit dense, but it's mostly to avoid printing
overlapping windows when the span between matches is smaller than our
configured window size. Otherwise, our `leading_graphemes` and
`trailing_graphemes` functions are doing the most interesting work. And that's
really where byte strings keep things simple in this program. If you were
forced to work with `&str`/`String` for this, then you're likely either paying
some non-trivial additional cost or adding some complexity to your code. Having
a grapheme cluster segmenter that works directly on byte strings ends up being
a nice convenience!

## Example: detecting invalid UTF-8

In this very short example, we're going to demonstrate how to detect invalid
UTF-8 by using `bstr`'s decode-one-codepoint-at-a-time API. We'll do this by
writing a program that prints only the lines from stdin that contain invalid
UTF-8. It will print the invalid UTF-8 bytes in their hexadecimal form and
colorize them to make it easier to see.

We'll start like we did with the previous examples. We won't need `lexopt` for
this one, but we keep `termcolor` around for colorizing.

```
$ mkdir badutf8
$ cd badutf8
$ touch main.rs
$ cargo init --bin
$ cargo add anyhow bstr termcolor
```

First, let's write a helper function that will be responsible for printing the
invalid UTF-8 bytes. That is, it should print them in hexadecimal form and
colorize them.

```rust
/// Write each byte in the slice in its hexadecimal form,
/// and with bold coloring.
fn write_invalid_utf8<W: WriteColor>(
    mut wtr: W,
    slice: &[u8],
) -> io::Result<()> {
    use termcolor::{Color, ColorSpec};

    let mut color = ColorSpec::new();
    color.set_fg(Some(Color::Red)).set_bold(true);
    wtr.set_color(&color)?;
    for &byte in slice.iter() {
        write!(wtr, r"\x{:X}", byte)?;
    }
    wtr.reset()?;
    Ok(())
}
```

And now let's see our `main` function that is responsible for iterating over
all lines, looking for invalid UTF-8 and printing the line:

```rust
use std::io::{self, Write};

use bstr::{io::BufReadExt, ByteSlice};
use termcolor::{ColorChoice, WriteColor};

/// Usage:
///   badutf8 < stdin
///   foo ... | badutf8
fn main() -> anyhow::Result<()> {
    let mut bufrdr = io::BufReader::new(io::stdin().lock());
    let mut wtr = termcolor::StandardStream::stdout(ColorChoice::Auto);
    let mut lineno = 0;
    bufrdr.for_byte_line(|mut line| {
        lineno += 1;
        if line.is_utf8() {
            return Ok(true);
        }
        write!(wtr, "{}:", lineno)?;
        loop {
            let (ch, size) = bstr::decode_utf8(line);
            if size == 0 {
                break;
            } else if ch.is_some() {
                wtr.write_all(&line[..size])?;
            } else {
                write_invalid_utf8(&mut wtr, &line[..size])?;
            }
            line = &line[size..];
        }
        write!(wtr, "\n")?;
        Ok(true)
    })?;
    Ok(())
}
```

This makes use of the [`bstr::decode_utf8`][decode-utf8] API. It permits
incrementally decoding one codepoint at a time from a byte string. It is
occasionally useful when you just want to pluck out a codepoint from somewhere
in a byte string, and have complete control over how invalid UTF-8 is handled.

Here's an example of how this program is used:

```
$ echo 'foo\xFFbar\xE2\x98quux' | badutf8
1:foo\xFFbar\xE2\x98quux
$ badutf8 < gecko-dev/third_party/aom/PATENTS
60:2.1. Affiliate.  \x93Affiliate\x94 means an entity that directly or indirectly
63:2.2. Control. \x93Control\x94 means direct or indirect control of more than 50% of
73:2.5. Final Deliverable.  \x93Final Deliverable\x94 means the final version of a
82:2.7. License. \x93License\x94 means this license.
84:2.8. Licensee. \x93Licensee\x94 means any person or entity who exercises patent
101:2.11. Reference Implementation. \x93Reference Implementation\x94 means an Encoder
105:2.12. Specification. \x93Specification\x94 means the specification designated by
```

You can't see the color here, but all of the hexadecimal numbers are bolded and
colored in red when printed to a terminal.

## Other crates that support byte strings

Almost every crate I publish that deals with text works on both `&str` and
`&[u8]`. Some examples:

* The [`bytes` submodule of the `regex` crate][regex-bytes] provides a `Regex`
  that can search a `&[u8]` instead of a `&str`. A `bytes::Regex` is also
  permitted to match invalid UTF-8 (or even split a codepoint if you want it
  to), where as the top-level `Regex` can never match invalid UTF-8 no matter
  what.
* The [`aho-corasick`][aho-corasick] crate provides APIs that work on anything
  that implements `AsRef<[u8]>`. This includes both the needles and the
  haystack.
* The [`memchr`][memchr] crate works exclusively on `&[u8]`.

The byte string support in these crates is absolutely critical for tools like
ripgrep to exist at all. Writing ripgrep using strings that are guaranteed to
be valid UTF-8 everywhere is flatly infeasible for much of the reasons
discussed earlier in this blog. But it's worth discussing one other reason as
well: file backed memory maps.

While searching files via memory maps isn't necessarily faster, it can be a bit
faster in some cases. ripgrep tries to use memory maps in those cases. A memory
map effectively exposes the contents of a file as a `&[u8]`. The full slice
might not actually be loaded into memory, but as your program accesses it, page
faults occur and the operating system loads data from the file into the `&[u8]`
for you automatically. It's all transparent and awesome. (But also, there are
pitfalls, and it's beyond the scope of this blog to explore them.)

So that `&[u8]` you get back might be huge. It might be bigger than available
memory. Now let's say none of the crates above had byte string support. How do
you run a regex search on a `&[u8]` when all you have are APIs that work on
`&str`? The regex crate doesn't provide any way to incrementally feed it data.
You could run it line-by-line, but that is quite slow and doesn't work in the
case of multi-line searches. You're kind of stuck. Your choices are:

* Don't use memory maps and miss out on the optimization.
* Modify the regex crate to support [searching streams of some
  kind][regex-streams].
* UTF-8 validate the entire `&[u8]` and convert it to a `&str`.

(**Note**: A similar problem exists with running regex or glob searches on
file paths. You can [get the underlying bytes of a file path to search on
Unix][unix-os-str-ext] without any additional cost, but on Windows, you're
pretty much forced to pay some kind of cost because the internet WTF-8
representation used by `Path` is hidden.)

These are all pretty bad choices. The last one in particular will force you to
do two passes over the data, which is likely in turn to dramatically slow
things down for large files. And it also won't let you deal with files that
have just a little invalid UTF-8 at all. _And_ it won't let you deal with
binary data at all either.

And that's why these crates support byte strings. It is by far the easiest
alternative. More to the point, none of these crates really need or benefit
much from using `&str` internally anyway. So the only cost of exposing a byte
string API is the API surface itself. A small price to pay when compared to the
alternatives.

## Should byte strings be added to std?

Some folks have expressed a desire for `bstr` or something like it to be put
into the standard library. I'm not sure how I feel about wholesale adopting
`bstr` as it is. `bstr` is somewhat opinionated in that it provides several
Unicode operations (like grapheme, word and sentence segmentation), for
example, that std has deliberately chosen to leave to the crate ecosystem.

Moreover, adding the `BStr` and `BString` API is likely to confuse matters and
add to "string fatigue" that Rust programmers sometimes experience. Adding new
byte string types is likely to cause at least some decision paralysis when it
comes to choosing between, say, `Vec<u8>` and `BString`. It's worth pointing
out that the primary advantage of the `BStr` and `BString` types is to serve as
a target for trait impls like `std::fmt::Debug` and `serde::{Deserialize,
Serialize}`. The standard library could help with the `Debug` impl by perhaps
providing a `debug()` method on `&[u8]`, similar to the `display()` method on
`Path`.

Otherwise, I think the highest value addition that std could adopt is substring
search where the needle and haystack are permitted to be `&[u8]`.

Other API additions are likely useful too (I'm a big fan of `bstr::decode_utf8`
for example), but I'm not sure whether they belong in std. It might be wise to
let `bstr 1.0` bake for a bit and see how it's used in the ecosystem after a
few years.

## Acknowledgments

Big thanks to [Thom Chiovoloni][thomcc] and [Ryan Lopopolo][lopopolo] for not
only their code contributions to `bstr`, but for also participating in API
design discussions. They were extremely helpful in fleshing out the current
API and catching mistakes.

[bstr1-release]: https://github.com/BurntSushi/bstr/releases/tag/1.0.0
[bstr]: https://docs.rs/bstr/1.*
[byteslice]: https://docs.rs/bstr/1.*/bstr/trait.ByteSlice.html
[bytevec]: https://docs.rs/bstr/1.*/bstr/trait.ByteVec.html
[bstr-slice]: https://docs.rs/bstr/1.*/bstr/struct.BStr.html
[bstr-owned]: https://docs.rs/bstr/1.*/bstr/struct.BString.html
[serde]: https://serde.rs/
[char]: https://doc.rust-lang.org/std/primitive.char.html
[B]: https://docs.rs/bstr/1.*/bstr/fn.B.html
[memmem]: https://man7.org/linux/man-pages/man3/memmem.3.html
[find_iter]: https://docs.rs/bstr/1.*/bstr/trait.ByteSlice.html#method.find_iter
[find_char]: https://docs.rs/bstr/1.*/bstr/trait.ByteSlice.html#method.find_char
[memchr-memmem]: https://docs.rs/memchr/2.*/memchr/memmem/index.html
[ripgrep]: https://github.com/BurntSushi/ripgrep
[read-read]: https://doc.rust-lang.org/std/io/trait.Read.html#tymethod.read
[gecko-dev]: https://github.com/mozilla/gecko-dev
[find-invalid-utf8]: https://github.com/BurntSushi/dotfiles/blob/cb01234174bd58194363e54e9c3c8b2ffa1774ef/bin/rust/find-invalid-utf8/main.rs
[validity-to-safety]: https://github.com/rust-lang/reference/pull/792
[ripgrep]: https://blog.burntsushi.net/ripgrep
[shiftor]: https://en.wikipedia.org/wiki/Bitap_algorithm
[str-contains]: https://doc.rust-lang.org/std/primitive.str.html#method.contains
[hyperfine]: https://github.com/sharkdp/hyperfine/
[finder]: https://docs.rs/bstr/1.*/bstr/struct.Finder.html
[bufreadext]: https://docs.rs/bstr/1.*/bstr/io/trait.BufReadExt.html
[simdutf8]: https://lemire.me/blog/2018/05/16/validating-utf-8-strings-using-as-little-as-0-7-cycles-per-byte/
[lexopt]: https://docs.rs/lexopt/0.2.*
[buf_lines_term]: https://docs.rs/bstr/1.*/bstr/io/trait.BufReadExt.html#method.for_byte_line_with_terminator
[bstr-graphemes]: https://docs.rs/bstr/1.*/bstr/trait.ByteSlice.html#method.graphemes
[bstr-words]: https://docs.rs/bstr/1.*/bstr/trait.ByteSlice.html#method.words
[grapheme-cluster]: https://www.unicode.org/reports/tr29/tr29-39.html#Grapheme_Cluster_Boundaries
[unicode-word]: https://www.unicode.org/reports/tr29/tr29-39.html#Word_Boundaries
[unicode-segmentation]: https://docs.rs/unicode-segmentation/1.*/
[ripgrep-grapheme]: https://github.com/BurntSushi/ripgrep/blob/60a1db34a69b0d57adb9c2725366e9d8adb5efdc/crates/printer/src/standard.rs#L1319-L1381
[bstr-grapheme-indices]: https://docs.rs/bstr/1.*/bstr/trait.ByteSlice.html#method.grapheme_indices
[std-char-indices]: https://doc.rust-lang.org/std/primitive.str.html#method.char_indices
[decode-utf8]: https://docs.rs/bstr/1.*/bstr/fn.decode_utf8.html
[thomcc]: https://github.com/thomcc
[lopopolo]: https://github.com/lopopolo
[regex-bytes]: https://docs.rs/regex/1.*/regex/bytes/index.html
[aho-corasick]: https://docs.rs/aho-corasick/0.7.*/aho_corasick/
[memchr]: https://docs.rs/memchr/1.*/memchr/
[regex-streams]: https://github.com/rust-lang/regex/issues/425
[unix-os-str-ext]: https://doc.rust-lang.org/std/os/unix/ffi/trait.OsStrExt.html#tymethod.as_bytes
