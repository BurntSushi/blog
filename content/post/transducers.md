+++
date = "2015-11-02T19:09:00-04:00"
title = "Index 1,000,000,000 Keys with Automata and Rust"
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

## Teaser

As a teaser to show where we're headed, let's take a quick look at an example.
We won't look at 1,000,000,000 strings quite yet. Instead, consider ~16,000,000
Wikipedia article titles (`384 MB`). Here's how to index them:

{{< high sh >}}
$ time fst set --sorted wiki-titles wiki-titles.fst

real    0m19.412s
{{< /high >}}

The resulting index, `wiki-titles.fst`, is `157 MB`. By comparison, `gzip`
takes `12` seconds and compresses to `91 MB`. (For some data sets, our indexing
scheme can beat `gzip` in both speed and compression ratio.)

However, here's something `gzip` cannot do: quickly find all article titles
starting with `Homer the`:

{{< high sh >}}
$ time fst grep wiki-titles.fst 'Homer the.*'
Homer the Clown
Homer the Father
Homer the Great
Homer the Happy Ghost
Homer the Heretic
Homer the Moe
Homer the Smithers
...

real    0m0.023s
{{< /high >}}

<!--*-->

By comparison, `grep` takes `0.3` seconds on the original uncompressed data.

And finally, for something that even `grep` cannot do: quickly find all article
titles within a certain edit distance of `Homer Simpson`:

{{< high sh >}}
$ time fst fuzzy wiki-titles.fst --distance 2 'Homer Simpson'
Home Simpson
Homer J Simpson
Homer Simpson
Homer Simpsons
Homer simpson
Homer simpsons
Hope Simpson
Roger Simpson

real    0m0.094s
{{< /high >}}

## Table of Contents

This article is pretty long, so I've put together a table of contents in case
you want to skip around.

The first section discusses finite state machines and their use as data
structures in the abstract. This section is meant to give you a mental model
with which to reason about the data structure. There is no code in this
seciton.

