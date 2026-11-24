# Rust DOOM

This is a Rust rewrite of the DOOM engine that accepts only behavior checked against id Software's Linux DOOM 1.10
source, tic by tic (a tic is the game's 1/35-second step). Anything not proven identical to the original is refused.
Most ports go for playable first and exact second; this one goes the other way round. (Rust DOOM is a working title.)

It is meant for exact-behavior work: recorded demos (files of a player's inputs) stay in sync only if every tic
matches, which is what speedrun and tool-assisted (TAS) research need.

> **Machine-Maintained, Human-Directed Project**
>
> This project is developed and maintained primarily by AI coding agents under
> human direction and oversight. Agents implement code, write tests, maintain
> documentation, and assist with development workflows.
>
> Changes are subject to automated validation and human-defined review
> requirements. AI-generated code can contain mistakes, and passing tests does
> not guarantee correctness. This project is not affiliated with id Software or
> Bethesda.

![The genuine linuxdoom-1.10 engine, the Rust port and their per-pixel difference playing the same scripted session on a synthetic room: a pickup message with its bonus palette flash, then barrel damage with the red palette flashes and the face reacting](docs/media/m5b-status-bar-side-by-side.gif)

*The original engine, the Rust engine and their difference, one scripted session: the difference panel stays black ([how](docs/TESTING.md#readme-animations)).*

## Where it stands

- **Game logic.** One player, six weapons, seven kinds of monster and Episode 1's map features
  ([what is admitted](docs/COMPATIBILITY.md#policies)).
- **Real maps.** All nine Episode 1 maps and the original's own `DEMO1`: every scripted run stays in sync with the
  original C code, tic for tic, and so do the more than 60 hand-played sessions checked so far (over 200,000 tics)
  ([scripted runs](docs/validation/M4B.md#real-maps-and-demo1); by hand: [M5c](docs/validation/M5C.md#hand-played-checks),
  [M5d](docs/validation/M5D.md#hand-played-checks), [M6a](docs/validation/M6A.md#5-after-the-hand-test-gate-2026-11-22-evening)).
- **Picture.** Renderer, status bar, messages and palette flashes: frames equal the original's wherever compared ([what is and is not](docs/COMPATIBILITY.md#playable-build-m5b)).
- **Sound.** Effects, no music: sounds started and samples mixed equal the original's own sound code on the runs
  checked ([which build, which runs](docs/COMPATIBILITY.md#sound-effects-m5c)).
- **Playable, in a limited way.** One Episode 1 session at 35 tics a second, from any start map: both exits, the
  intermission, the next map with what the player carries, dying and coming back, the secret route
  ([what it supports](docs/COMPATIBILITY.md#episode-1-as-one-session-m6a)). The chain is compared with the original's
  own engine tic by tic, on synthetic maps and locally on real ones ([evidence](docs/validation/M6A.md)).

Not yet, and next (M6a part 2, a separate pull request): the finale after E1M8 (the session stops there), the screen melt
(the new picture appears at once), the two easy skills and E1M7 on `uv`. Not yet, later: the automap, music, menus, general
demo playback, other episodes, nightmare skill, or any claim of a match beyond the runs recorded so far ([what is missing](docs/ROADMAP.md#vanilla-completeness-checklist)).

## Play it

`doom-play` plays Episode 1 from the map you pick (E1M1 to E1M9, skill `hmp` or `uv`) with your own `DOOM.WAD`: each exit
leads to the tally screen and the next map, and it stops at the exit of E1M8 (the ending is not built yet). To really play
Doom, use an established source port such as [Chocolate Doom](https://www.chocolate-doom.org/).

There is no download yet: playing and the tests below both build from source. You need Rust 1.85.0 or later
([rustup](https://rustup.rs/)) and a C compiler (on Windows, the Visual Studio build tools). From the cloned top folder:

```sh
cargo run --release -p doom-play -- --iwad /path/to/DOOM.WAD --map 1 --skill hmp --sessions /path/for/sessions
```

- **Controls.** The original's defaults: arrows move and turn, right Ctrl fires, Space uses, right Shift runs, `1`-`8`
  pick weapons, mouse left fires (`--bindings modern` gives WASD). Esc pauses (R restarts, Q quits), End mutes.
- **Limits.** Needs an OpenGL 2.1 driver; no menus, saving or cheat codes; no screen melt between pictures yet; E1M7 on `uv` is refused
  ([every limit, and what has been tried](docs/COMPATIBILITY.md#playable-build-m5b)).

## How it's verified
The original C source is the ground truth. Each behavior runs in both the original and the Rust code, and the whole game state is
compared after every tic. Expected results come from a separate program written from the C output alone, so Rust is never checked
against itself; sanitizers, fuzzing and mutation testing back that up. Anything not proven identical is refused, not guessed.

Who writes the code, who decides and how each change is reviewed: [how it's built](docs/HOW_ITS_BUILT.md).

## Run the tests (no game files needed)
Built with the same Rust and C compiler as above; no game files, graphics, audio or network:

```sh
cargo test --workspace --locked
cargo run -p doom-sim --example player_barrels --locked  # exploding barrels, chain reactions, monsters
```

## The original's quirks

The [quirk register](docs/QUIRKS.md) lists hundreds of bugs, accidents and surprises in the original source, one row
each: the C file and line that causes it, what a player sees, and where this engine stands on it. None is fixed, on
purpose: vanilla means quirks included. Planned, not promised: an optional "fixed" mode, off unless chosen, with the
register as its list of bugs to correct.

## See it match

![The original C engine and the Rust engine replay the same synthetic test run side by side; every tic matches.](docs/media/m4b-tele1-side-by-side.svg)

*Game logic: one synthetic run in the original's code and in Rust, tic by tic ([how](docs/TESTING.md#m4b-synthetic-mechanism-wads)).*

![The original C renderer and the Rust renderer draw the same synthetic rooms side by side, with a third panel showing their pixel difference, which stays black.](docs/media/m5a-renderer-side-by-side.gif)

*Renderer: three synthetic rooms; the difference stays black ([how](docs/TESTING.md#readme-animations)).*

![Waveforms of the genuine sound mixer, the Rust sound mixer and their difference for one synthetic session, in three panels; the difference panel is flat.](docs/media/m5c-mixer-waveforms.svg)

*Sound: one synthetic session through both mixers; the difference is flat ([how](docs/TESTING.md#readme-animations)).*

## Where it's (maybe) going

Everything the original does is planned, bar a few DOS-era and X11 pieces. The rest are ideas, not promises.

- **[Track 1: Faithful](docs/ROADMAP.md#track-1-faithful).** Planned: original demo playback, the other episodes and Doom II, just as exact.
- **[Track 2: Presentation](docs/ROADMAP.md#track-2-presentation).** Ideas: a modern renderer, smooth high frame rates, VR; none can change gameplay.
- **[Track 3: Beyond vanilla](docs/ROADMAP.md#track-3-beyond-vanilla).** Ideas: scripting, a live editor with rewind, TAS tools, all as separate optional modes.

## For developers, contributing and license

Docs start at the [index](docs/README.md); contributors read [CONTRIBUTING.md](CONTRIBUTING.md), AI agents
[AGENTS.md](AGENTS.md). id Software's source, history and attribution are preserved; the Rust workspace is GPL-2.0-only
([license](LICENSE.TXT), [original README](README.TXT)); game data is never distributed ([assets](docs/ASSETS.md)).
