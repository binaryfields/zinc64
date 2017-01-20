# Zinc64

## Overview

zinc64 is a quickly evolving emulator for Commodore 64 written in Rust. It implements MOS 6502 CPU, MOS 6526 CIA, MOS 6581 SID, MOS 6567/6569 VIC chipset as well as various devices/perhiperhals available with C64.

### Story

zinc64 was started as an exercise to learn Rust and explore Commodore 64 hardware in more detail. Somewhere around mid 2016 I needed to feed my 8-bit nostalgia so I picked up a working Commodore 64 (physical version) and started to assemble various accessories required to get software onto it. Soon enough I had picked up a copy of C64 Programmer's Reference Guide and the rest is now history.

### Rust

I have been following Rust development for a while and since it has reached its first stability milestone, version 1.0 back in May 2015, I have been meaning to write something more substantial with it. Exploring Rust and its features is really the second driver behind this emulator. Coming from Scala background, a lot of concepts and features in Rust felt right at home. Things like type inference, immutable by default, traits, ADTs, pattern matching all contribute to writing code that parallels many of the things I've done in Scala. The biggest departure between the two languages is quite obviously memory management. While ownership/borrowing is not conceptually hard, it takes a bit of practice to fully appreciate its application in the code base.

Overall, my impression of Rust is that it provides wonderful ergonomics for the developer while blending performance that only low-level languages provide with safety of a high level language (typically garbage collected one). While the language ecosystem may still be young and some libraries (futures/asyncio) need a bit more time to mature, the language already delivers a major productivity boost. The experience I had with it while writing code for the emulator can be largely summarized as "if it compiles, it works". Very impressive, big props to Rust folks for pulling off an excellent development platform.

## Getting Started

1. Install Rust compiler or follow steps @ https://www.rust-lang.org/en-US/install.html.

        curl https://sh.rustup.rs -sSf | sh

2. Clone this repository.

        git clone https://digitalstreamio/zinc64

3. Build the emulator.

        cd zinc64
        cargo build --release

4. Run the emulator.

        ./target/release/zinc64

## Examples

I've included a number of examples from Kick Assembler that I've used to test various components of the emulator. They can be found in the bin folder of this repository and started with the emulator's autostart option.

        ./target/release/zinc64 --autostart bin/SineAndGraphics.prg

## Status

| Class   | Component     | Status      |
|---------|---------------|-------------|
| Chipset | 6510 CPU      | Done
| Chipset | Memory        | Done
| Chipset | 6526 CIA      | Done
| Chipset | 6581 SID      | In-Progress
| Chipset | 6567 VIC      | Done
| Device  | Cartridge     | Done
| Device  | Floppy        | Not Started
| Device  | Datassette    | Done
| Device  | Keyboard      | Done
| Device  | Joystick      | Done
| Format  | Bin           | Done
| Format  | Crt           | Done
| Format  | D64           | Not Started
| Format  | Prg           | Done
| Format  | Tap           | Done
| Format  | T64           | Not Started

## Issues

- VIC sprite implementation is currently hacked up and needs more work
- VIC bad line handling needs work
- SID sampling is WIP

## Keyboard Shortcuts

| Shortcut  | Function          |
|-----------|-------------------|
| Alt-Enter | Toggle Full Screen
| Alt-F9    | Reset
| Alt-P     | Toggle Pause
| Alt-Q     | Quit
| Alt-W     | Warp Mode
| Ctrl-F1   | Tape Play/Stop
| NumPad-2  | Joystick Bottom
| NumPad-4  | Joystick Left
| NumPad-5  | Joystick Fire
| NumPad-6  | Joystick Right
| NumPad-8  | Joystick Top

## Credits

- Commodore folks for building an iconic 8-bit machine
- Rust developers for providing an incredible language to develop in
- Thanks to Rafal Wiosna for passing onto me some of his passion for 8-bit machines ;)
- Thanks to Klaus Dormann for his 6502_65C02_functional_tests, without which I would be lost
- Thanks to Dag Lem for his reSID implementation
- Thanks to Christian Bauer for his wonderful "The MOS 6567/6569 video controller (VIC-II) and its application in the Commodore 64" paper
- Thanks to Peter Schepers for his "Introduction to the various Emulator File Formats"
- Thanks to c64-wiki.com for my to go reference on various hardware components

