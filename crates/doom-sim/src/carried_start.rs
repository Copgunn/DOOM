//! Wire 26 (`PlayerMonstersEpisode1V4`, M6a, owner's ruling U2 = B1): wire 25's tic
//! semantics, unchanged, plus a level start that is not always a new game.
//!
//! In linuxdoom-1.10 a level after the first is loaded inside a `G_Ticker`
//! (`G_DoWorldDone` or `G_DoReborn`, g_game.c:1172-1179, 924-932) by
//! `G_DoLoadLevel` (g_game.c:445-497) and `P_SetupLevel` (p_setup.c:583-693), and
//! the player it spawns is what `G_PlayerFinishLevel` (g_game.c:779-792) left at
//! the exit, unless the player is reborn (`G_PlayerReborn`, g_game.c:800-833, from
//! `P_SpawnPlayer`, p_mobj.c:659-660; a player dead at the load is made
//! `PST_REBORN` by `G_DoLoadLevel`, g_game.c:477-480). Neither random index is
//! reset there: only `G_InitNew` calls `M_ClearRandom` (g_game.c:1414,
//! m_random.c:69-72). This module is that load for the simulation: the three
//! start kinds of [`LevelStart`], the refusals of carried values the C cannot
//! hold, the start record DSIM42 encodes, and the read-only [`LevelFinish`] the
//! session layer takes from a finished level.
use crate::{
    player_weapons::{PlayerWeaponRuntime, PlayerWeaponState},
    weapons::{AMMO_CAPACITIES, MAX_BOB_BITS},
    *,
};
use std::{error::Error, fmt};

/// Explicit Episode 1 part 4 context (M6a, wire 26, DSIM42): wire 25's world with
/// a carried level start; older contexts stay frozen.
pub const PLAYER_EPISODE1_V4_CONTEXT_SCHEMA: &[u8] = b"PLAYER-MONSTERS-EPISODE1-4-v1";

/// The largest tic count a flash psprite can hold with a null state at a carried
/// start: `P_SetupPsprites` nulls the state and keeps the count (p_pspr.c:836-837),
/// which is at most the tics of the flash state an admitted weapon was in at the
/// exit (`S_PISTOLFLASH` 7, info.c:153; the shotgun's 4 and 3, the chaingun's 5,
/// the launcher's 3 and 4: info.c:166-167, 191-192, 199-202).
pub(crate) const MAX_CARRIED_FLASH_TICS: i32 = 7;

/// How a wire-26 level begins (DSIM42 `level.start.kind`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartKind {
    /// A new game: `G_InitNew` (`M_ClearRandom`, both indices 0) and a player
    /// made by `G_PlayerReborn` (the session's first level). Encoded 0.
    NewGame,
    /// The player carried from the previous level (`G_DoWorldDone`). Encoded 1.
    Carried,
    /// A player made by `G_PlayerReborn` with the carried `P_Random` index: the
    /// single-player reload of `G_DoReborn`, or a level loaded with the player dead
    /// at the exit (g_game.c:477-480). Encoded 2.
    Reborn,
}
impl StartKind {
    /// The DSIM42 encoding.
    pub const fn wire_id(self) -> u8 {
        match self {
            Self::NewGame => 0,
            Self::Carried => 1,
            Self::Reborn => 2,
        }
    }
}

/// The level start the wire-26 constructor takes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LevelStart {
    /// Wire 25's start exactly: a fresh player and both random indices 0.
    NewGame,
    /// The player `G_PlayerFinishLevel`, `G_DoLoadLevel` and `P_SpawnPlayer` carry.
    Carried(CarriedStart),
    /// A fresh player (`G_PlayerReborn`) with the carried `P_Random` index.
    Reborn {
        /// `prndindex` when the load begins.
        p_random: u8,
    },
}
impl LevelStart {
    /// The start's kind.
    pub const fn kind(&self) -> StartKind {
        match self {
            Self::NewGame => StartKind::NewGame,
            Self::Carried(_) => StartKind::Carried,
            Self::Reborn { .. } => StartKind::Reborn,
        }
    }
}

