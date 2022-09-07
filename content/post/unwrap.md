+++
date = "2022-08-08T09:00:00-04:00"
title = "Using unwrap() in Rust is Okay"
author = "Andrew Gallant"
url = "unwrap"

[blackfriday]
plainIdAnchors = true
+++

One day before Rust 1.0 was released, I published a
[blog post covering the fundamentals of error handling][rust-error-handling].
A particularly important but small section buried in the middle of the article
is named "[unwrapping isn't evil][unwrapping-isnt-evil]". That section briefly
described that, broadly speaking, using `unwrap()` is okay if it's in
test/example code or when panicking indicates a bug.

I generally still hold that belief today. That belief is put into practice in
Rust's standard library and in many core ecosystem crates. (And that practice
predates my blog post.) Yet, there still seems to be widespread confusion about
when it is and isn't okay to use `unwrap()`. This post will talk about that
in more detail and respond specifically to a number of positions I've seen
expressed.

This blog post is written somewhat as a FAQ, but it's meant to be read in
sequence. Each question builds on the one before it.

**Target audience**: Primarily Rust programmers, but I've hopefully provided
enough context that the principles espoused here apply to any programmer.
Although it may be tricky to apply an obvious mapping to languages with
different error handling mechanisms, such as exceptions.

<!--more-->

## Table of Contents

