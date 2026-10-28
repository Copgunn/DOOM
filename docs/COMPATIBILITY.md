# Compatibility contract

## Reference and current limits

Initial target: **Linux DOOM 1.10**, preserved in `linuxdoom-1.10/`, upstream
[id-Software/DOOM](https://github.com/id-Software/DOOM) commit
`a77dfb96cb91780ca334d0d4cfd86957558007e0`.
M1 parses IWAD/PWAD bytes and structurally validates classic maps. M2 implements
a narrow headless simulation with isolated C arithmetic, RNG, scheduling, geometry,
XY and thrust comparisons. M3a separately implements bounded grounded player
sliding and solid actor collisions with original C comparisons. M3b adds an
explicit static multi-sector/vertical profile with independent C traces. M3c adds
validated typed actors and a bounded state/action subset with separate C traces.
Rendering, demo playback, full gameplay synchronization
and authentic IWAD validation remain unverified.

DOS DOOM 1.9 is a separate future target. The released Linux source is not a
complete DOS specification. Linux's `VERSION = 110` and matching demo header
acceptance do not establish DOS 1.9 replay or synchronization compatibility.

## Findings from the preserved source

| Reference | Observed behavior | Engineering implication |
| --- | --- | --- |
| `doomdef.h` | `TICRATE = 35`, `VERSION = 110` | Separate simulation tics from presentation and version guarantees |
| `m_fixed.h`, `m_fixed.c` | 16 fractional bits, widened multiply, guarded division; active `FixedDiv2` uses `double` | Specify widths, overflow and rounding from evidence; do not blindly assume integer division is equivalent |
| `m_random.c` | Separate `prndindex`/`rndindex`, increment-before-read table lookup, reset to zero | Preserve gameplay call order and reset behavior independently of presentation |
| `p_tick.c` | Append thinkers, traverse list order, defer removal; players before thinkers/specials | Define deterministic scheduling and spawn/removal behavior with reference traces |
| `w_wad.h`, `w_wad.c` | Signed 32-bit directory fields, eight-byte names, backward name search for patch precedence | Bound input safely; preserve ordered duplicate/override resolution |
| `doomdata.h`, `p_setup.c` | Ordered classic map lumps, binary records, inter-record references | Decode explicit little-endian layouts and validate before use |
| `g_game.c` (`G_BeginRecording`, `G_DoPlayDemo`) | Writes version 110 and rejects a different demo version | Test version-specific headers separately from complete demo compatibility |

## Implemented M1 policies and deviations

- Signed little-endian WAD counts/offsets/lengths must be nonnegative and in
  bounds. Checked arithmetic and configurable limits precede access/allocation.
  Empty lumps at offsets zero through EOF are accepted; past-EOF empty markers
  are rejected. Shared/overlapping ranges, including header/directory overlap,
  are allowed when bounded. Directories need not be at the end of the file.
- Names retain all eight original bytes. Lookup truncates/pads queries to eight
  bytes, stops at NUL and folds ASCII case on both stored names and queries.
  `W_AddFile` uses `strncpy` (padding after NUL), while `W_CheckNumForName`
  uppercases only the query and scans backward. Symmetric folding extends lookup
  to lowercase directory names as requested; strict lowercase-directory bug
  emulation is not claimed. Non-ASCII bytes are not locale-folded. Raw
  `LumpName` equality remains exact byte equality.
- Later matching entries and later archives win. Maps select the last marker,
  then require THINGS through BLOCKMAP in reference order within one archive.
  Groups cannot spill across archives or borrow independently overridden lumps.
  A malformed overriding map errors instead of silently falling back.
- E#M# and MAP## markers are recognized. TEXTMAP identifies UDMF; BEHAVIOR
  immediately after the classic group identifies Hexen. Known XNOD/ZNOD,
  XGLN/ZGLN and xNd4/xNd5 node signatures are explicitly unsupported. No format
  inference can reliably distinguish arbitrary unmarked alternative layouts.
- Fixed records must have exact widths: things 10, lines 14, sides 30, vertices
  4, segs 12, subsectors 4, nodes 28, sectors 26 bytes. Essential geometry tables
  cannot be empty. Ordinary references follow signed nonnegative Linux 1.10
  shorts, limiting tables to 32768 entries; extended unsigned-index maps are
  unsupported. Only sidedef -1 is a sentinel, mapped to None; front sides and
  two-sided back sides must exist. Seg side selectors must address existing sides.
- Every subsector has a nonempty bounded seg range with one front sector. BSP
  children address valid nodes/leaves, all cycles are rejected, and partition
  deltas cannot both be zero. A node-free map requires exactly one subsector.
  Shared/unreachable acyclic nodes/leaves and overlapping subsector ranges are
  permitted; node ordering need not be topological. The last node is the root.
- REJECT must contain at least ceil(sectors²/8) bytes; trailing bytes are retained.
  Short-table overreads/emulation are intentionally rejected.
- BLOCKMAP uses signed positive dimensions, signed nonnegative word offsets,
  offsets beyond the complete cell table, leading zero list words, validated
  linedef references and -1 terminators. Shared lists/suffixes are allowed;
  unused trailing words are harmless. The exposed list includes the leading
  zero as linedef 0, as `P_BlockLinesIterator` does. Large unsigned/extended
  BLOCKMAP encodings and malformed-list recovery are unsupported.

These checks reject unsafe input the C implementation may overread, divide by
zero on, or traverse indefinitely. They establish structural safety, not that a
node builder's partitions/bounds match geometry, that a BLOCKMAP covers every
intersecting line, or that textures/thing types resolve. Those semantic and
runtime questions require later verification.

Default archive limits: 512 MiB input, 1,000,000 lumps, 128 MiB per lump.
Default map limits: 64 MiB per lump, 32768 records per table, 262144 BLOCKMAP
cells, 4194304 BLOCKMAP words, 128 MiB cumulative decoded/scratch allocation.
Applications can lower limits; raising them cannot enable unsupported formats.
All caller-provided input bytes must already have been acquired within the
application's own I/O and memory limits.

## Implemented M2 behavior and evidence limits

Source findings recorded before implementation are in [M2_REFERENCE.md](M2_REFERENCE.md).
Reproduction, adapters and compiler/source hashes are under `tools/reference/`
and `tests/reference/provenance.json`.

- `Fixed` is signed i32 16.16. Add/sub/negate and narrowed multiplication wrap
  explicitly; widened i64 multiplication uses arithmetic shift. Division retains
  the original `abs(a)>>14 >= abs(b)` guard and active binary64 divide, scale by
  65536 and toward-zero conversion. The conservative guard can saturate before
  a mathematically representable quotient; this is tested, not replaced.
- Safe policies use unsigned INT_MIN magnitude, arithmetic negative shifts and
  low-bit narrowing. Guarded zero division saturates by sign, including 0/0 ->
  MAX. Direct `div2_checked` returns errors for zero/range failure, replacing
  fatal/undefined conversions. Map conversion and angle shifts wrap explicitly.
- The GNU C profile compares 1,303 arithmetic results, excluding abs(INT_MIN)
  guard inputs and zero-divisor direct helper calls. Rust extremes have separate
  policy assertions. Recorded valid double-helper samples also match an integer
  experiment; this is not exhaustive proof or justification to replace the
  active double path. GNU narrowing/right-shift assumptions and the unsigned
  angle-shift adapter are explicit in the provenance.
- Exact RNG table values and both indexes match 512 C lookups, including wrapping.
  Reset, restore and stream separation are tested. Parameterized spawning consumes
  one gameplay draw for lastlook modulo four as observed in P_SpawnMobj; complete
  type/state-driven spawning is not implemented.
- Thinkers append, including same-traversal tail spawning. Current/already-visited
  marks wait until a later traversal; unvisited marked thinkers retire without
  updating. Rust saves links before retirement; the C adapter retains storage to
  avoid the original read-after-free. Generation checks, deterministic lowest-slot
  reuse, exhaustion retirement, clear, empty scheduling and exact order are tested.
  Explicit callback visit-budget failure retains completed updates; this safe
  bound is a project policy. Handles are store-scoped, not global identifiers.
- Each step is one 35 Hz logical tic: ordered command slots, thinkers, then
  Specials/Respawn markers and tic increment. Specials/respawn gameplay is absent.
  Pause freezes commands/RNG/events/time. u64 tic exhaustion rejects before mutation.
- Supported worlds have one static sector, one-sided lines, no line/sector
  specials and grounded non-player movers. Other profiles fail explicitly. Line
  predicates match 162 C cases; XY state/substeps/candidates match two 96-step
  layouts, including distinct cell lists proving x-outer/y-inner BLOCKMAP order.
  Deduplication follows first encounter including dummy linedef zero, without
  depending on a wrapping global validcount.
- XY preserves ±30-unit clamping, positive-only >15-unit splitting, toward-zero
  half displacement versus arithmetic-shift remainder, destination-box collision,
  non-player blocked momentum clearing, strict stop thresholds and `0xe800`
  friction. It is not a swept or circular collision approximation.
- Ground angle/thrust matches 112 C-adapter cases using exact source sine/cosine
  data. Binding commands does not make movers complete vanilla players: blocking
  and friction use the non-player profile. Sliding, actor collisions, Z/gravity,
  portals/steps/dropoffs, view/weapon state, monsters, specials and respawn remain
  incomplete. Nonzero buttons fail explicitly rather than doing invented work.
- [DSIM-v1 state](STATE_FORMAT.md) includes static map data, slot/generation/links,
  processing order, actors, commands, RNG and events. Independent Python encoding
  of C-derived coasting values checks full first-tic bytes and twelve per-tic
  goldens. Longer 768-call traces test lifecycle, pause, wrapping, presentation
  independence and first divergence. Project consistency and hashes are not
  full-engine synchronization evidence.

Initial C provenance: Linux x86_64, GCC 14.2.0, GNU89/O0, explicit signed char,
wrapping arithmetic, disabled FP contraction and standard excess-precision flags.
Recorded compiler behavior and adapters define the comparison boundary. Portable
C undefined behavior, old x87 builds, DOS 1.9, authentic assets and geometric/spatial
correctness beyond M1 structural validation are not thereby established. Traces
made from private assets must never become public fixtures or logs.

## M3a grounded player and actor collision profile

`Simulation::player_collision` selects profile 2 and DSIM-v2; `new` and
`with_actor_limit` retain M2 profile 1 and byte-identical v1 fixtures. Binding a
player in profile 2 enables original grounded player XY response, not complete
P_PlayerThink. Solidity is an independent zero/MF_SOLID-only choice. No arbitrary
flag bits can silently enable missiles, pickups, shootable/Z or teleport mechanics.

Player blocking invokes original three-corner sliding, fixed DDA/intercept math,
source fine/tangent tables, stable equal-fraction ordering, 0x800 approach fudge,
two retries and Y-first/X-second fallback. Axis/angled walls, corners, endpoints,
negative momentum and reordered collision ties are compared. The stop threshold
requires zero forward/side input for a player; angle-only input still permits
stopping. Non-players retain blocked momentum clearing.

Things precede lines, using X-outer/Y-inner cells expanded by MAXRADIUS=32. Strict
axis-square overlap excludes self and permits exact radius-sum contact. Non-solid
movers still block against solid candidates. Same-cell successful moves reinsert
at head. Spatial removal is immediate even while thinker storage remains pending;
slot reuse appends to thinker order and independently prepends to spatial order.
Setup spawning permits overlap, as P_SpawnMobj does, rather than inventing an
occupancy-rejection rule. No pickups mean no removal side effects during collision
traversal; generic thinker removal scheduling remains covered by M2 tests.

The bounded profile rejects multi-sector/two-sided/special geometry, degenerate
lines, overflowing deltas, vertices outside BLOCKMAP, over-limit data (128 lines,
4096 cells, 65536 words, 1024 actor slots), and incomplete conservative coverage.
Coverage requires all line-bbox cells to list the line; this can reject valid
vanilla diagonal blockmaps. M1's parser and M2's source-list behavior are unchanged.
Duplicate/shared lists preserve first-encounter order and have bounded compilation.
Malformed/unsupported setup errors are distinct from normal collision blocking.

Rust actor-overlap differences widen before absolute value; signed-overflow and
abs(INT_MIN) cases have safe tested policies, not C-equivalence claims. Other
fixed operations explicitly wrap/narrow/shift. Dynamic intercept storage replaces
unchecked C buffer writes; stable ordering and bounded traversal are preserved.
The reference adapter excludes undefined position-difference/absolute-value cases,
uses explicit unsigned angle shifts, keeps original C files unchanged and aborts
unsupported gameplay paths. See [investigation](M3A_REFERENCE.md) and the separate
`tests/reference/m3a_provenance.json`; M2 provenance remains unchanged.

Observed local evidence: 252 C tics match XY, momentum, angle, ordered attempts,
line/actor candidates and chosen slide walls. Independently encoded DSIM-v2
coasting bytes/digests match twelve tics. Long player-wall/actor/lifecycle traces
match across independent instances and presentation read schedules, and detect
an injected RNG divergence at the first field/tic. These are scoped comparisons
and project consistency checks, not complete vanilla gameplay synchronization.
Recoverable failed steps preserve full canonical state; allocations can still
terminate according to Rust's process-level policy. No per-tic full-state clone.

Complete player state/view/animation, portals/steps/dropoffs, vertical movement,
gravity, combat/monsters/weapons, specials/respawn, demos and rendering remain
unsupported. No commercial assets were provisioned or tested.

## M3b static-sector/vertical profile

`Simulation::multisector` / `with_multisector_limit` select profile 3 and DSIM-v3.
Older profiles and golden fixtures remain unchanged. The new profile implements
original BSP lookup (last-node root, stored children, exact side rules, first-seg
leaf sector), not generic polygon containment. Lookup is iterative and bounded.
Structural validity does not prove that BSP partitions or block lists match a
node builder's intended geometry.

Two-sided openings use min ceilings, max floors and min dropoff floor. Crossing
boxes accumulate constraints in original things-before-lines, X-outer/Y-inner
candidate order. One-sided walls and ML_BLOCKING block; ML_BLOCKMONSTERS blocks
ordinary non-players. Source TryMove checks interval fit, headroom, >24-unit step,
then >24-unit dropoff without MF_DROPOFF. Exact clearance and 24-unit equality
pass. Player binding and dropoff permission are independent. The slide traverser
uses its original opening/height rules, including its asymmetry with line flags
and dropoff; final movement still performs full TryMove checks.

Successful XY attempts update constraints and BSP membership, then relink cell and
sector heads even inside the same cell/sector. The contacted floor can be higher
than the center sector while straddling a portal. XY precedes Z for each thinker.
Airborne input changes angle but adds no thrust; z>floor skips ground friction.
Z adds momz, clips at floor, applies normal gravity above floor (zero momz first
becomes -2 units; otherwise subtract one), then clips ceiling and resets positive
momz there. MF_NOGRAVITY is supported alone, not full MF_FLOAT/flying mechanics.
No terminal velocity is invented. Ordinary solid collision remains the source
XY-only overlap rule even for actors separated vertically.

M3b explicitly trusts supplied structurally valid BLOCKMAP lists. M3a's
conservative bbox coverage rejects legitimate diagonal layouts because not all
bbox cells intersect the line. M3b reports conservative omissions, preserves
source candidate order and never rebuilds missing data. A deliberately omitted
wall permits movement just as C does; acceptance is not a spatial-correctness
guarantee. Old M3a strict behavior is preserved. Sparse and incomplete layouts,
reverse traversal, diagonal openings and multiple adjacent openings are tested.

Bounds: 128 lines/sectors, 4096 BLOCKMAP cells/nodes/leaves, 65536 words, 1024
logical actors. Reject degenerate lines, inconsistent two-sided flags, specials,
inverted sectors and a global vertical span that overflows signed fixed-point
subtraction. Zero-height openings are valid blockers. Actors must fit their spawn
sector; floor/ceiling/absolute initial Z and explicit momentum are supported.
M1 rejects invalid references/cycles before simulation. Raw coordinate/Z overflow
uses explicit safe wrapping policies outside the defined C comparison domain.

Observed isolated C evidence: 696 physical tics, 261 destination probes, 1280 BSP
lookups, 512 side predicates and 20 opening records. Both movement results and
ordered constraints/candidates/slide events/sector lists match. Separate Python
encoding verifies 96 DSIM-v3 digests and five full states across steps, transitions,
drop, gravity and landing. C fixture conversion uses representable multiplication
instead of negative signed loader shifts; ABI/compiler assumptions and unsupported
branches are explicit in `m3b_provenance.json`. No Rust output generates vectors.

Actor/player state transitions, view/bob/landing-camera feedback, sound, damage,
missiles/skulls/corpses/floating monsters, pickups, teleportation/no-clip, dynamic
heights/doors/lifts, specials, enhanced stacked-sector/visual portals, demos and
rendering remain unsupported. C view side effects execute only in the isolated
adapter; comparisons cover physical state, not complete P_PlayerThink. Reference
matches and deterministic project tests do not establish full-engine or DOS sync.

## Determinism objectives

- Exactly 35 logical tics per second; rendering rate and platform clocks cannot
  affect a simulation step.
- Explicit fixed-point/integer behavior, RNG sequencing, and actor/thinker order.
- Identical commands, initial state, compatibility profile, and resource bytes
  produce identical canonical state sequences across supported platforms.
- Keep input, filesystem work, audio, and rendering outside gameplay authority.
- Never use unordered containers or parallel scheduling to determine gameplay order.

## Extending verification

M1 tests generated archives/maps, malformed bounds/references, patch precedence,
adversarial sharing and iterative deep BSP validation. Separate WAD/map fuzz
targets use only synthetic seeds. Bounded smoke testing is not exhaustive proof
of compatibility or absence of defects. M2 runs canonical state and repeatability
checks in debug/release across supported platforms. Extend that schema as player,
sector/special and other gameplay state becomes implemented; retain ordered RNG,
actor and command/event coverage, excluding pointers, padding, GPU objects and clocks.

M3 compares per-tic traces against a pinned, reproducibly built reference using
legal fixtures. Capture reference commit, build settings, data hashes, commands,
profile, and first divergent field/tic. M4 adds version-specific demo fixtures,
header rejection tests, complete playback, and synchronization diagnostics.
Checksums are useful summaries, but a mismatch must have field-level diagnostics.

Reference C undefined behavior, host ABI differences, and intentional safe-input
rejections require documented decisions and regression cases, not accidental
Rust behavior. Performance benchmarks come after correctness baselines and must
record toolchain, platform, build profile, dataset, and repeatability.

## Assets

CI requires no proprietary IWAD. Generate fixtures in code or use explicitly
redistributable data with provenance and license recorded. Never commit commercial
IWADs, extracted proprietary assets, or private repository content. Locally
available reference-engine helpers are environment-specific and are not evidence
of Rust compatibility unless a comparison was actually performed.

## M3c actor-state profile

`Simulation::actor_states` / `with_actor_registry` select profile 4 and DSIM-v4.
All 137 types/967 states/74 action identities are recognized. Only admitted
spawn/death closures execute: 67 types fit the flat 128-unit reference fixture,
with five source actions (Fall/Pain/Scream/XScream/PlayerScream). Source actions
emit inert sound observations, never audio. Recognition is not monster support.

Direct spawning skips state actions/zero chaining; thinker countdown/entry runs
after XY/Z. Entered zero chains execute all actions; -1 persists; null entry
preserves old presentation/timer fields while unlinking and marking removal.
Native probe scenarios verify recursive entry, RNG, child append/same-pass think,
mutual targets and deferred retirement against extracted source bodies. Retained
IDs resolve unavailable after retirement; original C dangling dereferences have
no safe compatibility meaning. No eager reference-clearing rule is invented.

Map placement supports single-player DOOM II, five skills, nomonsters, original
DoomEd lookup/order, positive-tic RNG randomization, signed angle quantization and
ambush. Player starts and bounded deathmatch starts are reported without full
player creation. Other modes and unsupported selected types reject explicitly;
whole-batch validation is a deliberate safe departure from fatal partial C setup.
Controlled death enters the registered death state only. Health, height, kill
flags/scoring/drop processing and randomized combat timers are untouched; terminal
objects can persist. Full players, combat/AI/weapons/projectiles/pickups, corpse
physics, resurrection, respawn, moving specials, scripts and demos remain later.

The exact bounds, flags, adapter assumptions, admitted set and 393 actor/444 event
observations are in [M3C_REFERENCE.md](M3C_REFERENCE.md). Independent v4 encoding
checks 99 digests and 24 complete states. All v1/v2/v3 evidence remains unchanged.
These comparisons establish only the declared Linux 1.10 subset, not a playable
game, authentic IWAD validation, full vanilla synchronization or DOS 1.9 behavior.

Source-special skull/boss checks use type IDs, independently of native display names.

## M3d1 / explicit profile 5

This profile adds a verified Linux 1.10 subset to M3c; profile 4 controlled death
retains the contract above. `damage_actor` implements the unbound, non-player-
pointer branch: early ignored nonshootable/dead targets; source angle/thrust and
forward-fall random branch; health subtraction; nonlethal pain draw/JUSTHIT/state,
reaction reset, target/threshold and conditional see entry; lethal kill flags,
quarter height, strict extreme-death threshold, action-before-randomized-timer
order, persistent corpse and source corpse friction exception. Gravity/dropoff
and solidity projections track the resulting source flags/actions.

`aim_line_attack` consumes no RNG/events. It implements source autoaim rather than
general P_CheckSight or monster perception. `line_attack` accepts supplied damage
and slope, preserves original DDA start nudge, BLOCKMAP callback order, box-diagonal
thing fractions, stable equal fractions, inclusive vertical/range boundaries and
source four/ten-unit hit backoff. Two-sided flags do not substitute for opening
rules. Real source blood/puff spawning consumes jitter, lastlook and timer draws
BEFORE damage; zero damage still spawns effects. These actors participate in
movement, state countdown and retirement.

Admission recognizes all source metadata but executes only validated state/action
closures and flags. Five original sound/fall actions remain the only supported
original callbacks. Profile 5 additionally admits NOBLOOD; source runtime JUSTHIT
and CORPSE are consequences, not permission to execute AI. Tests use unbound
MT_PLAYER and type 136 `FIXTURE_PAWN`, copied from source player metadata with
explicit health/pain/mass/flags/reaction/height overrides, never a real player_t.
Native fixtures isolate mechanics and do not establish vanilla monster support.

Bound damage participants/shooters, unsupported flags/actions, COUNTKILL/scoring,
four item-drop identities, sky ceilings, line specials, resurrection/nightmare
respawn, inventory/armor/immunity, weapons/psprites/ammo, AI, projectiles, radius
attacks and advanced effects reject or remain outside the available API. All
selected damage closures and effect dimensions are prevalidated. Native registry
changes to source impact types/states reject. Damage 0..=1000, positive mass and
spawn health, pain chance 0..=256 and positive range <=2048 define this slice.
Unsafe coordinate setup, >128 intercepts, incomplete 64-cell DDA or insufficient
conservative spawn/action capacity reject before canonical mutation. These safe
errors do not reproduce original memory overflow or fatal partial setup.

48 C scenarios compare 431 captures, 38,052 live actor-field values, candidate/
intercept/callback order and full supported multi-tic death/RNG sequences. 63
independent v5 captures include 17 byte fixtures. Original C jitter has negative
signed shifts and unspecified operand evaluation: the isolated adapter explicitly
evaluates left then right RNG and multiplies by 1024, documented in provenance.
C-undefined extremes are tested as Rust safety policies. Long repeated runs and
fuzzing are project consistency evidence. None establishes full vanilla/DOS demo
synchronization, authentic IWAD validation or a playable game.

## M3d2 / explicit profile 6

Pistol, shotgun and optional chaingun now execute through source player commands
and psprites. Unpowered fist is included for exact exhausted-ammo fallback.
Source LIVE initialization uses health 100, fist+pistol ownership, 50 clips,
maxammo [200,50,300,50] and attackdown true. Initialization immediately executes
Raise, producing weapon Y=122. One player can occupy any slot 0..3; the profile
admits at most 64 actors. Full P_SpawnPlayer/HUD/camera/intermission/respawn parity
is excluded. Weapon bob and PLAY/RUN/ATK state timing are implemented; complete
player view/landing effects remain outside the verified weapon slice.

ATTACK=1 and CHANGE=4 with selection bits 3..5 are supported. Reject USE, SPECIAL,
unknown bits, selections without CHANGE, deferred weapons and active weapon buttons
in inactive slots before mutation. Valid unowned selections are ignored. Weapon
changes wait for source ready/refire/lower states; attack during raise/animations,
held fire, release/repress and last/zero rounds follow source timing. wp_nochange
is **10**, after NUMWEAPONS=9. No reload mechanic is invented for these weapons.

Source psprite null entry retains tics/coordinates; zero states chain, -1 persists,
misc1 gates both coordinates, and current state/tics after recursive actions govern
continuation. Flash updates on its creation tic. A_WeaponReady, Lower, Raise,
ReFire, FirePistol, FireShotgun, FireCGun, Light0/1/2 and unpowered Punch execute
in the separate psprite dispatcher. A_GunFlash was investigated but is not reached
by these admitted weapons; rocket/BFG paths requiring it remain excluded.

BulletSlope probes base/+/-2^26 at range 1024 without changing actual shot angle.
Pistol first fire is accurate, refire spreads; shotgun aims once and fires seven
sequential pellets. Damage draw precedes spread draws, then real impact/RNG and
actor damage complete before the next pellet. Chaingun has two firing states,
source flash offset and a sound-before-zero-ammo return. Fist uses its original
unpowered damage/spread/aim, hit sound and target-facing turn. Berserk is excluded.
Source spread's undefined negative shift/evaluation order is adapted to ordered
left/right draws and multiplication, as recorded in provenance.

P_FireWeapon's unconditional P_NoiseAlert is implemented, including source sector
line order, openings, at most one SOUNDBLOCK crossing and persistent sound targets.
It does not execute monster perception/AI. Target handles retain generations after
removal and never alias reused slots. Rendering remains independent.

Only exact original state tables/source types and the explicit player-state fixture
variation are admitted. Shootable types are unbound source player pawns or the
validated type-136 fixture; source pain/see/death closures are preserved. Native
combat probes, AI/callback spawning/removal, scoring/drop types and unsupported
weapon actions cannot enter this profile. The private firing route admits the
initialized bound shooter, while direct combat and target damage remain protected.
Capacity reserves an impact for every requested bullet, even a predicted miss,
so a full store can conservatively reject a source request that would spawn fewer
effects. Pending retirement supplies no capacity credit. Trace/coordinate limits,
work and invalid setup errors preserve
all canonical state. Source-undefined extreme player bob momentum rejects safely.

38 original-body C scenarios compare 4,890 player captures, 132,030 player fields,
290,913 live actor fields, actor angles, sector sound targets and combined weapon/
lifecycle event order. Repeated firing covers RNG wrap, sequential targets and
complete pain/death progression. Independent v6 encoding checks 69 captures and
24 full byte fixtures, including initialization/raising/attack-start and settled
results; per-tic C comparisons cover transient firing states separately. Eleven
additional unit/policy tests and fuzzing are project consistency evidence.
No complete players, multiplayer gameplay, inventory/powers, chainsaw/supershotgun,
projectiles, demos, authentic IWAD or full vanilla/DOS synchronization is claimed.

## M3d2b projectile profile (DSIM-v7)

Only original MT_PLASMA (34) executes as a missile: flags 67088, speed 25, radius
13, height 8, damage multiplier 5, states 107/108 then explosion 109..113/null.
Recognizing other source metadata does not admit other missiles. Explicit launch
selects P_SpawnMissile or P_SpawnPlayerMissile semantics; no firing action, ammo
consumption or actual player initialization/damage is added. Unbound MT_PLAYER
objects and labelled player-state fixtures 15/17/136 isolate damage/species rules;
the latter do not establish baron/knight/monster gameplay.

41 original-body C scenarios compare 157,872 actor fields, 13,189 ordered
observations, 1,194 movement attempts, 6,229 candidate visits, 16 openings and
549 RNG draws. This includes a synthetic launch wrapper inside original
P_RunThinkers. Original spawn/physics/state/damage bodies execute; the wrapper
is scheduling evidence, not original AI. Independent v7 encoding covers 129
captures/35 byte streams, including full wall/dropoff lifecycles, lethal settled
state and second-batch slot generations. Project-only policy tests cover errors,
unsafe setup, protected players, presentation independence and boundary predicates.

Live missiles require an active, generation-valid owner. Pending/retired owners
reject a step before mutation; callers may remove the missile first. Explosion
frames retain the original owner identity without dereferencing it. Intended aim
targets are not retained as tracers. Source half-step-before-relink, positive-only
subdivision, residual XY after explosion, strict XY/vertical-touch distinctions,
no missile friction and source supplied BLOCKMAP lists are preserved. Missing
line coverage is diagnostic; the omission fixture deliberately retains the
required zero word and omits other lines.

Exact arithmetic/setup/admission bounds are in [M3D2B_REFERENCE.md](M3D2B_REFERENCE.md).
Outside them, rejection is an explicit safe departure, not emulation of C overflow
or dangling pointers. All sky/special maps remain rejected. There is no splash,
rocket/BFG/other fireball, monster perception/attacks, inventory, armor/player
damage, demos, real-IWAD/full vanilla or DOS synchronization claim. Earlier
profiles/schemas/fixtures and their behavioral contracts remain unchanged.

## M3d3a perception profile (DSIM-v8)

The admitted slice matches original Linux 1.10 P_CheckSight and P_LookForPlayers
on synthetic classic maps: REJECT sector-pair bits, BSP start-child-first order,
seg-side openings, line duplicate stamps, fixed intercepts/slopes, four-slot
active/health order, lastlook stop/count rules and unsigned FOV/close-range tests.
The source horizontal P_DivlineSide `x==node.y` equality quirk is retained. The
ML_BLOCKING flag and omitted BLOCKMAP walls do not replace sight geometry.

A_Look resets threshold, assigns a shootable sound target before ambush sight,
and falls back to player scanning after failed ambush confirmation. Failed
fallback may leave that target assigned without waking. Source wake RNG groups,
global SPIDER/CYBORG sounds and immediate state-entry timing are compared to C.
The native fixture types 19/21/136 copy player-shaped metadata with labelled
states 1/2 and action-free see. These are not executable original monsters;
original A_Chase handoffs reject before mutation.

The perception roster's active flag, signed player health and generation ID are
separate from actor health, source MT_PLAYER metadata and full player bindings.
Active IDs must resolve even with nonpositive health. References are limited to
admitted stable player body states or the native look/see states so no earlier
thinker can invalidate them mid-tic. Inactive retained IDs cannot alias slot reuse.
Pending/stale sound IDs reject; a tic with a possible looker conservatively checks
all retained sector sound IDs because movement may change the observed sector.
Pure sight does not depend on these unrelated roster/cache invariants.

Rust-only policies terminate the original all-inactive infinite scan with false
and unchanged lastlook, reject unsafe identities and bound map coordinates/node
deltas/sector heights to +/-512, actor XYZ to +/-1024, expanded sight work to 8192
and actors to 64. Nonzero XY motion requires current coordinate within +/-994;
Z momentum is at most 64 and next pre-gravity Z must fit +/-1024. These are explicit
admission restrictions, not limit-removing compatibility. Query/scan/action/step
rejections preserve the full canonical state. See [reference findings](M3D3A_REFERENCE.md).

68 original-C cases and helper vectors are source equivalence evidence within
this slice; v8 goldens validate a project schema, and 12 policy tests validate
Rust ownership/atomicity. Neither implies full monster AI, pursuit/attacks,
commercial-map admission, Linux/DOS demo synchronization or savegame compatibility.

## M3d3b pursuit profile (DSIM-v9)

The source slice implements P_Move, P_TryWalk, P_NewChaseDir and non-attacking
A_Chase on synthetic classic maps. Eight source directions use 65536 cardinal and
47000 diagonal components times integer metadata speed. Spawn starts EAST/zero;
NODIR returns false without a query. Successful P_Move directly updates position,
constraints, links and floor Z; it clears INFLOAT. A blocked move leaves them
unchanged. TryWalk draws count&15 only after success. NewChaseDir retains strict
+/-10 thresholds, diagonal attempt, unconditional axis-priority draw, old direction
retry, random forward/reverse scan and turnaround last. No modern pathfinding.

A_Chase countdown/turning precedes target checks. Dead-but-shootable actors can
remain targets while their health clears threshold. Missing/nonshootable targets
use allaround player-record scanning; failure enters spawn/Look immediately and
can leave the old nonshootable identity stored. Reacquisition returns before
walking/active sound. JUSTATTACKED clears and picks a new direction except on
nightmare (fastparm=false). Normal predecrement movecount short-circuits P_Move
when negative. Nonzero active-sound metadata draws even if it selects silence.

Only labelled native look/chase fixtures have executable Chase. Their original
player-shaped metadata has melee/missile states explicitly removed, including
S_PLAY_ATK1. Source player bodies retain original inert animation metadata and
never execute Chase. Native speeds 1..16, radii 0..32, positive fitting heights and
state durations 1..35 are admitted. Original monster attack closures reject before
mutation; no attack check/action, float adjustment or door opening is stubbed.
Single-player source skills are supported with fastparm=false/netgame=false.

Maps must have line/sector specials 0, preserve supplied BLOCKMAP lists and satisfy
inherited M3b and sight bounds (+/-512 map values, 8192 expanded sight work).
Native current flags admit SOLID/SHOOTABLE/AMBUSH/JUSTHIT/JUSTATTACKED/DROPOFF/
PICKUP/INFLOAT/NOTDMATCH; no FLOAT, NOCLIP, TELEPORT, SPECIAL or respawning monsters.
PICKUP cannot reach a pickup callback because no admitted body has SPECIAL.
Source thing collision is Z-independent for walking, including unbound player
metadata; ML_BLOCKMONSTERS applies. A 55-unit opening test uses two individually
fitting sectors whose overlap is 55, so the height-fit failure is independently
exercised without violating type admission.

At most 64 actors; direct callback XY must fit+/-1008, tic XY+/-978. This proves
at most 30 physical+16 direct units within the +/-1024 sight/query slice. Source
counter INT_MIN rejects; reaction also rejects INT_MIN+1 because nested loss->
Look->Chase can decrement twice. Target/active roster/retained sound IDs must
resolve to active stable generations before chase effects; pending/stale IDs
reject rather than emulate dangling C pointers. Inactive old roster IDs remain
encoded without aliasing reuse. Empty-roster termination remains a Rust policy.
These restrictions can conservatively reject operations whose actual branch would
not use every dependency. Recoverable failures preserve full canonical state.

C equivalence covers 96 scenarios, including full thinker tics, under documented
ABI/arithmetic assumptions. The unsigned movedir BAM shift adapter defines the
original signed-shift intent without changing its decisions. Twelve ownership/
atomicity tests and independent DSIM-v9 bytes are project evidence, separately
identified. None establishes complete original monster AI, DOS/Linux demo sync,
commercial-map compatibility or a DOOM savegame format.

## M3d3c bounded attack profile (DSIM-v10)

The source-equivalent subset is Linux DOOM 1.10 range and Chase attack selection plus
FaceTarget and one fully admitted PosAttack animation on labelled native type 136.
Melee uses metadata radius and strict approximate-distance rejection before sight.
Missile sight precedes JUSTHIT clearing, reaction delay, signed distance/type
adjustments/caps and its final random comparison. Ordinary skills require zero
movecount; nightmare bypasses that gate. fastparm/netgame are fixed false.
Melee sound precedes state entry; missile JUSTATTACKED follows entry/actions.

Native3/5/18/19/21 arithmetic fixtures do not admit original Arch-vile, Revenant,
Lost Soul, Spider or Cyberdemon behavior. Action-free melee/missile markers prove
selection only. Exact named states 4/5/6 reproduce POSS 10/8/8 animation, both facing
calls, shadow/spread order, ((R%5)+1)*3 damage and return to native Chase. Existing
M3d1 aim/intercepts, real blood/puffs, thrust, health, pain/death and source target
relationships are shared, not replaced with presentation events.

Rust policy restricts one unshootable first thinker on its floor, with zero
physical momentum and speed1..16 direct walking; other shootable actors are
unbound dummy type 135 source-body fixtures. No player_t, binding/damage, armor or roster
health mirroring is introduced. Public direct combat remains in its earlier
profiles. Unsupported callbacks, pending/stale attack dependencies, removable
effect references, capacity and unsafe geometry/arithmetic reject atomically.
Map/query/sight/intercept/actor limits remain +/-512,+/-1024,8192,128,64. Owner
margins stay1008 direct/978 per tic; other live XY is within 994 for a tic. Effects
are prepared within 994 XY and 1018 Z, covering jitter and newborn vertical motion.

Source C comparisons include exact fixed-unit melee boundaries, type/cap and
short-circuit decisions, ordinary/nightmare gates, double shadow facing, all five
damage values, hit/wall/true miss, pain/lethal/extreme death, repeated tics,
retirement/reuse and RNG wrap. The open far-distance fixture uses real two-sided
self-referencing boundary geometry and original sight, without raising limits.
Ordered multiplication defines the historical negative-shift/call-order intent;
see source notes/provenance for ABI assumptions and excluded branches. Independent
v10 bytes and seven Rust safety tests are separate evidence. Full original
monsters, other attacks/melee damage, floating/doors/specials, actual players,
commercial-map validation and DOS/Linux demos remain unimplemented/unverified.
