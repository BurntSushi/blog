+++
date = "2020-01-27T17:55:00-05:00"
title = "Archlinux on the System76 Darter Pro"
author = "Andrew Gallant"
url = "system76-darter-archlinux"

[blackfriday]
plainIdAnchors = true
+++

This is a quick post reviewing my Archlinux setup on a System76 Darter Pro
(model: darp6) with Coreboot, along with some thoughts about the laptop in
general. This is my first laptop upgrade since I
[purchased a ThinkPad T430 in July 2012](/lenovo-thinkpad-t430-archlinux)

Target audience: Archlinux users looking for a compatible Linux laptop.

<!--more-->

## Specs

* i7-10510U (quad core, eight threads, 8M cache, 1.8GHz w/ turbo up to 4.9GHz)
* 15.6" 1920x1080 IPS display
* Integrated Intel graphics, no discrete graphics card
* 32GB memory
* Samsung SSD 970 EVO Plus 500GB (nvme)
* Weight: 3.6 lbs


## Why the Darter?

I had always assumed that my next laptop would just be another ThinkPad. The
T430 was generally great for me. All the hardware worked. Suspend/resume was
flawless. The laptop ran reasonably cool. I extended its life by being able to
purchase replacement batteries, add RAM and an SSD. Battery life was decent.

So when it came time to buy a new laptop, I went to Lenovo's web site... and
literally could not find how to customize and purchase a laptop. I was able to
customize a T495, but I wanted an Intel chip, and the T495 appears to come with
an AMD chip. The T490 was on sale with an Intel chip, but could not be
customized beyond 16GB of memory. The T590 was also an option, but was
"temporarily unavailable."

At this point, I started looking for other options. I briefly considered a Dell
XPS, since I've heard good things about it, but my friend got one and had lots
of problems with it. Plus, I had a Dell XPS before my ThinkPad that I don't
have fond memories of. It ran rather hot. Granted, it was a M1530 and plenty
of things could have improved for the better since then. Nevertheless, the XPS
options aren't great. The XPS 13 Developer Edition is too small for my tastes,
and only goes up to 16GB of memory. The XPS 15 has no developer edition, and
the only way to go above 16GB of memory is to get one with a bundled NVIDIA
chip, which is a hard pass for me, especially in a laptop.

System76 is something I've always had my eye on. I've known others who've had
System76 laptops and have generally been happy with them. I was also attracted
to the fact that their laptops are specifically built for Linux, which was
obviously pretty important to me, since such things can generally be such a
crapshoot.

I still saw getting a System76 laptop as somewhat of a risk, since surprisingly
little is written about them and how well they work with Archlinux. Either this
meant few people ran Archlinux on them or people ran Archlinux on them and just
didn't have any issues. On top of that, there was surprisingly little written
about the Darter Pro in general. There were a couple YouTube reviews, but none
of them really gave me the full picture. Even something as simple as, "does it
run hot on your lap" wasn't covered. I also couldn't find answers to questions
like, "can the trackpad be clicked?"

Regardless, this was a measured risk. Since System76 specifically builds their
laptops with Linux compatibility in mind, I figured it would probably be okay.
But just because something works on Ubuntu doesn't mean it will just magically
work out of the box on Archlinux. Hence, I'm writing this article.


## Progress

One of the nice things about waiting so long to upgrade your laptop is that you
really get to notice the technological progress. For example, while my
ThinkPad's screen was 1.6" smaller than the Darter's, the width and height of
the two laptops are comparable in addition to the Darter being about 25%
lighter. Granted, my T430---even eight years ago---wasn't exactly the slimmest
thing on the market, but still, the jump to a noticeably smaller form factor
was nice.

With that said, the Darter still maintains enough thickness to fit
standard USB ports, a microphone jack and a full size ethernet port. I
very much appreciate that I don't need to use any dongles and would be quite
happy if this were the thinnest laptop I ever owned.