/// What a level carries into the next one: every field of `player_t`
/// (d_player.h:83-166) that `G_PlayerFinishLevel` (g_game.c:779-792),
/// `G_DoLoadLevel` (g_game.c:477-482), `P_SetupLevel` (p_setup.c:594-604),
/// `P_SpawnPlayer` (p_mobj.c:642-700) and `P_SetupPsprites` (p_pspr.c:831-842)
/// leave alone and the simulation models, plus the `P_Random` index.
///
/// Not here, because the load clears them: `powers` and `cards` (g_game.c:785-786),
/// `extralight`, `fixedcolormap`, `damagecount`, `bonuscount` (788-791, again at
/// p_mobj.c:677-682), `refire` and `message` (p_mobj.c:675-676), `frags`
/// (g_game.c:481), the three counts (p_setup.c:596-600), both psprite states and
/// the pending weapon (p_pspr.c:836-840). Not here, because the simulation does not
/// model them: `viewheight` (set to `VIEWHEIGHT` at p_mobj.c:683), `deltaviewheight`
/// and `viewz` are the display's (doom-render's view state; crates/doom-sim/src/level.rs
/// keeps none of them), `didsecret` and `cmd` are the session's, and the stale
/// `attacker` (a pointer into the freed level, never read before `P_DamageMobj`
/// writes it, p_inter.c:874-875) starts as NULL.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CarriedStart {
    /// `player->health`, also the new mobj's health (p_mobj.c:673): 1..=200.
    pub health: i32,
    /// `armortype` and `armorpoints`.
    pub armor: PlayerArmorState,
    /// `weaponowned` as bits (bit `i` is `weaponowned[i]`).
    pub weapons_owned: u16,
    /// `readyweapon`; `P_SetupPsprites` makes it the pending weapon and brings it up.
    pub ready_weapon: Weapon,
    /// `ammo` in `ammotype_t` order (clip, shell, cell, misl).
    pub ammo: [i32; 4],
    /// `maxammo`: the `G_PlayerReborn` table, doubled by the first backpack.
    pub max_ammo: [i32; 4],
    /// `backpack`.
    pub backpack: bool,
    /// `cheats`: not modelled by the simulation, so only 0 is admitted.
    pub cheats: i32,
    /// `attackdown`: the value the intermission's last tic left
    /// (`WI_checkForAccelerate`, wi_stuff.c:1476-1497).
    pub attack_down: bool,
    /// `usedown`, likewise.
    pub use_down: bool,
    /// `bob` (raw 16.16): kept by the load, recomputed by the first `P_CalcHeight`.
    pub bob: Fixed,
    /// `psprites[ps_weapon].sx` (raw 16.16): `P_BringUpWeapon` resets only `sy`
    /// (p_pspr.c:151).
    pub weapon_x: Fixed,
    /// `psprites[ps_flash].tics`, kept with a null state (p_pspr.c:836-837).
    pub flash_tics: i32,
    /// `psprites[ps_flash].sx` and `.sy` (raw 16.16).
    pub flash_position: [Fixed; 2],
    /// `prndindex` when the load begins.
    pub p_random: u8,
}

