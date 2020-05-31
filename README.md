# Sonarigo

A sampling instrument right now playing (some) SFZ samples.


## Introduction

The project aims to provide a LV2 plugin to use SFZ instruments in a LV2
host. Right now one needs external applications like LinuxSampler which rather
complicates things.


## Status

The development is in an early stage. Only the features needed for the
[Salamander Grand Piano](https://sfzinstruments.github.io/pianos/salamander).

There is a rudimentary jack application and a rudimentary LV2 plugin.


## Installation

Tricky. The thing is written in Rust, so you at first need to have a running
rust compiler compile it and Cargo, Rust's package manager to fetch the
dependencies. On Ubuntu you can install the packages `rustc` and `cargo`. On
other distros there are probably similar packages. Also take a look at the
recommendations on the [Rust page](https://www.rust-lang.org/tools/install) and
in the [Cargo Book](https://doc.rust-lang.org/cargo/getting-started/installation.html).

Once you have a running Rust/Cargo setup, clone this repository, and run
```
install_lv2.sh
```
from within the directory from a terminal. You should see a bunch of messages
in your terminal. Finally it should say `sonarigo.lv2 successfully installed`.

Then you should find `Sonarigo` in plugins hosts like Ardour and Carla.

This works at least on Linux. About other systems I don't know.

## Usage

Quite easy. The generic GUI lets you select an SFZ file and adjust the output
gain. That's it.



## Todo

### Things I will definitely do

* Implement the important `loop_*` opcodes.


### Things I will probably do

* Write documentation


### Things I probably won't do but would love to see someone else do

* Write installation scripts for systems other than Linux.

* Package the whole thing

### Things I might do but don't promise (and of course welcome pull requests)

* Implement other opcodes. If you need a particular one or a particular set,
  and don't want can't implement that on your own, your best chances are to
  point me to a freely available SFZ instrument, that impressively shows the
  usefulness of this particular opcode.

* Implement support for gig format. Again, if you need it, point me to an
  impressive freely available instrument.


### Things I probably won't do but welcome pull requests

* Implement opcodes that demand further DSP like `comp_*`, `eq*`, `fil*`,
  `gate*`, `resonance*`, `reverb*`. In my opinion these should be accomplished
  by other plugins downstream. If you want to convince me from the opposite,
  show me a freely available instrument that impressively shows it.



### Things I definitely won't do but welcome pull requests

* Implement support for the sf2 sample format. There are already good lv2
  solutions for sf2 out there. I recommend 'a-fluidsynth' inside Ardour.

* Write a GUI


### Things I definetly won't do and probably even will reject pull requests

* Support for other plugin formats like VST. I don't use them and I could not
  maintain the support.