I also upgraded the resolution from 1600x900 to 1920x1080, which I was okay
with due to the increase in screen size. I specifically wanted to avoid
anything higher than that because, in my experience, Linux environments just do
not handle HiDPI that well. Moreover, since I run my own window manager, I am
not too keen on adding support for HiDPI to it.


## Pop!_OS

I requested to have `Pop!_OS` installed on my laptop, although I never had
any intent to use it. For the most part, I just wanted to try it out. The
onboarding experience was actually really great. It prompted me to encrypt my
disk, which I did, and it took me quickly through the rest of the installation.
If I were normally an Ubuntu user, I would have strongly considered sticking
with `Pop!_OS`. Alas, I've been spoiled by a rolling release distro for over 10
years, and I don't see that changing any time soon.


## Software that I typically use

I live most of my life in a terminal. I don't game. I use Firefox and watch the
occasional YouTube video. I do not use a desktop environment, and instead use
my own [homegrown window manager](https://github.com/BurntSushi/wingo).
Moreover, my laptop is not my primary workstation. It's mostly used if I'm on
the go or would rather work somewhere other than my home office.

For wifi, I used wicd for years, but have somewhat recently (in the past couple
months) migrated to NetworkManager. I got tired of wicd occasionally crapping
out on me or failing to connect with poor failure modes. NetworkManager
doesn't seem to be that much better, but it does seem to connect more quickly
than wicd did.

Otherwise, that's pretty much it. The only other GUI applications I run are
`xsane` occasionally. I used to run `konversation` for IRC, but all of the old
IRC channels I used to hang out in are either dead or dying.


## Installation

I've installed Archlinux perhaps dozens of times over the years. I've done it
enough that I've
[scripted most of it](https://gist.github.com/BurntSushi/b76e2f161ed98797f88a0811c492b98c),
which drastically reduces installation time. Installing Archlinux on the Darter
Pro pretty much went without incident. I encrypted my root filesystem via LUKS
and that all worked great too.

One of the things that was a somewhat unknown to me was Coreboot. I had never
used it before. It wasn't a major selling point to me, but it otherwise did not
impact the installation process. It was a normal EFI install.


## X11 woes

I had thought the days of writing X11 configuration files were behind me, but
that is apparently not the case. My hope is that this is due to the CPU being
so new
([released in Q3 2019](https://ark.intel.com/content/www/us/en/ark/products/196449/intel-core-i7-10510u-processor-8m-cache-up-to-4-90-ghz.html)),
and that there are still perhaps bugs in the drivers.

In any case, when I first started X, it didn't quite work right. It's pretty
hard to explain, but basically, I was presented with a black screen along with
my cursor. If I tried opening a terminal via a key combination, I could tell
that it worked since my cursor would change to a text selection tool when it
hovered over the terminal. But I could not see the terminal.

I tried various configuration knobs in a
`/etc/X11/xorg.conf.d/20-intel.conf`
config file, and I got varying results. In some cases, I would get lots of
weird screen artifacts, but could otherwise see most of everything. In other
configurations, my web browser would work perfectly fine but my GPU accelerated
terminal emulator ([alacritty](https://github.com/alacritty/alacritty)) would
have a semi-garbled display. I eventually settled on this configuration:

```
Section "Device"
  Identifier "Intel Graphics"
  Driver "intel"

  Option "AccelMethod" "sna"
  Option "TearFree" "true"
  Option "DRI" "false"
EndSection
```

And everything seems to work OK. I got most of these tips from the
[Intel Graphics](https://wiki.archlinux.org/index.php/Intel_graphics)
page of the Archlinux Wiki. I don't know if this is the minimal configuration,
but I'm happy to leave well enough alone, although I do wonder what I'm missing
out on by disabling DRI. On the bright side, YouTube videos play fine in 1080p
without tearing. I've had to do battles to get that to work in the past.

When I tried `Pop!_OS`, X11 worked fine. It is likely it was using an older
Linux kernel (I'm on 5.4.14) which maybe could have impacted things? (I could
try an LTS kernel on Arch, but haven't bothered yet.)

I did look at X's logs but nothing stood out at me.


## Driver support

The screen brightness keys worked in `Pop!_OS` but did not work in Archlinux
out of the box. I solved this relatively easily by adding the following to my
`.xbindkeysrc`, using the `xbacklight` package:

```
"xbacklight -inc 20"
  XF86MonBrightnessUp

"xbacklight -dec 20"
  XF86MonBrightnessDown
```

The keyboard backlight also seemed to work without needing to install any
drivers. But the keyboard backlight does not appear to have any memory.
Instead, I have to fiddle with its brightness and color every time I wake the
machine or reboot it. I was hoping that installing the system76 drivers from
the AUR would fix that, but alas, no dice. I ended up installing these (along
with `linux-headers`):

```
$ pacman -Qs system76
local/firmware-manager-git r152.df334ae-1
    Generic framework and GTK UI for firmware updates from system76-firmware
    and fwupd, written in Rust.
local/system76-acpi-dkms 1.0.1-1
    System76 ACPI Driver (DKMS)
local/system76-dkms-git 109-1
    The system76 driver kernel module (DKMS)
local/system76-driver-git 19.10.1.r0.g966d6c0-1
    System76 Driver for System76 computers
local/system76-firmware-daemon-git 1.0.7.r4.g4acf0ee-1
    System76 Firmware Daemon provides a daemon for installing firmware updates.
local/system76-io-dkms 1.0.1-1
    This DKMS driver controls System76 IO devices
local/system76-oled 0.1.3-2
    Control brightness on System76 OLED displays
local/system76-power-git r234.a76dbae-1
    System76 Power Management
```

It's not clear exactly what I'm getting out of these things. I may start
selectively uninstalling this stuff and see what breaks.


## Suspend/resume

Suspend at least always seems to work.

Resume on the other hand has been hit or miss. When it works, it's great.
WiFi reconnects in less than 5 seconds after opening the lid. This is another
difficult to describe issue like my X11 woes above. But basically, when I
resume the laptop from suspend, the screen comes on, but the entire machine is
completely unresponsive. Neither the keyoard nor trackpad work, and the actual
machine itself appears completely stuck. After _about_ a minute, the machine
becomes unstuck and resumes normal operation.

Now when I say "stuck," I don't just mean that I can't interact with it. The
actual machine itself appears stuck. Namely, if I leave a terminal open with
`dmesg --follow` running when I suspend the machine, then its output is also
paused when I resume the machine until about a minute passes and the machine
becomes unstuck. At which point, the output of `dmesg` resumes. (And `dmesg`
doesn't appear to show anything interesting related to my problem.)

I reliably reproduced this issue for at least a couple hours and spent quite a
bit of time searching for solutions and trying all sorts of things:

* I tried changing `/sys/power/mem_sleep` to `s2idle`. (It was originally
  set to `deep`.) `s2idle` apparently uses less power but wakes marginally
  quicker. The same issue persisted.
* I tried different combinations of suspending/waking the machine, whether via
  closing/opening the lid, or using the `Fn+F12` shortcut. I was able to
  reproduce the issue via all combinations, including with different time
  intervals between them.

One thing that popped up a few times while searching for solutions to this
problem was that flashing the firmware seems to have fixed it for some folks,
even if the firmware wasn't an upgrade. So that's when I decided to try
[System76's firmware manager](https://github.com/pop-os/firmware-manager).
The binary name for this is strangely `com.system76.FirmwareManager`. Upon
running it, it did seem to find some firmware updates. But when I went to try
to install it, I got an error: "failed to find mount: /boot/efi"

![calling Schedule method failed: "failed to find mount: /boot/efi"](/images/system76-darter-archlinux/firmware-manager-mount-failed.png)

I figured the problem here was that it was expecting my boot partition to be at
`/boot/efi` instead of where I put it, `/boot`. I thought there must be some
way to override this setting, but it turns out that it is
[hard coded](https://github.com/pop-os/system76-firmware/blob/dde2142ccfc6aeed90873d639b38049303385967/daemon/src/main.rs#L29-L30).
So I forked the firmware daemon,
[patched it to point at `/boot` instead](https://github.com/pop-os/system76-firmware/compare/master...BurntSushi:master)
and reinstalled the `system76-firmware-daemon-git` AUR package, but edited the
`PKGBUILD` to point at my repo instead. (Thank you Archlinux for making this
kind of patching so easy!)

Finally, I re-ran the firmware update tool, but now I get this error in the
console output:

```
[ERROR] firmware_manager src/lib.rs:330: fwupd client error: unable to ping the dbus daemon
```

and no firmware updates appear.

After more poking around, I finally found `system76-firmware-cli`. I ran

```
$ system76-firmware-cli schedule
```

and it grabbed the latest firmware and scheduled an update. I rebooted and the
firmware installed successfully. Yay. Why isn't this method more visible in the
documentation for updating firmware? I only found it by poking around. I don't
think I saw this recommended anywhere else.

Unfortunately, even after that little journey, I was still able to reproduce
the suspend/waking problem. I re-traced my steps with the above ideas a few
more times, but nothing seemed to work.

At this point, I filed a support ticket with System76 and doubled down on
debugging while I waited for a response. It turns out that
[Intel has a great guide](https://01.org/blogs/rzhang/2015/best-practice-debug-linux-suspend/hibernate-issues)
for debugging suspend/resume issues. One of the helpful tips in that post was
to use a tool to _profile_ the kernel during a suspend/resume cycle. It was
easier than I expected it to be. All I did was add `initcall_debug` to my
kernel boot parameters, restarted and then ran the following:

```
$ git clone https://github.com/01org/suspendresume.git
$ suspendresume
$ sudo ./analyze_suspend.py -rtcwake 10 -f -m mem
```

This put my laptop through a suspend/resume cycle and then collated the data
it collected (which took a bit of time). When it completed, it dumped a few
files into a directory `./suspend-xxxxxx-yyyyyy`. It contains the relevant
dmesg and ftrace raw data, along with a helpful HTML view of the callgraph.

You can see an examples of the callgraphs (warning: the HTML files are a couple
hundred MBs big):

* [Callgraph without the freeze issue.](https://burntsushi.net/stuff/system76-darter-archlinux-resume-freeze/suspend-200125-101101/krusty_mem.html)
* [Callgraph with the freeze issue.](https://burntsushi.net/stuff/system76-darter-archlinux-resume-freeze/suspend-200125-103047/krusty_mem.html)

The problem should _immediately_ jump out at you: in the latter case, the
kernel spent about 85 seconds trying to talk to my thunderbolt port. That 85
seconds lines up almost exactly with how long my system would freeze for.

This gave me more things to search for, unfortunately, I didn't find much.
The only relevant bit I found was this
[bug report against Linux 5.3](https://bugs.launchpad.net/intel/+bug/1843790),
but was seemingly fixed. (I am on Linux 5.4.14.)

Thankfully, I have never owned a Thunderbolt device and have no immediate plans
to change that. So I tried disabling Thunderbolt support altogether:

```
$ sudo sh -c 'echo "blacklist thunderbolt" > /etc/modprobe.d/blacklist.conf'
```

(And then reboot. Or `sudo rmmod thunderbolt` if you don't want it to be
persistent.)

And so far, that seems to have done the trick. I haven't seen the freeze issue
occur since I've done this.

If it turns out that this issue reappears and I can't fix it, then I'll
probably have to return this laptop. Flawless suspend/resume is table stakes.
With that said, this very well could be a kernel bug from the looks of things.


## General Thoughts

I think the above about covers the Archlinux specifics for this laptop. Other
than the X11 issue and the wake issue, everything works great.

As far as the laptop itself goes, I generally like it. It feels sturdy but is
much slimmer and lighter than my previous laptop. The touchpad feels great,
although I've been spoiled by a clickable touchpad at work. I love keyboard
backlights and this one is great, although the lack of memory/programmability
(AFAIK) kind of stinks. The keyboard itself feels great to type on, and while
there is some "flex" in the middle of the keyboard, it is not remotely
noticeable while typing. I can only notice it when I try to press down firmly,
and even then, it is very slight. I'm not too impressed with the battery life
so far, but it appears serviceable. It's at least as good as my ThinkPad. The
laptop doesn't get too warm on my lap unless I'm pegging the CPUs; again, it
seems on par with the ThinkPad. Rounding out the good parts, I like the screen.
It's big, bright, crisp and the bezels are small.

There are definitely some things that I don't like about this laptop though.

1. I very much miss the dedicated volume keys on the ThinkPad. I want to
be able to adjust the volume quickly and without looking. Having to use the
`Fn` key does not make that easy. I may wind up just adding some new keybinding
for this to work around it.

2. As I mentioned above, the fact that the keyboard backlight doesn't
have any memory is fairly annoying. I'd be happy if it was programmable in some
way (I don't think it is?), because then I could add memory myself pretty
easily and in a way that is customized to my liking.

3. The keyboard layout is... not great. It is shifted over to the left because
of the number pad, and this was mentioned as a complaint in some of the scant
reviews that I read about this machine. This shift is actually something I
don't mind too much; I've mostly already gotten use to it. The thing that I
don't like is the placement of keys like `Page Up`, `Page Down`, `Home` and
`End`. I use those keys a lot, and I really appreciated their location on my
T430's keyboard. The `Page Up` and `Page Down` keys are so far away that I have
to pick up my hand and move it to the right to reach them.

4. Because the keyboard is shifted over to the left, the left side of my right
hand tends to come down and touch the touchpad quite a bit. Now, I have
libinput's "disable touchpad while typing" feature enabled, and it does
actually work pretty well. But still, my hand will occasionally graze the
touchpad while my hands are resting on the keyboard, which will either cause
the cursor to move (mildly annoying) or accidentally click something (very
annoying) because I have tap-to-click enabled. So far, this hasn't been
frequent enough to be a deal breaker for me, but it's not pleasant. I'd
probably opt for removing the number pad and shifting the keyboard over and/or
making the touchpad smaller.

5. The touchpad's scroll sensitivity isn't sensitive enough and it doesn't
appear configurable through libinput. I don't know enough to to know whether
this is a hardware thing or a software configuration thing. But basically,
whenever I use two finger scrolling, my fingers have to move a bit before
scrolling actually starts. I'd rather scrolling start sooner. I'd also rather
that scrolling be faster without having to change my pointer sensitivity. On
the bright side, this delay is consistent and predictable, so it doesn't
actually drive me nuts.

6. There is no numlock led light as far as I can tell. Which makes it
impossible to know its current status at a glance. I may wind up adding
something to my custom status bar (via `dzen2`) that indicates the current
state.

After using this laptop more, I've found the random clicks caused by my hand
while typing (or while my hand is resting on the keyboard) to be very annoying.
For now, I've disabled tap-to-click. The pointer still moves occasionally, but
at least I no longer get spurious clicks. I created a quick script to toggle
tap-to-click for when I want it back:

```
#!/bin/sh

device="SynPS/2 Synaptics TouchPad"
prop="libinput Tapping Enabled"
current="$(
  xinput list-props "$device" | rg "${prop}[\s()0-9]+:\s+([0-9]+)" -or '$1'
)"
if [ "$current" = "0" ]; then
  xinput set-prop "$device" "$prop" 1
else
  xinput set-prop "$device" "$prop" 0
fi
```

And then bound it to a keyboard shortcut via `xbindkeys`.


## Conclusion

And I think that pretty much covers it. None of the negatives in the previous
section are enough to be deal breakers for me, mostly because there just aren't
an infinite number of choices on the market. However, if the suspend/resume
issue comes back and can't be resolved, then I don't see myself living with
that. I'd rather go back to using my old T430.