/// The carried field a refused [`CarriedStart`] names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CarriedField {
    /// Health outside 1..=200: a carried player is alive (a dead one is reborn,
    /// g_game.c:477-480), and no pickup raises health above 200 (p_inter.c:385-386,
    /// 402-403, 412).
    Health,
    /// Armour points outside 0..=200 (p_inter.c:258-263, 393-394) or a type that
    /// disagrees with them: points become 0 only with type 0 (p_inter.c:861-867)
    /// and a nonzero amount always comes with a type (258-263, 395-396).
    Armor,
    /// An owned set or ready weapon outside the simulation's domain: fist and
    /// pistol always owned (g_game.c:826-827, never cleared), the ready weapon
    /// owned, nothing beyond the Episode 1 policy's weapons.
    Weapons,
    /// An amount above its maximum (`P_GiveAmmo` clamps, p_inter.c:106-107) or
    /// negative.
    Ammo,
    /// Maxima other than the table (`maxammo`, p_inter.c:58), doubled exactly when
    /// the backpack is held (p_inter.c:589-595, the only other writer).
    MaxAmmo,
    /// Nonzero cheat flags: the simulation does not model cheats.
    Cheats,
    /// `bob` outside 0..=`MAXBOB` (p_user.c `P_CalcHeight` clamps it).
    Bob,
    /// Psprite coordinates or a flash count outside what `A_WeaponReady`,
    /// `A_Lower`, `A_Raise` and `P_MovePsprites` write (p_pspr.c:331-334, 392-398,
    /// 429-434; the flash copies the weapon's) or the flash states hold.
    Psprites,
}

/// A refused wire-26 construction; nothing is constructed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LevelStartError {
    /// A carried value the C cannot hold, or the simulation does not model.
    Carried(CarriedField),
    /// The caller's context gave its own random streams: under wire 26 the start
    /// kind alone decides the `P_Random` index.
    Context,
    /// The map, policy or skill was refused by wire 25's constructor.
    Registry(OriginalRegistryError),
    /// A load step (`P_LoadThings`, `P_SetupPsprites`, `P_SpawnSpecials`) refused.
    Setup(SimError),
}
impl fmt::Display for LevelStartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "level start: {self:?}")
    }
}
impl Error for LevelStartError {}

impl CarriedStart {
    /// The checks of [`CarriedField`], in that order.
    pub fn validate(&self) -> Result<(), LevelStartError> {
        let refuse = |f| Err(LevelStartError::Carried(f));
        if !(1..=200).contains(&self.health) {
            return refuse(CarriedField::Health);
        }
        if !(0..=200).contains(&self.armor.points)
            || (self.armor.kind == ArmorType::None) != (self.armor.points == 0)
        {
            return refuse(CarriedField::Armor);
        }
        let max = AMMO_CAPACITIES.map(|m| m << u32::from(self.backpack));
        if self.max_ammo != max {
            return refuse(CarriedField::MaxAmmo);
        }
        if (WeaponInventory {
            ready: self.ready_weapon,
            owned: self.weapons_owned,
            ammo: [0; 4],
        })
        .validate_with(max, true)
        .is_err()
        {
            return refuse(CarriedField::Weapons);
        }
        if self.ammo.iter().zip(max).any(|(a, m)| !(0..=m).contains(a)) {
            return refuse(CarriedField::Ammo);
        }
        if self.cheats != 0 {
            return refuse(CarriedField::Cheats);
        }
        if !(0..=MAX_BOB_BITS).contains(&self.bob.bits()) {
            return refuse(CarriedField::Bob);
        }
        // The psprite domain of the weapon-state validator (`PlayerWeaponState::
        // validate_for_policy`): x within -15..=17 units (`A_WeaponReady`'s
        // `FRACUNIT + FixedMul(bob, finecosine)`, p_pspr.c:331-332, the flash copying
        // the weapon's, 881-882), y within 0..=128 units (`WEAPONTOP` plus the bob,
        // `A_Lower` to `WEAPONBOTTOM`, `A_Raise` to `WEAPONTOP`: 333-334, 392-398,
        // 429-434), and a null flash's count within the flash states' tics.
        let x = -15 * 65536..=17 * 65536;
        if !x.contains(&self.weapon_x.bits())
            || !x.contains(&self.flash_position[0].bits())
            || !(0..=128 * 65536).contains(&self.flash_position[1].bits())
            || !(0..=MAX_CARRIED_FLASH_TICS).contains(&self.flash_tics)
        {
            return refuse(CarriedField::Psprites);
        }
        Ok(())
    }
    /// The weapon state `P_SetupPsprites` starts from (p_pspr.c:831-842): both
    /// psprite states null and the pending weapon the ready one; the weapon
    /// psprite's state is set by `P_BringUpWeapon` before any tic.
    fn weapon_state(&self) -> PlayerWeaponState {
        let w = |x: i32| u16::try_from(x).unwrap_or(u16::MAX);
        PlayerWeaponState {
            ready: self.ready_weapon,
            pending: None,
            owned: self.weapons_owned,
            bullets: w(self.ammo[0]),
            shells: w(self.ammo[1]),
            cells: w(self.ammo[2]),
            rockets: w(self.ammo[3]),
            backpack: self.backpack,
            attack_down: self.attack_down,
            refire: 0,
            bob: self.bob,
            extra_light: 0,
            psprites: [
                Psprite {
                    state: None,
                    tics: 0,
                    position: [self.weapon_x, Fixed::from_map_units(128)],
                },
                Psprite {
                    state: None,
                    tics: self.flash_tics,
                    position: self.flash_position,
                },
            ],
        }
    }
}

