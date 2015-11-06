+++
date = "2015-11-02T19:09:00-04:00"
title = "Index 1,000,000,000 Strings with Automata"
author = "Andrew Gallant"
url = "transducers"
draft = true

[blackfriday]
plainIdAnchors = true
+++

It turns out that finite state machines are useful for things other than
expressing computation. Finite state machines can also be used to compactly
represent ordered sets or maps of strings that can be searched very quickly.

In this article, I will teach you about finite state machines as a *data
structure* for representing ordered sets and maps. This includes introducing
an implementation [written in Rust](https://github.com/BurntSushi/fst). I will
also show you how to build them using a simple command line tool. Finally, I
will discuss a few experiments culminating in indexing over 1,000,000,000 URLs
(134 GB) from the
[July 2015 Common Crawl Archive](http://blog.commoncrawl.org/2015/08/july-2015-crawl-archive-available/).

Along the way, we will talk about memory maps, automaton intersection with
regular expressions, fuzzy searching with Levenshtein distance and streaming
set operations.

**Target audience**: Some familiarity with programming. No experience with
automata theory or Rust is required.

<!--more-->

## Table of Contents

This article is pretty long, so I've put together a table of contents in case
you want to skip around.

The first section discusses finite state machines and their use as data
structures in the abstract. This section is meant to give you a mental model
with which to reason about the data structure. There is no code in this
seciton.

The second section takes the abstraction developed in the first
section and demonstrates it with an implementation. This section
is mostly intended to be an overview of the implementation of the
[`fst`](https://github.com/BurntSushi/fst) crate. This section contains code.

The third and final section demonstrates use of a simple command line tool to
build indexes. We will look at some real data sets and attempt to reason about
the performance of finite state machines as a data structure.

* [Finite State Machines as Data Structures](#finite-state-machines-as-data-structures)
    * [Ordered Sets](#ordered-sets)
    * [Ordered Maps](#ordered-maps)

## Finite State Machines

A finite state machine (FSM) is a collection of states and a collection of
transitions that move from one state to the next. One state is marked as the
start state and zero or more states are marked as final states. An FSM is
always in exactly one state at a time.

FSM's are rather general and can be used to model a number of processes. For
example, consider an approximation of the daily life of my cat Cauchy:

![The daily life of my cat Cauchy](/images/transducers/dot/cauchy.png)

Some states are "asleep" or "eating" and some transitions are "food is served"
or "something moved." There aren't any final states here because that would be
unnecessarily morbid!

Notice that the FSM approximates our notion of reality. Cauchy cannot be both
playing and asleep at the same time, so it satisfies our condition that the
machine is only ever in one state at a time. Also, notice that transitioning
from one state to the next only requires a single input from the environment.
Namely, being "asleep" carries no memory of whether it was caused by getting
tired from playing or from being satisfied after a meal. Regardless of how
Cauchy fell asleep, he will always wake up if he hears something moving or if
the dinner bell rings.

Cauchy's finite state machine can perform computation given a sequence of
inputs. For example, consider the following inputs:

* food is served
* loud noise
* quiet calm
* food digests

If we apply these inputs to the machine above, then Cauchy will move through
the following states in order: "asleep," "eating," "hiding," "eating," "litter
box." Therefore, if we observed that food was served, followed by a loud noise,
followed by quiet calm and finally by Cauchy's digestion, then we could
conclude that Cauchy was currently in the litter box.

This particularly silly example demonstrates how general finite state machines
truly are. For our purposes, we will need to place a few restrictions on the
type of finite state machine we use to implement our ordered set and map
data structures.

### Ordered Sets

An ordered set is like a normal set, except the keys in the set are ordered.
That is, an ordered set provides ordered iteration over its keys.
Typically, an ordered set is implemented with a binary search tree or a btree,
and an unordered set is implemented with a hash table.
In our case, we will look at an implementation that uses a
*deterministic acyclic finite state acceptor* (abbreviated FSA).

A deterministic acyclic finite state acceptor is a finite state machine that
is:

1. Deterministic. This means that at any given state, there is at most one
   transition that can be traversed for any input.
2. Acyclic. This means that it is impossible to visit a state that has
   already been visited.
3. An acceptor. This means that the finite state machine "accepts" a particular
   sequence of inputs if and only if it is in a "final" state at the end of the
   sequence of inputs. (This criterion, unlike the former two, will change when
   we look at ordered maps in the next section.)

How can we use these properties to represent a set? The trick is to store the
keys of the set in the transitions of the machine. This way, given a sequence
of inputs (i.e., characters), we can tell whether the key is in the set
based on whether evaluating the FSA ends in a final state.

Consider a set with one key "jul." The FSA looks like this:

![A set with one element, FSA](/images/transducers/sets/set1.png)

Consider what happens if we ask the FSA if it contains the key "jul." We need
to process the characters in order:

* Given `j`, the FSA moves from the start state `0` to `1`.
* Given `u`, the FSA moves from `1` to `2`.
* Given `l`, the FSA moves from `2` to `3`.

Since all members of the key have been fed to the FSA, we can now ask: is the
FSA in a final state? It is (notice the double circle around state `3`), so we
can say that `july` is in the set.

Consider what happens when we test a key that is *not* in the set. For example,
`jun`:

* Given `j`, the FSA moves from the start state `0` to `1`.
* Given `u`, the FSA moves from `1` to `2`.
* Given `n`, the FSA cannot move. Processing stops.

The FSA cannot move because the only transition out of state `2` is `l`,
but the current input is `n`. Since `l != n`, the FSA cannot follow that
transition. As soon as the FSA cannot move given an input, it can conclude that
the key is not in the set. There's no need to process the input further.

Consider another key, `ju`:

* Given `j`, the FSA moves from the start state `0` to `1`.
* Given `u`, the FSA moves from `1` to `2`.

In this case, the entire input is exhausted and the FSA is in state `2`. To
determine whether `ju` is in the set, it must ask whether `2` is a final state
or not. Since it is not, it can report that the `jul` is not in the set.

Let's add another key to set to see what it looks like. The following FSA
represents an ordered set with keys "jul" and "mar":

![A set with two elements, FSA](/images/transducers/sets/set2.png)

The FSA has grown a little more complex. The start state `0` now has two
transitions: `j` and `m`. Therefore, given the key `mar`, it will first
follow the `m` transition.

There's one other important thing to notice here: the state `3` is *shared*
between the `jul` and `mar` keys. Namely, the state `3` has two transitions
entering it: `y` and `h`. This sharing of states between keys is really
important, because it enables us to store more information in a smaller space.

Let's see what happens when we add `jun` to our set, which shares a common
prefix with `jul`:

![A set with three elements, FSA](/images/transducers/sets/set3.png)

Do you see the difference? It's a small change. This FSA looks very much like
the previous one. There's only one difference: a new transition, `n`, from
states `5` to `3` has been added. Notably, the FSA has no new states! Since
both `jun` and `jul` share the prefix `ju`, those states can be reused for both
keys.

Let's switch things up a little bit and look at a set with the following keys:
`october`, `november` and `december`:

![A set with four elements, FSA](/images/transducers/sets/set4.png)

Since all three keys share the suffix `ber` in common, it is only encoded into
the FSA exactly once. Two of the keys share an even bigger suffix: `ember`,
which is also encoded into the FSA exactly once.

Before moving on to ordered maps, we should take a moment and convince
ourselves that this is indeed an *ordered* set. Namely, given an FSA, how can
we iterate over the keys in the set?

To demonstrate this, let's use a set we built earlier with the keys `jul`,
`jun` and `mar`:

![A set with three elements, FSA](/images/transducers/sets/set3.png)

We can enumerate all keys in the set by walking the entire FSA by following
transitions in lexicographic order. For example:

* Initialize at state `0`. `key` is empty.
* Move to state `4`. Add `j` to `key`.
* Move to state `5`. Add `u` to `key`.
* Move to state `3`. Add `l` to `key`. **Emit** `jul`.
* Move back to state `5`. Drop `l` from `key`.
* Move to state `3`. Add `n` to `key`. **Emit** `jun`.
* Move back to state `5`. Drop `n` from `key`.
* Move back to state `4`. Drop `u` from `key`.
* Move back to state `3`. Drop `j` from `key`.
* Move to state `1`. Add `m` to `key`.
* Move to state `2`. Add `a` to `key`.
* Move to state `3`. Add `r` to `key`. **Emit** `mar`.

This algorithm is straight-forward to implement with a stack of the states to
visit and a stack of transitions that have been followed. It has time
complexity `O(n)` in the number of keys in the set with space complexity `O(k)`
where `k` is the size of the largest key in the set.

### Ordered Maps

As with ordered sets, an ordered map is like a map, but with an ordering
defined on the keys of the map. Just like sets, ordered maps are typically
implemented with a binary search tree or a btree, and unordered maps are
typically implemented with a hash table. In our case, we will look at an
implementation that uses a *deterministic acyclic finite state transducer*
(abbreviated FST).

A deterministic acyclic finite state transducer is a finite state machine that
is (the first two criteria are the same as the previous section):

1. Deterministic. This means that at any given state, there is at most one
   transition that can be traversed for any input.
2. Acyclic. This means that it is impossible to visit a state that has
   already been visited.
3. A **transducer**. This means that the finite state machine emits a value
   associated with the specific sequence of inputs given to the machine. A
   value is emitted if and only if the sequence of inputs causes the machine to
   end in a final state.

In other words, an FST is just like an FSA, but instead of answering "yes"/"no"
given a key, it will answer either "no" or "yes, and here's the value
associated with that key."

<!--
Initial number of urls: 7,563,934,593
Number of unique urls: 1,649,195,774

One hour to create 544 FST of ~10,000,000 URLs each.
  Each about 128MB
Three hours to join all 544 FST's into one.
  Size: 27GB

30 minutes to write all URLs from FST to file.
  Size: 134GB

1 hour 32 minutes to create FST from sorted list.
  Max heap size: 72MB
-->

<!--
{{< code-rust "test" >}}
use fst::{IntoStreamer, Streamer, Levenshtein, Set};

// A convenient way to create sets in memory.
let keys = vec!["fa", "fo", "fob", "focus", "foo", "food", "foul"];
let set = try!(Set::from_iter(keys));

// Build our fuzzy query.
let lev = try!(Levenshtein::new("foo", 1));

// Apply our fuzzy query to the set we built.
let stream = set.search(lev).into_stream();
let keys = try!(stream.into_strs());
assert_eq!(keys, vec![
    "fo",   // 1 deletion
    "fob",  // 1 substitution
    "foo",  // 0 insertions/deletions/substitutions
    "food", // 1 insertion
]);
{{< /code-rust >}}
-->

<!--
{{< high sh >}}
$ git clone git://github.com/BurntSushi/blog
{{< /high >}}
-->
