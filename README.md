stwgs is a gamepad to keyboard/mouse mapping CLI utility for FreeBSD (should also work with Linux).
There is a corresponding Vulkan overlay, which is currently not that useful.

## Features

- granular per axis bindings
- text configs (with somewhat passable error messages)
- no Python
- no CMake either

## Limitations

- automatic config switching is out of scope
- Bluetooth? What's that?
- Steam Controller's BLE mode doesn't work (see the previous item)

## Requirements

- a Steam Controller or a gamepad supported by the SDL's HIDAPI backend
- whatever stable Rust version that is in the FreeBSD repos
- SDL 2
- D-Bus (however disappointing that might be)

## Usage

1. install Rust;
2. build and install SDL with HIDAPI support;
3. clone this repo;
4. connect your gamepad, make sure its USB device node is accessible;
5. run `cargo run -- load examples/ut99.cfg` (or whatever) from the repo root dir.

## Configuration

Unsurprisingly, the configuration involves a comma-separated list of bindings
going from individual gamepad buttons/axes to keyboard/mouse state or
internal mapper actions (like mode switching):
```
input(A).bind(Kb.A), // map gamepad button A to keyboard key A
input(B).bind(Kb.B), // etc
```

Transformations can be applied as necessary:
```
input(Yaw  ).scale(-15).bind(Ms.X), // take gyro's yaw angular velocity, multiply by -15, add that number to mouse axis x
input(Roll ).scale(-20).bind(Ms.X),
input(Pitch).scale(-15).bind(Ms.Y),
```

To spice things up a little some of those transformations accept other
transformation chains (aka pipelines) as their parameters:
```
input(Yaw  ).gate(input(RPadTouch)).scale(-15).bind(Ms.X), // take gyro's yaw angular velocity but only when the right pad is touched, etc
input(Roll ).gate(input(RPadTouch)).scale(-20).bind(Ms.X), // could pass invert(input(RPadTouch)) instead
input(Pitch).gate(input(RPadTouch)).scale(-15).bind(Ms.Y), // or anything that has Pipeline[bool] type
```

There are also variables and user-defined functions:
```
def rad(deg) = deg * 3.14159265358979323846264338327950288 / 180.0;

def dpad_button(pad, direction) =
  pad.as_ring_sector_button(
    direction    = rad(direction),
    angle        = rad(120/*deg*/),
    inner_radius = 0.25,
    outer_radius = 1.2,
    margin       = 0.1);

let left_pad = merge(input(LPadX), input(LPadY));

dpad_button(left_pad,  90/*deg*/).bind(Kb.W),
dpad_button(left_pad, 180/*deg*/).bind(Kb.A),
dpad_button(left_pad, -90/*deg*/).bind(Kb.S),
dpad_button(left_pad,   0/*deg*/).bind(Kb.D),
```

And, of course, layout support:
```
layer foo {
  input(A).bind(Kb.A)
},

layer bar {
  input(B).bind(Kb.B)
},

// a pipeline can belong to multiple layers
// pipelines are stateful, so this is handy for preserving exact state through mode transitions
layer foo | bar {
  input(X).bind(Kb.X)
},

input(Y).cycle_modes({foo, bar, foo | bar}), // mode is an arbitrary combination of layers
```

[to be continued in the wiki]