/// How a wire-26 level began, as DSIM42 encodes it: constant for the run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StartRecord {
    pub kind: StartKind,
    /// The player the load set up; for a new game and a reborn player the
    /// `G_PlayerReborn` values, with the start's `P_Random` index.
    pub player: CarriedStart,
    /// The sound records the load made (`P_BringUpWeapon`'s `sfx_sawup`,
    /// p_pspr.c:145-146), in call order.
    pub sounds: Vec<sound::SoundRecord>,
}

/// `G_PlayerReborn`'s player (g_game.c:815-831): health 100, no armour, fist and
/// pistol owned, pistol ready, 50 bullets, the maxima table, both latches set.
/// `bob` and the psprite coordinates are zero (the `memset`, 815).
pub(crate) const fn reborn_player(p_random: u8) -> CarriedStart {
    CarriedStart {
        health: original_player::INITIAL_PLAYER_HEALTH,
        armor: PlayerArmorState {
            kind: ArmorType::None,
            points: 0,
        },
        weapons_owned: 3,
        ready_weapon: Weapon::Pistol,
        ammo: [50, 0, 0, 0],
        max_ammo: AMMO_CAPACITIES,
        backpack: false,
        cheats: 0,
        attack_down: true,
        use_down: true,
        bob: Fixed::ZERO,
        weapon_x: Fixed::ZERO,
        flash_tics: 0,
        flash_position: [Fixed::ZERO; 2],
        p_random,
    }
}

/// Which exit a finished level requested (`G_ExitLevel` or `G_SecretExitLevel`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LevelExit {
    /// `G_ExitLevel` (g_game.c:1002-1006).
    Normal,
    /// `G_SecretExitLevel` (g_game.c:1009-1018).
    Secret,
}

/// What the session layer takes from a level whose next tic `advance` refuses
/// with `LevelExited` or `PlayerReborn` (read-only; nothing is cleared). Both
/// requests are reported because the simulation decides no order between them:
/// the C runs the reborn first (g_game.c:611-623, observed by the genuine items
/// `USEAT131`, `USEAT130`, `USEAT132` and `PROBE`), and that order is the session's.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LevelFinish {
    /// The exit requested in a completed tic, if any (`gameaction == ga_completed`).
    pub exit: Option<LevelExit>,
    /// A dead player pressed use (`PST_REBORN`, p_user.c:227-228).
    pub reborn: bool,
    /// The player is dead (`PST_DEAD` or `PST_REBORN`).
    pub player_dead: bool,
    /// What a living player carries (`G_PlayerFinishLevel`'s clears are not
    /// fields of [`CarriedStart`]), with the current `P_Random` index; `None` for a
    /// dead player, whose next level starts reborn. The latches are the exit
    /// tic's: the intermission rewrites both every tic (wi_stuff.c:1476-1497), so
    /// the session replaces them with its last intermission tic's values.
    pub carried: Option<CarriedStart>,
    /// `prndindex` now.
    pub p_random: u8,
    /// `totalkills`, `totalitems`, `totalsecret` (`wminfo.maxkills` and the rest,
    /// g_game.c:1112-1114).
    pub totals: [i32; 3],
    /// The player's `killcount`, `itemcount`, `secretcount` (g_game.c:1122-1131).
    pub counts: [i32; 3],
    /// `leveltime`: the completed level tics (`wminfo.plyr[].stime`, g_game.c:1130).
    pub level_time: u64,
    /// `gamemap`.
    pub map: u8,
}