* [What is my position?](#what-is-my-position)
* [What is `unwrap()`?](#what-is-unwrap)
* [What does it mean to "panic"?](#what-does-it-mean-to-panic)
* [What is error handling?](#what-is-error-handling)
* [Should `unwrap()` be used for error handling?](#should-unwrap-be-used-for-error-handling)
* [What about "recoverable" vs "unrecoverable" errors?](#what-about-recoverable-vs-unrecoverable-errors)
* [So one should never panic?](#so-one-should-never-panic)
* [So one should never use `unwrap()` or `expect()`?](#so-one-should-never-use-unwrap-or-expect)
* [What is a runtime invariant?](#what-is-a-runtime-invariant)
* [So why not make all invariants compile-time invariants?](#so-why-not-make-all-invariants-compile-time-invariants)
* [What about when invariants can't be moved to compile time?](#what-about-when-invariants-cant-be-moved-to-compile-time)
* [Why not return an error instead of panicking?](#why-not-return-an-error-instead-of-panicking)
* [When should `unwrap()` be used even if it isn't necessary?](#when-should-unwrap-be-used-even-if-it-isnt-necessary)
* [Why not use `expect()` instead of `unwrap()`?](#why-not-use-expect-instead-of-unwrap)
* [Should we lint against uses of `unwrap()`?](#should-we-lint-against-uses-of-unwrap)
* [Why are panics so great?](#why-are-panics-so-great)

## What is my position?

I think it's useful to state up front a number of my positions on error
handling and panicking. This way, readers know exactly where I'm coming from.

* Panicking should not be used for error handling in either applications or
libraries.
* It is possibly acceptable to use panicking for error handling while
prototyping, in tests, benchmarks and documentation examples.
* If a Rust program panics, then it signals a bug in the program. That is,
correct Rust programs don't panic.
* There is always a way to assign "blame" as to the fault of the panic. It's
either the fault of the function that panicked, or the fault of the caller of
that function.
* Outside of domains that need to use formal methods (or similar) to prove the
correctness of their programs, it is impossible or impractical to move every
invariant into the type system.
* Therefore, when runtime invariants arise, one has a few choices:
  1. One can make the function partial by causing it to panic on some subset of
     inputs (i.e., a precondition violation). In this case, if the function
     panics, then the bug is in the caller.
  2. Assume the invariant is never broken and panic when it is (i.e., an
     internal invariant). In this case, if the function panics, then the bug is
     in the callee.
  3. In the case of a precondition violation, one may return to the caller when
     it is violated. (For example, by returning an error.) However, this should
     _never_ be used in the case of an internal invariant violation because it
     leaks implementation details.
* In cases (1) and (2) above, it is fine to use `unwrap()`, `expect()` and
slice index syntax, among many other things.
* Prefer `expect()` to `unwrap()`, since it gives more descriptive messages
when a panic does occur. But use `unwrap()` when `expect()` would lead to
noise.

The rest of this article will justify these positions.

## What is `unwrap()`?

Since the ideas expressed in this post are *not* specific to Rust, I think it's
important to cover what `unwrap()` actually is. `unwrap()` refers to a method
defined on both `Option<T>` and `Result<T, E>` that returns the underlying `T`
in the case of a `Some` or `Ok` variant, respectively, and panics otherwise.
Their definitions are very simple. For `Option<T>`:

```rust
impl<T> Option<T> {
  pub fn unwrap(self) -> T {
    match self {
      Some(val) => val,
      None => panic!("called `Option::unwrap()` on a `None` value"),
    }
  }
}
```

And now for `Result<T, E>`:

```rust
impl<T, E: std::fmt::Debug> Result<T, E> {
  pub fn unwrap(self) -> T {
    match self {
      Ok(t) => t,
      Err(e) => panic!("called `Result::unwrap()` on an `Err` value: {:?}", e),
    }
  }
}
```

The key tension I'm trying to address in this post is whether and how much one
should use `unwrap()`.

## What does it mean to "panic"?

When a panic occurs, there are generally one of two things that will happen:

* The process aborts.
* If the target supports it, the stack unwinds. If the unwinding isn't caught,
then it will result in the process aborting with a message and an indication of
the source of the panic.

Which thing happens depends on how the program was compiled. It can be
controlled via the [`panic`][cargo-panic] profile setting in the `Cargo.toml`.

When unwinding occurs, it is possible to [catch the panic][catch-unwind] and
do something with it. For example, a web server might catch panics that occur
inside of request handlers to avoid bringing down the entire server. Another
example is a test harness that catches a panic that occurred in a test, so that
other tests may be executed and the results pretty printed instead of bringing
down the entire harness immediately.

While panics can be used for error handling, it is generally regarded as a poor
form of error handling. Notably, the language does not have good support for
using panics as error handling and, crucially, unwinding is not guaranteed to
occur.

When a panic causes unwinding that is never caught, the program will likely
abort once the entire stack has been unwound and print the message carried by
the object in the panic. (I say "likely" because one can set panic handlers and
panic hooks.) For example:

```rust
fn main() {
    panic!("bye cruel world");
}
```

Running it gives:

```
$ cargo build
$ ./target/debug/rust-panic
thread 'main' panicked at 'bye cruel world', main.rs:2:5
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

As the note says, backtraces can be enabled:

```
$ RUST_BACKTRACE=1 ./target/debug/rust-panic
thread 'main' panicked at 'bye cruel world', main.rs:2:5
stack backtrace:
   0: rust_begin_unwind
             at /rustc/0f4bcadb46006bc484dad85616b484f93879ca4e/library/std/src/panicking.rs:584:5
   1: core::panicking::panic_fmt
             at /rustc/0f4bcadb46006bc484dad85616b484f93879ca4e/library/core/src/panicking.rs:142:14
   2: rust_panic::main
             at ./main.rs:2:5
   3: core::ops::function::FnOnce::call_once
             at /rustc/0f4bcadb46006bc484dad85616b484f93879ca4e/library/core/src/ops/function.rs:248:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

Panics are not terribly useful or friendly as error messages for end users
of an application. However, panics typically provide very useful debugging
information to the programmer. Speaking from experience, a stack trace is
often enough information to understand precisely what went wrong inside the
application. But it's unlikely to be helpful for an end user. For example, it
would be poor form to panic if opening a file failed:

```rust
fn main() {
    let mut f = std::fs::File::open("foobar").unwrap();
    std::io::copy(&mut f, &mut std::io::stdout()).unwrap();
}
```

Here's what happens when we run the above program:

```
$ ./target/debug/rust-panic
thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: Os { code: 2, kind: NotFound, message: "No such file or directory" }', main.rs:2:47
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

The error message isn't totally useless in this scenario, but it doesn't
include the file path and it doesn't include any surrounding context informing
the user of what the application was trying to do when it ran into an I/O
error. It also contains a lot of noise that isn't useful to an end user.

In summary:

* Panics are great for programmers. They give a message, a stack trace and
line numbers. They are, on their own, often enough information to diagnose
a bug.
* Panics are not so great for end users. They're better than a silent abort,
but panic messages often lack context relevant for end users and are often
written specifically for programmers.

## What is error handling?

Error handling is what one does in one's code when something "goes wrong."
Without getting too deep into this, there are a few different ways to handle
errors in Rust:

1. One can abort with a non-zero exit code.
2. One can panic with the error. It might abort the process. It might not. As
described in the previous section, it depends on how the program was compiled.
3. One can handle errors as normal values, typically with `Result<T, E>`. If an
error bubbles all the way up to the `main` function, one might [print the error
to `stderr`][ripgrep-main] and then abort.

All three are perfectly valid error handling strategies. The problem is that
the first two lead to a very poor user experience for applications in the
context of Rust programs. Therefore, (3) is generally regarded as best
practice. The standard library and all core ecosystem libraries use (3).
Additionally, as far as I'm aware, all "popular" Rust applications use (3) as
well.

One of the most important parts of (3) is the ability to attach additional
context to error values as they are returned to the caller. The
[`anyhow`][anyhow] crate makes this effortless. Here is a snippet from an
in-progress `regex-cli` tool that I'm working on:

```rust
use anyhow::Context;

if let Some(x) = args.value_of_lossy("warmup-time") {
    let hdur: ShortHumanDuration = x.parse().context("--warmup-time")?;
    margs.bench_config.approx_max_warmup_time = Duration::from(hdur);
}
```

The important bit here is the `x.parse().context("--warmup-time")?` piece. For
those unfamiliar with Rust, I'll break it down:

* `x` is a `Cow<'a, str>`, which is "either an owned `String` or a borrowed
  `&str`." [`Cow` stands for "copy-on-write."][cow]
* `parse()` is short-hand for [`FromStr::from_str`][from_str], which parses
  a string into some other data type. In this case, a `ShortHumanDuration`.
  Since parsing can fail, `parse()` returns a `Result<T, E>`.
* `context()` comes from the [`anyhow::Context`][anyhow-context] trait. It is
  an "[extension trait][ext-trait]" that adds methods to a `Result<T, E>`. In
  this case, `context("--warmup-time")` is adding a short message to the
  error's causal chain.
* The `?` suffix operator says, "if the `Result<T, E>` is a `Ok(T)`, then
  give back the `T`, otherwise, return `E` as an error in the current
  function." (Note that this is not a precise description of what `?` does. See
  [the "question mark operator" section of the Rust reference][question-mark]
  for more details.)

The end result is that if one passes an invalid value to the `--warmup-time`
flag, then the error message will include `--warmup-time`:

```
$ regex-cli bench measure --warmup-time '52 minutes'
Error: --warmup-time

Caused by:
    duration '52 minutes' not in '<decimal>(s|ms|us|ns)' format
```

This makes it clear which part of the input provided by the user was
problematic.

(Note: `anyhow` is great for application oriented code, but if one is building
a library intended for others to use, I'd suggest writing out concrete error
types and providing an appropriate [`std::fmt::Display`][std-fmt-display] impl.
The [`thiserror`][thiserror] crate removes some of the boiler plate involved in
doing that, but I'd skip it to avoid the procedural macro dependencies if one
isn't already using procedural macro dependencies for something else.)

## Should `unwrap()` be used for error handling?

It is somewhat common to see `unwrap()` used for error handling in the
following three scenarios:

1. Quick one-off programs, prototyping or programs one might write for personal
use. Since the only end user is the programmer of the application, panics
aren't necessarily a bad user experience.
2. In tests. In general, Rust tests fail if they panic and pass if they don't
panic. So `unwrap()` in that context is quite all right, since it's likely that
a panic is exactly what one wants anyway. Do note that one can
[return `Result` from unit tests][result-in-tests], which permits using `?`
in tests, for example.
3. In documentation examples. In the past, it used to be quite a bit more
work to treat errors as values instead of using panics in documentation
examples. These days though, [`?` can be used in doctests][result-in-docs].

For me personally, I don't have a super strong opinion on whether `unwrap()`
*should* be used in any of the above scenarios. Here is where I fall on each of
them:

1. Even in quick programs or programs only built for myself, I treat errors as
values. `anyhow` makes this incredibly simple. Just `cargo add anyhow` and then
use `fn main() -> anyhow::Result<()>`. That's it. There's no huge ergonomic
advantage to using panicking for error handling in this context. `anyhow` will
even emit backtraces.
2. I liberally use `unwrap()` in tests. I rarely if ever use `?` in unit tests.
This might be because I started writing Rust before unit tests could return
`Result<T, E>`. I've never seen a compelling advantage to change what I'm doing
here and writing out longer signatures.
3. I have generally gravitated toward treating errors as values instead of
panicking in documentation examples. In particular, all one has to do is add
`# Ok::<(), Box<dyn std::error::Error>>(())` to the bottom of most examples,
and now `?` can be used in examples. It's easy to do and shows code that
tends to be more idiomatic. With that said, *real* error handling tends
to add context to errors. I would consider that idiomatic, yet I don't do
it in documentation examples. Additionally, documentation examples tend
to be targeting the demonstration of some particular facet of an API, and
expecting them to be perfectly idiomatic in every other aspect---especially
if it distracts focus from the point of the example---seems unrealistic.
So generally, I think `unwrap()` in documentation is okay, but I've been
gravitating away from it because it's easy to do.

So, in summary, I'd say "do not use `unwrap()` for error handling in Rust" is
a fine first approximation. But reasonable people can disagree over whether to
use `unwrap()` in some scenarios (as discussed above) due to its terseness.

With that said, I believe it is uncontroversial to state that `unwrap()` should
not be used for error handling in Rust libraries or applications that are
intended for others to use. That's a value judgment. One can disagree with it,
but I think it would be hard to argue that using `unwrap()` for error handling
leads to a good user experience. Therefore, I think most folks are aligned
here: `unwrap()`, and more generally, panicking, is not an adequate method of
error handling in Rust.

## What about "recoverable" vs "unrecoverable" errors?

The ["Error Handling" chapter in the Rust Book][rust-book-error-handling]
popularized the idea of thinking about errors as "recoverable" versus
"unrecoverable." That is, if an error is "recoverable" then one should treat
it as a normal value and use `Result<T, E>`. On the other hand, if an error is
unrecoverable then it's okay to panic.

I've personally never found this particular conceptualization to be helpful.
The problem, as I see it, is the ambiguity in determining whether a particular
error is "recoverable" or not. What does it mean, exactly?

I think it's much more helpful to be concrete. That is, if a panic occurs, then
there's a bug somewhere in the program. If the panic occurs inside of a
function because a documented precondition is not upheld, then the fault is
with the caller of the function. Otherwise, the fault is with the
implementation of that function.

That's all one needs to know to determine whether to treat errors as values or
to treat them as panics. Some examples:

* Is it a bug if the program couldn't open a file at a path specified by the
end user? Nope. So treat this as an error value.
* Is it a bug if the program couldn't build a regular expression from a static
string literal? Yup. The programmer typed that regex. It should be correct. So
a panic is appropriate.

## So one should never panic?

Generally, yes, correct Rust programs should not panic.

Does this mean that if panicking was used for error handling in a quick
Rust "script" that it is therefore not correct? [David Tolnay has
suggested][david-and-russell] that this borders on a form of [Russell's
paradox][russell-paradox], and I tend to agree with him. Alternatively, one can
think of the script or prototype as having bugs that are marked as `wontfix`.

## So one should never use `unwrap()` or `expect()`?

No! Routines like `unwrap()` or `expect()` only panic if its value is not what
the caller expected. If the value is _always_ what the caller expects, then it
follows that `unwrap()` and `expect()` will never result in a panic. If a panic
does occur, then this generally corresponds to a _violation of the expectations
of the programmer_. In other words, a runtime invariant was broken and it led
to a bug.

This is starkly different from "don't use `unwrap()` for error handling." The
key difference here is we _expect_ errors to occur at some frequency, but we
_never_ expect a bug to occur. And when a bug does occur, we seek to remove the
bug (or declare it as a problem that won't be fixed).

A lot of confusion around `unwrap()`, I think, comes from well meaning folks
saying things like "don't use `unwrap()`," when what they _actually_ mean is
"don't use panicking as an error handling strategy." This is doubly confused by
a different set of people who do actually literally mean ["don't use
`unwrap()`", ever, in any circumstance, to the point that it shouldn't have
existed in the first place][never-unwrap]. This is triply confused by yet
another set of people that say "don't use `unwrap()`," but actually mean,
"don't use `unwrap()`, `expect()`, slice indexing or any other panicking
routine even if one proves that panicking is impossible."

In other words, there are really two problems I'm trying to address in this
post. One is the problem of determining when one should use `unwrap()`. The
other is the problem of communication. This happens to be an area where
imprecision leads to what *appears* to be strangely inconsistent advice.

## What is a runtime invariant?

It is something that _should_ always be true, but the guarantee is _maintained_
at runtime as opposed to being proven at compile time.

A simple example of an invariant is an integer that is never zero. There are
a few ways to set this up:

* Use a [`std::num::NonZeroUsize`][non-zero-usize]. This maintains the
  invariant at _compile time_ because construction of the type guarantees
  that it cannot be zero.
* Use a `Option<usize>` and rely on the caller providing this value to use
  `None` when the inner `usize` is `0`. This maintains the invariant at
  _runtime_ because the construction of `Option<usize>` is not encapsulated.
* Use a `usize` and rely on the caller providing this value to never set it
  to `0`. This also maintains the invariant at _runtime_.

(Note: A `std::num::NonZeroUsize` has benefits other than enforcing this
particular invariant at compile time. Namely, it permits the compiler to do
a memory layout optimization where in a `Option<NonZeroUsize>` has the same
size in memory as a `usize`.)

In this case, if one needs an invariant like "an integer that is never zero,"
then utilizing a type like `NonZeroUsize` is a very compelling choice with
few downsides. It does introduce a little noise in the code when needing to
actually use the integer, since one has to call `get()` to get an actual
`usize`, and an actual `usize` is probably needed to do things like arithmetic
or use it to index slices.

## So why not make all invariants compile-time invariants?

In some cases, it can't be done. We'll cover that in the next section.

In other cases, it _can_ be done, but one chooses not to for some reason. One
such reason is API complexity.

Consider one real world example from my [`aho-corasick`][aho-corasick]
crate (which provides an implementation of the
[Aho-Corasick algorithm][wiki-aho-corasick]). Its
[`AhoCorasick::find_overlapping_iter`][ac-overlap] method panics
if the `AhoCorasick` automaton wasn't built, at runtime, with a
["match kind" of "standard"][ac-match-kind]. In other words, the
`AhoCorasick::find_overlapping_iter` routine imposes a documented precondition
on the caller to promise to only call it when `AhoCorasick` was built in a
certain way. I did it this way for a few reasons:

* Overlapping search only makes sense if the "match kind" is set to "standard."
* Setting the "match kind" is almost always going to be something done by the
  programmer, and not something that is controlled by input to the program.
* API simplicity.

What do I mean by "API simplicity?" Well, this panic could be removed by moving
this runtime invariant to a compile time invariant. Namely, the API could
provide, for example, an `AhoCorasickOverlapping` type, and the overlapping
search routines would be defined only on that type and not on `AhoCorasick`.
Therefore, users of the crate could never call an overlapping search routine on
an improperly configured automaton. The compiler simply wouldn't allow it.

But this adds a lot of additional surface area to the API. And it does it in
really pernicious ways. For example, an `AhoCorasickOverlapping` type would
still want to have normal non-overlapping search routines, just like
`AhoCorasick` does. It's now reasonable to want to be able to write routines
that accept any kind of Aho-Corasick automaton and run a non-overlapping
search. In that case, either the `aho-corasick` crate or the programmer using
the crate needs to define some kind of generic abstraction to enable that. Or,
more likely, perhaps copy some code.

I thus made a _judgment_ that having one type that can do everything---but
might fail loudly for certain methods under certain configurations---would be
best. The API design of `aho-corasick` isn't going to result in subtle logic
errors that silently produce incorrect results. If a mistake is made, then the
caller is still going to get a panic with a clear message. At that point, the
fix will be easy.

In exchange, we get an overall simpler API. There is only one type that can be
used to search with. One needn't to answer questions like, "wait which type
do I want? Now I have to go understand both and try to fit the puzzle pieces
together." And if one wants to write a single generic routine that accepts any
automaton and does a non-overlapping search, well, it doesn't need generics.
Because there is only one type.

## What about when invariants can't be moved to compile time?

Consider how one might implement a search using a deterministic finite
automaton (DFA). A basic implementation is only a few lines, so it's easy to
include it here:

```rust
type StateID = usize;

struct DFA {
  // The ID of the starting state. Every search starts here.
  start_id: StateID,
  // A row-major transition table. For a state 's' and a byte 'b',
  // the next state is 's * 256 + b'.
  transitions: Vec<StateID>,
  // Whether a particular state ID corresponds to a match state.
  // Guaranteed to have length equal to the number of states.
  is_match_id: Vec<bool>,
}

impl DFA {
  // Returns true if the DFA matches the entire 'haystack'.
  // This routine always returns either true or false for all inputs.
  // It never panics.
  fn is_match(&self, haystack: &[u8]) -> bool {
    let mut state_id = self.start_id;
    for &byte in haystack {
      // Multiple by 256 because that's our DFA's alphabet size.
      // In other words, every state has 256 transitions. One for each byte.
      state_id = self.transitions[state_id * 256 + usize::from(byte)];
      if self.is_match_id[state_id] {
        return true;
      }
    }
    false
  }
}
```

There are a few places where a panic might occur here:

* `state_id * 256 + byte` might not be a valid index into `self.transitions`.
* `state_id` might not be a valid index into `self.is_match_id`.
* The `state_id * 256` multiplication might panic in debug mode. In release
mode, currently, it will perform wrapping multiplication but that could change
to panicking on overflow in a future Rust version.
* Similarly, the `+ usize::from(byte)` addition might panic for the same
reason.

How would one guarantee, at compile time, that a panic will never occur given
the arithmetic and slice accesses? Keep in mind that the `transitions` and
`is_match_id` vectors might be built from user input. So however it's done, one
can't rely on the compiler knowing the inputs to the DFA. The input from which
the DFA was built might be an arbitrary regex pattern.

There's no feasible way to push the invariant that the DFA is constructed and
searched correctly to compile time. It has to be a runtime invariant. And who
is responsible for maintaining that invariant? The implementation that builds
the DFA and the implementation that uses the DFA to execute a search. Both of
those things need to be in agreement with one another. In other words, they
share a secret: how the DFA is laid out in memory. (Caveat: I have been wrong
about the impossibility of pushing invariants into the type system before.
I admit to the possibility here, my imagination is not great. However, I am
fairly certain that doing so would entail quite a bit of ceremony and/or be
limited in its applicability. Still though, it would be an interesting exercise
even if it doesn't fully fit the bill.)

If anything panicked, what would that mean? It *has* to mean that there is
a bug in the code somewhere. And since the documentation of this routine
guarantees that it never panics, the problem has to be with the implementation.
It's either in how the DFA was built or it's in how the DFA is being searched.

## Why not return an error instead of panicking?

Instead of panicking when there's a bug, one could return an error. The
`is_match` function from the previous section can be rewritten to return an
error instead of panicking:

```rust
// Returns true if the DFA matches the entire 'haystack'.
// This routine always returns either Ok(true) or Ok(false) for all inputs.
// It never returns an error unless there is a bug in its implementation.
fn is_match(&self, haystack: &[u8]) -> Result<bool, &'static str> {
  let mut state_id = self.start_id;
  for &byte in haystack {
    let row = match state_id.checked_mul(256) {
      None => return Err("state id too big"),
      Some(row) => row,
    };
    let row_offset = match row.checked_add(usize::from(byte)) {
      None => return Err("row index too big"),
      Some(row_offset) => row_offset,
    };
    state_id = match self.transitions.get(row_offset) {
      None => return Err("invalid transition"),
      Some(&state_id) => state_id,
    };
    match self.is_match_id.get(state_id) {
      None => return Err("invalid state id"),
      Some(&true) => return Ok(true),
      Some(&false) => {}
    }
  }
  Ok(false)
}
```

Notice how much more complicated this function got. And notice how ham-fisted
the documentation is. Who writes things like "these docs are totally wrong
if the implementation is buggy"? Have you seen that in any non-experimental
library? It doesn't make much sense. And why return an error if the docs
guarantee that an error will never be returned? To be clear, one *might* want
to do that for API evolution reasons (i.e., "maybe some day it will return an
error"), but this routine will never return an error under any circumstances in
any possible future scenario.

What is the benefit of a routine like this? If we were to [steelman][steelman]
advocates in favor of this style of coding, then I think the argument is
probably best limited to certain high reliability domains. I personally don't
have a ton of experience in said domains, but I can imagine cases where one
does not want to have any panicking branches in the final compiled binary
anywhere. That gives one a lot of assurance about what kind of state one's code
is in at any given point. It also means that one probably can't use Rust's
standard library or most of the core ecosystem crates, since they are all going
to have panicking branches somewhere in them. In other words, it's a very
expensive coding style.

The really interesting bit to this coding style---pushing runtime invariants
into error values---is that it's actually impossible to properly document the
error conditions. Well documented error conditions *relate the input to a
function* to some failure case in some way. But one literally can't do that for
this function, because if one could, one would be documenting a bug!

## When should `unwrap()` be used even if it isn't necessary?

Consider an example where the use of `unwrap()` could actually be avoided, and
the cost is only minor code complexity. This [adapted snippet was taken from
the `regex-syntax` crate][regex-syntax-snippet]:

```rust
enum Ast {
  Empty(std::ops::Range<usize>),
  Alternation(Alternation),
  Concat(Concat),
  // ... and many others
}

// The AST representation of a regex like 'a|b|...|z'.
struct Alternation {
  // Byte offsets to where this alternation
  // occurs in the concrete syntax.
  span: std::ops::Range<usize>,
  // The AST of each alternation.
  asts: Vec<Ast>,
}

impl Alternation {
    /// Return this alternation as the simplest possible 'Ast'.
    fn into_ast(mut self) -> Ast {
        match self.asts.len() {
            0 => Ast::Empty(self.span),
            1 => self.asts.pop().unwrap(),
            _ => Ast::Alternation(self),
        }
    }
}
```

The `self.asts.pop().unwrap()` snippet will panic if `self.asts` is empty. But
since we checked that its length is non-zero, it cannot be empty, and thus the
`unwrap()` will never panic.

But why use `unwrap()` here? We could actually write it without `unwrap()` at
all:

```rust
fn into_ast(mut self) -> Ast {
  match self.asts.pop() {
    None => Ast::Empty(self.span),
    Some(ast) => {
      if self.asts.is_empty() {
        ast
      } else {
        self.asts.push(ast);
        Ast::Alternation(self)
      }
    }
  }
}
```

The issue here is that if `pop()` leaves `self.asts` non-empty, then we do
actually want to create an `Ast::Alternation` since there are two or more
sub-expressions. If there's zero or one sub-expressions, then there's a simpler
representation available to us. So in the case of more than one sub-expression,
after we pop one, we actually need to push it back on to `self.asts` before
building the alternation.

The rewritten code lacks `unwrap()`, which is an advantage, but it's circuitous
and strange. The original code is much simpler, and it is trivial to observe
that the `unwrap()` will never lead to a panic.

## Why not use `expect()` instead of `unwrap()`?

`expect()` is like `unwrap()`, except it accepts a message parameter and
includes that message in the panic output. In other words, it adds a little
extra context to a panic message if a panic occurs.

I think it is a good idea to generally recommend the use of `expect()` over
`unwrap()`. However, I do not think it's a good idea to ban `unwrap()`
completely. Adding context via `expect()` helps inform readers that the writer
considered the relevant invariants and wrote a message saying what, exactly,
was expected.

`expect()` messages tend to be short though, and generally don't contain the
full justification for *why* the use of `expect()` is correct. Here's another
[example from the `regex-syntax` crate][regex-syntax-expect]:

```rust
/// Parse an octal representation of a Unicode codepoint up to 3 digits long.
/// This expects the parser to be positioned at the first octal digit and
/// advances the parser to the first character immediately following the octal
/// number. This also assumes that parsing octal escapes is enabled.
///
/// Assuming the preconditions are met, this routine can never fail.
fn parse_octal(&self) -> ast::Literal {
  // Check documented preconditions.
  assert!(self.parser().octal);
  assert!('0' <= self.char() && self.char() <= '7');
  let start = self.pos();
  // Parse up to two more digits.
  while self.bump()
    && '0' <= self.char()
    && self.char() <= '7'
    && self.pos().offset - start.offset <= 2
  {}
  let end = self.pos();
  let octal = &self.pattern()[start.offset..end.offset];
  // Parsing the octal should never fail since the above guarantees a
  // valid number.
  let codepoint =
    std::u32::from_str_radix(octal, 8).expect("valid octal number");
  // The max value for 3 digit octal is 0777 = 511 and [0, 511] has no
  // invalid Unicode scalar values.
  let c = std::char::from_u32(codepoint).expect("Unicode scalar value");
  ast::Literal {
    span: Span::new(start, end),
    kind: ast::LiteralKind::Octal,
    c,
  }
}
```

There are two uses of `expect()` here. In each case, the `expect()` message
is somewhat useful, but the real meat of why `expect()` is okay in both cases
comes in the form of comments. The comments explain why the `from_str_radix`
and `from_u32` operations will never fail. The `expect()` message just gives an
additional hint that makes the panic message slightly more useful.

Whether to use `unwrap()` or `expect()` comes down to a judgment call. In the
`into_ast()` example above, I think `expect()` adds pointless noise, because
the surrounding code so trivially shows why the `unwrap()` is okay. In that
case, there isn't even any point in writing a comment saying as much.

There are other ways that `expect()` adds noise. Some examples:

```rust
Regex::new("...").expect("a valid regex");
mutex.lock().expect("an unpoisoned mutex");
slice.get(i).expect("a valid index");
```

My contention is that none of these really add any signal to the code, and
actually make the code more verbose and noisy. If a `Regex::new` call fails
with a static string literal, then a nice error message is already printed. For
example, consider this program:

```rust
fn main() {
    regex::Regex::new(r"foo\p{glyph}bar").unwrap();
}
```

And now run it:

```
$ cargo run
    Finished dev [unoptimized + debuginfo] target(s) in 0.00s
     Running `target/debug/rust-panic`
thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: Syntax(
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
regex parse error:
    foo\p{glyph}bar
       ^^^^^^^^^
error: Unicode property not found
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
)', main.rs:4:36
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrac
```

Basically, at a certain point, writing the same `expect()` message over and
over again for the same common operations becomes a tedious exercise. Instead,
good judgment should be employed to determine whether to use `unwrap()` or
`expect()` in any given situation.

(Note: with respect to the `Regex` example, some people say that an invalid
regex in a string literal should result in the program failing to compile.
Clippy actually has a lint for that, but in general, it's not possible for
`Regex::new` to do that via Rust's `const` facilities. If it were to be
possible, then most of the Rust language would need to be usable inside a const
context. One could write a procedural macro instead, but `Regex::new` would
still need to exist.)

## Should we lint against uses of `unwrap()`?

One common argument against the idea of using good judgment is that it *can*
be nice to remove human judgment from the equation. If one lints against
`unwrap()`, then one *forces* every programmer to write something other than
`unwrap()`. The thinking goes that if one forces this step, then programmers
might think more deeply about whether their code can panic or not than they
would otherwise. Needing to write `expect()` and come up with a message, I
agree, exercises more brain cells and probably does result in folks thinking
more deeply about whether a panic can occur.

While I don't think such a lint is entirely unreasonable in certain contexts, I
would still make an argument against it.

Firstly, as I've already alluded to, I think many cases of `expect()` add
unnecessary noise to the code that clutters it up and makes it more verbose. In
many cases, it is either immediately obvious why an `unwrap()` won't fail, or
if it requires a more detailed argument, it's more likely to be found in a
comment than in an `expect()` message.

Secondly, `unwrap()` *is* idiomatic. To be clear, I am making a descriptive
statement. I am not saying it *ought* to be idiomatic. I'm saying that it
already is, based on its widespread usage in both the standard library and core
ecosystem crates. It's not just widespread in my own code. This *suggests* that
`unwrap()` isn't problematic in practice, although I recognize that claim has
some confounding factors.

Thirdly, there are *many* common things that can panic but don't require
writing `unwrap()`:

* Slice index syntax. For example, `slice[i]` panics when `i` is out of bounds.
The panic message is a bit better than what one would normally see with
`slice.get(i).unwrap()`, but still, a panic will result. If one bans `unwrap()`
because it's easy to thoughtlessly write, should one therefore also ban slice
index syntax?
* Overflow in arithmetic operations currently wraps in release mode, but it
panics in debug mode. It is possible that it will panic in release mode in the
future. If one bans `unwrap()` because it's easy to thoughtlessly write, should
one therefore also ban the use of fundamental operators like `+` and `*`? (That
it doesn't panic in release mode today doesn't mean bugs don't occur in release
mode! It's likely that arithmetic silently wrapping will probably lead to a
bug. So why not ban it and force folks to use, for example, `wrapping_add` and
`checked_add` everywhere instead? Remember, we're not trying to avoid panics.
We're trying to avoid bugs.)
* When using `RefCell` for interior mutability, its methods `borrow()` and
`borrow_mut()` will panic if a borrowing violation occurs at runtime. The same
argument applies here.
* Allocations themselves can fail, which currently will result in aborting the
process. Which is even worse than a panic. (Although, my understanding is that
it's desirable for failed allocations to panic and not abort.) Does this mean
one should be more cautious about allocations too?

The obvious hole in my argument is "don't let perfect be the enemy of the
good." Just because we can't or won't lint against every other thing that can
panic, that doesn't mean we shouldn't try to improve the situation by linting
against `unwrap()`. But I would argue that things like slice index syntax and
arithmetic operators are common enough that banning `unwrap()` won't make an
appreciable difference.

Fourthly and finally, banning `unwrap()` gives some non-zero probability to the
possibility that folks will start writing `expect("")` instead. Or `expect("no
panic")` if `expect("")` is banned. I'm sure most folks are familiar with lints
that inspire that sort of behavior. How many times have you seen a comment for
a function `frob_quux` that said "This frob's quux"? That comment is probably
only there because a lint told the programmer to put it there.

But as I said, I understand reasonable people can disagree here. I do not have
a bullet proof argument against linting `unwrap()`. I just happen to think the
juice isn't worth the squeeze.

## Why are panics so great?

Panics are the singular reason why bugs often don't require running Rust
programs in a debugger. Why? Because a lot of bugs result in a panic and
because panics give stack traces and line numbers, one of the most important
things (but not the only thing) that a debugger provides. Their greatness
extends beyond that. If a Rust program panics in the hands of an end
user, they can share that panic message and can probably stomach setting
`RUST_BACKTRACE=1` to get a full stack trace. That's an easy thing to do and is
especially useful in contexts where a reproduction is difficult to obtain.

Because panics are so useful, it makes sense to use them wherever possible:

* Use `assert!` (and related macros) to aggressively check preconditions and
runtime invariants. When checking preconditions, make sure the panic message
relates to the documented precondition, perhaps by adding a custom message. For
example, `assert!(!xs.is_empty(), "expected parameter 'xs' to be non-empty")`.
* Use `expect()` when including a message adds meaningful context to the panic
message. If `expect()` is associated with a precondition, then the importance
of a clear panic message goes up.
* Use `unwrap()` when `expect()` would add noise.
* Use other things like slice index syntax when an invalid index implies a bug
in the program. (Which is very usually the case.)

Of course, when possible, pushing runtime invariants to compile-time invariants
is generally preferred. Then one doesn't have to worry about `unwrap()` or
`assert!` or anything else. The invariant is maintained by virtue of the
program compiling. Rust is exceptionally well suited to pushing a lot of
runtime invariants to compile-time invariants. Indeed, its entire mechanism of
maintaining memory safety depends crucially on it.

Sometimes though, as shown above, it is either not always possible or not
always desirable to push invariants into the type system. In that case, be
happy to panic.

[rust-error-handling]: https://blog.burntsushi.net/rust-error-handling/
[unwrapping-isnt-evil]: https://blog.burntsushi.net/rust-error-handling/#a-brief-interlude-unwrapping-isnt-evil
[cargo-panic]: https://doc.rust-lang.org/cargo/reference/profiles.html#panic
[catch-unwind]: https://doc.rust-lang.org/std/panic/fn.catch_unwind.html
[ripgrep-main]: https://github.com/BurntSushi/ripgrep/blob/4dc6c73c5a9203c5a8a89ce2161feca542329812/crates/core/main.rs#L48-L53
[anyhow]: https://docs.rs/anyhow/1.*
[from_str]: https://doc.rust-lang.org/std/str/trait.FromStr.html
[anyhow-context]: https://docs.rs/anyhow/1.*/anyhow/trait.Context.html
[ext-trait]: https://rust-lang.github.io/rfcs/0445-extension-trait-conventions.html
[result-in-tests]: https://doc.rust-lang.org/book/ch11-01-writing-tests.html#using-resultt-e-in-tests
[result-in-docs]: https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html#using--in-doc-tests
[question-mark]: https://doc.rust-lang.org/stable/reference/expressions/operator-expr.html#the-question-mark-operator
[cow]: https://doc.rust-lang.org/std/borrow/enum.Cow.html
[david-and-russell]: https://github.com/rust-lang/project-error-handling/issues/50#issuecomment-1092145473
[russell-paradox]: https://en.wikipedia.org/wiki/Russell%27s_paradox
[never-unwrap]: https://www.thecodedmessage.com/posts/2022-07-14-programming-unwrap/
[non-zero-usize]: https://doc.rust-lang.org/std/num/struct.NonZeroUsize.html
[aho-corasick]: https://docs.rs/aho-corasick/0.7.*
[ac-overlap]: https://docs.rs/aho-corasick/0.7.*/aho_corasick/struct.AhoCorasick.html#method.find_overlapping_iter
[ac-match-kind]: https://docs.rs/aho-corasick/latest/aho_corasick/struct.AhoCorasickBuilder.html#method.match_kind
[steelman]: https://en.wikipedia.org/wiki/Straw_man#Steelmanning
[regex-syntax-snippet]: https://github.com/rust-lang/regex/blob/159a63c85eb77ec321301bc4c4ebfb90343edc2b/regex-syntax/src/ast/mod.rs#L551-L573
[regex-syntax-expect]: https://github.com/rust-lang/regex/blob/159a63c85eb77ec321301bc4c4ebfb90343edc2b/regex-syntax/src/ast/parse.rs#L1527-L1562
[rust-book-error-handling]: https://doc.rust-lang.org/book/ch09-00-error-handling.html
[std-fmt-display]: https://doc.rust-lang.org/std/fmt/trait.Display.html
[thiserror]: https://docs.rs/thiserror/1.*/
[wiki-aho-corasick]: https://en.wikipedia.org/wiki/Aho%E2%80%93Corasick_algorithm