The second section takes the abstraction developed in the first section and
demonstrates it with an implementation. This section is mostly intended to
be an overview of how to use my [`fst`](https://github.com/BurntSushi/fst)
library. This section contains code. We will discuss some implementation
details, but will avoid the weeds. It is okay to skip this section if you don't
care about the code and instead only want to see experiments on real data.

The third and final section demonstrates use of a simple command line tool to
build indexes. We will look at some real data sets and attempt to reason about
the performance of finite state machines as a data structure.

* [Finite state machines as data structures](#finite-state-machines-as-data-structures)
    * [Ordered sets](#ordered-sets)
    * [Ordered maps](#ordered-maps)
    * [Construction](#construction)
        * [Trie construction](#trie-construction)
        * [FSA construction](#fsa-construction)
        * [FST construction](#fst-construction)
        * [Construction in practice](#construction-in-practice)
        * [References](#references)
* [The FST library](#the-fst-library)

## Finite state machines as data structures

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

### Ordered sets

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

It is worth pointing out here that the number of steps required to confirm
whether a key is in the set or not is bounded by the number of characters in
the key! That is, the time it takes to lookup a key is not related at all to
the size of the set.

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

![A set with three elements (suffixes),
FSA](/images/transducers/sets/set3-suffixes.png)

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

### Ordered maps

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

In the previous section, representing a set only required one to store the
keys in the transitions of the machine. The machine "accepts" an input sequence
if and only if it represents a key in the set. In this case, a map needs to do
more than just "accept" an input sequence; it also needs to return a value
associated with that key.

One way to associate a value with a key is to attach some data to each
transition. Just as an input sequence is consumed to move the machine from
state to state, an *output sequence* can be produced as the machine moves from
state to state. This additional "power" makes the machine a *transducer*.

Let's take a look at an example of a map with one element, `jul`, which is
associated with the value `7`:

![A map with one element, FST](/images/transducers/maps/map1.png)

This machine is the same as the corresponding set, except that the first
transition `j` from state `0` to `1` has the output `7` associated with it.
The other transitions, `u` and `l`, also have an output `0` associated with
them that isn't shown in the image.

As with sets, we can ask the map if it contains the key "jul." But we also need
to return the output. Here's how the machine processes a key lookup for "jul":

* Initialize `value` to `0`.
* Given `j`, the FST moves from the start state `0` to `1`. Add `7` to `value`.
* Given `u`, the FST moves from `1` to `2`. Add `0` to `value`.
* Given `l`, the FST moves from `2` to `3`. Add `0` to `value`.

Since all inputs have been fed to the FST, we can now ask: is the FST in a
final state? It is, so we know `jul` is in the map. Additionally, we can report
`value` as the value associated with the key `jul`, which is `7`.

Not so amazing, right? The example is a bit too simplistic. A map with a single
key isn't very instructive. Let's see what happens when we add `mar` to the
map, associated with the value `3`:

![A map with two elements, FST](/images/transducers/maps/map2.png)

The start state has grown a new transition, `m`, with an output of `3`. If we
lookup the key `jul`, then the process is the same as in the previous map:
we'll get back a value of `7`. If we lookup the key `mar`, then the process
looks like this:

* Initialize `value` to `0`.
* Given `m`, the FST moves from the start state `0` to `1`. Add `3` to `value`.
* Given `a`, the FST moves from `1` to `2`. Add `0` to `value`.
* Given `r`, the FST moves from `2` to `3`. Add `0` to `value`.

The only change here---other than following different input transitions---is
that `3` was added to `value` in the first move. Since all subsequent moves add
`0` to `value`, the machine reports `3` as the value associated with `mar`.

Let's keep going. What happens when we have keys that share a common prefix?
Consider the same map as above, but with the `jun` key added associated with
the value `6`:

![A map with three elements, FST](/images/transducers/maps/map3.png)

As with sets, an additional `n` transition was added connecting states `5` and
`3`. But there were two additional changes!

1. The `0->4` transition for input `j` had its output changed from `7` to `6`.
2. The `5->3` transition for input `l` had its output changed from `0` to `1`.

Those changes in outputs are really important, because it now changes some of
the details for looking up the value associated with the key `jul`:

* Initialize `value` to `0`.
* Given `j`, the FST moves from the start state `0` to `4`. Add `6` to `value`.
* Given `u`, the FST moves from `4` to `5`. Add `0` to `value`.
* Given `l`, the FST moves from `5` to `3`. Add `1` to `value`.

The final value is still `7`, but we arrived at the value differently. Instead
of adding `7` in the initial `j` transition, we only added `6`, but we made up
the extra `1` by adding it in the final `l` transition.

We should also convince ourselves that looking up the `jun` key is correct too:

* Initialize `value` to `0`.
* Given `j`, the FST moves from the start state `0` to `4`. Add `6` to `value`.
* Given `u`, the FST moves from `4` to `5`. Add `0` to `value`.
* Given `n`, the FST moves from `5` to `3`. Add `0` to `value`.

The first transition adds `6` to `value`, but we never add anything more than
`0` to `value` on any subsequent transitions. This is because the `jun` key
does not go through the same final `l` transition that `jul` does. In this way,
both keys have distinct values, but we've done it in a way that shares much of
the data structure between keys with common prefixes.

Indeed, the key property that enables this sharing is that each key in the map
corresponds to a *unique path* through the machine. Therefore, there will
always be some combination of transitions followed for each key that is unique
to that particular key. All we have to do is figure out how to place the
outputs along the transitions. (We will talk briefly about how to do this in
the next section.)

This sharing of outputs works for keys with both common prefixes and suffixes too. Consider the
keys `tuesday` and `thursday`, associated with the values `3` and `5`,
respectively (for day of the week).

![A map with two elements (suffixes),
FST](/images/transducers/maps/map2-suffixes.png)

Both keys have a common prefix, `t`, and a common suffix, `sday`. Notice that
the values associated with the keys also have a common prefix with respect to
addition on the values. Namely, `3` can be written as `3 + 0` and `5` can be
written as `3 + 2`. This idea is captured in the machine; the common prefix `t`
has an output of `3`, while the `h` transition (which is not present in
`tuesday`) has the output `2` associated with it. Namely, when looking up the
key `tuesday`, the first output on `t` will be emitted, but the `h` transition
won't be followed, so the `2` output associated with it won't be emitted. The
rest of the transitions have an output of `0`, which does not change the final
`value` emitted.

The way I've described outputs might seem a bit restrictive; what if they
aren't integers? Indeed, the types of outputs that can be used in an FST are
limited to things with the following operations defined:

* Addition.
* Subtraction.
* Prefix (i.e., find the prefix of two outputs).

Outputs must also have an additive identity, `I`, such that the following laws
hold:

* `x + I = 0`
* `x - I = x`
* `prefix(x, y) = I` when `x` and `y` do not share a common prefix.

Integers satisfy this algebra trivially (where `prefix` is defined as `min`)
with the added benefit that they are very small. Other types can be made to
satisfy this algebra, but for now, we will only work with integers.

We only needed to use addition in the above examples, but we will need the
other two operations for *building* a FST. That's what we'll cover next.

### Construction

In the previous two sections, I have been careful to avoid talking about the
construction of finite state machines that are used to represent ordered sets
or maps. Namely, construction is a bit more complex than simple traversal.

To keep things simple, we place a restriction on the elements in our set or
map: they must be added in lexicographic order. This is an onerous restriction,
but we will see later how to mitigate it.

To motivate construction of finite state machines, let's talk about *tries*.

#### Trie construction

A trie can be thought of as a deterministic acyclic finite state acceptor.
Therefore, everything you learned in the previous section on ordered sets
applies equally well to them. The only difference between a trie and the FSAs
shown in this article is that a trie permits the sharing of *prefixes* between
keys while an FSA permits the sharing of both prefixes and suffixes.

Consider a set with the keys `mon`, `tues` and `wed`. Here is the corresponding
FSA that benefits from sharing both prefixes and suffixes:

![Set of first three days of week, FSA](/images/transducers/sets/days3.png)

And here is the corresponding trie, which only shares prefixes:

![Set of first three days of week, Trie](/images/transducers/dot/days3-trie.png)

Notice that there are now three distinct final states, and the keys `tues` and
`thurs` require duplicating the final transition for `s` to the final state.

Constructing a trie is reasonably straight-forward. Given a new key to insert,
all one needs to do is perform a normal lookup. If the input is exhausted,
then the current state should be marked as final. If the machine stops before
the input is exhausted because there are no valid transitions to follow, then
simply create a new transition and node for each remaining input. The last
node created should be marked final.

#### FSA construction

Recall that the only difference between a trie and an FSA is that an FSA
permits the sharing of suffixes between keys. Since a trie is itself an FSA,
we could construct a trie and then apply a
[general minimization
algorithm](https://en.wikipedia.org/wiki/DFA_minimization),
which would achieve our goal of sharing suffixes.

However, general minimization algorithms can be expensive both in time and
space. For example, a trie can often be *much larger* than an FSA that shares
structure between suffixes of keys. Instead, if we can assume that keys
are added in lexicographic order, we can do better. The essential trick is
realizing that when inserting a new key, any parts of the FSA that don't share
a prefix with the new key can be frozen. Namely, no new key added to the FSA
can possibly make that part of the FSA smaller.

Some pictures might help explain this better. Consider again the keys `mon`,
`tues` and `thurs`. Since we must add them in lexicographic order, we'll add
`mon` first, then `thurs` and then `tues`. Here's what the FSA looks like after
the first key has been added:

![FSA construction, step 1](/images/transducers/dot/days3-fsa-1.png)

This isn't so interesting. Here's what happens when we insert `thurs`:

![FSA construction, step 2](/images/transducers/dot/days3-fsa-2.png)

The insertion of `thurs` caused the first key, `mon`, to be frozen (indicated
by blue coloring in the image). When a particular part of the FSA has been
frozen, then we know that it will never need to be modified in the future.
Namely, since all future keys added will be `>= thurs`, we know that no future
keys will start with `mon`. This is important because it lets us reuse that
part of the automaton without worrying about whether it might change in the
future. Stated differently, states that are colored blue are candidates for
reuse by other keys.

The dotted lines represent that `thurs` hasn't actually been added to the FSA
yet. Indeed, adding it requires checking whether there exists any reusable
states. Unfortunately, we can't do that yet. For example, it is true that
states `3` and `8` are equivalent: both are final and neither has any
transitions. However, it is not true that state `8` will always be equal to
state `3`. Namely, the next key we add could, for example, be `thursday`. That
would change state `8` to having a `d` transition, which would make it not
equal to state `3`. Therefore, we can't quite conclude what the key `thurs`
looks like in the automaton yet.

Let's move on to inserting the next key, `tues`:

![FSA construction, step 3](/images/transducers/dot/days3-fsa-3.png)

In the process of adding `tues`, we deduced that the `hurs` part of the `thurs`
key could be frozen. Why? Because no future key inserted could possibly
minimize the path taken by `hurs` since keys are inserted in lexicographic
order. For example, we now know that the key `thursday` cannot ever be part of
the set, so we can conclude that the final state of `thurs` is equivalent to
the final state of `mon`: they are both final and both have no transitions, and
this will forever be true.

Notice that state `4` remained dotted: it is possible that state `4` could
change upon subsequent key insertions, so we cannot consider it equal to any
other state just yet.

Let's add one more key to drive the point home. Consider the insertion of
`zon`:

![FSA construction, step 4](/images/transducers/dot/days3-fsa-4.png)

We see here that state `4` has finally been frozen because no future
insertion after `zon` can possibly change the state `4`. Additionally, we could
also conclude that `thurs` and `tues` share a common suffix, and that, indeed,
states `7` and `9` (from the previous image) are equivalent because neither of
them are final and both have a single transition with input `s` *that points to
the same state*. It is critical that both of their `s` transitions point to the
same state, otherwise we cannot reuse the same structure.

Finally, we must signal that we are done inserting keys. We can now freeze the
last portion of the FSA, `zon`, and look for redundant structure:

![FSA construction, step 5](/images/transducers/dot/days3-fsa-5.png)

And of course, since `mon` and `zon` share a common suffix, there is indeed
redundant structure. Namely, the state `9` in the previous image is equivalent
in every way to state `1`. This is only true because states `10` and `11` are
also equivalent to states `2` and `3`. If that weren't true, then we couldn't
consider states `9` and `1` equal. For example, if we had inserted the key
`mom` into our set and still assumed that states `9` and `1` were equal, then
the resulting FSA would look something like this:

![FSA construction, step 6, wrong](/images/transducers/dot/days3-fsa-6.png)

And this would be wrong! Why? Because this FSA will claim that the key `zom` is
in the set---but we never actually added it.

Finally, it is worth noting that the construction algorithm outlined here can
run in `O(n)` time where `n` is the number of keys. It is easy to see that
inserting a key initially into the FST without checking for redundant structure
does not take any longer than looping over each character in the key, assuming
that looking up a transition in each state takes constant time. The trickier
bit is: how do we find redundant structure in constant time? The short answer
is a hash table, but I will explain some of the challenges with that in the
section on [construction in practice](#construction-in-practice).

#### FST construction

Constructing deterministic acyclic finite state *transducers* works in much the
same way as constructing deterministic acyclic finite state *acceptors*. The
key difference is the placement and sharing of outputs on transitions.

To keep the mental burden low, we will reuse the example in the previous
section with keys `mon`, `tues` and `thurs`. Since FSTs represent maps, we will
associate the numeric day of the week with each key: `2`, `3` and `5`,
respectively.

As before, we'll start with inserting the first key, `mon`:

![FST construction, step 1](/images/transducers/dot/days3-fst-1.png)

(Recall that the dotted lines correspond to pieces of the FST that may
change on subsequent key insertion.)

This isn't so interesting, but it is at least worth noting that the output `2`
is placed on the first transition. Technically, the following transducer would
be equally correct:

![FST construction, step 1, alternate](/images/transducers/dot/days3-fst-1-alt.png)

However, placing the outputs *as close to the initial state as possible* makes
it much easier to write an algorithm that shares output transitions between
keys.

Let's move on to inserting the key `thurs` mapped to the value `5`:

![FST construction, step 2](/images/transducers/dot/days3-fst-2.png)

As with FSA construction, insertion of the key `thurs` allows us to conclude
that the `mon` portion of the FST will never change. (As represented in the
image by the color blue.)

Since the `mon` and `thurs` keys don't share a common prefix and they are the
only two keys in the map, their entire output values can each be placed in the
first transition out of the start state.

However, when we add the next key, `tues`, things get a little more
interesting:

![FST construction, step 3](/images/transducers/dot/days3-fst-3.png)

As with FSA construction, this identifies another portion of the FST that can
never change and freezes it. The difference here is that the output on the
transition from state `0` to `4` has changed from `5` to `3`. This is because
the `tues` key's value is `3`, so if the initial `t` transition added `5` to
the value, then the value would be too big. We want to share as much structure
as is possible, so when we identify a common prefix, we look for the common
prefix in the output values as well. In this case, the prefix of `5` and `3` is
`3`. Since `3` is the value associated with the key `tues`, its remaining
transitions can all have an output of `0`.

However, if we changed the output of the `0->4` transition from `5` to `3`, the
value associated with the key `thurs` would now be wrong. We then have to
"push" the left over value from taking the prefix of `5` and `3` down. In this
case `5 - 3 = 2`, so we add `2` to each transition on `4` (except for the new
`u` transition we added).

In this way, we preserve the outputs of previous keys, add a new output for a
new key and share as much structure as possible in the FST.

As with before, let's try adding one more key. This time, let's pick a key that
has a more interesting impact on outputs. Let's add `tye` to the map and
associate it with the value `99` to see what happens.

![FST construction, step 4](/images/transducers/dot/days3-fst-4.png)

Insertion of the `tye` key allowed us to freeze the `es` part of the `tues`
key. In particular, as with FSA construction, we identified equivalent states
so that `thurs` and `tues` could share states in the FST.

What was different here for FST construction is that the output associated with
the `4->9` transition (which was just added for the `tye` key) has an output of
`96`. It chose `96` because the transition prior to it, `0->4`, has an output
of `3`. Since the common prefix of `99` and `3` is `3`, the output of `0->4` is
left unchanged, and the output for `4->9` is set to `99 - 3 = 96`.

For completeness, here is the final FST after indicating that no more keys will
be added:

![FST construction, step 5](/images/transducers/dot/days3-fst-5.png)

The only real change here from the previous step is that the final transition
of the `tye` key is connected to the final state shared by all other keys.

#### Construction in practice

Actually writing the code to implement the pictorally described algorithms
above is a bit beyond the scope of this article. (A fast implementation of it
is of course freely available in my [`fst`](https://github.com/BurntSushi/fst)
library.) However, there are some important challenges worth discussing.

One of the critical use cases of an FST data structure is its ability to store
and search a very large number of keys. This goal is somewhat at odds with the
algorithm described above, since it requires one to keep all frozen states in
memory. Namely, in order to detect whether there are parts of the FST that can
be reused for a given key, you must be able to actually search for equivalent
states.

The literature which describes this algorithm (linked in the next section)
states that one can use a hash table for this, which provides constant time
access to any particular state (assuming a good hash function). The problem
with this approach is that a hash table usually incurs some kind of overhead,
in addition to actually storing all of the states in memory.

It is possible to mitigate the onerous memory required by sacrificing
guaranteed minimality of the resulting FST. Namely, one can maintain a hash
table that is *bounded in size*. This means that commonly reused states are
kept in the hash table while less commonly reused states are evicted. In
practice, a hash table with about `10,000` slots achieves a decent compromise
and closely approximates minimality in my own unscientific experiments. (The
actual implementation does a little better and stores a small LRU cache in each
slot, so that if two common but distinct nodes map to the same bucket, they can
still be reused.)

An interesting consequence of using a bounded hash table which only stores some
of the states is that construction of an FST can be *streamed* to a file on
disk. Namely, when states are frozen as described in the previous two sections,
there's no reason to keep all of them in memory. Instead, we can immediately
write them to disk (or a socket, or whatever).

The end result is that we can **construct an approximately minimal FST from
pre-sorted keys in linear time and in constant memory**.

#### References

The algorithms presented above are not my own. (I did, to the best of my
knowledge, come up with the LRU cache idea. But that's it!)

I got the algorithm for FSA construction from
[Incremental construction of minimal acyclic finite-state
automata](http://www.mitpressjournals.org/doi/pdfplus/10.1162/089120100561601).
In particular, section 3 does a reasonably good job of explaining the
particulars, but the paper overall is a good read.

I got the algorithm for FST construction from
[Direct Construction of Minimal Acyclic Subsequential
Transducers](http://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.24.3698&rep=rep1&type=pdf).
The whole paper is a really good read, but I had to read it about 3-5 times
over the course of a week to really let it sink in. There is pseudo-code for
the algorithm near the end of the paper, which is very readable once your brain
gets acclimated to what all of the variables mean.

Those two papers pretty much cover everything in the article so far. However,
there is more worth reading to actually write an efficient implementation. In
particular, this article will not cover in detail how nodes and transitions are
represented in an FST. The short answer is that the representation of an FST is
a sequence of bytes in memory and the vast majority of states take up exactly
one byte of space. Indeed, representing finite state machines is an active area
of research. Here are two papers that helped me the most:

* [Experiments with Automata
  Compression](http://www.researchgate.net/profile/Jii_Dvorsky/publication/221568039_Word_Random_Access_Compression/links/0c96052c095630d5b3000000.pdf#page=116)
  (Unfortunately, if you click on this link, researchgate.net will seem to
  redirect you to a very unfriendly UI. If you just want the PDF already, copy
  the link and paste it directly in your address bar. The actual article is on
  page 116 of the PDF or page 105 of the conference collection.)
* [Smaller Representation of Finite State
  Automata](http://www.cs.put.poznan.pl/dweiss/site/publications/download/fsacomp.pdf)

For an excellent but very long and in depth overview on the field, [Jan
Daciuk's disseration](http://www.pg.gda.pl/~jandac/thesis.ps.gz) (gzipped
PostScript warning) is excellent.

For a short and sweet experimentally motivated overview of construction
algorithms,
[Comparison of Construction Algorithms for Minimal, Acyclic, Deterministic,
Finite-State Automata from Sets of
Strings](http://www.cs.mun.ca/~harold/Courses/Old/CS4750/Diary/q3p2qx4lv71m5vew.pdf)
is very good.

## The FST library



<!--
Initial number of urls: 7,563,934,593
Number of unique urls: 1,649,195,774

One hour to create 544 FST of ~10,000,000 URLs each.
  Each about 128MB
Three hours to join all 544 FST's into one.
  Size: 27GB

30 minutes to write all URLs from FST to file.
  Size: 134GB

1 hour 22 minutes to create FST from sorted list.
  Max heap size: 56MB
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