impl Simulation {
    /// Episode 1, part 4 (M6a, wire 26, ruling U2 = B1): one whole level load,
    /// `P_SetupLevel` with the given start, ready for tic 0. The world is wire 25's
    /// ([`Simulation::original_player_episode1_v3_map`]) byte for byte in every
    /// state field and its tic semantics are unchanged; captures are DSIM42, read
    /// by [`crate::Diagnostic::decode_episode1_v4`]: DSIM41 under policy 26 and the
    /// [`PLAYER_EPISODE1_V4_CONTEXT_SCHEMA`] context plus the start group.
    ///
    /// The load runs wire 25's setup calls in their order: the THINGS in lump
    /// order (each non-player thing's `P_Random` draws, the player's body at its
    /// record: p_setup.c:313-346, p_mobj.c:506, 787-788), `P_SetupPsprites`, then
    /// `P_SpawnSpecials`, all from the start's `P_Random` index. `P_SetupPsprites`
    /// runs inside `P_LoadThings` in the C (p_mobj.c:686) and after it here; it
    /// draws no random number (`A_Raise`, p_pspr.c:429-434) and the load makes no
    /// other sound, so neither the draws nor the records move. A carried chainsaw
    /// records `sfx_sawup` (p_pspr.c:145-146) in the start group and, as every
    /// setup call's record does under wire 25, in `events.sounds.pending`, from
    /// where it is published first with tic 0's records: the genuine stream has it
    /// in the load's `G_Ticker`, before tic 0's sounds.
    ///
    /// `context.initial_rng` must be the default: the start alone gives the index
    /// (0 for a new game, `M_ClearRandom`). A refused start constructs nothing.
    pub fn original_player_episode1_v4_level(
        map: Map,
        map_number: u8,
        skill: Skill,
        context: MapContextOptions,
        start: LevelStart,
    ) -> Result<Self, LevelStartError> {
        if context.initial_rng != DoomRng::default() {
            return Err(LevelStartError::Context);
        }
        let player = match start {
            LevelStart::NewGame => reborn_player(0),
            LevelStart::Reborn { p_random } => reborn_player(p_random),
            LevelStart::Carried(carried) => {
                carried.validate()?;
                carried
            }
        };
        let context = MapContextOptions {
            initial_rng: DoomRng::from_indices(player.p_random, 0),
            ..context
        };
        let mut sim = Self::episode1_v2_with(
            OriginalExecutionProfile::PlayerMonstersEpisode1V4,
            map,
            map_number,
            skill,
            context,
        )
        .map_err(LevelStartError::Registry)?;
        let setup = |e: SimError| LevelStartError::Setup(e);
        if let LevelStart::Carried(carried) = start {
            let p = sim
                .original_player
                .as_mut()
                .ok_or(LevelStartError::Setup(PlayerBodyError::Profile.into()))?;
            p.health = carried.health;
            p.armor = Some(carried.armor);
            p.use_down = carried.use_down;
        }
        sim.spawn_map_things().map_err(setup)?;
        match start {
            LevelStart::Carried(carried) => {
                // `mobj->health = p->health` (p_mobj.c:673).
                let mo = sim
                    .original_player
                    .as_ref()
                    .and_then(|p| p.mo)
                    .ok_or(LevelStartError::Setup(PlayerBodyError::Binding.into()))?;
                sim.actors
                    .get_mut(mo)
                    .and_then(|a| a.life.as_mut())
                    .ok_or(LevelStartError::Setup(PlayerBodyError::Binding.into()))?
                    .health = carried.health;
                let state = PlayerWeaponRuntime::Pack(carried.weapon_state());
                sim.checked_demon_operation(|host| {
                    let result = crate::player_pistol::run(
                        host,
                        weapons::PlayerWeaponOperation::Setup,
                        TicCommand::default(),
                        0,
                        None,
                        None,
                        Some(state),
                    );
                    if let Err(error) = result {
                        host.fail_sight(error);
                    }
                    result
                })
                .and_then(|r| r)
                .map_err(setup)?;
            }
            LevelStart::NewGame | LevelStart::Reborn { .. } => sim
                .initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
                .map_err(setup)?,
        }
        sim.spawn_level_specials().map_err(setup)?;
        sim.level_start = Some(StartRecord {
            kind: start.kind(),
            player,
            sounds: sim.pending_sounds.clone(),
        });
        Ok(sim)
    }
    /// Wire 26: how the level began; `None` under every other policy.
    pub fn level_start_kind(&self) -> Option<StartKind> {
        self.level_start.as_ref().map(|s| s.kind)
    }
    /// Wire 26: the player the load set up (for a new game and a reborn player
    /// `G_PlayerReborn`'s), as DSIM42's start group records it.
    pub fn level_start_player(&self) -> Option<CarriedStart> {
        self.level_start.as_ref().map(|s| s.player)
    }
    /// Wire 26: the sound records the load made, in call order (DSIM42
    /// `level.start.sounds`): `sfx_sawup` at the player's body when the carried
    /// ready weapon is the chainsaw, nothing otherwise. They are also the head of
    /// tic 0's [`Simulation::sound_records`].
    pub fn load_sound_records(&self) -> &[sound::SoundRecord] {
        self.level_start
            .as_ref()
            .map_or(&[][..], |s| s.sounds.as_slice())
    }
    /// The exit a completed tic requested (wires 22 and later: `gameaction ==
    /// ga_completed`, with `secretexit` from wire 23). Read-only.
    pub fn level_exit_request(&self) -> Option<LevelExit> {
        self.level_exit.then_some(if self.level_secret_exit {
            LevelExit::Secret
        } else {
            LevelExit::Normal
        })
    }
    /// Whether a dead player pressed use (`PST_REBORN`). Read-only.
    pub fn reborn_requested(&self) -> bool {
        self.original_player.as_ref().is_some_and(|p| p.reborn)
    }
    /// What the session layer takes from a finished level (wires 25 and 26),
    /// read-only: see [`LevelFinish`]. Callable at any tic; the session calls it
    /// when `advance` refuses with `LevelExited` or `PlayerReborn`.
    pub fn level_finish(&self) -> Result<LevelFinish, PlayerBodyError> {
        let p = self
            .original_player
            .as_ref()
            .ok_or(PlayerBodyError::Profile)?;
        if !p.policy.has_sound_records() {
            return Err(PlayerBodyError::Profile);
        }
        let weapon = p
            .weapon
            .and_then(PlayerWeaponRuntime::pack)
            .ok_or(PlayerBodyError::Invariant)?;
        let stats = self.statistics.ok_or(PlayerBodyError::Invariant)?;
        let p_random = self.rng.indices()[0];
        let dead = p.state == PlayerBodyState::Dead;
        let carried = (!dead).then(|| CarriedStart {
            health: p.health,
            armor: p.armor.unwrap_or_default(),
            weapons_owned: weapon.owned,
            ready_weapon: weapon.ready,
            ammo: weapon.ammo(),
            max_ammo: weapon.max_ammo(),
            backpack: weapon.backpack,
            cheats: 0,
            attack_down: weapon.attack_down,
            use_down: p.use_down,
            bob: weapon.bob,
            weapon_x: weapon.psprites[0].position[0],
            flash_tics: weapon.psprites[1].tics,
            flash_position: weapon.psprites[1].position,
            p_random,
        });
        Ok(LevelFinish {
            exit: self.level_exit_request(),
            reborn: p.reborn,
            player_dead: dead,
            carried,
            p_random,
            totals: [stats.total_kills, stats.total_items, stats.total_secrets],
            counts: [
                stats.single_player_kill_count,
                p.pickups.map_or(0, |k| k.item_count),
                p.secret_count,
            ],
            level_time: self.tic,
            map: self.level_map.ok_or(PlayerBodyError::Invariant)?,
        })
    }
}

