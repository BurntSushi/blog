+++
date = "2023-06-01T22:26:00-04:00"
title = "Regex engine internals as a library"
author = "Andrew Gallant"
url = "regex-internals"

[blackfriday]
plainIdAnchors = true
+++

Over the last several years, I've rewritten [Rust's `regex`
crate][regex-github] to enable better internal composition, and to make it
easier to add optimizations while maintaining correctness. In the course of
this rewrite I created a new crate, [`regex-automata`], which exposes much
of the `regex` crate internals as their own APIs for others to use. To my
knowledge, this is the first regex library to expose its internals to the
degree done in `regex-automata` as a separately versioned library.

This blog post discusses the problems that led to the rewrite, how the rewrite
solved them and a guided tour of `regex-automata`'s API.

**Target audience**: Rust programmers and anyone with an interest in how one
particular finite automata regex engine is implemented. Prior experience with
regular expressions is assumed.

<!--more-->

## Table of Contents

* [Brief history](#brief-history)
* [The problems](#the-problems)
    * [Problem: composition was difficult](#problem-composition-was-difficult)
    * [Problem: testing was difficult](#problem-testing-was-difficult)
    * [Problem: requests for niche APIs](#problem-requests-for-niche-apis)
    * [Problem: fully compiled DFAs](#problem-fully-compiled-dfas)
* [Follow along with regex-cli](#follow-along-with-regex-cli)
* [Flow of data](#flow-of-data)
* [Literal optimizations](#literal-optimizations)
    * [Motivating literal optimizations](#motivating-literal-optimizations)
    * [Literal extraction](#literal-extraction)
    * [Searching for literals](#searching-for-literals)
* [The NFA data type](#the-nfa-data-type)
    * [A simple NFA example](#a-simple-nfa-example)
    * [NFA optimization: sparse states](#nfa-optimization-sparse-states)
    * [NFA optimization: minimal UTF-8 automata](#nfa-optimization-minimal-utf-8-automata)
    * [NFA optimization: literal trie](#nfa-optimization-literal-trie)
    * [NFA future work](#nfa-future-work)
* [Regex engines](#regex-engines)
    * [Common elements among regex engines](#common-elements-among-regex-engines)
    * [Engine: Pike VM](#engine-pike-vm)
    * [Engine: bounded backtracker](#engine-bounded-backtracker)
    * [Engine: one-pass DFA](#engine-one-pass-dfa)
    * [Engine: DFA](#engine-dfa)
    * [Engine: hybrid NFA/DFA](#engine-hybrid-nfadfa)
    * [The meta regex engine](#the-meta-regex-engine)
* [Differences with RE2](#differences-with-re2)
* [Testing strategy](#testing-strategy)
* [Benchmarking](#benchmarking)
* [Costs](#costs)


## Brief history

In September 2012, an [issue was filed on the Rust
repository][first-regex-issue] requesting that a regex library be added to the
Rust Distribution. Graydon Hoare later [commented in that thread][graydon-re2]
that they preferred RE2. For those that don't know, [RE2] is a regex engine
that uses finite automata to guarantee `O(m * n)` worst case search time
while providing a Perl-like syntax that excludes features that are not known
how to implement efficiently. RE2's design is described by its author, Russ
Cox, in a [series of articles on implementing a regex engine using finite
automata][rsc-regexp].

In April 2014, I [showed up and said I was working on a regex engine inspired
by RE2][inspired-by-re2]. I treated Cox's articles as a blueprint for how to
build a regex library. Soon there after, I [published an RFC to add a regex
library to the "Rust Distribution."][first-regex-rfc] This was before Rust 1.0
and Cargo (the second version, not [the first][first-cargo]), and the "Rust
Distribution" referred to `rustc`, `std` and several "supporting" libraries
that were all bundled together. This RFC proposed adding a `regex` crate to
that list of supporting libraries.

Ten days later, [the RFC was approved][first-regex-rfc-approved]. The next
day, I [submitted a pull request to `rust-lang/rust`][first-regex-pr],
adding it to the Rust distribution. Things moved fast back then. Notice also
that I had originally called the crate `regexp`. The PR to Rust involved a
discussion about naming that eventually resulted in it being called `regex`
instead.

Two years later in May 2016, I [wrote an RFC to release `regex
1.0`][regex-1.0-rfc]. That took a few months to be approved, but it wasn't
until a couple years later in May 2018 that I [actually released `regex
1.0`][regex-1.0-pr].

Before `regex 1.0` was released, I had been steadily working on a complete
re-imagining of the crate internals. From a [commit message in March
2018][regex-syntax-0.5]:

> The [regex-syntax] rewrite is intended to be the first phase in an effort to
> overhaul the entire regex crate.

I didn't know exactly where I was going at that point in time, but in
March 2020, I started work in earnest on rewriting the actual matching
engines. A little more than three years later, [`regex 1.9` has been
released][regex-1.9-release] with the completed rewrite.

## The problems

What kinds of problems were facing the `regex` crate that warranted a full
rewrite? And moreover, why publish the rewritten internals as its own crate?

There are a host of things to discuss here.

### Problem: composition was difficult

Following in the [tradition of RE2][rsc-regexp], the `regex` crate contains a
number of different strategies that it can use to implement a search. Sometimes
*multiple* strategies are used in a single search call.

There are generally two dimensions, often at odds with one another, to each
strategy: performance and functionality. Faster strategies tend to be more
limited in functionality. For example, a fast strategy might be able to report
the start and end of a match but not the offsets for each capture group in the
regex. Conversely, a slower strategy might be needed to report the offsets of
each capture group.

When I originally wrote the `regex` crate, I implemented a single strategy
(the `PikeVM`) and didn't do any thoughtful design work for how to incorporate
alternative strategies. Eventually, new strategies were added organically:

* A `BoundedBacktracker` that can report capture group offsets like the
`PikeVM`, but does so using a backtracking strategy. Its main limitation is the
memory used to ensure its backtracking is bounded to `O(m * n)`, so it can only
be used for small haystacks/regexes. Its main upside is that its usually faster
than the `PikeVM`.
* A hybrid NFA/DFA (also know as a "lazy DFA") that can execute very quickly,
but can only report the start and end of a match. It ignores capture groups
completely.
* A literal strategy where a regex corresponds to a language that is both
finite and small. Examples: `foo`, `foo{2}`, `foo|bar`, `foo[1-3]`. In this
case, we could just use a single or multi-substring search algorithm without
any kind of regex engine at all.

(We'll get into why these strategies have these trade offs in more detail later
in the blog.)

And with the above strategies came the composition of them:

* When the caller requests capture group offsets, it is usually faster to
run the lazy DFA first to find the bounds of a match, and then only run the
`PikeVM` or `BoundedBacktracker` to find the capture group offsets. In this
way, especially for cases where matches are somewhat rare, most of the work is
done by the much faster lazy DFA.
* When a regex begins with a prefix that corresponds to a small finite
language, we can implement a *prefilter* that searches for occurrences of
that language. Each occurrence corresponds to a *candidate match* for the
overall regex. For each such candidate match, we run the full regex engine to
confirm whether it's an actual match or not. So for example, `foo\w+` would
look for occurrences of `foo` in a haystack, and then run the regex `foo\w+`
at the offset at which the occurrence of `foo` began. If there's a match, stop
and report it. Otherwise, restart the search for `foo` after the previous
occurrence of `foo`.

Over time, I wanted to add both more strategies *and* add more ways of
composing them. But in an organically grown infrastructure, the `regex`
crate was beginning to buckle under its weight. Loosely speaking, all of the
following were problems:

* Not all strategies were necessarily designed to be composed with others.
The `PikeVM`, for example, was the first strategy and it suffered from this.
Specifically, it could not deal with starting and stopping a search in a
subsequence of a slice, which is necessary in order to compose it with the lazy
DFA. For example, at one point, the `PikeVM` would say that `\babc\b` matched
in `abcxyz` if its search started at offset `0` and ended at offset `3`. But
the trailing `\b` should not match after `c` because a `x` follows it.
* It was difficult to reason about which strategy would be used for any given
regex.
* There were repeated `match` expressions re-implementing various logic that
was easy to go out of sync.
* The construction of a regex did not holistically account for the fact that
some strategies don't need to be constructed at all. For example, I at one
point added an optimization to the `regex` crate (prior to the rewrite) that
just used [Aho-Corasick] for regexes like `foo1|foo2|...|fooN`, but it was
extremely hacky to do that in a way that didn't *also* result in a [Thompson
NFA] being unnecessarily built that would never actually be used.

Basically, at the very least, many of the strategies needed a makeover and the
infrastructure that composes them probably needed to be rewritten.

### Problem: testing was difficult

While the `regex` crate exposes a public interface that acts as a single regex
engine, as we just discussed, it uses multiple strategies internally depending
on the situation. Many of these strategies are regex engines themselves, and it
is absolutely critical that they behave the same on the same inputs.

As one example, a common case is when the caller requests the offsets of each
capture group in a match. The way this usually works is to run the lazy DFA
to find the bounds of the match, and then a slower regex engine like the
`PikeVM` or the `BoundedBacktracker` on the bounds of the match to report the
capture group offsets. What happens then, if the lazy DFA finds a match where
the other engines don't? Oops. It's a bug.

The problem here is that prior to `regex 1.9`, all of the strategies used
internally are not part of any public API, and that makes them difficult to
independently test. One can of course test the public API, but the logic for
selecting which internal regex engines to use is complicated enough that there
isn't always a clear and obvious mapping between the pattern itself and which
regex engine will be used internally. Moreover, that mapping can change as the
logic evolves. So writing tests just against the public API is not something
that gives us clear coverage over all of the internal engines. And even if one
could do it, it would make debugging test failures more annoying than necessary
because you have to reason through the logic for which strategies are selected.

One approach to this is to put all tests inside the crate so that tests have
access to internal APIs. But you really want to leverage the same tests
across all of the engines, so doing this requires defining the tests in some
structured format, looping over them and running them on each engine. Since the
test infrastructure was not written with testing each individual strategy in
mind, I ended up not going this route.

Instead, I did some unholy hacks to make the existing test suite work:

* I exposed [some internal APIs][regex-github-hidden-api] to make it possible
to configure and build the internal strategies from outside the crate.
* I made it so one could actually build the main `Regex` type from those
internal APIs using an [undocumented `From` implementation][regex-from-exec].
* I wrote the tests using [macros][regex-test-macro].
* I created test targets for each internal regex engine I wanted to test,
and each test target was responsible for [defining the aforementioned
macros][regex-test-target] in a way that used the regex engine I wanted to
test.

This was overall a hacky mess and it really needed a rethink. Exposing the
internal engines in their own public API was not strictly a requirement to
improve the situation, but it does make it possible to run a test suite across
all engines without either relying on undocumented APIs or putting the tests
inside the crate itself.

### Problem: requests for niche APIs

Over the years, there were several requests for additional APIs to the `regex`
crate, but were ones I considered too niche to be worth expanding the API
surface, or where I wasn't totally clear on what the API ought to be.

One of the most popular such requests was better multi-pattern support. Namely,
the `regex` crate provides a [`RegexSet`] API that permits one to search for
possibly overlapping matches of zero or more regexes. The catch is that the API
only reports which patterns matched anywhere in the haystack. One cannot use
the API to get either the match offsets or the offsets of capture groups. While
useful, it isn't as useful as it could be if it supported the full `Regex` API.

As with adding multiple internal regex engines and the testing strategy, the
`RegexSet` API was bolted on to the existing implementation in a fairly hacky
way. Making it capable of reporting match offsets would require either a major
refactoring of all existing engines or a rewrite.

But separately from that, it wasn't totally clear to me how to expose APIs that
report match offsets in the context of the overlapping search done by the
`RegexSet` APIs. Having more room to experiment with alternative APIs, for
example, a `RegexSet` that does non-overlapping searches and reports match
offsets, would be something that others could use without needing to complicate
the `regex` crate API.

There have been other requests for additional APIs too:

* The ability to execute an anchored search without needing to put a `^` in the
pattern. This is especially useful in the context of running a regex on a
*part* of the haystack that you know matches, but where you want to extract
capture groups. It's also useful in the context of an iterator that only
reports adjacent matches. The `regex` crate could be augmented to support this,
but there's no easy way of extending existing APIs without either duplicating
all of the search routines, or unnecessarily making "anchored mode" an option
that one can pass to the regex. (Which, at that point, you might as well just
put `^` at the beginning of the pattern.)
* The ability to run a regex search without it doing synchronization internally
to get mutable scratch spaced used for a search. One might want to do this
to avoid those synchronization costs in some cases. But to do it would in turn
also require duplicating search APIs and exposing a new type that represents
the "mutable scratch space."
* Executing a [regex on streams and/or non-contiguous
haystacks][regex-streams]. This is especially useful for running a regex on
data structures like ropes, which are often found in text editors. This is a
big topic and not a problem I've even attempted to approach, but my hope is
that with more of the `regex` crate internals exposed, it might be tractable
for others to more easily experiment with solutions to this problem.

By publishing a new separately versioned crate that contains much of the
`regex` crate internals, it provides "breathing room" for more APIs that folks
want without needing to clutter up the general purpose regex API that services
the vast majority of all regex use cases. Namely, by targeting the crate toward
"expert" use cases, we make no show of trying to keep the API small and simple.
Indeed, as we'll see, the API of `regex-automata` is sprawling and complex. And
by making it separately versioned, we can put out breaking change releases at a
much faster cadence than what we do for the `regex` crate.

This line of reasoning is not too dissimilar from the line of reasoning that
led to the publication of the [`regex-syntax`] crate. Namely, folks (including
myself) wanted access to a regex parser for their own projects. I certainly
didn't want to expose a parser in the `regex` crate itself because of the added
complexity and the fact that I wanted to be able to evolve the parser and its
data types independently of the `regex` crate. (That is, `regex-syntax` has
breaking change releases more frequently than `regex` itself.) By putting it
into a separate crate, I could simultaneously use it as an implementation
detail of the `regex` crate while also making it available for others to use.

### Problem: fully compiled DFAs

The birth of `regex-automata` was not actually the result of my crusade to
rewrite the regex crate. Its birth coincided with a desire to build fully
compiled DFAs, serialize them and then provide a barebones search runtime that
could zero-copy deserialize those DFAs and use them for a search. I used the
original version of `regex-automata` to [build DFAs][bstr-fsm] for implementing
various Unicode algorithms inside of [`bstr`].

In the course of building the initial version of `regex-automata`, I realized
that I needed to rebuild an NFA data structure and a compiler for it that was
very similar to one found in the `regex` crate. At a certain point, I began
wondering whether it might be possible to share that code since it is quite
non-trivial and a place where a lot of interesting optimizations occur.

I had briefly considered building a new crate like `regex-nfa` that both the
`regex` crate and `regex-automata` could depend upon. But after more thought,
it became apparent that there was more code that could be shared between
`regex-automata` and `regex`. For example, a lot of the [determinization]
process can be written generically such that it works for both fully compiled
DFAs and for lazy DFAs.

At that point, the right abstraction boundary seemed like it was closer to
"regex engine" than it was "an NFA." So I reframed `regex-automata` as less
about *just* DFAs and more about a menagerie of regex engines. The plan at that
point was, roughly, to put all of the regex engines in `regex-automata` and
make the `regex` crate itself just a thin wrapper around `regex-automata`. By
setting things up this way, it should reduce the friction from migrating from
the `regex` crate to `regex-automata` if one should need access to the lower
level APIs.

In this way, we can build fully compiled DFAs using precisely the same code
that the `regex` crate uses for its lazy DFA. And precisely the same code that
the `regex` crate uses to convert the parsed representation of a regex pattern
into an NFA. Heck, this even makes it possible to use fully compiled DFAs in
the `regex` crate in some cases. (This is normally a big no-no in a general
purpose regex engine because fully compiled DFAs are not only quite bloated,
but they take worst case exponential time to build. That is very inappropriate
for a regex engine you might use to compile untrusted patterns, or even in
cases where you just need compilation time of a regex to be "reasonable."
Building a DFA may not be reasonable, especially when Unicode is involved.)

## Follow along with regex-cli

`regex-cli` is a program maintained as part of `regex` crate that provides
convenient command line access to many of the APIs exposed in `regex-syntax`,
`regex-automata` and `regex`. It also includes some useful utilities, such as
serializing fully compiled DFAs to a file and generating Rust code to read
them.

I will use `regex-cli` at points thoughout this blog post, so if you'd like to
follow along, you can install it straight from the `regex` crate repository:

```
$ cargo install --git https://github.com/rust-lang/regex regex-cli
```

Here's a pair of examples that shows the impact that Unicode has on the
`.` regex. First, the version with Unicode enabled:

```
$ regex-cli debug thompson '.' --no-table

thompson::NFA(
>000000: binary-union(2, 1)
 000001: \x00-\xFF => 0
^000002: capture(pid=0, group=0, slot=0) => 10
 000003: \x80-\xBF => 11
 000004: \xA0-\xBF => 3
 000005: \x80-\xBF => 3
 000006: \x80-\x9F => 3
 000007: \x90-\xBF => 5
 000008: \x80-\xBF => 5
 000009: \x80-\x8F => 5
 000010: sparse(\x00-\t => 11, \x0B-\x7F => 11, \xC2-\xDF => 3, \xE0 => 4, \xE1-\xEC => 5, \xED => 6, \xEE-\xEF => 5, \xF0 => 7, \xF1-\xF3 => 8, \xF4 => 9)
 000011: capture(pid=0, group=0, slot=1) => 12
 000012: MATCH(0)

transition equivalence classes: ByteClasses(0 => [\x00-\t], 1 => [\n], 2 => [\x0B-\x7F], 3 => [\x80-\x8F], 4 => [\x90-\x9F], 5 => [\xA0-\xBF], 6 => [\xC0-\xC1], 7 => [\xC2-\xDF], 8 => [\xE0], 9 => [\xE1-\xEC], 10 => [\xED], 11 => [\xEE-\xEF], 12 => [\xF0], 13 => [\xF1-\xF3], 14 => [\xF4], 15 => [\xF5-\xFF], 16 => [EOI])
)
```

And now the version with Unicode disabled:

```
$ regex-cli debug thompson '(?-u:.)' --no-table --no-utf8-syntax

thompson::NFA(
>000000: binary-union(2, 1)
 000001: \x00-\xFF => 0
^000002: capture(pid=0, group=0, slot=0) => 3
 000003: sparse(\x00-\t => 4, \x0B-\xFF => 4)
 000004: capture(pid=0, group=0, slot=1) => 5
 000005: MATCH(0)

transition equivalence classes: ByteClasses(0 => [\x00-\t], 1 => [\n], 2 => [\x0B-\xFF], 3 => [EOI])
)
```

The output here shows the Thompson NFA compiled by `regex-automata` for the
regex pattern given. The `regex-cli debug` command can print lots of different
data types from the regex crate ecosystem:

```
$ regex-cli debug
Prints the debug representation of various things from regex-automata and
regex-syntax.

This is useful for ad hoc interactions with objects on the command line. In
general, most objects support the full suite of configuration available in code
via the crate.

USAGE:
    regex-cli debug <command> ...

COMMANDS:
    ast        Print the debug representation of an AST.
    dense      Print the debug representation of a dense DFA.
    hir        Print the debug representation of an HIR.
    literal    Print the debug representation of extracted literals.
    onepass    Print the debug representation of a one-pass DFA.
    sparse     Print the debug representation of a sparse DFA.
    thompson   Print the debug representation of a Thompson NFA.
```

There is also a `regex-cli find` command that can run ad hoc searches. For
example, to run a multi-pattern search with capture groups using the meta regex
engine:

```
$ regex-cli find capture meta \
   -p '(?<email>[.\w]+@(?<domain>[.\w]+))' \
   -p '(?<phone>(?<areacode>[0-9]{3})-[0-9]{3}-[0-9]{4})' \
   -y 'foo@example.com, 111-867-5309'
     parse time:  20.713µs
 translate time:  22.116µs
build meta time:  834.731µs
    search time:  142.537µs
  total matches:  2
0:{ 0: 0..15/foo@example.com, 1/email: 0..15/foo@example.com, 2/domain: 4..15/example.com }
1:{ 0: 17..29/111-867-5309, 1/phone: 17..29/111-867-5309, 2/areacode: 17..20/111 }
```

See the [`regex-cli` README] for a few other examples.

## Flow of data

Before diving into details, it's worth pausing for a moment first to introduce
some terms and briefly describe the flow of data through the regex engine.
That is, when you call `Regex::new` with a pattern string, we'll trace the
transformations done on the pattern string that turn it into something that can
search haystacks.

* A pattern string is first parsed into an [`Ast`]. An `Ast` is a structured
representation of the pattern.
* An `Ast` is translated into an [`Hir`]. An `Hir` is another structured
representation of the pattern, but contains a lot less detail than an `Ast`.
Things like Unicode case folding and Unicode character class references are
all expanded as part of translation.
* An `Hir` is then used to build two things. First is a [literal sequence],
which corresponds to a sequence of literals extracted from the pattern that
are used to optimize regex searches in some cases. If possible, a literal
sequence is used to build a [`Prefilter`]. Secondly, an `Hir` is used to
construct an [`NFA`].
* At this point, an `NFA` is used to build a variety of regex engines:
  * A [`PikeVM`] can handle all possible regexes that are supported by parsing.
  A `PikeVM` can also report offsets for matching capture groups.
  * A [`BoundedBacktracker`] uses backtracking but explicitly bounds itself
  to avoid repeating work. Like the `PikeVM`, it can report offsets for
  matching capture groups.
  * A [one-pass DFA] that supports a very limited subset of regexes, but can
  report offsets for matching capture groups very quickly.
  * A fully compiled [dense DFA]. It can only report the overall start and end
  of a match (when combined with a second reverse DFA), but is very fast. The
  main downside is that its construction algorithm has worst case `O(2^m)`
  time and space complexity.
  * A [lazy DFA] that builds itself during a search from an `NFA`. In some
  cases it can be slower than a `PikeVM`, but in most cases is as fast as
  a fully compiled `DFA` and lacks the downside of `O(2^m)` worst case
  construction time/space.
* All of the above regex engines, including a `Prefilter` if one was
constructed, are composed into a single [meta regex engine].
* The `regex` crate itself is a thin wrapper around the meta regex engine from
the `regex-automata` crate.

We'll discuss each of these things in more detail throughout the blog, but
it's difficult to avoid referencing some of these things before they get a
full treatment. For that reason, the above is meant to give you a very general
blueprint of `regex` crate internals.

## Literal optimizations

In this section, we will begin our journey into the `regex` crate internals
by talking about a critical optimization technique that it uses: literal
extraction from regexes. For example, the regex `(foo|bar|quux)(\s+\w+)`
describes a regular language where all elements in the language start with one
of `foo`, `bar` or `quux`. That is, every match of that regex is guaranteed to
begin with one of those three literals.

### Motivating literal optimizations

Why does this matter? Why do we care about literals at all? We care about them
because of the following two observations:

1. There exist algorithms for searching for one or a small number of literals
that are extremely fast. Speed is usually obtained by exploiting the
"simplicity" of searching for plain literals and using [vector instructions] to
process many bytes in a haystack in a single step.
2. In *very* broad strokes, general algorithms for searching for matches of a
regular expression cannot be accelerated easily.

The key observation is that while there are certainly many different techniques
to implementing a regex engine (which we will cover a subset of in more depth
later), none of them can consistently be as fast as a well optimized substring
search implementation that uses vector instructions. In practice, I've often
found that the difference is at least one order of magnitude, and sometimes
more.

### Literal extraction

Let's show some examples. For this, we can utilize `regex-cli`, as it exposes
a `regex-cli debug literal` sub-command for extracting literals from regexes.
Let's start simple:

```
$ regex-cli debug literal 'bar'
           parse time:  13.967µs
       translate time:  7.008µs
      extraction time:  405ns
    optimization time:  1.479µs
                  len:  Some(1)
           is finite?:  true
            is exact?:  true
      min literal len:  Some(3)
      max literal len:  Some(3)
longest common prefix:  Some("bar")
longest common suffix:  Some("bar")

E("bar")
```

In the future, I'll trim the output since there is a lot of extra information
shown. But let's go through them:

* Parse time refers to the time it takes to turn the `bar` pattern into a
structured [`Ast`] value.
* Translate time refers to the time it takes to turn the `Ast` value into
an [`Hir`] value.
* Extraction time refers to the time it takes to turn the `Hir` value into
a [literal sequence].
* Optimization time refers to the time it takes to "optimize" the literal
sequence. This might be as simple as removing duplicate literals and as
aggressive as shrinking the sequence in various ways based on heuristics. We'll
see more examples of this later.
* `len` is the number of literals in the sequence extracted.
* Finite refers to whether the sequence has a finite number of elements. An
infinite sequence represents the sequence of all possible literals, and usually
means that literal optimizations aren't possible or aren't believed to be
fruitful.
* Exact refers to whether every element in the literal sequence is exact or
not. An exact literal refers to a literal that reached a match state from the
place where literal extraction began. Since this command extracts prefixes, an
exact literal corresponds to an overall match of the regex. If a literal is
not exact, then it is said to be inexact.
* Minimum literal length refers to the length, in bytes, of the shortest
literal in the sequence.
* Maximum literal length refers to the length, in bytes, of the longest
literal in the sequence.
* Longest common prefix represents a single literal that is a prefix of all
elements in the sequence. Infinite sequences and finite sequences containing
zero elements lack a common prefix. All other sequences have a common prefix of
at least the empty string.
* Longest common suffix represents a single literal that is a suffix of all
elements in the sequence. Infinite sequences and finite sequences containing
zero elements lack a common suffix. All other sequences have a common suffix of
at least the empty string.

Finally, after the above meta data, the extracted sequence is shown. Since the
regex is just the literal `bar`, the sequence contains a single exact element
corresponding to `bar`. If `bar` were a strict prefix, then the sequence would
be the same but `bar` will be inexact:

```
$ regex-cli debug literal --no-table 'bar+'
I("bar")
```

In this case, `bar` is inexact because the `r` at the end can match one or more
times. In fact, because of the unbounded repetition operator being applied
to a non-empty string, the language described by this regex is infinite. It
therefore follows that one cannot enumerate all literals and that at least some
of them extracted must be inexact (if any are extracted at all).

But one does not need to write a regex that describes an infinite language
in order to get an inexact literal. Here's an example of a regex that describes
a finite language, but for which only inexact literals are extracted:

```
$ regex-cli debug literal --no-table 'bar[a-z]'
I("bar")
```

Literal extraction *could* have enumerated every literal, for example,
`bara`, `barb`, `barc`, ..., `barz`. But instead it didn't. Why? It turns out
that literal extraction is one big heuristic. A dark art, if you will. We have
to go back to *why* we're doing literal extraction at all in the first place:
to identify candidate matches in a haystack very quickly before using a slower
regex engine to confirm whether a match exists at that location.

The trick here is that the choice of *what* literals to search for might be
just as important as choosing *how* to search for them. The algorithm with the
highest throughput in the world isn't going to help you if your haystack is
1,000,000 `a`s and your regex is just the literal `a`. The key here is that a
good literal optimization achieves both of the following things:

* Minimizes the false positive rate of candidates. That is, most candidates it
reports lead to a match.
* Minimizes its impact on the latency of the search. That is, when a prefilter
is active, it ideally results in running the regex engine on only a small
portion of the haystack. If a prefilter reports candidates frequently, then
even if it has a 0% false positive rate, its impact on latency is likely to be
hurting overall search times.

The reason why I called literal optimizations a *dark art* is because it is
impossible to know, before a search begins, how to optimally choose the above
two things. The reason is because they both depend on the haystack itself, and
scanning the haystack to "study" it is almost certainly going to result in a
net negative for overall search times. Therefore, we have to guess at how to
minimize the false positive rate while reducing our impact on latency. That's
why it's a dark art.

Thankfully, there are some guidelines we can follow that usually give us a good
result:

* A smaller sequence of literals is usually better than a larger sequence,
but not if this results in elements that are extremely short. That is, 1 or 2
bytes in length. Short elements are likely to match much more frequently, and
so we'd rather not have them. For example, if we had 5,000 literals that were
all limited to lowercase ASCII letters, we could trivially shrink the number of
literals to at most 26 by taking the first byte of each literal. But this new
sequence of literals is likely to match a lot more frequently, and thus result
in a higher hit to latency and a higher false positive rate. It would be better
to shrink the sequence while retaining longer literals that are less likely to
match.
* Longer literals are generally better than short literals, but not if it would
result in a large sequence. Longer literals are usually more discriminative,
that is, they lead to a lower false positive rate since they are less likely to
match by chance. But one doesn't want to prioritize long literals arbitrarily.
For example, you might have a sequence containing the literals `foobar`,
`foobaz`, `fooquux`, but a better sequence would probably be `foo` even though
it's shorter than all three literals in the sequence. A single element sequence
is nice because it means we can potentially use a single-substring search
algorithm (which is probably fast).

Literal extraction tries to adhere to the above guidelines as much as possible,
but there are some other heuristics that often come into play. For example, the
ASCII space character, `U+0020`, is unusually common. If a sequence would
otherwise contain a space, then the sequence is made infinite when optimized,
and this effectively disables literal optimizations. For example, this regex
has three prefix literals extracted:

```
$ regex-cli debug literal --no-table '(?:foo|z|bar)[a-z]+'
I("foo")
I("z")
I("bar")
```

But this one doesn't. The only difference is that the `z` was replaced with a
space:

```
$ regex-cli debug literal --no-table '(?:foo| |bar)[a-z]+'
Seq[∞]
```

This heuristic takes place during the "optimization" pass of a literal
sequence. The heuristic notices that one of the literals is just a space
character, assumes this will lead to a high false positive rate and makes the
sequence infinite. When a sequence is infinite, it communicates that there
is no small set of finite literals that would (likely) serve as a good
prefilter. If we disable optimization, we can see that the space character is
included:

```
$ regex-cli debug literal --no-table --no-optimize '(?:foo| |bar)[a-z]+'
I("foo")
I(" ")
I("bar")
```

To make matters more complicated, if we use a different regex that leads to
a small finite sequence of literals that are all exact, then the literal
containing the space character doesn't result in the overall sequence being
made infinite:

```
$ regex-cli debug literal --no-table 'foo| |bar'
E("foo")
E(" ")
E("bar")
```

This is useful because when all the literals are exact, the regex engine can
be skipped completely. In this case, there doesn't have to be any prefilter
at all. One can just use the multi-substring algorithm directly to report
matches. Therefore, the concern about a high false positive rate is irrelevant,
because every match produced by searching for the literals is a real match.

At this point, we should cover why I'm using the term literal *sequence*
instead of literal *set*. Namely, the order of the literals extracted matters.
It matters because the `regex` crate tries to simulate Perl-like semantics.
That is, that the matches reported are done *as if* employing a backtracking
search. This is also called leftmost-first matching, and in this context, the
`|` operator is not commutative. For example:

```
$ regex-cli debug literal --no-table 'sam|samwise'
E("sam")

$ regex-cli debug literal --no-table 'samwise|sam'
E("samwise")
E("sam")
```

These are two different sequences. Both are minimal with respect to the
corresponding regexes. In the first case, `sam|samwise` will only ever match
`sam`, since `sam` is a prefix of `samwise` and comes before `samwise` in the
pattern. Therefore, a literal sequence consisting of just `sam` is correct,
since `samwise` can never match. In the second case, `samwise|sam` can match
either branch. Even though `sam` is a prefix of `samwise`, since `samwise`
appears first, it will be preferred when `samwise` is in the haystack.

(Note: POSIX regex engines don't implement regexes this way. Instead, they have
leftmost-longest semantics, where the longest possible match always wins. In
this case, `|` is a commutative operator. Some other regex engines, such as
[Hyperscan], implement "report all matches" or "earliest match" semantics. In
that case, `abc|a` would match both `a` and `abc` in the haystack `abc`.)

Our last examples show that literal extraction is somewhat intelligent. For
example:

```
$ regex-cli debug literal --no-table 'abc?de?[x-z]ghi'
I("abcde")
I("abcdx")
I("abcdy")
I("abcdz")
I("abdex")
I("abdey")
I("abdez")
I("abdxg")
I("abdyg")
I("abdzg")
```

That is, literal extraction knows how to expand things like `?` and even small
character classes. This works as long as the literal sequence size stays
under [several different heuristic limits][literal-extractor]. (Notice also
that literal extraction could have enumerated every element in the language
described by this regex, in full, but optimization chose to shrink it in
accordance with its heuristics.)

Another example of "intelligence" is that case insensitivity, including
Unicode awareness, is taken into account as well:

```
$ regex-cli debug literal --no-table '(?i)She'
E("SHE")
E("SHe")
E("ShE")
E("She")
E("sHE")
E("sHe")
E("shE")
E("she")
E("ſHE")
E("ſHe")
E("ſhE")
E("ſhe")
```

This actually isn't a result of literal extraction implementing Unicode case
folding, but rather due to the translation from an `Ast` to an `Hir` doing the
case folding for us:

```
$ regex-cli debug hir --no-table '(?i)She'
Concat(
    [
        Class(
            {
                'S'..='S',
                's'..='s',
                'ſ'..='ſ',
            },
        ),
        Class(
            {
                'H'..='H',
                'h'..='h',
            },
        ),
        Class(
            {
                'E'..='E',
                'e'..='e',
            },
        ),
    ],
)
```

That is, literal extraction sees this regex as one that is equivalent to
`[Ssſ][Hh][Ee]`. All it does is expand the classes as it would any other regex.

### Searching for literals

Once you've extracted some literals, you now need to figure out how to search
for them.

The single substring case is somewhat easy: you pick the fastest algorithm
you can for finding a substring in a haystack. There's not much more to it.
You don't need to care about the order of literals at this point since there
is only one of them. For this case, the `regex` crate uses the
[`memmem`] module from the [`memchr`] crate.

There are several different aspects to the algorithm used in `memchr::memmem`:

* Its principal algorithm is [Two-Way], which runs in `O(n)` worst case time
and constant space.
* In cases where the needle and haystack are both very short, [Rabin-Karp]
is used in an effort to minimize latency.
* On `x86_64`, a variant of the "[generic SIMD]" algorithm is used. Basically,
two bytes are chosen from the needle, and occurrences for those two bytes in
their proper positions are searched for using vector instructions. When a
match of those two bytes is found, then a full match of the needle is checked.
(Notice that this is just another variant of the prefilter mechanism. We pick
an operation that can quickly find candidates and then perform a more expensive
verification step.)

For the generic SIMD algorithm, instead of always choosing the first and last
bytes in the needle, we choose two bytes that we believe are "rare" according
to a background frequency distribution of bytes. That is, we assume that bytes
like `Z` are far less common than bytes like `a`. It isn't always true, but
we're in heuristic-land here. It's true commonly enough that it works well in
practice. By choosing bytes that are probably rarely occurring from the needle,
we hope to maximize the amount of time spent in the vector operations that
detect candidates, and minimize the number of verifications we need to perform.

The multi-substring case is a bit trickier. Here, we need to make sure we
treat the literals as a sequence and prioritize matches for literals earlier
in the sequence over literals that come later. It's also typically true that
multi-substring search will be slower than single-substring case, because
there's just generally more work to be done. Here, the principal algorithm
employed is [Teddy], which is an algorithm that I ported out of [Hyperscan].
At a high level, it uses vector instructions to detect candidates quickly and
then a verification step to confirm those candidates as a match.

The [Aho-Corasick] algorithm is also used in some cases, although usually the
regex engine will just prefer to construct a lazy DFA since performance is
similar. Aho-Corasick can still help as a prefilter when the lazy DFA cannot be
used though. Aho-Corasick will also typically do better than a lazy DFA when
the number of literals is extremely large (around tens of thousands).

There is a lot more work I hope to do in the multi-substring case going
forward.

## The NFA data type

If there was a central data type inside the `regex` crate, it would probably be
the [`NFA`]. More specifically, it is a Thompson NFA, which means it was built
by an algorithm similar to [Thompson's construction]. Thompson's construction
builds an NFA from a structured representation of a regex in `O(m)` time, where
`m` is proportional to the size of the regex after counted repetitions have
been expanded. (For example, `a{5}` is `aaaaa`.) The algorithm works by mapping
each type of regex expression to a mini NFA unto itself, and then defining
rules for composing those mini NFAs into one big NFA.

NFAs are a central data type because they can be used directly, as-is,
to implement a regex engine. But they can also be transformed into other
types (such as DFAs) which are in turn used to implement different regex
engines. Basically, at present, if you want to build a regex engine with
`regex-automata`, then you have to start with a Thompson NFA.

Before exploring NFAs in more detail, let's look at a simple example.

### A simple NFA example

As with literal extraction, `regex-cli` can be helpful here by letting us print
a debug representation of an NFA when given a regex:

```
$ regex-cli debug thompson 'a'
        parse time:  9.856µs
    translate time:  3.005µs
  compile nfa time:  18.51µs
            memory:  700
            states:  6
       pattern len:  1
       capture len:  1
        has empty?:  false
          is utf8?:  true
       is reverse?:  false
   line terminator:  "\n"
       lookset any:  ∅
lookset prefix any:  ∅
lookset prefix all:  ∅

thompson::NFA(
>000000: binary-union(2, 1)
 000001: \x00-\xFF => 0
^000002: capture(pid=0, group=0, slot=0) => 3
 000003: a => 4
 000004: capture(pid=0, group=0, slot=1) => 5
 000005: MATCH(0)

transition equivalence classes: ByteClasses(0 => [\x00-`], 1 => [a], 2 => [b-\xFF], 3 => [EOI])
)
```

In the future, I'll trim the output to just the NFA itself. So let's explain
what the rest of the output means here:

* The parse and translate timings are the same as they were for literal
extraction. That is, the time to build `Ast` and `Hir` values, respectively.
* Compilation time refers to the time it takes to compile an `Hir` value into
an `NFA`.
* Memory refers to the number of bytes of heap memory used by the NFA.
* States is the number of states in the NFA.
* Pattern length is the number of patterns in the NFA.
* Capture length is the number of capture groups compiled into the NFA. When
capture groups are enabled (they are by default), then there is always at least
1 group corresponding to the overall match.
* "has empty" refers to whether the NFA can match the empty string or not.
* "is utf8" refers to whether the NFA is guaranteed to never match any
invalid UTF-8. This includes *not* matching the empty string between code
units in a UTF-8 encoded codepoint. For example, `💩` is UTF-8 encoded as
`\xF0\x9F\x92\xA9`. While the empty regex will match at every position, when
the NFA is in UTF-8 mode, the only matches it would report are immediately
before the `\xF0` and immediately after `\xA9`.
* "is reverse" refers to whether the NFA matches the regex in reverse. This
means that it matches the language described by the original regex, but with the
elements in the language reversed.
* Line terminator refers to the line terminator used for the `(?m:^)`, `(?m:$)`
and `.` regexes.
* "lookset any" is the set of all look-around assertions in the regex.
* "lookset prefix any" is the set of all look-around assertions that occur in
the prefix of the regex. Every match may match zero or more of these.
* At the end, the "transition equivalence classes" refers to a partitioning of
all possible byte values into sets of equivalence class. The rule is that each
byte in an equivalence class can be treated as equivalent to one another with
respect to whether a match occurs.

Other than that, the main output is the NFA itself. Let's walk through it:

```
>000000: binary-union(2, 1)
 000001: \x00-\xFF => 0
^000002: capture(pid=0, group=0, slot=0) => 3
 000003: a => 4
 000004: capture(pid=0, group=0, slot=1) => 5
 000005: MATCH(0)
```

The `^` before state `2` indicates that it is the "anchored" starting state,
while the `>` before state `0` indicates that it is the "unanchored" starting
state. The former is used when running an anchored search and the latter is
used for an unanchored search.

An unanchored search starts with `binary-union(2, 1)`. This indicates that
NFA traversal should first go to state `2` (a `capture` state), and only if
that path fails should it try going to state `1`. In this case, state `1`
matches any byte and loops back around to state `2`. In effect, states `0` and
`1` represent an implicit `(?s-u:.)*?` prefix at the beginning of the regex,
effectively making it possible for it to match anywhere in the haystack.

A `capture` state is an unconditional epsilon transition that only exists to
cause a side effect: it instructs the virtual machine executing the NFA to
store the current offset in the slot included in the `capture` state. If one is
doing NFA traversal outside the context of a virtual machine (or something else
that cares about capture groups), the `capture` states are effectively ignored
by treating them as unconditional epsilon transitions with no side effects. For
example, this is how they are handled during [determinization] (the process of
converting an NFA to a DFA).

Once at state `3`, one must check whether the byte at the current position is
equivalent to `a`, and if so, moves to state `4`, which is another `capture`
state. Traversal finally moves to state `5`, which is a match state for the
pattern with identifier `0`.

### NFA optimization: sparse states

One of the main problems with a Thompson NFA comes from the thing that makes it
a decent choice for a general purpose regex engine: its construction time is
worst case `O(m)`, but this is achieved through liberal use of *epsilon
transitions*. Epsilon transitions are transitions in the NFA that are taken
without consuming any input. They are one of two ways that a search via an NFA
simulation can wind up in multiple states simultaneously. (The other way is for
a single NFA state to have multiple outgoing transitions for the same haystack
symbol.)

Why are espilon transitions a problem? Well, when performing an NFA traversal
(whether it's for a search or for building some other object such as a DFA), an
epsilon transition represents an added cost you must pay whenever one is found.
In particular, every NFA state has something called an epsilon closure, which
is the set of states reachable via following epsilon transitions recursively.
Depending on where it occurs in the NFA, an epsilon closure may be re-computed
many times. The epsilon closure for a particular state may change during
traversal because some epsilon transitions may be conditional, such as the ones
corresponding to anchor assertions like `^`, `$` and `\b`.

Let's take a look at one relatively simple optimization that I made in the
new NFA compiler. First, let's see how `regex <1.9` compiled the regex
`[A-Za-z0-9]`:

```
$ regex-debug compile --bytes '[A-Za-z0-9]'
0000 Save(0) (start)
0001 Split(2, 3)
0002 Bytes(0, 9) (goto: 6)
0003 Split(4, 5)
0004 Bytes(A, Z) (goto: 6)
0005 Bytes(a, z)
0006 Save(1)
0007 Match(0)
```

(Note: `regex-debug` is an older hacky version of a command line tool for
interacting with the `regex` crate. It is no longer available, although you can
always check out an older tag in the `regex` crate repository and build it.)

The `Split` instruction corresponds to an NFA state with two unconditional
epsilon transitions (it's the same as the `binary-union` instruction in the
previous section). The `Save` instruction is for capture groups and `Bytes`
is for checking whether a single byte or a contiguous ranges of bytes matches
the current haystack symbol. In this case, the character class is implemented
with multiple `Split` instructions. Notice, for example, that the epsilon
closure of state `0` is `{1, 2, 3, 4, 5}`.

Now let's see what the new NFA compiler does:

```
$ regex-cli debug thompson --no-table '[A-Za-z0-9]'
thompson::NFA(
>000000: binary-union(2, 1)
 000001: \x00-\xFF => 0
^000002: capture(pid=0, group=0, slot=0) => 3
 000003: sparse(0-9 => 4, A-Z => 4, a-z => 4)
 000004: capture(pid=0, group=0, slot=1) => 5
 000005: MATCH(0)
)
```

(We can ignore states `0` and `1`, the old NFA doesn't have an equivalent
prefix for unrelated reasons.)

Here, instead of epsilon transitions, we just have a single `sparse` NFA
state. The name `sparse` refers to the fact that the state contains more
than one contiguous range of bytes, with each range potentially pointing to
a different state. `sparse` is used because the representation used doesn't
permit a constant time determination of whether a particular haystack symbol
has a matching transition. (i.e., One has to use a linear or binary search.)
It accomplishes the same thing as the `Split` instructions in the old NFA,
but without any explicit epsilon transitions. This results in less overhead
because there's no need to compute an epsilon closure through multiple `Split`
instructions. There's just one state, and finding the next transition requires
looking up which range, if any, the current haystack symbol matches.

The main downside of this particular optimization is that the `sparse` state
(in the current representation of the NFA) uses indirection to support this. So
it may have harmful cache effects and may result in more heap memory used in
some cases. But the overhead of dealing with all of the epsilon transitions, in
practice, tends to trump that. It's possible this indirection will be removed
in the future.

### NFA optimization: minimal UTF-8 automata

One interesting aspect of the old NFA compiler is that it could produce two
different kinds of NFAs: an NFA whose alphabet was defined over Unicode code
points and an NFA whose alphabet was defined over arbitrary bytes. (Not just
UTF-8 code units. Even bytes that can never be a valid UTF-8 code unit,
like `\xFF`, are permitted in this byte oriented NFA.) The Unicode NFA is
principally used when using an NFA regex engine (the PikeVM or the bounded
backtracker, we'll get to those later), where as the byte oriented NFA is
used whenever the lazy DFA engine is used. A byte oriented NFA is required
for use with the lazy DFA because a lazy DFA really wants its alphabet to be
defined over bytes. Otherwise, you wind up with difficult to solve performance
problems. (A byte oriented NFA can be used with the NFA regex engines, but this
only occurs when the regex can match invalid UTF-8. In this case, a Unicode
alphabet cannot be used.)

This led to at least three problems. First is that the byte oriented NFA was
often slower, primarily because of the epsilon transition problem we talked
about in the previous section. That is, a byte oriented NFA usually had more
`Split` instructions where as the Unicode NFA would look more like the `sparse`
state in the new NFA compiler. For example:

```
$ regex-debug compile '[A-Za-z0-9]'
0000 Save(0) (start)
0001 '0'-'9', 'A'-'Z', 'a'-'z'
0002 Save(1)
0003 Match(0)
```

Notice that there are no `Split` instructions at all.

The second problem follows from the first. Since the byte oriented NFA is
usually slower, we would actually compile both a Unicode NFA and a byte
oriented NFA. That way, we could use the Unicode NFA with the NFA regex engines
and the byte oriented NFA with the lazy DFA. It works, but it's wasteful.

The third problem is that the NFA regex engines need to work on both the
Unicode and byte oriented versions of the NFA. This complicates things and has
been the reason for bugs. This problem could likely be mitigated to an extent
with better design, but it's a complication.

So what does this have to do with UTF-8 automata? Well, a byte oriented NFA
still needs to be able to deal with Unicode classes. But if the alphabet is
bytes and not codepoints, how does it do it? It does it by building UTF-8
automata into the NFA. For example, here's an NFA (from the new compiler) that
can match the UTF-8 encoding of any Unicode scalar value:

```
$ regex-cli debug thompson --no-table '(?s:.)'
thompson::NFA(
>000000: binary-union(2, 1)
 000001: \x00-\xFF => 0
^000002: capture(pid=0, group=0, slot=0) => 10
 000003: \x80-\xBF => 11
 000004: \xA0-\xBF => 3
 000005: \x80-\xBF => 3
 000006: \x80-\x9F => 3
 000007: \x90-\xBF => 5
 000008: \x80-\xBF => 5
 000009: \x80-\x8F => 5
 000010: sparse(\x00-\x7F => 11, \xC2-\xDF => 3, \xE0 => 4, \xE1-\xEC => 5, \xED => 6, \xEE-\xEF => 5, \xF0 => 7, \xF1-\xF3 => 8, \xF4 => 9)
 000011: capture(pid=0, group=0, slot=1) => 12
 000012: MATCH(0)
)
```

The way this is achieved is by starting from a contiguous range of Unicode
codepoints and then generating a sequence of byte oriented character classes
that match the UTF-8 encoding of that range of codepoints. This functionality
is provided by [`regex-syntax`'s `utf8` module][utf8-ranges]. So for example,
`(?s:.)` would look like this:

```
[0-7F]
[C2-DF][80-BF]
[E0][A0-BF][80-BF]
[E1-EC][80-BF][80-BF]
[ED][80-9F][80-BF]
[EE-EF][80-BF][80-BF]
[F0][90-BF][80-BF][80-BF]
[F1-F3][80-BF][80-BF][80-BF]
[F4][80-8F][80-BF][80-BF]
```

Translating that into an NFA can be done by just treating it as an alternation,
like this:

```
[0-7F]|[C2-DF][80-BF]|...|[F4][80-8F][80-BF][80-BF]
```

And that will work and it's correct. The problem is that it will generate
very large NFAs for some very common cases. See, the problem with Unicode is
that it tends to introduce extremely large character classes into a regex.
Classes like `\w`, for example, match 139,612 distinct codepoints (at time of
writing). The ASCII version of `\w` only matches 63 codepoints. This is a
categorical difference, and there are plenty of tricks that will work for a
small number like 63 that just won't scale to numbers like 139,612.

The old regex crate did not naively compile UTF-8 automata like the approach
above. Indeed, there is a lot of redundant structure in the classes produced
by the `utf8` module above. The old regex crate noticed this and tried to factor
out common suffixes so that they were shared whenever possible. But this still
led to extremely large NFAs:

```
$ regex-debug compile --bytes '\w' | tail -n20
3545 Bytes(\xb0, \xb0) (goto: 3466)
3546 Bytes(\xf0, \xf0) (goto: 3545)
3547 Split(3550, 3551)
3548 Bytes(\x80, \x8c) (goto: 28)
3549 Bytes(\xb1, \xb1) (goto: 3548)
3550 Bytes(\xf0, \xf0) (goto: 3549)
3551 Split(3554, 3555)
3552 Bytes(\x8d, \x8d) (goto: 2431)
3553 Bytes(\xb1, \xb1) (goto: 3552)
3554 Bytes(\xf0, \xf0) (goto: 3553)
3555 Split(3558, 3562)
3556 Bytes(\x84, \x86) (goto: 28)
3557 Bytes(\xa0, \xa0) (goto: 3556)
3558 Bytes(\xf3, \xf3) (goto: 3557)
3559 Bytes(\x80, \xaf) (goto: 3563)
3560 Bytes(\x87, \x87) (goto: 3559)
3561 Bytes(\xa0, \xa0) (goto: 3560)
3562 Bytes(\xf3, \xf3) (goto: 3561)
3563 Save(1)
3564 Match(0)
```

Notice here that we're only showing the last 20 lines of output. But the NFA
produced has 3,564 states. Wow. And there are epsilon transitions everywhere.
It's truly a mess, and the only reason why the old regex crate does as well as
it does is because the lazy DFA usually bails it out by compiling some subset
of what is actually used into a DFA.

Now let's look at what the new NFA compiler does:

```
$ regex-cli debug thompson --no-table '\w' | tail -n20
 000292: \xB0-\xB9 => 310
 000293: sparse(\x84 => 115, \x85 => 291, \x86 => 210, \xAF => 292)
 000294: \x80-\x9F => 310
 000295: sparse(\x80-\x9A => 5, \x9B => 294, \x9C-\xBF => 5)
 000296: sparse(\x80-\x9B => 5, \x9C => 282, \x9D-\x9F => 5, \xA0 => 55, \xA1-\xBF => 5)
 000297: sparse(\x80-\xA1 => 310, \xB0-\xBF => 310)
 000298: sparse(\x80-\xB9 => 5, \xBA => 297, \xBB-\xBF => 5)
 000299: \x80-\xA0 => 310
 000300: sparse(\x80-\xAE => 5, \xAF => 299)
 000301: \x80-\x9D => 310
 000302: sparse(\xA0-\xA7 => 5, \xA8 => 301)
 000303: sparse(\x80-\x8A => 310, \x90-\xBF => 310)
 000304: sparse(\x80-\x8C => 5, \x8D => 303, \x8E-\xBF => 5)
 000305: sparse(\x80-\x8D => 5, \x8E => 236)
 000306: sparse(\x90 => 193, \x91 => 231, \x92 => 235, \x93 => 238, \x94 => 239, \x96 => 247, \x97 => 118, \x98 => 248, \x9A => 250, \x9B => 256, \x9C => 257, \x9D => 276, \x9E => 290, \x9F => 293, \xA0-\xA9 => 118, \xAA => 295, \xAB => 296, \xAC => 298, \xAD => 118, \xAE => 300, \xAF => 302, \xB0 => 118, \xB1 => 304, \xB2 => 305)
 000307: sparse(\x84-\x86 => 5, \x87 => 236)
 000308: \xA0 => 307
 000309: sparse(0-9 => 310, A-Z => 310, _ => 310, a-z => 310, \xC2 => 3, \xC3 => 4, \xC4-\xCA => 5, \xCB => 6, \xCC => 5, \xCD => 7, \xCE => 8, \xCF => 9, \xD0-\xD1 => 5, \xD2 => 10, \xD3 => 5, \xD4 => 11, \xD5 => 12, \xD6 => 13, \xD7 => 14, \xD8 => 15, \xD9 => 16, \xDA => 5, \xDB => 17, \xDC => 18, \xDD => 19, \xDE => 20, \xDF => 21, \xE0 => 53, \xE1 => 93, \xE2 => 109, \xE3 => 116, \xE4 => 117, \xE5-\xE9 => 118, \xEA => 137, \xEB-\xEC => 118, \xED => 140, \xEF => 155, \xF0 => 306, \xF3 => 308)
 000310: capture(pid=0, group=0, slot=1) => 311
 000311: MATCH(0)
```

There are not only far fewer states, but there are *zero* epsilon transitions.
While this is due in part to the use of the `sparse` state optimization
described in the previous section, it does not account for all of it.

The new NFA compiler achieves this by using [Daciuk's algorithm] for computing
minimal DFAs from a sorted sequence of non-overlapping elements. That's exactly
what we get from the `utf8` module. In practice, we don't necessarily generate
minimal DFAs because of the memory usage required, but instead sacrifice strict
minimality in favor of using a bounded amount of memory. But it's usually close
enough.

The reverse case is not as easy. The reverse case cannot be handled so easily
because there is no simple way to reverse sort the output of the `utf8` module
in a way that works with Daciuk's algorithm (as far as I know). To work around
this, I built a bespoke data structure called a [range trie] that re-partitions
the output of the `utf8` module in reverse such that it's sorted and non-overlapping.
Once this is done, we can use Daciuk's algorithm just like we do for forward
case. The problem, though, is that this can increase the time it takes to build
an NFA quite a bit. Because of that, one needs to opt into it. First, without
the reverse shrinkng:

```
$ regex-cli debug thompson --no-table --no-captures '\w' -r | tail -n20
 001367: \xB1 => 722
 001368: \x80-\x8C => 1367
 001369: \x80-\xBF => 1368
 001370: \x8D => 1367
 001371: \x80-\x8A => 1370
 001372: \x90-\xBF => 1370
 001373: \x8E-\xBF => 1367
 001374: \x80-\xBF => 1373
 001375: \xB2 => 722
 001376: \x80-\x8D => 1375
 001377: \x80-\xBF => 1376
 001378: \x8E => 1375
 001379: \x80-\xAF => 1378
 001380: \xF3 => 1386
 001381: \xA0 => 1380
 001382: \x84-\x86 => 1381
 001383: \x80-\xBF => 1382
 001384: \x87 => 1381
 001385: \x80-\xAF => 1384
 001386: MATCH(0)
```

And now with it:

```
$ regex-cli debug thompson --no-table --no-captures '\w' -r --shrink | tail -n20
 000469: sparse(\x90 => 2, \x92 => 2, \x97 => 2, \xA0-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xB0 => 2, \xB1 => 2, \xE3 => 488, \xE4 => 488, \xE5-\xE9 => 488, \xEA => 488,
 \xEB-\xEC => 488, \xEF => 488)
 000470: sparse(\x97 => 2, \x9A => 2, \xA0-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xB0 => 2, \xB1 => 2, \xE3 => 488, \xE4 => 488, \xE5-\xE9 => 488, \xEA => 488, \xEB-\xEC
=> 488)
 000471: sparse(\x80 => 3, \x81 => 387, \x82 => 6, \x83 => 387, \x84 => 397, \x85 => 451, \x86 => 174, \x87 => 6, \x88 => 100, \x89 => 100, \x8A => 13, \x8B => 348, \x8C => 14, \x8D => 45
2, \x8E => 16, \x8F => 88, \x90 => 19, \x91 => 62, \x92 => 20, \x93 => 343, \x94 => 21, \x95 => 62, \x96 => 23, \x97 => 61, \x98 => 179, \x99 => 27, \x9A => 27, \x9B => 441, \x9C => 446,
\x9D => 236, \x9E => 28, \x9F => 461, \xA0 => 454, \xA1 => 442, \xA2 => 31, \xA3 => 428, \xA4 => 33, \xA5 => 467, \xA6 => 35, \xA7 => 330, \xA8 => 455, \xA9 => 468, \xAA => 388, \xAB => 4
43, \xAC => 43, \xAD => 414, \xAE => 84, \xAF => 447, \xB0 => 438, \xB1 => 416, \xB2 => 363, \xB3 => 457, \xB4 => 67, \xB5 => 340, \xB6 => 199, \xB7 => 141, \xB8 => 465, \xB9 => 374, \xBA
 => 53, \xBB => 417, \xBC => 459, \xBD => 56, \xBE => 469, \xBF => 470, \xC3 => 488, \xC4-\xCA => 488, \xCC => 488, \xCD => 488, \xCE => 488, \xCF => 488, \xD0-\xD1 => 488, \xD2 => 488, \
xD3 => 488, \xD4 => 488, \xD5 => 488, \xD6 => 488, \xD8 => 488, \xD9 => 488, \xDA => 488, \xDC => 488, \xDD => 488, \xDF => 488)
 000472: sparse(\x91 => 2, \x92 => 2, \x93 => 2, \x97 => 2, \x98 => 2, \x9F => 2, \xA0 => 7, \xA1-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xAE => 2, \xB0 => 2, \xB1 => 2, \
xB2 => 2, \xE1 => 488, \xE2 => 488, \xE3 => 488, \xE4 => 488, \xE5-\xE9 => 488, \xEA => 488, \xEB-\xEC => 488, \xED => 488)
 000473: sparse(\x92 => 2, \x94 => 2, \x97 => 2, \x98 => 2, \x9D => 2, \xA0-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xAE => 2, \xB0 => 2, \xB1 => 2, \xE1 => 488, \xE3 => 48
8, \xE4 => 488, \xE5-\xE9 => 488, \xEB-\xEC => 488, \xED => 488)
 000474: sparse(\x91 => 2, \x97 => 2, \x98 => 2, \xA0-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xB0 => 2, \xB1 => 2, \xE2 => 488, \xE3 => 488, \xE4 => 488, \xE5-\xE9 => 488,
 \xEA => 488, \xEB-\xEC => 488, \xEF => 488)
 000475: sparse(\x90 => 2, \x91 => 2, \x96 => 2, \x97 => 2, \x9C => 2, \xA0-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xB0 => 2, \xB1 => 2, \xE0 => 488, \xE1 => 488, \xE3 =>
488, \xE4 => 488, \xE5-\xE9 => 488, \xEA => 488, \xEB-\xEC => 488)
 000476: sparse(\x90 => 2, \x96 => 2, \x97 => 2, \xA0-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xB0 => 2, \xB1 => 2, \xE0 => 488, \xE3 => 488, \xE4 => 488, \xE5-\xE9 => 488,
 \xEA => 488, \xEB-\xEC => 488, \xEF => 488)
 000477: sparse(\x80 => 218, \x81 => 387, \x82 => 6, \x83 => 387, \x84 => 472, \x85 => 451, \x86 => 174, \x87 => 387, \x88 => 12, \x89 => 100, \x8A => 13, \x8B => 348, \x8C => 14, \x8D =>
 452, \x8E => 16, \x8F => 426, \x90 => 19, \x91 => 62, \x92 => 20, \x93 => 473, \x94 => 21, \x95 => 62, \x96 => 23, \x97 => 61, \x98 => 179, \x99 => 157, \x9A => 27, \x9B => 441, \x9C =>
446, \x9D => 236, \x9E => 28, \x9F => 461, \xA0 => 454, \xA1 => 442, \xA2 => 31, \xA3 => 428, \xA4 => 33, \xA5 => 467, \xA6 => 305, \xA7 => 317, \xA8 => 463, \xA9 => 468, \xAA => 388, \xA
B => 443, \xAC => 223, \xAD => 414, \xAE => 43, \xAF => 447, \xB0 => 438, \xB1 => 474, \xB2 => 363, \xB3 => 457, \xB4 => 140, \xB5 => 340, \xB6 => 266, \xB7 => 141, \xB8 => 465, \xB9 => 2
01, \xBA => 108, \xBB => 417, \xBC => 475, \xBD => 476, \xBE => 77, \xBF => 470, \xC3 => 488, \xC4-\xCA => 488, \xCC => 488, \xCE => 488, \xCF => 488, \xD0-\xD1 => 488, \xD2 => 488, \xD3
=> 488, \xD4 => 488, \xD5 => 488, \xD8 => 488, \xD9 => 488, \xDA => 488, \xDC => 488, \xDD => 488)
 000478: sparse(\x97 => 2, \x9D => 2, \xA0-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xAE => 2, \xB0 => 2, \xB1 => 2, \xE3 => 488, \xE4 => 488, \xE5-\xE9 => 488, \xEA => 488,
 \xEB-\xEC => 488)
 000479: sparse(\x91 => 2, \x96 => 2, \x97 => 2, \x98 => 2, \xA0-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xAE => 2, \xAF => 2, \xB0 => 2, \xB1 => 2, \xE0 => 488, \xE3 => 48
8, \xE4 => 488, \xE5-\xE9 => 488, \xEA => 488, \xEB-\xEC => 488)
 000480: sparse(\x96 => 2, \x97 => 2, \x98 => 2, \xA0-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xAE => 2, \xAF => 2, \xB0 => 2, \xB1 => 2, \xE3 => 488, \xE4 => 488, \xE5-\xE
9 => 488, \xEB-\xEC => 488, \xEF => 488)
 000481: sparse(\x90 => 2, \x97 => 2, \x98 => 2, \x9D => 2, \xA0-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xAE => 2, \xB0 => 2, \xB1 => 2, \xE0 => 488, \xE1 => 488, \xE3 =>
488, \xE4 => 488, \xE5-\xE9 => 488, \xEB-\xEC => 488, \xEF => 488)
 000482: sparse(\x91 => 2, \x97 => 2, \x98 => 2, \xA0-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xAE => 2, \xB0 => 2, \xB1 => 2, \xE0 => 488, \xE1 => 488, \xE3 => 488, \xE4 =
> 488, \xE5-\xE9 => 488, \xEA => 488, \xEB-\xEC => 488, \xEF => 488)
 000483: sparse(\x91 => 2, \x97 => 2, \xA0-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xB0 => 2, \xB1 => 2, \xE0 => 488, \xE1 => 488, \xE2 => 488, \xE3 => 488, \xE4 => 488, \x
E5-\xE9 => 488, \xEA => 488, \xEB-\xEC => 488)
 000484: sparse(\x91 => 2, \x97 => 2, \xA0-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xB0 => 2, \xB1 => 2, \xE0 => 488, \xE1 => 488, \xE2 => 488, \xE3 => 488, \xE4 => 488, \x
E5-\xE9 => 488, \xEA => 488, \xEB-\xEC => 488, \xEF => 488)
 000485: sparse(\x97 => 2, \xA0-\xA9 => 2, \xAA => 2, \xAB => 2, \xAC => 2, \xAD => 2, \xB0 => 2, \xB1 => 2, \xE3 => 488, \xE4 => 488, \xE5-\xE9 => 488, \xEA => 488, \xEB-\xEC => 488)
 000486: sparse(\x80 => 4, \x81 => 396, \x82 => 6, \x83 => 387, \x84 => 472, \x85 => 451, \x86 => 174, \x87 => 387, \x88 => 12, \x89 => 100, \x8A => 195, \x8B => 348, \x8C => 14, \x8D =>
452, \x8E => 16, \x8F => 426, \x90 => 19, \x91 => 62, \x92 => 20, \x93 => 473, \x94 => 114, \x95 => 62, \x96 => 23, \x97 => 61, \x98 => 179, \x99 => 27, \x9A => 27, \x9B => 441, \x9C => 4
46, \x9D => 236, \x9E => 28, \x9F => 478, \xA0 => 462, \xA1 => 442, \xA2 => 31, \xA3 => 479, \xA4 => 33, \xA5 => 467, \xA6 => 305, \xA7 => 480, \xA8 => 481, \xA9 => 399, \xAA => 482, \xAB
 => 443, \xAC => 43, \xAD => 414, \xAE => 43, \xAF => 447, \xB0 => 438, \xB1 => 474, \xB2 => 363, \xB3 => 457, \xB4 => 483, \xB5 => 484, \xB6 => 108, \xB7 => 141, \xB8 => 465, \xB9 => 374
, \xBA => 108, \xBB => 417, \xBC => 71, \xBD => 476, \xBE => 57, \xBF => 485, \xC3 => 488, \xC4-\xCA => 488, \xCC => 488, \xCD => 488, \xCE => 488, \xCF => 488, \xD0-\xD1 => 488, \xD2 =>
488, \xD3 => 488, \xD4 => 488, \xD5 => 488, \xD6 => 488, \xD8 => 488, \xD9 => 488, \xDA => 488, \xDB => 488, \xDC => 488, \xDD => 488)
^000487: sparse(0-9 => 488, A-Z => 488, _ => 488, a-z => 488, \x80 => 58, \x81 => 72, \x82 => 78, \x83 => 86, \x84 => 96, \x85 => 111, \x86 => 123, \x87 => 136, \x88 => 143, \x89 => 153,
\x8A => 165, \x8B => 172, \x8C => 177, \x8D => 186, \x8E => 194, \x8F => 202, \x90 => 217, \x91 => 222, \x92 => 224, \x93 => 227, \x94 => 233, \x95 => 238, \x96 => 244, \x97 => 251, \x98
=> 257, \x99 => 258, \x9A => 269, \x9B => 274, \x9C => 279, \x9D => 285, \x9E => 295, \x9F => 298, \xA0 => 312, \xA1 => 315, \xA2 => 320, \xA3 => 322, \xA4 => 328, \xA5 => 333, \xA6 => 33
8, \xA7 => 341, \xA8 => 345, \xA9 => 351, \xAA => 360, \xAB => 365, \xAC => 368, \xAD => 375, \xAE => 383, \xAF => 385, \xB0 => 395, \xB1 => 402, \xB2 => 408, \xB3 => 411, \xB4 => 418, \x
B5 => 425, \xB6 => 429, \xB7 => 435, \xB8 => 440, \xB9 => 444, \xBA => 450, \xBB => 460, \xBC => 466, \xBD => 471, \xBE => 477, \xBF => 486)
 000488: MATCH(0)
```

So shrinking in the reverse case still helps quite a bit in terms of generating
tighter NFAs with fewer states. But because of the extra compile time hit,
it is currently disabled by default. Therefore, the minimal UTF-8 automata
optimization only applies to forward NFAs. (Reverse NFAs are created for use
with DFAs, since a DFA requires a reverse scan to find the start of each
match.) However, we do still look for redundant suffixes and share them in the
reverse case when this extra NFA shrinking isn't enabled.

### NFA optimization: literal trie

If you haven't noticed a theme by now, one of the biggest problems with a
Thompson NFA is its epsilon transitions. It is really the critical thing about
a Thompson NFA that makes it scale poorly with respect to the size of a regex.
This is why, when using Thompson based regex engines, increasing the size
of the regex can impact search times. Because of that, Thompson based regex
engines often (but not always) have alternative engines that mitigate this
weakness in one way or another. For example, by using a lazy DFA. However, a
lazy DFA cannot be used in every circumstance, and even a lazy DFA can become
overwhelmed by a large enough regex.

So in this section, we'll talk about another NFA optimization that works to
reduce epsilon transitions. In this case, we're going to be limiting ourselves
to an alternation of literals. Let's take a look at an example using the old
NFA compiler:

```
$ regex-debug compile 'zap|z|zapper'
0000 Save(0) (start)
0001 Split(2, 5)
0002 'z'
0003 'a'
0004 'p' (goto: 13)
0005 Split(6, 7)
0006 'z' (goto: 13)
0007 'z'
0008 'a'
0009 'p'
0010 'p'
0011 'e'
0012 'r'
0013 Save(1)
0014 Match(0)
```

Here, we're building an NFA for the regex `zap|z|zapper`. The way it's compiled
is with nested `Split` instructions. It starts with a `Split(2, 5)` that points
to the beginning of `zap` and another `Split(6, 7)` instruction. This second
instruction then points to the beginning of `z` and `zapper`. So for this
regex, when looking for a match, the epsilon closure of all these splits is
enumerated for every character in the haystack.

In contrast, let's look at what the new NFA compiler does:

```
$ regex-cli debug thompson --no-table 'zap|z|zapper'
thompson::NFA(
>000000: binary-union(2, 1)
 000001: \x00-\xFF => 0
^000002: capture(pid=0, group=0, slot=0) => 11
 000003: p => 12
 000004: a => 3
 000005: r => 12
 000006: e => 5
 000007: p => 6
 000008: p => 7
 000009: a => 8
 000010: union(4, 12, 9)
 000011: z => 10
 000012: capture(pid=0, group=0, slot=1) => 13
 000013: MATCH(0)
```

(Again, ignore states `0` and `1`, which correspond to the optional unanchored
`(?s-u:.)*?` prefix. The old NFA compiler doesn't emit those states for
unrelated reasons.)

Here, before an epsilon transition is seen at all, a `z` must first be matched
in state `11`. Only after that is the `union(4, 12, 9)` state seen. (This
`union` is equivalent to the nested `Split` instructions in the old NFA
compiler, but combines them all into one state.) If this NFA were used for a
search, one wouldn't need to compute a beefy epsilon closure for every byte in
the haystack. One would only need to do it after a `z` byte is seen, which is
much more rare. In effect, the regex was rewritten to `z(?:ap||apper)`.

So what's going on here? In this particular case, it almost looks like a common
prefix has been factored out. But the optimization at work here is a bit more
general than that. Consider the regex `abc|xyz`. There are no common prefixes
here. First, let's see what the old NFA compiler does:

```
$ regex-debug compile 'abc|xyz'
0000 Save(0) (start)
0001 Split(2, 5)
0002 'a'
0003 'b'
0004 'c' (goto: 8)
0005 'x'
0006 'y'
0007 'z'
0008 Save(1)
0009 Match(0)
```

Here, we see a `Split` instruction again that forks out to the start of `abc`
and then `xyz`.

Now the new NFA compiler:

```
$ regex-cli debug thompson --no-table 'abc|xyz'
thompson::NFA(
>000000: binary-union(2, 1)
 000001: \x00-\xFF => 0
^000002: capture(pid=0, group=0, slot=0) => 7
 000003: c => 8
 000004: b => 3
 000005: z => 8
 000006: y => 5
 000007: sparse(a => 4, x => 6)
 000008: capture(pid=0, group=0, slot=1) => 9
 000009: MATCH(0)
)
```

Here there are no epsilon transitions at all. The `a` and `x` have been lifted
out into the same sparse state, with each them forking off to their respective
suffixes, `bc` and `yz`.

What's happening here is that the NFA compiler is recognizing an alternation
of literals, [compiling it into a trie][literal trie], and then converting
that trie to an NFA directly in a way that minimizes epsilon transitions. The
key trick to this optimization is ensuring that leftmost-first semantics are
preserved. For example, in the `zap|z|zapper` example above, one might be
tempted to rewrite it as `z(?:ap(?:per)?)?`. But this does not have the same
matches! This regex will match `zapper` in the haystack `zapper`, but the
original `zap|z|zapper` will match `zap`. The literal trie achieves this by
partitioning the transitions in each trie state into chunks, where a chunk is
created whenever a match (of one of the literals) is seen. If a normal trie was
created, then the preference order required by leftmost-first semantics would
be lost when translating the trie back to an NFA.

### NFA future work

There are two aspects I'd like to explore for future work on NFAs.

First is the [Glushkov NFA]. A Glushkov NFA has a worse time complexity
for compilation, but it comes with the advantage of not having any epsilon
transitions. (Instead, it is an NFA by virtue of permitting a state to have
multiple transitions defined for the same haystack symbol.) Because of the
worse compilation time complexity, a Glushkov NFA probably can't be used in
every case, but it's certainly plausible to use it for a subset of smaller
regexes. A Glushkov NFA is possibly more amenable to bit-parallel techniques
that are sadly underused in the `regex` crate at present. One of the big
questions marks for me here is how well a Glushkov NFA will fair with big
Unicode classes. (Perhaps such things will be a disqualifiying criterion.)

Second is storing an NFA in a single contiguous allocation. This might make
access patterns faster and more cache friendly, but perhaps more importantly,
it could permit zero-copy serialization and deserialization. The downside of
doing this is code complexity and potentially more use of `unsafe`, but there
are some potentially nice benefits too.

## Regex engines

Like [RE2], the `regex` crate is internally composed of several different regex
engines. Most of them have already been mentioned so far. In this section, we
will do a bit of a deeper dive on each of them and explain why they exist.
Finally, we'll wrap up by exploring the meta regex engine, which combines all
of the regex engines into a single engine.

But why have so many regex engines? The reason is essentially engineering: the
implementation of regex engines with more functionality tends to search more
slowly than regex engines that have less functionality. We could use only a
single regex engine that supports all the functionality we want, namly the
[`PikeVM`], but the search performance would likely be disappointing to a lot
of users. This fundamentally drives the addition of other engines. None of
the other engines can support the full range of functionality provided by the
`regex` crate, and so buck stops with the `PikeVM`.

In addition to using `regex-cli` to show how to run each regex engine, we'll
also look at short example Rust programs. To follow along with the Rust
program examples, you'll want to setup a Cargo project and add `regex-automata`
as a dependency:

```
$ cargo init --bin
$ cargo add 'regex-automata@0.3'
```

Then you can edit `src/main.rs` to add source code and run the program with
`cargo run`.

### Common elements among regex engines

While there are many regex engines in the `regex-automata` crate, all of them
share very similar APIs. Because of that, it's worth covering a few of those
common elements first.

The three most important types are [`Input`], [`Match`] and [`MatchError`].

`Input` is a small abstraction for setting the parameters of a single search.
Its only required parameter is a haystack, that is, the sequence of bytes to
search. Most search APIs accept anything that implements the `Into<Input>`
trait, and both `&[u8]` and `&str` implement `Into<Input>`. Optional parameters
consist of the span of the haystack to search, whether to execute an anchored
search and whether to stop the search early as soon as a match is found instead
of greedily trying to match as much as possible.

`Match` represents the data reported whenever a match is found in a
haystack. The data consists of two elements. The first is the span of
byte offsets in the haystack where the match was found. The second is the
[`PatternID`] corresponding to the pattern that matched. (Every regex engine
in `regex-automata` has first class multi-pattern support. Pattern IDs are
assigned, starting from zero, in an auto-incrementing fashion based on the
order of the patterns given to the regex engine constructor.)

`MatchError` represents an error that occurred during a search. When an error
occurs, it is not possible to determine whether a match exists or not. That is,
the result is indeterminate. For this reason, many of the basic search APIs
have a return type of `Result<Option<Match>, MatchError>`. Errors can occur
during a search for a variety of reasons. For example, a DFA can be configured
to quit immediately whenever a certain byte is seen. Another example is the
[`BoundedBacktracker`], which will fail if the length of the haystack exceeds a
configured limit. One of the main features of the [meta regex engine], as we'll
discuss later, is providing a facade on top of a composition of regex engines
that never results in an error being returned to the caller.

There are some other themes common to most regex engines. For example, the
construction of most engines is done by a `Builder` and configured by one or
more `Config` values. We'll talk about these more as they come up. See also the
[API themes] section in the `regex-automata` crate documentation.

### Engine: Pike VM

As mentioned above, the buck stops with the [`PikeVM`]. That is, the `PikeVM`
supports the full suite of regex functionality that one can parse with
`regex-syntax`, and it supports this for any haystack of any length. Other
regex engines have various limitations. For example:

* The `BoundedBacktracker` only works in cases where `len(haystack) *
len(regex)` is below a configured limit. In practice, this often means one can
only use it with short haystacks.
* The one-pass DFA only works a small subset of NFAs that satisfy the
"one-pass" criterion.
* The lazy DFA and full DFA cannot match Unicode word boundaries. Both have
heuristics for treating a Unicode word boundary as an ASCII word boundary when
the haystack only consists of ASCII, but if a non-ASCII byte is seen, both can
quit and return an error instead of completing the search.

Other than a [`Cache`][pikevm-cache] value, a `PikeVM` can be built directly
from an NFA without any additional work. That is, its search works by a
"simulation" of the NFA itself. (Confusingly, this is sometimes referred to
as the "DFA algorithm.") The actual implementation is structured similarly to
a virtual machine, with each NFA state acting as an instruction. The `PikeVM`
works by moving from NFA state to the next, and computing epsilon closures
on the fly. Since it's possible to be in multiple NFA states simultaneously,
the `PikeVM` keeps track of every active state. The transition function then
applies to each of those states. The `PikeVM` also keeps track of capture group
positions.

The main problem with the `PikeVM` is performance, and its poor performance is
primarily rooted in having to keep track of so much state. The capture group
positions are required to report the start and end match offsets, while the
currently active states must be tracked in order to guarantee worst case
`O(m * n)` time. That is, in contrast to a backtracking approach, the `PikeVM`
visits each byte in the haystack at most a constant number of times, and it
does so by computing all possible active states in lock-step. This adds quite
a bit of overhead, and it can be exacerbated by regexes with a lot of epsilon
transitions. (This is one of the reasons why, earlier in this blog, we spent so
much time talking about optimizations in the NFA to elide epsilon transitions.)

Now that we've talked a little about how the `PikeVM` works, let's look at a
few examples. Here's a lightly annotated Rust program showing how to use it for
a search:

```rust
use regex_automata::{nfa::thompson::pikevm::PikeVM, Match};

fn main() {
    // This creates a PikeVM directly from a pattern string. But one can also
    // build a PikeVM directly from an NFA using `PikeVM::builder()`.
    let re = PikeVM::new(r"\b\w+\b").unwrap();
    // Most regex engines in regex-automata require some kind of mutable
    // scratch space that can be written to during a search. The meta regex
    // engine has APIs that hide this fact from you, but when you use the
    // underlying regex engines directly, you must create and pass around these
    // cache values explicitly.
    let mut cache = re.create_cache();

    let mut it = re.find_iter(&mut cache, "Σέρλοκ Χολμς");
    assert_eq!(Some(Match::must(0, 0..12)), it.next());
    assert_eq!(Some(Match::must(0, 13..23)), it.next());
    assert_eq!(None, it.next());
}
```

And the equivalent `regex-cli` command:

```
$ regex-cli find match pikevm --no-table -p '\b\w+\b' -y 'Σέρλοκ Χολμς'
0:0:12:Σέρλοκ
0:13:23:Χολμς
```

Notice the use of Unicode word boundaries on a non-ASCII haystack. Only the
`PikeVM`, `BoundedBacktracker` and one-pass DFA support this. The lazy DFA
and fully compiled DFA would return an error in this case.

The other important thing to notice here is that the search APIs do not
return an error. Indeed, the `PikeVM` can never return an error under any
circumstances (nor will it ever panic). This is actually a unique property
among the regex engines in `regex-automata`. Every other regex engine can
return an error during a search for one reason or another.

We can also make use of multi-pattern support with capture groups
simultaneously. (This is something that the `regex` crate cannot do, and many
have requested this support from its `RegexSet` API. That still doesn't exist,
but you can now at least drop down to `regex-automata` and do it. This same
example also works with the meta regex engine.)

```rust
use regex_automata::nfa::thompson::pikevm::PikeVM;

fn main() {
    let re = PikeVM::new_many(&[
        r"(?<email>[.\w]+@(?<domain>[.\w]+))",
        r"(?<phone>(?<areacode>[0-9]{3})-[0-9]{3}-[0-9]{4})",
    ])
    .unwrap();
    let mut cache = re.create_cache();

    let hay = "foo@example.com, 111-867-5309";
    let mut it = re.captures_iter(&mut cache, hay);

    let caps = it.next().unwrap();
    assert_eq!(0, caps.pattern().unwrap().as_usize());
    assert_eq!("example.com", &hay[caps.get_group_by_name("domain").unwrap()]);

    let caps = it.next().unwrap();
    assert_eq!(1, caps.pattern().unwrap().as_usize());
    assert_eq!("111", &hay[caps.get_group_by_name("areacode").unwrap()]);

    assert!(it.next().is_none());
}
```

And the equivalent `regex-cli` command:

```
$ regex-cli find capture pikevm --no-table \
   -p '(?<email>[.\w]+@(?<domain>[.\w]+))' \
   -p '(?<phone>(?<areacode>[0-9]{3})-[0-9]{3}-[0-9]{4})' \
   -y 'foo@example.com, 111-867-5309'
0:{ 0: 0..15/foo@example.com, 1/email: 0..15/foo@example.com, 2/domain: 4..15/example.com }
1:{ 0: 17..29/111-867-5309, 1/phone: 17..29/111-867-5309, 2/areacode: 17..20/111 }
```

Notice how the capture groups are different for each pattern. The caller is
responsible for using the correct capture group name based on which pattern
matches.

### Engine: bounded backtracker

The [`BoundedBacktracker`] uses a backtracking algorithm to execute a search
using a Thompson NFA directly. This is sometimes also (confusingly) referred
to as the "NFA algorithm." The key difference between the backtracking
implementation in `regex-automata` and most other implementations is that
it uses additional state to avoid re-tracing steps already taken during
backtracking. This allows us to guarantee worst case `O(m * n)` time, but at
the expense of `O(m * n)` space.

(Classical backtracking is also technically bounded theoretically, but the
"bounded" in the name "bounded backtracker" refers to the explicit bound used
in the implementation to guarantee worst case `O(m * n)` time.)

The benefit of a the bounded backtracker is purely that it is usually faster
than the `PikeVM`. In rough experiments, it's usually about twice as fast.

Here's a quick example, like the `PikeVM` example previously, but using the
bounded backtracker:

```rust
use regex_automata::{nfa::thompson::backtrack::BoundedBacktracker, Match};

fn main() {
    let re = BoundedBacktracker::new(r"\b\w+\b").unwrap();
    // A bounded backtracker needs a cache just like the PikeVM. The Cache
    // keeps track of work already done, and also contains scratch space for
    // backtracking's call stack, which is stored on the heap.
    let mut cache = re.create_cache();

    // Unlike the PikeVM, the bounded backtracker can fail to run a search.
    // This occurs when `len(regex) * len(haystack)` exceeds the configured
    // visited capacity. We'll see an example of this below.
    let mut it = re.try_find_iter(&mut cache, "Σέρλοκ Χολμς");
    assert_eq!(Some(Ok(Match::must(0, 0..12))), it.next());
    assert_eq!(Some(Ok(Match::must(0, 13..23))), it.next());
    assert_eq!(None, it.next());
}
```

And the equivalent `regex-cli` command:

```
$ regex-cli find match backtrack --no-table -p '\b\w+\b' -y 'Σέρλοκ Χολμς'
0:0:12:Σέρλοκ
0:13:23:Χολμς
```

This exame should look almost identical to the `PikeVM` example. One important
difference is that instead of calling `re.find_iter(..)`, we instead called
`re.try_find_iter(..)`. Namely, as mentioned above, the bounded backtracker can
return an error when a search would require more memory than what is configured.
The relevant configuration knob is [`Config::visited_capacity`]. We can also
query just how big of a haystack a bounded backtracker can search without
failing:

```rust
use regex_automata::nfa::thompson::backtrack::BoundedBacktracker;

fn main() {
    let re = BoundedBacktracker::new(r"\b\w+\b").unwrap();
    println!("{:?}", re.max_haystack_len());
}
```

At the time of writing, the output of this program on my machine is `6635`.
That might seem pretty short, and that's because the regex is perhaps bigger
than you might think it is. Namely, since `\w` is Unicode-aware by default,
`\w` matches over 100,000 distinct codepoints. While we could increase the
maximum haystack length that we could search by setting a bigger value for
`Config::visited_capacity`, we can *also* increase it by decreasing the size of
our regex by disabling Unicode mode:

```rust
use regex_automata::nfa::thompson::backtrack::BoundedBacktracker;

fn main() {
    let re = BoundedBacktracker::new(r"\b\w+\b").unwrap();
    println!("{:?}", re.max_haystack_len());
}
```

The output of this program on my machine is now `233015`. That's nearly two
orders of magnitude difference!

Overall, when possible, one should prefer using the bounded backtracker over
the `PikeVM`. They both have the same time complexity guarantees, but the
bounded backtracker tends to be faster in practice.

### Engine: one-pass DFA

Before talking about the [one-pass DFA], it makes sense to motivate its
existence in a bit more detail. Namely, one important aspect of both the
`PikeVM` and the bounded backtracker is that they support reporting the offsets
of matching capture groups in the pattern. For example, using the `PikeVM` with
`regex-cli`:

```
$ regex-cli find capture pikevm --no-table \
   -p '(?<year>[0-9]{4})-(?<month>[0-9]{2})-(?<day>[0-9]{2})' \
   -y '2023-07-02'
0:{ 0: 0..10/2023-07-02, 1/year: 0..4/2023, 2/month: 5..7/07, 3/day: 8..10/02 }
```

Capture groups are quite useful because they permit de-composing a regex match
down into constituent parts that are independently useful. As in the above
example, we didn't _just_ match a date, we matched the individual components
of that date and made each of those components easily available via APIs. For
example, using the `regex` crate itself:

```rust
use regex::Regex;

fn main() {
    let pat = r"(?<year>[0-9]{4})-(?<month>[0-9]{2})-(?<day>[0-9]{2})";
    let re = Regex::new(pat).unwrap();
    let Some(caps) = re.captures("2023-07-02") else { return };
    println!(" year: {:?}", &caps["year"]);
    println!("month: {:?}", &caps["month"]);
    println!("  day: {:?}", &caps["day"]);
}
```

Without capture groups, regexes become a lot less convenient.

The problem with capture groups is that they aren't something that cleanly fit
into the theoretical model of regular languages and finite automata. (They
require something more expressive known as [tagged finite automata].) As a
result, capture groups are bolted on to the classical NFA simulation, and
the result is named `PikeVM`. Capture groups are also part of the classic
"NFA algorithm" or backtracking, as the matching offsets of each group can be
recorded as the backtracking search progresses through the haystack.

But that's generally where capture groups stop being supported. DFAs simply
do not support them, and there is no obvious way to make them support capture
groups in general without evolving to something like tagged finite automata.

However, there is one case where we can bolt capture groups into something that
executes like a DFA: a one-pass NFA. One can think of a one-pass NFA as an NFA
that can be converted into a DFA where each DFA state maps to at most one NFA
state. That is, when performing a search using an NFA simulation, then at each
possible character in the haystack there is at most one NFA state to transition
to.

The intuition behind this special case is that only one copy of the matching
capture groups needs to be kept. (In the `PikeVM`, there are up to `len(regex)`
copies of capture groups kept, as there is no way to know which capture groups
will wind up being part of the final match.) If one can detect this case, then
a new DFA can be constructed from the NFA in the linear time, and this DFA can
execute a search such that a constant number of CPU instructions are used to
process each character in the haystack.

The end result of this is a [one-pass DFA] and it generally runs quite a bit
faster than either the `PikeVM` or the bounded backtracker. In other words, it
represents the fastest way to report the offsets of matching capture groups in
the `regex` crate.

The problem with a one-pass DFA is that, as a DFA, it uses a lot more memory.
(The memory problem is mitigated by giving one-pass DFA construction a
configurable fixed budget of memory, and if it's exceeded, one-pass DFA
construction fails.) Additionally, many regexes are not one-pass. For example,
all unanchored searches in the `regex` crate are done by adding an implicit
`(?s-u:.)*?` prefix to the beginning of the regex. That prefix is itself not
one-pass when followed by any non-empty regex. Therefore, a one-pass DFA only
supports anchored searches.

The "only anchored" search limitation might make it seem like the one-pass
DFA has very limited utility, but as we'll see in more detail, the meta regex
engine uses anchored searches quite a bit even if the original regex itself
isn't anchored. This can occur when the caller asked for the offsets of
matching capture groups. The meta regex engine starts by looking for an overall
match using a DFA engine, and then once a match is found, an anchored search
is used on only the matched part of the haystack to report offsets for each
matching capture group. In this way, the utility of the one-pass DFA is quite
high.

Using the one-pass DFA directly is possible, and it looks similar to past
examples but with some small deviations because of the one-pass DFA's more
limited API:

```rust
use regex_automata::{dfa::onepass::DFA, Match};

fn main() {
    let re = DFA::new(r"\b\w+\b").unwrap();
    let mut cache = re.create_cache();

    // A one-pass DFA doesn't expose any iterator APIs directly because it only
    // supports anchored matches. Thus, any iterator that would be returned
    // would only support adjacent matches. Such a thing is a valid use case,
    // but it wouldn't match the semantics of every other iterator in
    // regex-automata.
    let Some(m) = re.find(&mut cache, "Σέρλοκ Χολμς") else { return };
    assert_eq!(Match::must(0, 0..12), m);
}
```

And the equivalent `regex-cli` command:

```
$ regex-cli find match onepass --no-table --anchored \
   -p '\b\w+\b' \
   -y 'Σέρλοκ Χολμς'
0:0:12:Σέρλοκ
```

Notice how we pass the `--anchored` flag to `regex-cli`. Without it, the
one-pass DFA search would return an error.

We can execute multiple searches as well. Even though the regex itself isn't
anchored, we don't have to limit ourselves to searches beginning at offset `0`:

```rust
use regex_automata::{dfa::onepass::DFA, Input, Match};

fn main() {
    let hay = "Σέρλοκ Χολμς";
    let re = DFA::new(r"\b\w+\b").unwrap();
    let mut cache = re.create_cache();

    let Some(m) = re.find(&mut cache, hay) else { return };
    assert_eq!(Match::must(0, 0..12), m);

    let input = Input::new(hay).range(13..);
    let Some(m) = re.find(&mut cache, input) else { return };
    assert_eq!(Match::must(0, 13..23), m);
}
```

The `Input` abstraction can be used in the same way with any other regex engine
as well.

It can be quite tricky to reason about whether a particular regex is one-pass
or not. For example, a regex can be one-pass when Unicode is disabled and not
one-pass when Unicode is enabled. For example:

```
$ regex-cli find match onepass --no-table --anchored \
   -p '\w+\s+\w+' \
   -y 'Σέρλοκ Χολμς'
failed to compile onepass DFA: one-pass DFA could not be built because pattern is not one-pass: conflicting transition
```

But if we disable Unicode mode, then the regex becomes one-pass:

```
$ regex-cli find match onepass --no-table --anchored \
   -p '(?-u)\w+\s+\w+' \
   -y 'Sherlock Holmes'
0:0:15:Sherlock\x20Holmes
```

This happens because, when Unicode mode is enabled, `\w` and `\s` partially
overlap. They of course do not overlap _logically_, as taking the codepoint
sets from each of `\w` and `\s` and intersecting them would produce the empty
set:

```
$ regex-cli debug hir --no-table '[\w&&\s]'
Class(
    {},
)
```

The overlap is actually at the byte level, and the byte level is how the
transitions in a one-pass DFA are defined. This overlap means that the DFA for
`\w+\s+\w+` when Unicode mode is enabled doesn't satisfy the property that
every state in the DFA maps to at most one NFA state. That is, there exists
codepoints in both `\w` and `\s` which start with the same leading UTF-8 code
units.

But when Unicode mode is disabled, not only do the codepoint sets not have any
overlap, but they don't have any overlap at the byte level either. Why? Because
the codepoints in `\w` and `\s` when Unicode is disabled are limited to ASCII
codepoints, and each ASCII codepoint is always encoded as a single UTF-8 code
unit corresponding to the ASCII codepoint number.

One should prefer a one-pass DFA over both the `PikeVM` and the bounded
backtracker because it is faster, although it can take longer to build and may
use more memory. However, because it can only be built from a very limited set
of regexes, one must be ready to deal with construction failing and falling
back to a different engine.

### Engine: DFA

The [DFA regex engine] is made up of two [dense DFAs][dense DFA]. One DFA is
responsible for a forward scan that finds the end of a match and the other DFA
is used to perform an anchored scan backwards from the end of a match to find
the start of a match. (This second DFA is built by reversing the concatenations
in an [`Hir`], building an NFA from that and then determinizing that reverse
NFA into a DFA.) We call these DFAs "dense" to distinguish them from [sparse
DFAs][sparse DFA]. A dense DFA uses a representation that optimizes for
search speed at the expense of more memory usage, while a sparse DFA uses a
representation that optimizes for less memory usage at the expense of search
speed.

Fully compiled DFAs are usually not found in general purpose regex engines
because building them has worst case `O(2^m)` time and space (where `m` is
proportional to `len(regex)`). For example, `[01]*1[01]{N}` compiles to an NFA
with approximately `N` states, and as `N` grows, the NFA grows linearly. But
the corresponding DFA has approximately `2^N` states, and as the DFA grows, the
number of states grows exponentially.

But the problem with a DFA is not just limited to its theoretical worst case
behavior. DFAs, especially dense DFAs, tend to use a lot of memory because each
state supports computing the next transition for any byte in constant time.
This fundamentally requires more memory to provide constant time random access.
When you combine this with large Unicode character classes, the result can be
disastrous. For example, let's compare some sizes for the regex `\w`. First
up is the NFA (which, remember, can be used directly as a regex engine in the
case of the `PikeVM` and bounded backtracker):

```
$ regex-cli debug thompson '\w' -q
[.. snip ..]
            memory:  17668
[.. snip ..]
```

So here, the NFA uses 17KB. That's not exactly small, but watch what happens
when we determinize the NFA into a DFA:

```
$ regex-cli debug dense dfa '\w' --start-kind unanchored -q
[.. snip ..]
          memory:  159296
[.. snip ..]
```

Memory balloons to about 160KB! (I've limited the DFA to just an unanchored
DFA. If one used `--start-kind both` instead, the default, then memory usage
would double.) And spending extra time to minimize the DFA doesn't help:

```
$ regex-cli debug dense dfa '\w' --start-kind unanchored --minimize -q
[.. snip ..]
          memory:  159296
[.. snip ..]
```

Sometimes minimization helps, but in this case, since we used [Daciuk's
algorithm] to build a minimal UTF-8 automaton into the NFA for `\w`, it follows
that determinizing that NFA into a DFA itself already results in a minimal DFA.
The real problem here is a result of our dense representation and the fact that
our alphabet is defined over bytes. We can make it a little better by switching
to a sparse representation:

```
$ regex-cli debug sparse dfa '\w' --start-kind unanchored -q
[.. snip ..]
          memory:  102257
[.. snip ..]
```

But we're still at over 100KB. Unicode character classes and fully compiled
DFAs just don't mix well. And in practice, it's often the case that one doesn't
need the full class compiled since it's somewhat rare to search haystacks with
a lot of difference scripts. More to the point, most searches are probably fine
with just the ASCII definition of `\w`, which is much smaller:

```
$ regex-cli debug thompson '(?-u)\w' -q
[.. snip ..]
            memory:  732
[.. snip ..]

$ regex-cli debug dense dfa '(?-u)\w' --start-kind unanchored -q
[.. snip ..]
          memory:  384
[.. snip ..]
```

In this case, the dense DFA is actually smaller than the corresponding NFA.

So with all of that said, why does a general purpose regex engine like
the `regex` crate have a DFA engine with such huge downsides? Doesn't the
exorbitant memory usage make it a non-starter? There are two angles to this.

First is that the DFA engine is actually disabled by default. One must opt
into it by enabling the `perf-dfa-full` feature. I did this because fully
compiled DFAs don't carry a ton of weight in the `regex` crate, since the lazy
DFA (discussed in the next section) is a better choice in the vast majority of
cases. However, fully compiled DFAs do provide some optimization opportunities
that are difficult for a lazy DFA to take advantage of. For example, in
the regex `(?m)^.*$`, a fully compiled DFA will notice that `.` doesn't
match a `\n`. It knows this by looking for states where most transitions are
self-transitions (transitions that loop back to the same state). It follows
that there are only a limited number of ways to leave that state. The DFA finds
these states and "accelerates" them by running `memchr` to find the bytes
in the haystack corresponding to non-self-transitions. You can see this in
practice with `regex-cli` with a little ad hoc benchmarking. First we'll start
with DFA state acceleration enabled (it's enabled by default):

```
$ regex-cli find match dense -bB -q --repeat 1000000 \
   -p '(?m-u)^.*$' \
   -y 'this is a long line about the quick brown fox that jumped over the lazy dog'
[.. snip ..]
                 search time:  56.600473ms
[.. snip ..]
```

And now with DFA state acceleration disabled:

```
$ regex-cli find match dense -bB -q --repeat 1000000 --no-accelerate \
   -p '(?m-u)^.*$' \
   -y 'this is a long line about the quick brown fox that jumped over the lazy dog'
[.. snip ..]
                 search time:  199.044059ms
[.. snip ..]
```

The search time with acceleration enabled is quite a bit faster. Notice
also that we've disabled Unicode. When Unicode is enabled, `.` matches the
UTF-8 encoding of any Unicode scalar value. This in turn makes the DFA more
complicated and inhibits the acceleration optimization in this case:

```
$ regex-cli find match dense -q --repeat 1000000 \
   -p '(?m)^.*$' \
   -y 'this is a long line about the quick brown fox that jumped over the lazy dog'
[.. snip ..]
                 search time:  204.897593ms
[.. snip ..]
```

While this form of DFA state acceleration is quite useful, it is somewhat
limited in the regexes it can be applied to in part because of Unicode. It is
also limited because the meta regex engine only chooses to use the DFA engine
when the regex is very small. Otherwise we open ourselves up to exorbitant
memory usage and exponentials while building a regex. The `regex` crate isn't
the fastest at compiling a regex, but taking exponential time is a big no-no.

Because of its somewhat limited utility and since the DFA engine adds a lot of
code that in turn increases compile times and binary size substantially, full
DFAs are disabled by default.

The second angle as to why full DFAs exist at all is because their search
runtime is extremely basic. They are the only regex engine in `regex-automata`
to not require any mutable `Cache` space while executing a search. Indeed,
let's take a look at an example:

```rust
use regex_automata::{
    dfa::{dense, regex::Regex},
    Match,
};

fn main() {
    let re = Regex::builder()
        // We need to enable heuristic Unicode word boundary support,
        // or else the regex below will fail to compile. Why? Because
        // \b is Unicode-aware by default, and the DFA engines don't
        // support it on haystacks with non-ASCII Unicode codepoints.
        // Enabling this comes with the downside of making it possible
        // for a search to return an error. Namely, when the DFA sees
        // a non-ASCII byte, it transitions to a special sentinel quit
        // state, which in turn causes the search to stop and return an
        // error.
        .dense(dense::Config::new().unicode_word_boundary(true))
        .build(r"\b\w+\b")
        .unwrap();

    // Note that `find_iter` will panic if the underyling search returns
    // an error! You can handle the error by using fallible APIs such as
    // Regex::try_search.
    let mut it = re.find_iter("Sherlock Holmes");
    assert_eq!(Some(Match::must(0, 0..8)), it.next());
    assert_eq!(Some(Match::must(0, 9..15)), it.next());
    assert_eq!(None, it.next());
}
```

And the equivalent `regex-cli` command:

```
$ regex-cli find match dense --no-table --unicode-word-boundary \
   -p '\b\w+\b' \
   -y 'Sherlock Holmes'
0:0:8:Sherlock
0:9:15:Holmes
```

Notice that no `Cache` is required. Indeed, because of how simple the search
runtime is for a DFA, and because the DFA internals were designed with this use
case in mind, a DFA can be [serialized to raw bytes][DFA serialization]. That
same DFA can be deserialized and used to execute a search in a free-standing
environment. That is, you don't need Rust's `std` or `alloc` libraries. Only
`core` is needed.

(The DFA serialization use case was what motivated the initial `regex-automata
0.1` release. It's currently used in the [`bstr`] crate for implementing some
of the Unicode segmentation algorithms.)

Fully compiled DFAs are most useful when your regexes don't make use of Unicode
features, or if you need access to lower level APIs. For example, this shows
how one can compute the transitions manually on a DFA:

```rust
use regex_automata::{
    dfa::{dense::DFA, Automaton},
    Anchored,
};

fn main() {
    let re = DFA::new(r"\W").unwrap();

    // DFAs can have multiple start states, but only when there are look-around
    // assertions. When there aren't any look-around assertions, as in this
    // case, we can ask for a start state without providing any of the
    // haystack.
    let mut sid = re.universal_start_state(Anchored::No).unwrap();
    // The UTF-8 encoding of 💩 is \xF0\x9F\x92\xA9.
    sid = re.next_state(sid, 0xF0);
    sid = re.next_state(sid, 0x9F);
    sid = re.next_state(sid, 0x92);
    sid = re.next_state(sid, 0xA9);
    sid = re.next_eoi_state(sid);
    assert!(re.is_match_state(sid));
}
```

In this example, we walked the DFA manually and fed the DFA one byte at a time.
This example is a little contrived, but it demonstrates some of the APIs that
provide low level control. There are many more examples documented on the
[`Automaton`] trait, which defines all of the lower level DFA routines that
dense and sparse DFAs implement.

### Engine: hybrid NFA/DFA

The hybrid NFA/DFA or ["lazy DFA" regex engine][lazy DFA regex engine] is
exactly like the DFA engine, except its transition table is built at search
time. In other words, while a full DFA is "ahead-of-time compiled," the lazy
DFA is "just-in-time compiled."

(Calling it a JIT would be somewhat misleading. In this domain, a JIT usually
refers to compiling a regex into machine code at runtime, such as the JIT in
PCRE2. That is not what's happening here.)

The lazy DFA generally has the same API as the fully compiled DFA, except that,
like the other regex engines, you need to pass a mutable `Cache` argument. The
`Cache` is where the transition table (among other things) is stored:

```rust
use regex_automata::{
    hybrid::{dfa, regex::Regex},
    Match,
};

fn main() {
    let re = Regex::builder()
        // As with the fully compiled DFA, we need to enable heuristic
        // Unicode word boundary support for the lazy DFA as well. It
        // will return an error if a non-ASCII codepoint is seen when
        // the regex contains a Unicode word boundary, just like the
        // full DFA.
        .dfa(dfa::Config::new().unicode_word_boundary(true))
        .build(r"\b\w+\b")
        .unwrap();
    let mut cache = re.create_cache();

    // Note that find_iter will panic if the underyling search returns
    // an error! You can handle the error by using fallible APIs such
    // as Regex::try_search.
    let mut it = re.find_iter(&mut cache, "Sherlock Holmes");
    assert_eq!(Some(Match::must(0, 0..8)), it.next());
    assert_eq!(Some(Match::must(0, 9..15)), it.next());
    assert_eq!(None, it.next());
}
```

And the equivalent `regex-cli` command:

```
$ regex-cli find match hybrid --no-table --unicode-word-boundary \
   -p '\b\w+\b' \
   -y 'Sherlock Holmes'
0:0:8:Sherlock
0:9:15:Holmes
```

This example is nearly identical to the full DFA, but with a `Cache` parameter.
The similarities extend to lower level APIs as well:

```rust
use regex_automata::{hybrid::dfa::DFA, Input};

fn main() {
    let re = DFA::new(r"\W").unwrap();
    let mut cache = re.create_cache();

    // DFAs can have multiple start states, but only when there are
    // look-around assertions. When there aren't any look-around
    // assertions, as in this case, we can ask for a start state
    // without providing any of the haystack. Full DFAs have a
    // dedicated routine for this because the universality can be
    // checked for us. But lazy DFAs don't compute all of their start
    // states up front. So we just kind of fake it and ask for a start
    // state given some dummy haystack.
    let mut sid = re.start_state_forward(&mut cache, &Input::new("")).unwrap();
    // The UTF-8 encoding of 💩 is \xF0\x9F\x92\xA9.
    sid = re.next_state(&mut cache, sid, 0xF0).unwrap();
    sid = re.next_state(&mut cache, sid, 0x9F).unwrap();
    sid = re.next_state(&mut cache, sid, 0x92).unwrap();
    sid = re.next_state(&mut cache, sid, 0xA9).unwrap();
    sid = re.next_eoi_state(&mut cache, sid).unwrap();
    assert!(sid.is_match());
}
```

Other than passing a `Cache` explicitly, these APIs are almost the same. The
main difference is that each of the operations might fail. Namely, depending on
the method, they can return either a [`MatchError`] or a [`CacheError`]. A
`MatchError` occurs when the start state can't be computed because it enters
a quit state. (Which means the search must terminate with an error. This, for
example, occurs when heuristic Unicode word boundary support is enabled and a
non-ASCII byte is seen when computing the start state.) A `CacheError` occurs
when the `Cache` provided exhausts its capacity.

At this point, it's worth talking a little more about the `Cache` because it is
both the lazy DFA's greatest strength and its greatest weakness. The lazy DFA,
as mentioned, works by building its transition table during a search. More
specifically, the following happens:

1. A maximum cache capacity is configured at construction time. The capacity is
not fully allocated up front. It's just a number establishing an upper bound on
how much heap memory can be used.
2. When the caller asks to compute the transition for the current state and
character from the haystack, the lazy DFA consults its `Cache`. If the transition
has already been computed and stored in the `Cache`, then it is returned as-is
immediately. Otherwise, the transition---and only that transition---is computed
by doing NFA powerset construction. This process takes worst case `O(m)` time.
3. If the cache fills up, it is cleared and thus transitions previously computed
will need to be-recomputed.
4. Optional configuration can be set to cause the lazy DFA to return an error
if the `Cache` is being used inefficiently. Efficiency is measured in terms
of how many bytes are searched per each DFA state computed. If few bytes are
searched compared to the number of DFA states in the `Cache`, then it's likely
that even the `PikeVM` would execute the search more quickly. (Other heuristics
are used here as well, such as the total number of times the `Cache` has been
cleared.)

In this way, at most one DFA state and transition is created for each byte of
haystack searched. Thus, the worst case search time for the lazy DFA is
`O(m * n)` and its worst case space usage is the fixed capacity set at
construction time. Since building a lazy DFA itself does not require the
construction of any DFA states (except for a few basic sentinel states), it
follows that the lazy DFA mitigates the theoretical worst case time and space
complexities for full DFAs. That is, we avoid the exponential construction
time. In the common case, most states/transitions are cached, and so the lazy
DFA runs in average case `O(n)` time. In practice, for most regexes, the lazy
DFA and the fully compiled DFA have the same search performance.

The lazy DFA also mitigates the exorbitant space usage for large Unicode
character classes. Since a lazy DFA only computes what it needs based on the
actual bytes searched, searching for a Unicode-aware `\w` in an ASCII-only
haystack only requires building the ASCII portion of `\w` into a DFA. This ends
up working amazingly well in practice.

The lazy DFA tends to do poorly for regexes that fill up its cache and cause
it to be cleared repeatedly. This might just be a result of the regex being
large or even a result of the haystack forcing a large portion of the DFA to
be constructed. (A regex can easily become large through counted repetitions
or even by adding a lot of patterns. A single lazy DFA gets built for a
multi-pattern regex.) In these cases, heuristics usually detect it and force
the lazy DFA to return an error. At this point, in the context of the meta
regex engine, the search will be retried with a different engine (usually the
`PikeVM`).

### The meta regex engine

The [meta regex engine] brings all of the aforementioned regex engines together
and exposes one single infallible API that tries to do the best possible thing
in any given situation. The API it exposes also absolves the caller of needing
to explicitly create and pass `Cache` values to each search call. Instead, the
meta regex engine handles this for you by keeping an internal thread safe pool
of `Cache` values. (The meta regex engine does expose lower level APIs that
permit passing a `Cache` explicitly. These are useful if one wants to avoid
the synchronization costs of the thread safe pool used internally.)

The end result here is that the meta regex engine very closely corresponds
to the top-level API in the `regex` crate. Indeed, `regex::Regex`,
`regex::RegexSet`, `regex::bytes::Regex` and `regex::bytes::RegexSet` are all
very thin wrappers around the meta regex engine. This is by design, because it
makes it easier to drop down from the high level convenience API that serves
99% of use cases to the lower level API that exposes a lot more knobs.

Internally, the meta regex engine implements roughly the following logic:

* If a regex engine isn't needed at all and the search can be performed using
single or multi-substring search algorithms directly, then the construction of
a regex (including an `NFA`) is avoided entirely.
* If possible, extract a small literal sequence from the prefix of the regex
that can be used as a [`Prefilter`].
* If possible, choose a "reverse" optimization:
    * If a regex is anchored at the end via `$`, then a search can proceed by
    doing a reverse scan from the end of the haystack. This is called the
    "reverse anchored" optimization.
    * If no suitable `Prefilter` has been found and a literal sequence can be
    extracted from the suffix of the regex, then we can scan for occurrences of
    that literal sequence and match the regex in reverse from each candidate
    position. This is called the "reverse suffix" optimization.
    * If no suitable prefix or suffix literal sequence could be found but a
    literal sequence could be extracted from an inner part of the regex that
    cleanly partitions the regex, then we can scan for occurrences of that
    inner literal sequence. We split the regex in half at the point where the
    inner literal sequence was extracted. The first half gets compiled into a
    reverse regex and the second half gets compiled into a forward regex. When
    a candidate is found, we can look for the start of the match by scanning
    backwards with the first half, and then look for the end of the match by
    scanning forwards with the second half.
* Otherwise, fall back to the "core" search strategy. The core strategy makes
use of all available regex engines: the `PikeVM`, the bounded backtracker, the
one-pass DFA, the lazy DFA and the full DFA. Only the `PikeVM` is required. The
way these engines compose together is roughly as follows:
    * Whenever possible, use the lazy DFA (or full DFA if available) to find
    the overall bounds of the match. If the DFA fails, then we fall back to
    either the `PikeVM`, the bounded backtracker or the one-pass DFA. The DFA
    can fail either because the regex contained a Unicode word boundary and the
    haystack contained a non-ASCII codepoint, or because the the lazy DFA was
    used and its `Cache` usage was inefficient according to some heuristic.
    * When capture groups are requested, then after the full match is found,
    either the `PikeVM`, bounded backtracker or one-pass DFA is used to report
    the offsets of each matching capture group. The order of preference is:
    one-pass DFA, bounded backtracker and then the `PikeVM`.

If one were to summarize the overall strategy, it can probably be distilled
down to these two points:

* Search for literals whenever possible.
* Avoid the `PikeVM` as much as possible.

That's pretty much it. In general, the more time we can spend in substring
search algorithms, the better. And the less time we can spend in specifically
the `PikeVM`, the better.

Many regex engines do some kind of literal optimization, and indeed, most
popular production grade regex engines spend a fair bit of effort in doing
so. [Hyperscan] is the gold standard here, but as far as I'm aware, most
other general purpose regex engines don't go to the lengths described above.
(One could argue about whether Hyperscan is a "general purpose" regex engine
or not. One of my own personal arguments against considering it one is its
match semantics. For example, `\w+` matches `abc` 3 times because Hyperscan
reports matches as they're seen. An undoubtedly correct choice given its
target domain.) The reverse suffix and reverse inner optimizations are
particularly tricky. They look easy, but there's a subtle performance problem
with them: they open you up to worst case quadratic behavior (in the size of
the haystack).

Consider the regex `[A-Z].*bcdefghijklmnopq` on a haystack of
`bcdefghijklmnopq` repeated over and over. There is no "good" prefix literal
sequence that can be extracted from this, so according to the logic above,
the meta regex engine will try the "reverse suffix" optimization by using
`bcdefghijklmnopq` as the suffix. This particular benchmark was devised to have
a worst case false positive rate: candidates are reported frequently and none
of them lead to a match. But that's just a bad heuristic. It doesn't in and of
itself lead to violating our time complexitiy guarantee (which is `O(m * n)`).
The problem here is that each time the suffix matches, a reverse scan includes
the `.*`, and that in turn scans all the way back to the beginning of the
haystack. So each candidate reported by the suffix match results in a complete
re-scan of the haystack back to the beginning. This changes our search routine
to have worst case `O(m * n^2)` time complexity. That's bad.

While it is possible to do syntactic analysis on a regex to determine whether
this quadratic behavior is possible, it doesn't predict it perfectly. For
example, one can clearly see that the suffix `bcdefghijklmnopq` overlaps with
the expression immediately before it, `.*`. That in turn means some kind of
quadratic behavior might be possible. But that doesn't mean it is inevitable.

Instead, the meta regex engine mitigates this by defining its own bespoke regex
search routines based on the DFA engines. Namely, it defines its own search
routine that will stop its reverse scan if it gets to a certain offset in the
haystack. That offset corresponds to the end of the last suffix match. So if
the search would otherwise exceed that offset, then we're exposed to quadratic
behavior. The meta regex engine detects this error case and falls back to the
"core" strategy described above, thus abandoning the optimization when it would
otherwise sacrifice our time complexity guarantees.

Roughly the same logic applies to the "reverse inner" optimization as well.

In summary, if you don't need low level access to individual regex engines but
you do want control over the many knobs exposed by the regex engine or want
to do multi-pattern matching, then the meta regex engine is a good choice.
Namely, all of the regex engines described before this each have their own
little caveats that make them less than ideal for general use in every case.

## Differences with RE2

If you've read [Russ Cox's article series on regular expressions][rsc-regexp],
then some portion of the previous section is likely to sound familiar.
Namely, RE2 has a [PikeVM][re2-nfa] (called just an "NFA"), a [bounded
backtracker][re2-bitstate] (called a "bitstate backtracker" in RE2), a
[one-pass DFA][re2-onepass] (called a "one-pass NFA" in RE2) and a [lazy
DFA][re2-dfa]. It also has a [meta regex engine][re2-meta] (although that term
isn't used) that composes its internal regex engines in a similar fashion as
described above. The only engine described above that RE2 doesn't have is a
fully compiled DFA.

So what, if any, are the differences between RE2 and the `regex` crate?

The similarities between them are much greater than the differences, but here's
a high level list of differences:

* RE2 supports leftmost-longest semantics as an option in addition to
leftmost-first. Leftmost-longest semantics are what POSIX regex engines use.
* RE2 has less support for Unicode. A full accounting of this is tricky
because RE2 does permit linking with [ICU] to add support for more Unicode
properties. However, RE2 does not have an option to make `\w`, `\s`, `\d` and
`\b` use Unicode definitions. RE2 also does not support character class set
operations beyond union. For example, it's harder to write something like
`[\pL&&\p{Greek}]` in RE2, which corresponds to the subset of codepoints
considered letters that are also in the Greek script. (Character class set
operations other than union aren't strictly Unicode specific features, but they
are most useful in the context of large Unicode character classes.)
* RE2 has a likely more memory efficient version of the PikeVM.
* RE2 has some limited support for literal optimizations, but overall does
a lot less here than what the `regex` crate does. RE2 does have a [Filtered
RE2] which permits the caller to do a limited form of their own literal
optimizations.
* RE2 uses the same transition cache across multiple threads for the lazy DFA
engine, which requires synchronization. Conversely, the `regex` crate requires
a distinct cache for each thread, which requires more memory.
* The `regex` crate now exposes both `regex-syntax` and `regex-automata` as
separately versioned libraries that provide access to its internals. RE2 does
not support this.
* The `regex-automata` library has first class support for multi-pattern
regexes in all engines. RE2 does have a "regex set," but it only reports which
patterns match in a haystack. `regex-automata`, on the other hand, can also
report match and capture group offsets for each matching pattern.

In the future, I hope to add more engines to `regex-automata`. Specifically,
I'd like to explore [Glushkov NFAs][Glushkov NFA] and a bit parallel regex
engine. I'd also like to explore [shift DFAs][shift DFA].

## Testing strategy

As described near the opening of this blog, testing had become a problem for
the `regex` crate. The macro hacks used to test each internal engine were
growing quite strained and they were generally difficult to work with and, more
importantly, understand.

My idea for revamping how the regex crate was tested was tied with the idea
that each internal engine would have its own first class API that could be
tested independently from the "main" regex engine. I also wanted to make the
tests consumable from any context instead of burying them in macros or Rust
source code. To that end, this is the strategy I settled upon:

* All regex tests are specified in TOML files.
* I published a crate, [`regex-test`], that knows how to read these TOML files
into a structured representation.
* I defined a single Rust unit test for each configuration of each regex engine
that I wanted to test. Inside this single unit test, all of the tests from
the TOML files that are applicable to the engine being tested are executed.

A little extra infrastructure was required in order to make this nice to use
because Rust's unit testing framework is not currently extensible. That means
I had to define my own environment variables for filtering on which tests to
run. (For example, if you're working on a single bug that causes many tests to
fail, it's often useful to just have one of those tests run.)

There are also oodles of other kinds of tests as well. For example, there are
over 450 documentation tests in `regex-automata` alone.

Finally, in the run up to the `regex 1.9`, I added a lot of additional fuzz
testing targets. I had a **ton** of help from [Addison Crump] and there were
at least a few bugs I wouldn't have found if it weren't for him.

## Benchmarking

At this point, this blog is already my longest one ever, and I haven't even
begun to discuss benchmarking. While I originally wanted to spend more time
on this topic in this blog---particularly given all of the talk about
optimizations---it just wasn't practical to do so.

Instead, I've published a regex barometer called [rebar]. It isn't just limited
to benchmarking the `regex` crate. It also benchmarks many other regex engines
as well. I believe it is the most comprehensive regex benchmark published to
date.

Across 242 benchmarks, `regex 1.9` is on average 1.5 times faster than `regex
1.7.3` for search times. (I compared with `1.7` instead of `1.8` because `1.8`
reflects a transition release that has some of the work described in this blog
post included. The `1.9` release just completes the transition.)

```
$ rebar rank record/all/2023-07-02/*.csv \
   --intersection \
   -M compile \
   -e '^(rust/regex|rust/regexold)$'
Engine         Version  Geometric mean of speed ratios  Benchmark count
------         -------  ------------------------------  ---------------
rust/regex     1.8.4    1.08                            242
rust/regexold  1.7.3    2.61                            242
```

But the time it takes to build a regex has regressed somewhat:

```
$ rebar rank record/all/2023-07-02/*.csv \
   --intersection \
   -m compile \
   -e '^(rust/regex|rust/regexold)$'
Engine         Version  Geometric mean of speed ratios  Benchmark count
------         -------  ------------------------------  ---------------
rust/regexold  1.7.3    1.07                            28
rust/regex     1.8.4    1.46                            28
```

The geometric mean reported above is a very crude aggregate statistic. I'm not
sure it really captures the extent of the improvements here. If you want to look
at individual benchmark differences, one can replace `rebar rank` with `rebar cmp`
in the above command. (And run it from the root of a checkout of the [rebar]
repository.)

```
$ rebar cmp record/all/2023-07-02/*.csv \
   --threshold-min 2 \
   --intersection \
   -M compile \
   -e '^(rust/regex|rust/regexold)$'
```

I've added `--threshold-min 2` to the above command to limit the comparisons
to ones where there is at least a 2x difference.

## Costs

No good deed goes unpunished. What has this rewrite cost me?

First and foremost, it has used up the vast majority of my free time for the
past several years. Compounding the problem, I have [a lot less of that free
time than I used to][murphy]. So projects like ripgrep haven't seen a release
for quite some time.

Secondly, this has introduced a fair bit more code. Building reusable abstractions
for others to use is a different beast than internal abstractions that only
`regex` crate hackers need to worry about. It usually results in more code,
which means bigger binary sizes and higher compile times.

Thirdly, those abstractions are now published and separately versioned. That
means I can't just break the APIs of those internal engines without publishing
an appropriate breaking change release of `regex-automata`. I won't be nearly
as conservative as doing so as I am with the `regex` crate, but it isn't free
to do. This will also impact contributors. Instead of just being able to
refactor code as necessary, one must now contend with the pressures of public
API design.

Because the `regex` crate already had a reputation for less-than-ideal binary
sizes and compile times, and since these changes were going to make that
_worse_, I decided on two different mitigations:

1. As discussed above, I made the fully compiled DFA regex engine opt-in. This
engine brings in quite a bit of code, but its impact on search performance is
modest.
2. I published a new crate, [`regex-lite`], that acts _nearly_ as a drop-in
replacement for the `regex` crate. Its design is based on optimizing almost
exclusively for binary size and compile time, at the expense of functionality
(namely, Unicode) and performance. You still get the `O(m * n)` time complexity
guarantee, but you don't get any of the fancy Unicode support and you don't get
fast search times. But the binary size and compile times are a lot better.
`regex-lite` has zero dependencies. It shares zero code---including rolling its
own regex parser---with the `regex` crate.

The `regex-lite` mitigation is still somewhat of an experiment, but it just
goes to show that making code artbitrarily reducible is difficult. Even though
the `regex` crate has a bunch of features for disabling both optimizations and
Unicode functionality, it still can't get anywhere close to the binary size
and compile times of `regex-lite`.

[regex-github]: https://github.com/rust-lang/regex/
[`regex-automata`]: https://github.com/rust-lang/regex/tree/master/regex-automata
[first-regex-issue]: https://github.com/rust-lang/rust/issues/3591
[graydon-re2]: https://github.com/rust-lang/rust/issues/3591#issuecomment-17009497
[RE2]: https://github.com/google/re2
[rsc-regexp]: https://swtch.com/~rsc/regexp/
[inspired-by-re2]: https://github.com/rust-lang/rust/issues/3591#issuecomment-39582811
[first-regex-rfc]: https://github.com/rust-lang/rfcs/pull/42
[first-cargo]: https://github.com/rust-lang/rust/pull/1149
[first-regex-rfc-approved]: https://github.com/rust-lang/rfcs/pull/42#issuecomment-41104032
[first-regex-pr]: https://github.com/rust-lang/rust/pull/13700
[first-rust]: https://github.com/BurntSushi/quickcheck/commit/c9eb2884d6a620b90b9986c65916eebc57084e89
[regex-1.0-rfc]: https://github.com/rust-lang/rfcs/pull/1620
[regex-1.0-pr]: https://github.com/rust-lang/regex/pull/471
[regex-syntax-0.5]: https://github.com/rust-lang/regex/commit/715a8072890af65d2095d39f534b4b3dc4caeae2
[Aho-Corasick]: https://en.wikipedia.org/wiki/Aho%E2%80%93Corasick_algorithm
[Thompson NFA]: https://en.wikipedia.org/wiki/Thompson%27s_construction
[regex-github-hidden-api]: https://github.com/rust-lang/regex/blob/5a34a39b72d85730065d3ffe4ce3715f2731e49a/src/lib.rs#L790-L801
[regex-from-exec]: https://github.com/rust-lang/regex/blob/5a34a39b72d85730065d3ffe4ce3715f2731e49a/src/re_unicode.rs#L174-L179
[regex-test-macro]: https://github.com/rust-lang/regex/blob/5a34a39b72d85730065d3ffe4ce3715f2731e49a/tests/fowler.rs#L5
[regex-test-target]: https://github.com/rust-lang/regex/blob/5a34a39b72d85730065d3ffe4ce3715f2731e49a/tests/test_backtrack.rs#L3-L11
[`RegexSet`]: https://docs.rs/regex/latest/regex/struct.RegexSet.html
[regex-streams]: https://github.com/rust-lang/regex/issues/425
[`regex-syntax`]: https://docs.rs/regex-syntax
[bstr-fsm]: https://github.com/BurntSushi/bstr/tree/b3cab1905c46ad7de78a032a61eef0437ed7fb58/src/unicode/fsm
[`bstr`]: https://github.com/BurntSushi/bstr
[determinization]: https://en.wikipedia.org/wiki/Powerset_construction
[vector instructions]: https://en.wikipedia.org/wiki/Single_instruction,_multiple_data
[`Ast`]: https://docs.rs/regex-syntax/0.7.*/regex_syntax/ast/enum.Ast.html
[`Hir`]: https://docs.rs/regex-syntax/0.7.*/regex_syntax/hir/struct.Hir.html
[literal sequence]: https://docs.rs/regex-syntax/0.7.2/regex_syntax/hir/literal/struct.Seq.html
[Hyperscan]: https://github.com/intel/hyperscan
[literal-extractor]: https://docs.rs/regex-syntax/latest/regex_syntax/hir/literal/struct.Extractor.html
[`memmem`]: https://docs.rs/memchr/2.*/memchr/memmem/index.html
[`memchr`]: https://docs.rs/memchr/2.*/memchr/
[Two-Way]: https://en.wikipedia.org/wiki/Two-way_string-matching_algorithm
[Rabin-Karp]: https://en.wikipedia.org/wiki/Rabin%E2%80%93Karp_algorithm
[generic SIMD]: http://0x80.pl/articles/simd-strfind.html
[Teddy]: https://github.com/BurntSushi/aho-corasick/tree/97e48b6dbdf9ebd50168540276fa3f14f403d42b/src/packed/teddy
[Aho-Corasick]: https://github.com/BurntSushi/aho-corasick/
[`NFA`]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/nfa/thompson/struct.NFA.html
[Thompson's construction]: https://en.wikipedia.org/wiki/Thompson%27s_construction
[utf8-ranges]: https://docs.rs/regex-syntax/0.7.*/regex_syntax/utf8/index.html
[Daciuk's algorithm]: https://blog.burntsushi.net/transducers/#references
[range trie]: TODO
[literal trie]: TODO
[Glushkov NFA]: https://en.wikipedia.org/wiki/Glushkov%27s_construction_algorithm
[`Prefilter`]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/util/prefilter/struct.Prefilter.html
[`PikeVM`]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/nfa/thompson/pikevm/struct.PikeVM.html
[`BoundedBacktracker`]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/nfa/thompson/backtrack/struct.BoundedBacktracker.html
[one-pass DFA]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/dfa/onepass/struct.DFA.html
[dense DFA]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/dfa/dense/struct.DFA.html
[sparse DFA]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/dfa/sparse/struct.DFA.html
[DFA regex engine]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/dfa/regex/struct.Regex.html
[lazy DFA]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/hybrid/dfa/struct.DFA.html
[lazy DFA regex engine]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/hybrid/regex/struct.Regex.html
[meta regex engine]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/meta/struct.Regex.html
[`Input`]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/struct.Input.html
[`Match`]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/struct.Match.html
[`MatchError`]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/struct.MatchError.html
[`PatternID`]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/struct.PatternID.html
[ICU]: https://icu.unicode.org/
[shift DFA]: https://gist.github.com/pervognsen/218ea17743e1442e59bb60d29b1aa725
[Filtered RE2]: https://github.com/google/re2/blob/2d39b703d02645076fead8fa409a1711f0e84381/re2/filtered_re2.h
[re2-nfa]: https://github.com/google/re2/blob/2d39b703d02645076fead8fa409a1711f0e84381/re2/nfa.cc
[re2-bitstate]: https://github.com/google/re2/blob/2d39b703d02645076fead8fa409a1711f0e84381/re2/bitstate.cc
[re2-onepass]: https://github.com/google/re2/blob/2d39b703d02645076fead8fa409a1711f0e84381/re2/onepass.cc
[re2-dfa]: https://github.com/google/re2/blob/2d39b703d02645076fead8fa409a1711f0e84381/re2/dfa.cc
[re2-meta]: https://github.com/google/re2/blob/2d39b703d02645076fead8fa409a1711f0e84381/re2/re2.cc#L648-L906
[API themes]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/#api-themes
[`regex-cli` README]: TODO
[regex-1.9-release]: TODO
[pikevm-cache]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/nfa/thompson/pikevm/struct.Cache.html
[`Config::visited_capacity`]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/nfa/thompson/backtrack/struct.Config.html#method.visited_capacity
[tagged finite automata]: https://arxiv.org/abs/1907.08837
[re2c]: http://re2c.org/
[DFA serialization]: https://github.com/rust-lang/regex/tree/ag/regex-automata/regex-cli#example-serialize-a-dfa
[`Automaton`]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/dfa/trait.Automaton.html
[`CacheError`]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_automata/hybrid/struct.CacheError.html
[`regex-test`]: https://burntsushi.net/stuff/tmp-do-not-link-me/regex/regex_test/
[Addison Crump]: https://github.com/addisoncrump
[rebar]: https://github.com/BurntSushi/rebar
[murphy]: https://github.com/BurntSushi/blog/commit/8ea55788f2eb8226343fdbefaaee189412bd3c1c
[`regex-lite`]: https://docs.rs/regex-lite
