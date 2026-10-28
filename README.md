# Rust DOOM

An independent, Rust-native rewrite of the original DOOM engine, maintained in
[Copgunn/DOOM](https://github.com/Copgunn/DOOM). The goal is deterministic vanilla
simulation, original WAD/map compatibility, reproducible demo playback, and
cross-platform presentation through a custom wgpu renderer, with Vulkan preferred
on supported platforms. Rendering remains future work.
This is an idiomatic rewrite with explicit ownership, not an official id Software project.

## Current status

M0 foundation is implemented, validated, and merged ([PR #1](https://github.com/Copgunn/DOOM/pull/1)).
M1 resource/map parsing is implemented, validated, and merged
([PR #2](https://github.com/Copgunn/DOOM/pull/2)). M2 deterministic headless
foundations are implemented, validated and merged
([PR #3](https://github.com/Copgunn/DOOM/pull/3), merge `10a1106`). M3a adds
reference-tested grounded player XY movement, wall sliding and solid actor
collision, merged in [PR #4](https://github.com/Copgunn/DOOM/pull/4) as `764dd96`.
M3b implements and validates static multi-sector movement and vertical physics
in [PR #5](https://github.com/Copgunn/DOOM/pull/5), merged as `c791ef8` after
owner-authorized correction-head and post-merge CI. M3c adds typed actor states,
spawning and controlled death in [PR #6](https://github.com/Copgunn/DOOM/pull/6),
merged as `4dbb0a5` after correction-head and post-merge CI. M3d1 damage/hitscan
foundations are implemented in [PR #7](https://github.com/Copgunn/DOOM/pull/7)
merged as `c924f18` after fresh correction-head and post-merge CI. M3d2 player
weapons and psprites are implemented in [PR #8](https://github.com/Copgunn/DOOM/pull/8)
merged as `955f690` after fresh correction-head and post-merge CI. M3d2b projectile
foundations implement bounded original plasma spawning, flight, direct impact,
explosion and removal in profile 7; [PR #9](https://github.com/Copgunn/DOOM/pull/9)
merged as `e109ee1` after final corrections and fresh/post-merge CI. M3d3a
perception and target acquisition in [PR #10](https://github.com/Copgunn/DOOM/pull/10)
merged as `839fbab` after final correction and successful correction/post-merge CI.
M3d3b pursuit in [PR #11](https://github.com/Copgunn/DOOM/pull/11) merged as
`0007af1` after correction-head and post-merge CI. Documentation
[PR #12](https://github.com/Copgunn/DOOM/pull/12) separately merged as `876662d`;
combined-master CI passed before M3d3c started. M3d3c implements bounded source
attack selection and a complete admitted A_PosAttack animation in profile 10,
on labelled synthetic actors, in [PR #13](https://github.com/Copgunn/DOOM/pull/13).
It merged regularly as `9b8fde8` after correction and fresh local/remote gates.
M3d3d adds bounded source shotgunner multi-pellet attacks in profile 11 in
[PR #14](https://github.com/Copgunn/DOOM/pull/14), regularly merged as `31b4830`
after correction and fresh/post-merge CI. M3d3e adds bounded chaingunner attack/
refire in profile 12 in [PR #15](https://github.com/Copgunn/DOOM/pull/15),
regularly merged as `5b91824` after documentation correction, macOS hosted-runner
recovery and successful exact-head/post-merge checks. M3d3f adds bounded normal-
timing Demon melee in profile 13 on `feat/m3d3f-demon-melee`, for separate review.
Complete original monsters,
actual player damage and a playable client remain future work. Exact gates and
limitations are in the [roadmap](docs/ROADMAP.md) and
[validation notes](docs/VALIDATION.md).

Implemented:

- Immutable borrowing WAD reader, checked signed directory fields, configurable
  limits, exact eight-byte names, safe lump access, ASCII-insensitive lookup,
  original ordering, and last-match precedence across archives.
- Classic THINGS, LINEDEFS, SIDEDEFS, VERTEXES, SEGS, SSECTORS, NODES, SECTORS,
  REJECT, and BLOCKMAP decoding for E#M#/MAP## groups. Validates references,
  sentinels, ranges, BSP cycles, visibility size, and shared block lists.
- Typed errors, fallible allocation, bounded validation work, synthetic malformed
  input/adversarial regressions, and separate runnable WAD/map fuzz targets.
- Explicit 16.16 fixed arithmetic, original owned RNG streams, validated actor
  handles and append-sensitive thinker scheduling.
- Grounded non-player XY motion, original line predicates and ordered BLOCKMAP
  collision, command-thrust foundations, canonical snapshots and first-divergence
  diagnostics, compared with isolated C vectors and independent encoding goldens.
- Explicit M3a profile: player-specific ground friction and blocking response,
  three-corner wall tracing, axis/angled slides, solid actor collision and
  cell-head ordering, DSIM-v2 snapshots, and an asserted headless arena.
- Explicit M3b profile: iterative original BSP lookup, two-sided openings,
  step/dropoff/clearance rules, XY/Z order, gravity/landing/ceiling contact,
  sector-head membership/order, reference-list BLOCKMAP policy and DSIM-v3.
- Explicit M3c profile: complete original actor metadata, validated executable
  subsets, ordered state actions, typed/map spawning, controlled death, safe
  targets and DSIM-v4. Metadata recognition does not imply monster gameplay.
- Explicit M3d1 profile: bounded unbound-actor damage, source autoaim/hitscan,
  pain/knockback/kill/corpses, real puff/blood thinkers and DSIM-v5.
- Explicit M3d2 profile: command-driven pistol/shotgun/chaingun, unpowered fallback
  fist, source psprites/ammo/refire/switching, sector noise targets and DSIM-v6.
- Explicit M3d2b profile: source MT_PLASMA launch/autoaim, missile collision, shared
  direct damage and complete explosion/removal, with DSIM-v7.
- Explicit M3d3a profile: original REJECT/BSP sight and player target acquisition,
  shared sector noise, ambush decisions and admitted A_Look with DSIM-v8.
- Explicit M3d3b profile: original walking directions, P_Move/TryWalk/NewChaseDir,
  non-attacking A_Chase, target reacquisition and DSIM-v9.
- Explicit M3d3c profile: source melee/missile range and Chase attack selection,
  FaceTarget, bounded PosAttack with shared real impacts/damage, and DSIM-v10.
- Explicit M3d3d profile: one shared aim and three sequential SPosAttack pellets,
  exact SPOS 10/10/10 timing/fullbright, bounded preflight and DSIM-v11.
- Explicit M3d3e profile: original CPOS 10/4/4/1, separately aimed shots, source
  refire short-circuit decisions, immediate see/Chase and DSIM-v12.
- Explicit M3d3f profile: normal SARG 8/8/8, three separate faces, source selection
  sound/current melee range and shared direct damage through DSIM-v13.
- Stable debug/release Linux/Windows/macOS CI, Rust 1.85 tests, and engineering guidance.

Choose the compatibility profile explicitly:

| Constructor | Schema | Supported subset |
| --- | --- | --- |
| `Simulation::new` | DSIM-v1 | M2 one-sector non-solid/non-player XY response |
| `Simulation::player_collision` | DSIM-v2 | M3a grounded player slides and solid collision, conservative coverage |
| `Simulation::multisector` | DSIM-v3 | M3b static classic sectors, BSP/openings, step/dropoff and XY/Z/gravity |
| `Simulation::actor_states` | DSIM-v4 | M3c typed actors and supported state/action lifecycle on static-sector physics |
| `Simulation::combat` | DSIM-v5 | M3d1 explicit damage/aim/hitscan for admitted unbound actors |
| `Simulation::player_weapons` | DSIM-v6 | M3d2 one live weapon player, source pistol/shotgun/chaingun and fallback fist |
| `Simulation::projectiles` | DSIM-v7 | M3d2b explicit original plasma launch, flight, direct impact and explosion/removal for unbound actors |
| `Simulation::monster_perception` | DSIM-v8 | M3d3a pure sight, four-slot perception roster, target scanning and shared noise; labelled fixture A_Look only |
| `Simulation::monster_pursuit` | DSIM-v9 | M3d3b perception plus non-attacking pursuit on labelled native look/chase fixtures; grounded static-map movement |
| `Simulation::with_monster_attack_registry` | DSIM-v10 | M3d3c explicit fixture registry: one native attacker, source range/selection/facing, bounded PosAttack and unbound damage targets |
| `Simulation::with_shotgunner_registry` | DSIM-v11 | M3d3d explicit SPOS fixture: source Look/Chase/facing, shared aim, three sequential hitscan pellets and real unbound damage/effects |
| `Simulation::with_chaingunner_registry` | DSIM-v12 | M3d3e explicit CPOS fixture: independently aimed single shots, source refire and immediate see/Chase handoffs |
| `Simulation::with_demon_melee_registry` | DSIM-v13 | M3d3f normal-timing SARG fixture: Look/Chase, 8/8/8, selection sound, three faces and shared direct melee damage |

M3b consumes supplied BLOCKMAP lists in reference order. Conservative omissions
are diagnostic, not automatic rebuilds or proof of broken geometry. Missing
collision data can allow source-compatible movement through omitted walls; see
[the investigated policy](docs/M3B_REFERENCE.md). Older profiles remain unchanged.
Classic two-sided lines are gameplay openings, not stacked sectors, room-over-room
or enhanced visual portals.

Complete player thinking/view/animation, weapons, monster AI,
combat, moving doors/lifts, specials and respawn remain incomplete. M3d–M3e
continue gameplay; M4 covers demos; M5 covers presentation/input/audio and the
custom wgpu renderer. Authentic IWAD compatibility, full synchronization and DOS
1.9 behavior are unverified. Read the [compatibility limits](docs/COMPATIBILITY.md).

## Monster perception example

```sh
cargo run -p doom-sim --example monster_perception --locked
python3 tools/reference/check_monster_perception.py
```

The asserted synthetic example covers slot-3 wraparound, ordinary hearing through
an occluding wall, blocked/visible ambush listeners and an eight-tic candidate
change. Its final DSIM-v8 digest is `30721bdec7a0d47e` in debug and release.
`check_sight(observer, target)` is read-only; `look_for_players` uses the separate
four-slot `PerceptionPlayer { active, health, actor }` roster. `noise_alert` accepts
a distinct target and emitter. Configure labelled fixture lookers with
`with_perception_registry` to execute `look` or the same A_Look in thinker order.
Original monsters whose see states immediately call A_Chase are rejected before
mutation. The profile does not enable weapon players, missiles or player damage.

The original horizontal sight classifier's `x==node.y` quirk is preserved; sight
uses REJECT and BSP/seg traversal independently of weapon BLOCKMAP tracing.
[Source findings and admission bounds](docs/M3D3A_REFERENCE.md) distinguish
68 C-equivalence scenarios from Rust safety policies, including terminating an
empty roster and rejecting stale identities. See [validation](docs/VALIDATION.md)
for counts and [DSIM-v8](docs/STATE_FORMAT.md) for schema evidence. No proprietary
assets are needed.

## Monster pursuit example

```sh
cargo run -p doom-sim --example monster_pursuit --locked
python3 tools/reference/check_monster_pursuit.py
```

This asserted synthetic demonstration covers perception-to-chase entry, direct
walking, a blocking wall and alternate directions, repeated pursuit, target loss
and reacquisition. Its final DSIM-v9 digest is `8c7fd6adfe73112d` in debug/release.
`with_pursuit_registry` admits labelled native fixtures whose see action is genuine
A_Chase and whose melee/missile states are absent. Copying source player metadata
requires explicitly clearing its missile-animation state. Full original monsters
remain unadmitted. All eight earlier examples remain available.

`move_pursuer`, `try_walk`, `new_chase_direction` and `chase` expose the supported
original routines; thinker state entry calls the same implementations. Walking
moves position immediately through the existing collision service. It does not
become momentum. Eight source direction vectors, source RNG/fallback priorities,
counters, turning, target reacquisition and active sounds are preserved. Pending
or stale target identities reject before mutation; missing/nonshootable targets
use the source scan/spawn path. A dead shootable target can still be pursued.

Profile 9 supports static maps without line/sector specials, non-floating bodies,
a separate four-slot perception roster and at most 64 actors. Weapons, missiles,
player damage, doors, floating AI and attacks remain separate. See
[source findings/admission](docs/M3D3B_REFERENCE.md), [v9 schema](docs/STATE_FORMAT.md)
and [validation](docs/VALIDATION.md) for C equivalence versus project policy evidence.

## Build and test

Install [Rust using rustup](https://rustup.rs/), then run from the repository root:

```sh
cargo build --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p doom-sim -p doom-types --release --locked
cargo doc --workspace --no-deps --locked
cargo run -p doom-map --example parse_map --locked
cargo run -p doom-sim --example headless --locked -- 12
cargo run -p doom-sim --example player_arena --locked
cargo run -p doom-sim --example multisector_arena --locked
cargo run -p doom-sim --example actor_states --locked
cargo run -p doom-sim --example combat_arena --locked
cargo run -p doom-sim --example player_weapons --locked
cargo run -p doom-sim --example projectiles --locked
```

`rust-toolchain.toml` selects stable with rustfmt and Clippy. The minimum compiler
is Rust 1.85 (edition 2024), tested separately in CI. The stable workspace has no
third-party dependencies and needs no IWAD, graphics SDK, C compiler, or platform
libraries. Lockfiles are committed; stable follows stable releases, so record the
exact compiler for regressions. API documentation is generated in `target/doc/`.
Optional fuzzing needs nightly, cargo-fuzz, and a C++ compiler. Reproducing the
isolated C vectors needs Python 3 and a supported C compiler; recorded-vector Rust
tests need neither. See [TESTING.md](docs/TESTING.md).

## Parsing APIs

The runnable `parse_map` example loads the original synthetic square fixture and
reports four vertices, four linedefs, and one sector. The same API accepts bytes
acquired by your application under its own I/O/memory limits:

```rust
use doom_map::{Map, MapLimits};
use doom_wad::{Limits, Wad, WadSet};

fn load_map(bytes: &[u8]) -> Result<Map, Box<dyn std::error::Error>> {
    let archives = [Wad::parse(bytes, Limits::default())?];
    let resources = WadSet::new(&archives);
    Ok(Map::load(&resources, "MAP01", MapLimits::default())?)
}
```

Keep byte buffers alive while using their `Wad` views. Add parsed patch archives
later in the `WadSet` sequence; later matches take precedence. `Wad::lumps()`
retains every directory entry; `find(name)` returns its last matching index and
`data(index)` returns `Option<&[u8]>`. `Map` owns its validated records and exposes
immutable slices such as `vertices()`, `linedefs()`, and `sectors()`.

Maps load from the overriding marker's complete ordered group, within one archive.
They do not assemble geometry using global individual-lump lookup. Unsupported
UDMF, Hexen, and known extended-node signatures produce explicit errors. Input
safety is stricter than the original C loader; see [compatibility](docs/COMPATIBILITY.md).

## Headless simulation APIs

Run the current demonstration:

```sh
cargo run -p doom-sim --example multisector_arena --locked
```

It runs six synthetic connected-sector scenarios (144 asserted C-derived tics):
a 24-unit step, rejected 25-unit step, exact 56-unit doorway, blocked 55-unit
doorway, allowed drop/gravity/landing, and movement through three sectors. Output
shows logical tic, raw fixed XYZ/momentum, sector and canonical digest. It requires
no window, renderer, audio, wall clock or commercial assets.

The original `headless -- 12` and `player_arena` examples preserve M2 coasting and
M3a angled/solid collision evidence. `python tools/reference/check_headless.py`
compares all six examples' debug/release output byte-for-byte.

Initialize M3b from a validated `Map`:

```rust
use doom_map::Map;
use doom_sim::{ActorSpec, SimError, Simulation, Snapshot, Solidity, TicCommand, VerticalSpec};

fn one_tic(map: Map) -> Result<Snapshot, SimError> {
    let mut sim = Simulation::multisector(map)?;
    let player = sim.spawn_vertical(
        ActorSpec::default(),
        Solidity::Solid,
        VerticalSpec { allow_dropoff: true, ..Default::default() },
    )?;
    sim.bind_player(0, player)?;
    let mut commands = [TicCommand::default(); 4];
    commands[0].forward = 25;
    sim.step(commands) // one logical 35 Hz tic; no elapsed-time argument
}
```

`VerticalSpec` selects floor/ceiling/absolute spawn Z, initial vertical momentum,
normal gravity or `Gravity::None` (MF_NOGRAVITY), and MF_DROPOFF permission. Player
binding does not silently add dropoff flags. Airborne actors turn but receive no
forward/side thrust; airborne XY skips ground friction. XY runs before Z for each
thinker. There is no jump/crouch/freelook, flying-monster or arbitrary-flag API.

`world().locate(position)` returns source BSP membership. `actor.location()` and
`actor.constraints()` distinguish the center's sector from contacted floor/ceiling
constraints while straddling a line. `sectors()` exposes read-only runtime heights;
`sector_actor_order`, `line_opening` and `check_multisector_position` expose
observational ordering/opening/destination decisions. Legacy flat collision and
dimension helpers reject M3b calls that lack actor/sector context.

`spawn` creates a non-solid mover and `spawn_with_solidity` selects zero/MF_SOLID;
in M3b these use default floor-Z setup. Setup may overlap actors like the source.
Non-solid movers still collide with solid candidates; supported ordinary actor
collision uses the original XY-only overlap predicate even when Z differs.
Removal unlinks spatial/sector membership immediately; thinker storage retires
later. Every successful move reinserts at both heads, including within a cell or
sector. Identity, thinker order and spatial/sector order remain separate.

Recoverable `step` errors preserve commands, actors, RNG, events and time.
Geometry/actor invariants are validated before stepping; allocation exhaustion
uses Rust's process-level policy. No full simulation clone is made each tic.

`run_commands` records whole command streams; `Snapshot::canonical_bytes()` and
`digest()` encode explicit state, while `compare_traces` reports the first
mismatching tic/field. Rendering can read snapshots/events without draining or
changing simulation. Hash equality alone is not proof of gameplay equivalence.
See [STATE_FORMAT.md](docs/STATE_FORMAT.md) for field widths/order and
[M2_REFERENCE.md](docs/M2_REFERENCE.md), [M3A_REFERENCE.md](docs/M3A_REFERENCE.md),
and [M3B_REFERENCE.md](docs/M3B_REFERENCE.md) for source findings and safe policies.

`doom-types::Fixed` uses explicit wrapping arithmetic and the reference's guarded
double division; `DoomRng` owns both original RNG indexes. `ThinkerStore` separates
slot/generation identity from append order, detects stale handles, defers retirement,
and permits same-traversal spawning through restricted callbacks. The current
simulation's built-in thinkers perform the selected profile's motion and, in
profile 4, admitted actor state/action processing after XY/Z.

## Headless combat foundations

```sh
cargo run -p doom-sim --example combat_arena --locked
```

This asserted synthetic arena checks a successful hitscan, a blocking wall, pain,
a lethal hit and 80 tics of death progression against original C evidence and
independent DSIM-v5 bytes. It ends with a persistent corpse and digest
`8906eb60707d089e`; the impact thinker has retired.

Select `Simulation::combat(map)` or `with_combat_registry` explicitly. The pure
`aim_line_attack` query returns the source target/slope window; `line_attack`
applies supplied angle/range/slope/damage, and `damage_actor(DamageRequest)`
applies direct damage with separate optional source and inflictor identities.
Recoverable combat errors preserve full canonical state, events and RNG.
Damage is bounded to 0..=1000; range is positive and at most 2048 fixed map units.

Only admitted unbound actors with supported state/action closures can receive
this damage. The arena uses source player-state pawns without a `player_t`,
including an explicitly labelled fixture definition. Monster metadata recognition
does not establish monster execution. Bound-player armor/inventory/death,
weapons/buttons/ammo/psprites, AI, item drops/scoring, missiles, radius attacks,
sky/special effects and full vanilla synchronization remain future work.
See [source findings and exact limits](docs/M3D1_REFERENCE.md).

## Player weapon foundations

```sh
cargo run -p doom-sim --example player_weapons --locked
```

The asserted example checks every player/psprite/ammo/RNG capture against original
C for pistol, seven-pellet shotgun, chaingun, held fire, switching, wall hits and
deaths. Its final independent DSIM-v6 digest is `1cc5a89450da5585`.

Choose `Simulation::player_weapons(map)`, spawn an original `ActorTypeId(0)` pawn,
then call `initialize_weapon_player(slot, pawn, WeaponInventory::default())`.
The source spawn subset is LIVE/health 100, owned fist+pistol, 50 clips, original
ammo capacities and attack-down true. Setup enters the raise action immediately,
so initial weapon Y is 122 units. One initialized player may occupy any of four
ordered slots; capacity is at most 64 actors. `with_weapon_registry` admits exact
source states and a narrowly labelled player-state fixture type for testing.

Supply `TicCommand.buttons = BT_ATTACK` to fire through the actual psprite timing.
Release with zero. Change using `BT_CHANGE | ((weapon as u8) << 3)`; supported
selections are fist, pistol, shotgun and chaingun. Valid unowned selections are
ignored as in the source. Explicit `WeaponInventory` setup can grant admitted
weapons/ammo for level/testing setup; it is not a pickup system. Source fallback
can select the fully supported unpowered fist when firearm ammo is exhausted.

Weapon/flash psprites, ammo, light, refire and sector noise targets are authoritative
state. Real impacts and damage use the existing combat pipeline. Direct combat
APIs still reject bound players; a private firing route admits only the initialized
shooter as source/inflictor. Player damage, armor, powers, full camera/spawn/respawn,
pickups, chainsaw, supershotgun, rockets/plasma/BFG and projectile actors remain
unsupported. Recoverable command/preflight errors preserve the entire snapshot.
See [source findings and policies](docs/M3D2_REFERENCE.md).

## Typed actor lifecycle (M3c)

```sh
cargo run -p doom-sim --example actor_states --locked
```

This asset-free example checks 80 tics against independently encoded C results:
19 state entries, two supported actions, a persistent terminal player-type object,
and fog null-state removal followed by retirement. It is a headless test scenario.

`ActorRegistry::vanilla()` recognizes all 137 original types, 967 states and 74
non-null action identities. `Simulation::actor_states(map)` selects profile 4;
`spawn_type(type_id, xy, SpawnHeight::Floor)` validates an executable type before
initializing it. The registry method `find_type("MT_PLAYER")` resolves symbolic names.
Spawning assigns the initial state directly without calling its action. Subsequent
state entry/countdowns run at 35 Hz, after each actor's XY/Z motion. State zero is
explicit removal; infinite states persist. Unsupported callbacks reject before
mutation. Bounds include 1024 actors, 2048 states, 256 types and 4096 map THINGS.

In a flat 128-unit sector, 67 source types are admitted: player-type pawns, inert
props, fog and a few animation-only effects. Actual admission checks dimensions
against every sector and validates spawn/death closures. The five original actions
are `A_Fall`, `A_Pain`, `A_Scream`, `A_XScream` and `A_PlayerScream`; sound requests
are inert events. See the [admission matrix and policies](docs/M3C_REFERENCE.md).

`spawn_map_things()` performs single-player DOOM II skill/nomonsters filtering,
DoomEd lookup, source-order spawning, positive-tic randomization, angle quantization
and ambush flags. It reports player/deathmatch starts explicitly; full player setup
and multiplayer modes remain unsupported. A bad selected type rejects the batch
without partial spawning or RNG drift.

`request_actor_death(id)` enters an unbound actor's registered death state. It
implements no damage, kill scoring, drops, corpse flags, height change or randomized
kill timer. `A_Fall` clears solidity; a terminal object can persist. Targets retain
generation-aware identities through removal and resolve unavailable after retirement.
`enter_actor_state` validates explicit state requests before effects. A separate
`ProbeAction` namespace tests scheduler/service capabilities; those callbacks are
synthetic probes, not vanilla AI or a modding language.

## Optional private IWAD directory

Keep lawfully obtained IWADs outside the checkout in a directory you control and
set `DOOM_IWAD_DIR` in your private shell. After separately authorizing assets for
that environment, run the read-only opt-in archive/map validation:

```sh
cargo test -p doom-map --test private_iwad -- --ignored
```

This test checks IWAD archives and their classic map groups without printing
private filenames, paths, or bytes. A separate `doom-wad` test of the same name
checks signatures only. Both stay ignored in public CI. They do not establish
authenticity, game initialization, rendering, demo compatibility, or gameplay.
The library accepts bytes; an engine client that automatically discovers/loads
assets is future work. No private assets are bundled.

Do not copy assets into a cloud workspace without explicit destination consent,
change source originals, or publish private test output. Local asset directories,
WAD/demo/package files, environment files, and private output are ignored as a
safeguard; ignore rules do not prevent forced staging. Never upload commercial
assets to GitHub branches, PRs, releases, CI artifacts, snapshots, or public logs.

## Layout and architecture

```text
crates/doom-types/     explicit fixed arithmetic, vanilla RNG, names and 35 Hz vocabulary
crates/doom-wad/       bounded borrowed archive views; depends on doom-types
crates/doom-map/       owned validated maps; depends on doom-wad and doom-types
crates/doom-sim/       owned headless state, scheduling, movement and snapshots
tests/support/        original synthetic fixture builders
tests/reference/      recorded C vectors, provenance and independent snapshot goldens
tools/reference/      reproducible isolated C harness and golden encoder
fuzz/                 separate nightly package, lockfile, targets and seed corpus
docs/                 architecture, roadmap, compatibility, testing, validation
.github/              stable cross-platform CI and PR template
linuxdoom-1.10/        preserved original engine; behavioral reference
ipx/ sersrc/ sndserv/  preserved original auxiliary source
README.TXT            original release notes and attribution
LICENSE.TXT           original GPL version 2 license
```

The simulation consumes validated map data and ordered per-tic commands. Each
step represents 1/35 second of logical time. Rendering, audio, wall clocks, and
I/O cannot influence its outcomes. See [architecture](docs/ARCHITECTURE.md).

## Roadmap and compatibility

| Milestone | Scope | Status |
| --- | --- | --- |
| M0 | Workspace, CI, documentation, repository guidance | Implemented, validated, merged |
| M1 | Bounded WAD and classic map parsing, regression tests, fuzzing | Implemented, validated, merged |
| M2 | Fixed arithmetic, RNG, ordered thinkers, narrow movement, snapshots | Implemented, validated, merged |
| M3a | Grounded player XY, wall sliding, solid actor collision | Implemented, validated, merged |
| M3b | Static sectors/BSP, openings, step/dropoff and XY/Z/gravity | Implemented, validated, merged |
| M3c | Actor states, spawning and controlled death | Implemented, validated and merged |
| M3d1 | Bounded damage/hitscan/kill foundations | Merged as c924f18 |
| M3d2 | Player weapons, psprites and ammo | Merged as 955f690 |
| M3d2b | Original plasma foundations | Merged as e109ee1 |
| M3d3a | Monster perception and target acquisition | Merged as 839fbab |
| M3d3b | Source pursuit and movement decisions | Merged as 0007af1 |
| M3d3c | Attack selection and bounded source hitscan action | Merged as 9b8fde8 |
| M3d3d | Source shotgunner three-pellet foundations | Merged as 31b4830 |
| M3d3e | Source chaingunner attack/refire foundations | Merged as 5b91824 |
| M3d3f | Source Demon melee foundations | Regularly merged as 75b2a2bc after correction and post-merge CI |
| M3d3g | Demon pain, ordinary death and corpse foundations | Implemented and validated; separate owner review in PR #17 |
| Later M3d3–M3e | Remaining attacks, integrated combat, interactions and moving sectors | Planned |
| M4 | Version-specific demo replay and desynchronization diagnostics | Planned |
| M5 | Presentation, input/audio, custom wgpu renderer; Vulkan preferred | Planned |

The initial behavioral reference is **Linux DOOM 1.10**, from
[id-Software/DOOM](https://github.com/id-Software/DOOM) at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. DOS DOOM 1.9 is a separate objective
requiring its own evidence. Demo version 110 does not establish DOS compatibility.
See [milestone acceptance criteria](docs/ROADMAP.md).

## Attribution, license, and assets

The original source was released by id Software, with release notes by John
Carmack and cleanup credited to Bernd Kreimeier. This fork preserves upstream
history, source notices, [README.TXT](README.TXT), and [LICENSE.TXT](LICENSE.TXT),
including the ZeniMax Media copyright notice.

Rewrite code is **GPL version 2 only** (`GPL-2.0-only`), consistent with the
repository's GPL-2.0 license. The original README is historical; the GPL governs
code use and redistribution, including commercial distribution under its terms.
The engine source license grants no rights to proprietary game data. Commercial
IWADs, textures, sprites, music, sounds, and other proprietary assets are not
included and must never be committed. Obtain data lawfully; public CI uses only
synthetic data. A future runnable engine needs separately licensed assets.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md), [AGENTS.md](AGENTS.md), and the
[testing guide](docs/TESTING.md). Work on a focused branch with meaningful tests
and relevant documentation, and open a PR against `master`. Correctness,
determinism, safety, and reviewable changes take priority over optimization.
Merging requires explicit owner approval.

## Projectile foundations

`Simulation::projectiles(map)` selects profile 7 with at most 64 actors. Spawn an
admitted unbound source/target using `spawn_type`, then call
`launch_projectile(ProjectileRequest { source, actor_type: ActorTypeId(34),
aim: ProjectileAim::Actor(target) })`; `PlayerAutoAim` selects the original
player-missile helper without enabling player damage or a plasma-rifle command.
The launch API derives source Z+32 and velocity from original metadata. A blocked
initial check returns a real explosion actor. Advance with `step` until the
registered explosion reaches null and the next thinker pass retires storage.

Only MT_PLASMA is executable as a missile. Live missiles require an active owner
generation; remove the missile before retiring its owner, or stepping rejects
atomically. Explosion frames retain the old identity safely. Custom projectile
metadata, splash, rockets/BFG, monster firing/AI, bound players and full weapon
integration remain unsupported. See the exact [admission and arithmetic bounds](docs/M3D2B_REFERENCE.md),
[player-damage dependency plan](docs/M3D2B_REFERENCE.md#actual-player-damage-dependencies-and-future-acceptance-plan)
and [validation evidence](docs/VALIDATION.md). All examples use public synthetic
fixtures; no private game data is required or accessed.

## Bounded monster attacks (M3d3c)

```sh
cargo run -p doom-sim --example monster_attacks --locked
python3 tools/reference/check_monster_attacks.py
```

The asserted synthetic demonstration acquires targets, walks, rejects an attack,
selects the copied source attack animation, faces twice at the original times,
aims/fires with source sound/spread/damage RNG, creates real impacts, applies
admitted unbound damage and returns to pursuit. C traces and independent v10
bytes verify every capture. The final 110-tic digest is `245e71d4a9c4023e`.
All nine earlier examples remain available; no IWAD or presentation device is used.

`with_monster_attack_registry(map, limit, registry, options)` admits exact named
fixture adaptations described in [M3D3C_REFERENCE](docs/M3D3C_REFERENCE.md) and
reproduced in the example's public fixture builder. Use `check_melee_range` for
pure range evidence, `check_missile_range` for the source flag/RNG decision,
`chase` for source selection, and `face_target`/`pos_attack` for bounded direct
callbacks. The same actions run through ordinary thinker/state countdowns.
Action-free melee/missile markers prove selection only. Only native type 136 admits
PosAttack; original monster metadata recognition does not grant execution.

One unshootable owner must be spawned first, remain thinker head and have zero
physical momentum on its floor. It still performs direct source walking. Native
135 targets use source player body states without player_t; roster health remains
separate from actor damage. Exact original blood/puff thinkers may retire/reuse
slots. Actual player binding/damage and profile 10 public direct-combat APIs reject.
This is a complete tested action closure within a narrow fixture slice, not
complete Zombieman/monster gameplay, melee damage, demos or a playable game.

## Shotgunner attacks (M3d3d)

```sh
cargo run -p doom-sim --example shotgunner_attacks --locked
python3 tools/reference/check_shotgunner_attacks.py
```

The asserted public synthetic example verifies a front target killed by pellet 2,
then a different target damaged by pellet 3; acquisition/pursuit; two original
facing calls; shotgun sound before internal facing; one shared aim and three
independent spread/damage rolls; real blood/puff effects; pain/death; return and
repeated state cycles. Every capture matches original-C evidence and independent
v11 bytes. The final 130-tic digest is `eb5b7166a162544e`. All ten previous examples
remain available; no asset, network, renderer or audio device is required.

Use `with_shotgunner_registry(map, limit, registry, options)` with the exact named
public fixture adaptations in [M3D3D_REFERENCE](docs/M3D3D_REFERENCE.md) and the
example fixture builder. `shotgun_attack(actor)` executes the direct action;
ordinary state entry/countdown executes the same closure. SPOS states preserve
10/10/10 tics and the second state's fullbright frame 32773. This differs from
PosAttack's 10/8/8 animation and sound order, whose v10 behavior remains unchanged.

The profile retains one unshootable first owner with zero physical momentum on
its floor, unbound dummy135 bodies and exact real impacts. Three sequential plans
use a small overlay and the shared combat executor, so prior health/flags/height/
state/momentum/RNG changes affect later pellets. No full world/store clone occurs
per tic. A non-null blast conservatively requires three free actor slots even
when the prospective shots miss; pending removals give no capacity credit. Null
returns need no slots. Complete Shotgun Guy, drops, other attacks, actual player
damage and full vanilla/demo compatibility remain outside this slice.

## Chaingunner attacks and refire (M3d3e)

```sh
cargo run -p doom-sim --example chaingunner_attacks --locked
python3 tools/reference/check_chaingunner_attacks.py
```

The asserted example compares eight public synthetic scenarios with original C
and independent v12 bytes at every capture. It covers hit/miss permutations,
a moving target and fresh aim, acquisition/pursuit, shotgun sound before internal
facing, real impacts/damage, dead/occluded refire exits with immediate Chase, and
repeated firing pairs. The 150-tic cycle ends at `54ac5f9c158c8e29`. All eleven
earlier examples remain available; no assets, network or devices are needed.

`with_chaingunner_registry(map, limit, registry, options)` accepts the exact named
fixture adaptations in [M3D3E_REFERENCE](docs/M3D3E_REFERENCE.md) and the example.
`chaingunner_attack(actor)` performs one original CPosAttack;
`chaingunner_refire(actor)` performs the original decision and immediate handoff.
Normal state entry/countdown uses the same services. CPOS preserves sprite 38,
frames 4/32773/32772/5 and 10/4/4/1 tics; every shot has its own facing, aim and
spread/damage/effect draws. Refire faces before its draw: below 40 retains the
loop, otherwise null/dead mobj/failed source sight returns immediately to see.

The original RNG never returns 40. Actual table values 39/41 are tested, with a
separate controlled-dependency C probe for the exact threshold. One stationary
first owner, unbound dummy bodies, validated references and existing work/movement
bounds retain preflight atomicity. Hits reserve their one impact slot; misses,
null actions and refire need no impact allocation. Full Chaingunner, drops, other
attacks, actual player damage and complete vanilla/demo compatibility stay deferred.

## Demon melee foundations (M3d3f)

```sh
cargo run -p doom-sim --example demon_melee --locked
python3 tools/reference/check_demon_melee.py
```

This asserted headless example uses public synthetic fixtures and compares every
capture with original C and independent DSIM-v13. It demonstrates source pursuit,
selection sound, 8/8/8 timing, three separate facing calls, hit/pain/death,
moving/occluded misses, return to Chase and real generation reuse. Its independent
150-tic digest is `b6c312bf5fbcc7dc`; all twelve older examples remain unchanged.

`with_demon_melee_registry(map, limit, registry, options)` admits the named native
`FIXTURE_DEMON_MELEE_136` and unbound source-body target 135. Original
`MT_SERGEANT` means Demon/type 12, not Shotgun Guy. The fixture explicitly adapts
health 150/mass 400 and default speed/radius/height 10/30/56; it executes source
Look/Chase and copied SARG attack states. `demon_melee_attack(actor)` performs
original FaceTarget, current melee range/sight and `(R%10+1)*4` direct damage
through shared combat. Attack sound 52 belongs to Chase selection; the later
strike adds no duplicate attack sound or hitscan blood/puff. A miss still faces
but draws no damage. Selection does not guarantee a hit 16 tics later.

Normal raw 8/8/8 timing is explicit; Nightmare's global timer rewrite is rejected.
The one unshootable first owner, zero physical momentum/floor placement, 64-actor,
map/query/work bounds and actual-player protections remain enforced. This is a
bounded fixture, with source-body target pain/death, not complete Demon/Spectre
pain/death/raise gameplay, player damage or a playable engine. See the
[source investigation](docs/M3D3F_REFERENCE.md),
[compatibility](docs/COMPATIBILITY.md), [schema](docs/STATE_FORMAT.md) and
[validation ledger](docs/VALIDATION.md). The M3d3g mission conditionally authorizes
[PR #16](https://github.com/Copgunn/DOOM/pull/16) regular merge after documentation
corrections, fresh mandatory checks and post-merge verification before new runtime work.

## M3d3g Demon pain, death and corpses

Explicit `ActorRegistry::demon_lifecycle_fixture()` and
`Simulation::with_demon_lifecycle_registry(...)` extend the named SARG fixture
with shootability, original pain chance 180, 2/2 pain states, 8/8/4/4/4/-1 ordinary
death states and original sound actions. Pain sound 26 waits for PAIN2; death
sound 64 waits for DIE2. Kill initialization quarters height and sets corpse/
dropoff flags while retaining solidity; A_Fall clears solidity later at DIE4.
The terminal corpse retains its identity, physics and spatial membership.

The profile reuses shared damage/hitscan, lifecycle, movement and thinker services.
Direct damage supports declared unbound body/self/environment sources; explicit
hitscan is restricted to synthetic source-body shooters. It preserves knockback,
retargeting, RNG and post-physics melee evaluation without a full-state tic clone.
Metadata copies source Demon fields with named state aliases, simplified stand/run
loops, no COUNTKILL/scoring and no raise state. Original Demon/Spectre types remain
recognized but non-executable. Full monsters, resurrection, player damage,
projectile integration, dynamic sectors and demo synchronization remain deferred.

```sh
cargo run -p doom-sim --example demon_lifecycle --locked
```

The asserted public synthetic example compares nine scenarios with original C and
independent DSIM-v14, including the 70-tic pain/death digest `eecc4b4dcc4fbca5`.
No IWAD, GPU, audio device or network is needed. See the
[source investigation](docs/M3D3G_REFERENCE.md), [compatibility](docs/COMPATIBILITY.md),
[state format](docs/STATE_FORMAT.md), [testing](docs/TESTING.md) and
[validation ledger](docs/VALIDATION.md). All thirteen earlier profiles/fixtures
remain preserved. [PR #17](https://github.com/Copgunn/DOOM/pull/17) requires separate owner review
and stays open/unmerged without auto-merge.