/// The DSIM42 start group (STATE_FORMAT "DSIM v42"), appended after the last
/// `events.sounds.pending` field.
pub(crate) fn start_fields(record: &StartRecord) -> Vec<(String, Vec<u8>)> {
    let p = &record.player;
    let i = |v: i32| v.to_le_bytes().to_vec();
    let mut f = vec![
        ("level.start.kind".into(), vec![record.kind.wire_id()]),
        ("level.start.p_random".into(), vec![p.p_random]),
        ("level.start.health".into(), i(p.health)),
        ("level.start.armor_points".into(), i(p.armor.points)),
        (
            "level.start.armor_type".into(),
            vec![p.armor.kind.wire_id()],
        ),
        (
            "level.start.weapons_owned".into(),
            p.weapons_owned.to_le_bytes().to_vec(),
        ),
        (
            "level.start.ready_weapon".into(),
            vec![p.ready_weapon as u8],
        ),
    ];
    for (k, a) in p.ammo.iter().enumerate() {
        f.push((format!("level.start.ammo.{k}"), i(*a)));
    }
    for (k, m) in p.max_ammo.iter().enumerate() {
        f.push((format!("level.start.max_ammo.{k}"), i(*m)));
    }
    f.extend([
        ("level.start.backpack".into(), vec![u8::from(p.backpack)]),
        ("level.start.cheats".into(), i(p.cheats)),
        (
            "level.start.attack_down".into(),
            vec![u8::from(p.attack_down)],
        ),
        ("level.start.use_down".into(), vec![u8::from(p.use_down)]),
        ("level.start.bob".into(), i(p.bob.bits())),
        ("level.start.weapon_x".into(), i(p.weapon_x.bits())),
        ("level.start.flash_tics".into(), i(p.flash_tics)),
        ("level.start.flash_x".into(), i(p.flash_position[0].bits())),
        ("level.start.flash_y".into(), i(p.flash_position[1].bits())),
        (
            "level.start.sounds.count".into(),
            (record.sounds.len() as u64).to_le_bytes().to_vec(),
        ),
    ]);
    for (k, r) in record.sounds.iter().enumerate() {
        f.push((format!("level.start.sounds.{k}"), sound::record_bytes(r)));
    }
    f
}

