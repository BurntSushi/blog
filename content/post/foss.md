+++
date = "2020-01-19T17:15:00-05:00"
title = "My FOSS Story"
author = "Andrew Gallant"
url = "foss"

[blackfriday]
plainIdAnchors = true
+++

I'd like to break from my normal tradition of focusing almost strictly on
technical content and share a bit of my own personal relationship with Free
and Open Source Software (FOSS). While everyone is different, my hope is that
sharing my perspective will help build understanding, empathy and trust.

This is not meant to be a direct response to the behavior of any other
maintainer. Nor should it be read as a prescription on the ideal behavior of
someone in FOSS. This is meant more as a personal reflection with the hope that
others will use it to reflect on their own relationship with FOSS. There is no
one true path to being a good FOSS maintainer. We all have our own coping
mechanisms.

This is also emphatically not meant as a call for help. This is about
understanding. This is not about a plea to change the economics of FOSS. This
is not about brainstorming ways to improve my mental health. This is not about
bringing on more maintainers. It's about sharing my story and attempting to
increase empathy among the denizens of FOSS.

**Target audience**: Anyone involved in FOSS.

<!--more-->


## Table of contents

* [History](#history)
* [Damned Emotions](#damned-emotions)
* [Festering Negativity](#festering-negativity)
* [Dealing via Boundaries](#dealing-via-boundaries)
* [Rudeness](#rudeness)
* [Entitlement](#entitlement)
* [Other Thoughts on Entitlement](#other-thoughts-on-entitlement)
* [Trust](#trust)
* [Better Than It Sounds](#better-than-it-sounds)
* [Conclusion](#conclusion)


## History

My very
[first FOSS project](http://web.archive.org/web/20040523030817/http://www.webtrickscentral.com/)
was released almost 16 years ago. It was a bulletin board system written in
PHP. Pretty much everyone was building those things back then, and it was also
how I learned to program. The project originally started as a school project to
host online discussions. (This was before schools had anything to do with the
web, other than host crappy web sites.) But that quickly became less of a focus
as I ran into my very first failure of estimation. It took much longer than
one semester to build it. It turned into a labor of love beyond just a school
project.

I've personally always found writing code to scratch an itch to be
intrinsically rewarding to me. I love all phases of it. Whether it's background
research, determining feasibility, laying out my initial plan of attack,
obsessing over writing the code and even dreaming about it, I love every minute
of it.

When I write code, I don't need to share it to enjoy it. But as my involvement
in FOSS increased, it quickly became a natural part of my process that I've
mostly continued for 16 years in one form or another. At its core, the thing
I love most about it is the act of sharing my code with others in a way that
lets them solve a problem more efficiently and effectively than they would have
without it. The more utility my code gets, the greater my enjoyment is. It
generally doesn't matter to me whether it's just another hacker scratching an
itch or a giant corporation doing something interesting at incredible scales.

My FOSS history continued for several years with various releases of my
bulletin board and
[wtcSQLite](http://web.archive.org/web/20050305041726/http://www.webtrickscentral.com/wtcSQLite.php), which was a cheap clone of
[phpMyAdmin](https://en.wikipedia.org/wiki/PhpMyAdmin),
but for SQLite.

When I moved to Linux from Windows sometime around 2009, I started scratching
more itches, but with Python and X11. This included
[PyTyle](https://github.com/BurntSushi/pytyle1) for bolting window tiling onto
a stacking window manager, and
[openbox-multihead](https://github.com/BurntSushi/openbox-multihead), which
added my own flavor of support for multiple monitors to
[Openbox](https://en.wikipedia.org/wiki/Openbox). These projects, combined
with doing some research work in Go, led to me building my own
[window manager](https://github.com/BurntSushi/wingo)
in Go, which I still use today.

That brings me to about 6 years ago, which is around the time that I started
writing Rust. My first Rust library was
[quickcheck](https://github.com/BurntSushi/quickcheck),
but that was followed by a flurry of others:
[regex](https://github.com/rust-lang/regex),
[docopt.rs](https://github.com/docopt/docopt.rs),
[rust-csv](https://github.com/BurntSushi/rust-csv),
[fst](https://github.com/BurntSushi/fst),
[termcolor](https://github.com/BurntSushi/termcolor),
[walkdir](https://github.com/BurntSushi/walkdir)
and many more over the next 6 years.

While the vast majority of my Rust projects are libraries, some of them are
command line tools, such as
[xsv](https://github.com/BurntSushi/xsv)
and
[ripgrep](https://github.com/BurntSushi/ripgrep).

While many of my older projects (non-Rust) are effectively dead or maintained
by others at this point, I have, for the most part, continued to maintain most
of the Rust projects I've started. Those that don't receive maintenance have
generally been supplanted by better crates built by others. (Such as
[crossbeam-channel](https://crates.io/crates/crossbeam-channel)
supplanting
[chan](https://crates.io/crates/chan).)

These days, while I still spend a lot of time coding because I love doing it, I
_also_ spend a lot of time reviewing code, debugging issues with end users,
responding to feature requests and other such things. Invariably, this means
interacting, working and communicating with other humans.


## Damned Emotions

When I was a young adult, I'd pride myself on being "logical" and "free of
emotional decision making." Like all good lies we tell to ourselves, there's
a kernel of truth to it. But for the most part, at least for me personally, I
am a deeply emotional being.

Emotions run deep and can be a really useful well to tap for intrinsic
motivation. For example, for some time after ripgrep was released, I began to
immediately _hate_ touching the
[code that was responsible for printing search results](https://github.com/BurntSushi/ripgrep/blob/0.9.0/src/printer.rs).
It was convoluted, buggy and difficult to change. While rewriting is a
perfectly logical decision to make on purely technical grounds only, I was
_motivated_ to do it because I _didn't like the way it made me feel_. My
emotion helped drive me to make things better for myself. For example, now that
printing is de-coupled and isolated into its own
[distinct library](https://docs.rs/grep-printer)
with thorough tests, I feel a heck of a lot better any time I need to journey
into that code and do something. It's still not my best work, but it's a big
improvement---at least from an emotional perspective---over the previous state.

Emotions are funny things because they can put you into really surprising
states. Sticking with our previous example, would re-writing the printing code
on purely technical reasons alone be enough? It's a fine decision to make, but
if I'm not motivated to do it, then it might never get done. If it doesn't get
done, then the most likely outcomes are that the software stagnates or becomes
buggy, or some combination of both. If the _emotional_ reasoning can motivate
me to do it, then the rewrite could lead to a much better future where more
features are implemented without sacrificing reliability.

Emotions cut both ways. For anyone who has released and maintained some
moderately popular piece of software, you will have invariably made contact
with other humans. The impact that another person can have on your emotional
state can be staggering. A positive gesture or comment can really brighten your
day. It's that feeling: _yes, sharing my code was so worth it just to help that
one person._ But as anyone who has been a FOSS maintainer can attest, positive
comments are almost always dwarfed by negative comments.

Negative comments aren't intrinsically bad. But they are the natural
consequence of sharing your code and inviting others to use it and _report
problems_. When a bug gets reported, you feel that twang of having let that
user down. When you wrote the code, you were sure you tested it well enough,
but it was _still wrong_. Will the bug reports never end? How much time did
that user just waste because of the bug? How much time will it take me to fix
it? Forget that, how much time will take me to just context switch into a mode
where I even have a _hope_ of fixing it?

These thoughts can encourage emotions that will eat away at you. And these are
pretty much the best case scenario when it comes to negative comments.


## Festering Negativity

I quickly learned to get over the feelings of inadequacy after a bug report
was filed. Indeed, _good_ bug reports with easy reproductions quickly turn
into _positive_ things because they tend to be so rare. Most bug reports lack
reproductions at all, even when you provide an issue template that explicitly
asks for one. The submitter probably means well, but there's just not enough
information to make the bug actionable. And so begins the back-and-forth to
determine how to isolate the bug.

For me personally, this is an area where I struggle the most. My emotions get
the best of me, because all I can think is: _why didn't this person take the
time to read and fill out the issue template?_ Or, in cases where bugs are user
errors that could be resolved by just reading the documentation, all I can
think of is: _I spent this time gifting this user some software, but they can't
even read the README before filing a bug report?_

It can be maddening. But that's emotions for you. They certainly aren't always
rational. The documentation could probably be clearer. Or the user could have
just missed that part of the documentation. Or the user doesn't have experience
maintaining FOSS projects or filing bug reports and maybe does not know how to
provide an easy reproduction. These are all perfectly reasonable things to have
happen, and it's why I do my best not to let my emotions get the best of me.
While the way I feel is important, empathizing with the person on the other end
of the wire is important too.

In particular, while I never write the words, "I invite you to use my code,"
there are a _ton_ of things I do _only_ because my intent is for others to use
my code. I write more thorough documentation than I would otherwise. I write
examples for others to follow. I set up continuous integration testing. I write
a README that usually explains how to get started. I share a link to my project
with others in various places on the Internet. If people accept this invitation
to use my code, or an invitation to file bugs by keeping the issue tracker
open, then I should also do my best not to punish them when they do. When poor
issues are filed, the reporter probably thinks they did the best they could.
And so long as they are filed in good faith, I really do try to respond in
kind.

This underscores the asymmetry of maintainers and users. For many users who
file bug reports, they might have one or two interactions with me. To them, a
single poorly written bug report isn't a big deal. But I'm on the wrong end
of this deal, because this plays itself out over and over again across all my
projects. All the time. Almost every day. Empathizing in this scenario can be
extraordinarily difficult, especially if you are already have a bad day. Which
happens sometimes.

Sometimes I let my impatience show through with curt replies. I am trying hard
to be better about this. It's a work in progress.


## Dealing via Boundaries

One of the things that comes from maintaining not just one popular project, but
several, is that there is an almost constant stream of bug reports and pull
requests coming in daily. Keeping up with it is almost impossible. My brain's
cache size is unusually small, so my ability to context switch between projects
is generally pretty poor. The general result of this phenomenon is that
projects I've touched recently tend to get its issues and pull requests dealt
with more quickly, since the project is probably mostly paged into my brain.

But other projects begin to pile up with issues and pull requests. The inbox
gets longer. Each day you see a new issue come in, and you say to yourself,
"yes, I will really look at it this time once I get home from work." But more
just keep coming in. Eventually you work up the motivation to context switch
back into that project because That User has pinged you once a month for four
months and their pull request is probably Really Important.

Sorry, that last sentence had a bit of snark in it, but it's also sincere. The
asymmetry of users and maintainers strikes again, but I do genuinely want to
clear the pull request queue and keep the project moving. I want to bring in
That User's contribution because I not only want them to keep using my code,
but I want them to be _happy_ about it too. In many cases, it might only take
me an hour or so to work through the pull requests and actionable issues.

But those 4 months weren't pleasant because I felt bad seeing those issues
languish in my inbox.

The solution that I've adopted for this phenomenon is one that I've used
extremely effectively in my personal life: establish boundaries. Courteously
but firmly setting boundaries is one of those magical life hacks that pays
dividends once you figure out how to do it. If you don't know how to do it,
then I'm not sure exactly how to learn how to do it unfortunately. But setting
boundaries lets you focus on what's important to _you_ and not what's important
to _others_.

Obviously, a balance must be struck. Setting boundaries doesn't mean you get to
focus only on what's important to you to the exclusion of everyone else 100% of
the time. But the ability to put up that wall and say, "No, I'm not doing X but
I'd be happy to do Y" has really done wonders for me. The secret, for me, is to
give reasons that are impossible for others to argue with by grounding them in
your own experiences and preferences.

So what does this have to do with FOSS? The key, for me anyway, was being able
to put up a boundary between myself and unattended issues and pull requests. I
had to find a way to say to myself: "I am volunteering my time and it is okay
if I don't respond in a timely manner. I trust that most other people will
understand this and be reasonable about it."

Another dimension of this appears through feature requests. Sometimes a feature
request might generally make sense for your project, but the maintenance burden
it implies could be large. I taught myself to set boundaries: it's okay to say
no to a feature solely on the grounds that you don't want the added maintenance
that comes from it. As has happened with me many times, you might change your
mind in the future! For example, if the relevant code improves to become more
maintainable, then you might find your willingness to adopt more features
increase. But if not, then I do my best to recognize my boundaries and decline
to give myself more work that is emotionally unfulfilling.

I wish I could write down the process I went through that allowed me to set
firm boundaries and stop feeling bad about issues piling up. It doesn't
alleviate the bad feelings completely, but it goes a long way.


## Rudeness

The obvious trolls are generally pretty easy for me to deal with, assuming
their volume isn't too high. Low effort trolls are just other people with
an obvious agenda to try to make you feel bad. Trolls generally don't have
anything invested and so their commentary has little weight. Or at least,
that's what I say to myself as a coping mechanism. Typically, I deal with
trolls by reporting them to GitHub staff and blocking them. In general, I
appear to be fortunate in the sense that I deal with these sorts of trolls
very infrequently.

Rudeness, on the other hand, comes in all shapes and sizes. My emotions compel
me to have a fairly rigid sense of decorum, so some might not consider all of
these things rude. But I do. Or at least uncouth.

* "Your tool doesn't work [for my niche use case], therefore it is broken."
* "Just chiming in to say that I would also really like this feature."
* Insisting that implementing a feature is "_just_ a simple matter of doing X."
* Passive aggressiveness when you opt to pass on a feature request.
* Unconstructively whining about software on [insert social medium here].
* Some low effort variation of "why are you reinventing the wheel" or
  "why not contribute to an existing project instead."

In many cases, rudeness is the result of genuine frustration on behalf of the
user. How many times have your cursed under your breath when a tool you were
using didn't behave like you think it should? It doesn't matter that the tool
was probably gifted to you for free. You're just trying to solve a problem and
the tool is getting in your way. I've certainly felt this way, and in my
opinion, it seems like a totally normal human emotion to have.

Sometimes this rudeness gets the better of us and ends up being expressed in
less than productive ways. I know I've certainly done it, and I've certainly
been on the receiving end of it as well. It's incredibly frustrating for all
those involved.

In other cases, some people are rude without knowing it. This could be because
of a language barrier, or because they just weren't aware of how their words
might make someone else feel. It's totally innocent, but it doesn't change how
it makes me feel when I'm on the receiving end of it.

Tackling this sort of rudeness can be really difficult. You might be someone
who is unaffected by it. I am not one of those people. I could _pretend_ I'm
unaffected by it, but I'm pretty sure that would lead to resentment towards
FOSS and more frustration.

This is where setting boundaries has helped me again. Again, putting aside
trolls, the vast majority of people who are rude generally turn out to respond
fairly well if you politely call them out on it. I've done it many times on my
issue trackers, and it has generally improved the situation. I don't feel
resentment because I'm doing something to defend myself, _and_ I feel better
when the other person apologizes, which is the case the vast majority of the
time.

Doing this can be as simple as, "I don't appreciate the way you said X. I think
it would be much more productive if we left that sort of thing out in the
future."

Now, in some cases, folks don't respond well to this. In my experience, they
usually ignore it. If they keep on being rude, I might repeat myself a couple
times, because sometimes folks need to hear something more than once for it to
sink in. At least, I know I sometimes do (much to the displeasure of my wife).
If this still doesn't work, and I am still bothered by how they're talking to
me, then I end the interaction. It might be as easy as closing or locking an
issue/pull request, or might be as extreme as blocking them on [insert social
medium here].


## Entitlement

A long time ago, I was talking to some of my closest friends after they had
traveled abroad. They had just recently come back to the United States and
shared a small story of culture shock. The punch line?

> I had never realized how much Americans like to **should** you to death.

Now, whether this is actually a property of American culture---or perhaps a
property of the company we keep---is not a point I wish to belabor. The point
is that, as humans, we love to talk about what other people _should_ be doing.
I grew up on the receiving end of this---especially from people in various
positions of authority---and have a really innate distaste for it.

I'm pretty convinced that most people don't even realize they're doing it. Or
more charitably, they're probably not trying to inject themselves into your
life to tell you that they know better, but rather, are just trying to offer
advice. At least, that's what I'm told if I call people out in particularly
egregious cases of being should'ed.

Backing up a bit, using the word "should" isn't necessarily bad on its own. One
thing that I think really changes its dynamic is whether it's _invited_ or not.
If you ask someone for advice on a topic, and they use phrases like "yeah you
should do X," then it doesn't quite sound as bad. But when it's uninvited, it
has a completely different feeling to it.

I've seen or experienced this in FOSS in a number of different ways:

* You should put out a new release.
* You should rewrite this in [insert programming language here].
* You should rename your project.
* You should [insert major architectural change here].
* You should change the license of your project.

The almost universally common thread here is the drive-by low-effort nature of
the advice. The advice might actually be something that's a really good idea.
But there's a certain entitlement that's showcased here's that's hard to
overlook when someone spends so little time making a suggestion that has
potentially _huge_ ramifications for your project. Thoughtful advice is almost
always welcome from my perspective, but when someone thoughtlessly tells me I
_should_ do something that would imply me spending lots of time on it, it can
be really grating.

While I still haven't mastered my ability to respond to this sort of
commentary, I do my best to continue to establish boundaries. I have two coping
strategies for this:

* For particularly common ones, like "when is the next release?", I declare
  that my free time is unscheduled. It helps to put it in a
  [FAQ-like document](https://github.com/BurntSushi/ripgrep/blob/master/FAQ.md#release).
* Otherwise, I try to apply the principle of proportion. If you give me one or
  two sentences thoughtlessly asking for something huge, then I'm only going to
  spend
  [one or two sentences in response](https://github.com/BurntSushi/ripgrep/issues/1323).

To reiterate, this type of commentary can sometimes lead to productive things
happening. For example, when I first started open sourcing projects in Rust,
I used the [UNLICENSE](https://unlicense.org/) exclusively. On one occasion,
I got a drive-by comment effectively telling me to use a "working" license
instead, along with some (what felt like) condescending lecture on how
licensing works. I didn't respond well to this and was incredibly frustrated by
it. It turned out the general advice was good, however, it wasn't until someone
else more thoughtfully brought it up that I actually decided to act on it.

In retrospect, it could seem like I was being petty. Like I was refusing to do
something that was better just because I didn't like the commenter's tone. But
that wasn't how I _lived_ it. Since I immediately took the defensive, my
emotions just did not let me think clearly about it.

The lesson here is that being thoughtful in one's communication is important to
advance your cause. If you're thoughtless, _even if you're correct_, you risk
working against your own ends because the _person_ on the other end might not
be able to look past your thoughtlessness.


## Other Thoughts on Entitlement

I don't think I've seen anyone (other than obvious trolls) sincerely claim a
real entitlement to my labor. That is, I've never had to actually quote the
"AS IS" warranty disclaimer in my licenses. Laws are not often good tools to
explain social norms. As a maintainer with open issue trackers, I am implicitly
inviting others to file bugs. At some level, even the act of opening a bug is a
form of entitlement, since there's some expectation---or perhaps hope---that by
reporting the bug, it will get fixed and benefit everyone. Indeed, that is my
intent with having an open issue tracker: I want people to file bugs and submit
pull requests, with the hope that they will get fixed and merged.

There is no legal relationship here. There is nothing in my licenses that say
I ought or have to do this. At least for me, it's an implied agreement among
humans acting in good faith.


## Trust

And that brings us to trust. Trust is an important value in FOSS. Not only do I
do my best to be discriminating in who I trust, but I also try to act in a way
that allows others to trust me.

One of the benefits of FOSS is its decentralized nature. You have tons of
people working in their own little corners with their own little specialties.
Using FOSS has an amplifying effect, because it lets you build on what tons
of us have already done. It absolves you from needing to build literally
everything you need, and instead lets you start focusing on solving your
particular problem more quickly.

As someone who uses FOSS and tries hard to be discriminating with the
dependencies I use, it is just not possible for me to review every line of code
I depend on. Even if I could somehow manage to _read_ it all, I certainly
wouldn't be able to _understand_ it all in enough detail to be confident that
it was doing what I thought it was doing.

This is where trust plays a huge role. Trust serves as a proxy for evaluating
some dimensions of the code I use. It helps me answer questions like:

* Is there a reasonable expectation that the code will behave as advertised?
* Will bugs be fixed in a reasonable time frame?
* Will the project continue to be maintained going forward?
* Does the project use good judgment when it comes to balancing competing
  concerns?

These are hefty things to levy upon a FOSS maintainer that performs their
duties in their free time. Regardless, these are table stakes for being a
trustworthy maintainer. Now, I do not need to use dependencies exclusively
from maintainers that I trust. That wouldn't be practical. Instead, trust is
just another criterion I use to evaluate which code I use. If the code is
written by someone I trust, then I'm much more likely to bring in a library
written by them that tries to solve a hard problem, or otherwise tries to walk
a fine line when it comes to balancing trade offs.

For example, I might not be willing to use a JSON parsing library written
by someone that I don't know that also used questionable performance
optimizations. But I could be convinced to overlook the lack of trust by
either reviewing the code myself, and/or the documentation for the project was
excellent. Still, it's a risk.

Either way, as a FOSS maintainer, I _want_ to be seen as someone who is
trustworthy. That is, I _care_ about my reputation. This is dangerous business
in this day and age, since social media is able to destroy a reputation
almost instantly. I'd be lying if I said that wasn't a constant fear gnawing at
the back of my mind. But it's important, for me, not to let fears like that
prevent me from doing what I love.

Having people trust me as a programmer is an enormous responsibility and one
that I do not take lightly. But that trust means others are going to be more
willing to use my code, which is ultimately what I want through my
participation in FOSS.


## Better Than It Sounds

So far, I've focused a lot on the negative. Any reasonable person might ask,
"why do you subject yourself to this?" In fact, the vast majority of my
communication with others in FOSS is fairly neutral. There's a good amount of
overtly positive communication as well. And when negativity arises, _most_
folks are quick to apologize when I enforce my boundaries. On one occasion,
someone felt bad enough to send me a gift card (which I donated) along with an
apology.

In order to be concrete, here are some of the things I enjoy about being a FOSS
maintainer:

* Hearing how people are using my code. And especially, hearing about how it
  helped them. I think my favorite anecdote was, "Yeah, we dropped your library
  into production and it pretty much just worked. No complaints."
* Getting a good bug report with an easy reproduction.
* Getting a good bug report that is hard to get a reproduction for, but with a
  reporter that is eager to help me debug with them. The best cases are almost
  like an asynchronous pair programming session, each trying to solve a
  mystery.
* The feeling of updating a changelog, no matter how small, just before a
  release. It's nice reflecting back on the work that has been done, not just
  by myself, but by others.
* While my time constraints usually prevent me from doing this, I love
  mentoring eager learners, no matter the experience level. I wish I could do
  this more.
* When contributors help me find
  [simple solutions to thorny problems](https://github.com/BurntSushi/ripgrep/issues/196#issuecomment-362854721).
  This happens a lot more than you might expect, and it's lovely when it does.
* Writing regression tests. There's nothing like encoding the knowledge that a
  user's bug cannot re-appear.

I think what ends up happening---and this isn't exactly an original
insight---is that the negativity eclipses the neutral and the positive. The
negative interactions stick in my memories and make it difficult to remember
all the good that has come from being a FOSS maintainer. Even sitting down and
writing the list above helped me remember how much positivity there is in FOSS.
It was almost cathartic.


## Conclusion

Being a FOSS maintainer has given me a lot of interesting experiences. Some
bad, some good. I've tried to express some of those experiences in this article
with the goal of helping everyone understand each other better. This article
doesn't necessarily generalize because these experiences are told through _my_
perception of the world. For example, my individualist perspective on life
greatly colors how I perceive FOSS. Namely, it's largely a _personal_ endeavor
for me, rather than a more altruistic attempt at improving a public good. A
different perspective could greatly change how one experiences FOSS.

My hope is that others will use these experiences to reflect on their own and
perhaps the experiences of others. I think this process can lead to greater
empathy and an overall better experience for everyone.

In this article, I listed a lot of behaviors that I considered negative. Not
everyone will see them as negatively as I do. _That's okay and expected._ More
to the point, I am certainly guilty of committing some of those negative
behaviors myself. We are not perfect and we will never be able to be purely
empathetic 100% of the time. This is a game of inches and my hope is that we
can do better, even if it's just a little bit.