/// The fixed names of the start group, in their order (without the records).
pub(crate) const START_FIELDS: [&str; 25] = [
    "level.start.kind",
    "level.start.p_random",
    "level.start.health",
    "level.start.armor_points",
    "level.start.armor_type",
    "level.start.weapons_owned",
    "level.start.ready_weapon",
    "level.start.ammo.0",
    "level.start.ammo.1",
    "level.start.ammo.2",
    "level.start.ammo.3",
    "level.start.max_ammo.0",
    "level.start.max_ammo.1",
    "level.start.max_ammo.2",
    "level.start.max_ammo.3",
    "level.start.backpack",
    "level.start.cheats",
    "level.start.attack_down",
    "level.start.use_down",
    "level.start.bob",
    "level.start.weapon_x",
    "level.start.flash_tics",
    "level.start.flash_x",
    "level.start.flash_y",
    "level.start.sounds.count",
];

/// DSIM42's reader rules for the start group (see
/// [`crate::Diagnostic::decode_episode1_v4`]): every fixed field present with its
/// width; a known kind; for a new game exactly `G_PlayerReborn`'s player with index
/// 0, for a reborn player that player with any index, for a carried player the
/// checks of [`CarriedStart::validate`]; the load's records exactly the
/// `sfx_sawup` start at the player's body (unplaced, s_sound.c:304) when a carried
/// ready weapon is the chainsaw and none otherwise; and no other `level.start.`
/// field. `field` reads a field by name; `mo` is `original_player.0.mo`.
pub(crate) fn validate_start_group<'a>(
    field: impl Fn(&str) -> Option<&'a [u8]>,
    mo: Option<&[u8]>,
    group_fields: usize,
) -> Result<(), DiagnosticError> {
    let get = |name: &str, width: usize| -> Result<&'a [u8], DiagnosticError> {
        let v = field(name).ok_or(DiagnosticError::Structure)?;
        if v.len() != width {
            return Err(DiagnosticError::Structure);
        }
        Ok(v)
    };
    let byte = |name: &str| get(name, 1).map(|v| v[0]);
    let int = |name: &str| get(name, 4).map(|v| i32::from_le_bytes([v[0], v[1], v[2], v[3]]));
    let flag = |name: &str| match byte(name)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(DiagnosticError::Schema),
    };
    let kind = match byte("level.start.kind")? {
        0 => StartKind::NewGame,
        1 => StartKind::Carried,
        2 => StartKind::Reborn,
        _ => return Err(DiagnosticError::Schema),
    };
    let owned = get("level.start.weapons_owned", 2)?;
    let mut ammo = [0; 4];
    let mut max_ammo = [0; 4];
    for k in 0..4 {
        ammo[k] = int(&format!("level.start.ammo.{k}"))?;
        max_ammo[k] = int(&format!("level.start.max_ammo.{k}"))?;
    }
    let player = CarriedStart {
        health: int("level.start.health")?,
        armor: PlayerArmorState {
            kind: ArmorType::try_from(byte("level.start.armor_type")?)
                .map_err(|_| DiagnosticError::Schema)?,
            points: int("level.start.armor_points")?,
        },
        weapons_owned: u16::from_le_bytes([owned[0], owned[1]]),
        ready_weapon: Weapon::from_number_for(byte("level.start.ready_weapon")?, true)
            .map_err(|_| DiagnosticError::Schema)?,
        ammo,
        max_ammo,
        backpack: flag("level.start.backpack")?,
        cheats: int("level.start.cheats")?,
        attack_down: flag("level.start.attack_down")?,
        use_down: flag("level.start.use_down")?,
        bob: Fixed::from_bits(int("level.start.bob")?),
        weapon_x: Fixed::from_bits(int("level.start.weapon_x")?),
        flash_tics: int("level.start.flash_tics")?,
        flash_position: [
            Fixed::from_bits(int("level.start.flash_x")?),
            Fixed::from_bits(int("level.start.flash_y")?),
        ],
        p_random: byte("level.start.p_random")?,
    };
    match kind {
        StartKind::NewGame if player != reborn_player(0) => return Err(DiagnosticError::Schema),
        StartKind::Reborn if player != reborn_player(player.p_random) => {
            return Err(DiagnosticError::Schema);
        }
        StartKind::Carried if player.validate().is_err() => return Err(DiagnosticError::Schema),
        _ => {}
    }
    let count = get("level.start.sounds.count", 8)?;
    let count = u64::from_le_bytes(count.try_into().map_err(|_| DiagnosticError::Structure)?);
    let saw = kind == StartKind::Carried && player.ready_weapon == Weapon::Chainsaw;
    if count != u64::from(saw) || group_fields != START_FIELDS.len() + usize::from(saw) {
        return Err(DiagnosticError::Structure);
    }
    if saw {
        let mo = mo
            .filter(|m| m.len() == 13 && m[0] == 1)
            .ok_or(DiagnosticError::Identity)?;
        let mut expected = vec![1];
        expected.extend(lifecycle::source_sound_id("sfx_sawup").to_le_bytes());
        expected.push(1);
        expected.extend(&mo[1..]);
        expected.push(0);
        if field("level.start.sounds.0") != Some(expected.as_slice()) {
            return Err(DiagnosticError::Identity);
        }
    }
    Ok(())
}
