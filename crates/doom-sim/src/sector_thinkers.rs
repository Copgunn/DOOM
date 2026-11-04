//! Sector thinkers (`P_SpawnSpecials` lights now, movers later) share the actor
//! processing order but live in their own slot class (M3h7 owner decision 4).
//!
//! Under `PlayerMonstersSpecialsV1` this module holds the static sector specials of
//! the M3h7 note §2.2–§2.4: the four `p_lights.c` thinkers and their spawners,
//! `P_FindMinSurroundingLight`/`getNextSector`, the `P_SpawnSpecials` sector scan
//! (run once by [`Simulation::spawn_level_specials`]) and `P_PlayerInSpecialSector`.
use crate::{
    ActorId, CombatError, DamageRequest, Event, MapScaleError, MapSpawnError, PlayerBodyError,
    PlayerBodyState, PursuitError, SimError, Simulation, StoreError, ThinkerContext, Visit,
    lifecycle::LifeHost,
};
use doom_types::DoomRng;

/// `p_spec.h` `GLOWSPEED`, `STROBEBRIGHT`, `FASTDARK`, `SLOWDARK`.
const GLOWSPEED: i32 = 8;
const STROBEBRIGHT: i32 = 5;
const FASTDARK: i32 = 15;
const SLOWDARK: i32 = 35;
/// `p_spec.c` `MAXLINEANIMS`: the `linespeciallist` size `P_SpawnSpecials` fills
/// with type-48 lines without a bound check.
pub(crate) const MAX_LINE_ANIMS: usize = 64;
/// Sector specials admitted at load by the specials policy (M3h7 decision 3):
/// lights 1, 2, 3, 8, 12, 13, 17, strobe-hurt 4, damage 5, 7, 16 and secret 9.
pub(crate) const ADMITTED_SECTOR_SPECIALS: [i16; 12] = [1, 2, 3, 4, 5, 7, 8, 9, 12, 13, 16, 17];
/// The one admitted line special: the scrolling wall (presentation only).
pub(crate) const SCROLLING_WALL: i16 = 48;
/// `p_lights.c` light arithmetic stays inside `short lightlevel` when every map
/// light lies in this range: a glow steps by 8 beyond its bounds and a flicker
/// reaches `minlight + 16` and `maxlight - 48` (checked at load).
pub(crate) const LIGHT_RANGE: std::ops::RangeInclusive<i16> = (i16::MIN + 64)..=(i16::MAX - 64);

/// `lightflash_t` (p_spec.h): sector special 1, `T_LightFlash`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LightFlash {
    /// Sector index.
    pub sector: u16,
    /// Tics before the next toggle.
    pub count: i32,
    /// Darkest neighbor light at spawn.
    pub minlight: i32,
    /// The sector's own light at spawn.
    pub maxlight: i32,
    /// Bright-phase mask, 64 (`P_Random() & 64`).
    pub maxtime: i32,
    /// Dark-phase mask, 7.
    pub mintime: i32,
}
/// `strobe_t`: specials 2, 3, 4 (unsynchronized) and 12, 13 (synchronized).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StrobeFlash {
    /// Sector index.
    pub sector: u16,
    /// Tics before the next toggle.
    pub count: i32,
    /// Darkest neighbor light, or 0 when it equals `maxlight`.
    pub minlight: i32,
    /// The sector's own light at spawn.
    pub maxlight: i32,
    /// `FASTDARK` 15 or `SLOWDARK` 35.
    pub darktime: i32,
    /// `STROBEBRIGHT` 5.
    pub brighttime: i32,
}
/// `glow_t`: special 8, `T_Glow`; it has no count.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Glow {
    /// Sector index.
    pub sector: u16,
    /// Darkest neighbor light at spawn.
    pub minlight: i32,
    /// The sector's own light at spawn.
    pub maxlight: i32,
    /// −1 down, 1 up.
    pub direction: i32,
}
/// `fireflicker_t`: special 17, `T_FireFlicker`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FireFlicker {
    /// Sector index.
    pub sector: u16,
    /// Tics before the next flicker.
    pub count: i32,
    /// Darkest neighbor light plus 16.
    pub minlight: i32,
    /// The sector's own light at spawn.
    pub maxlight: i32,
}
/// Payload of the sector-thinker slot class: the four `p_lights.c` light thinkers
/// and the three movers. Every policy before the specials policy builds its store
/// with sector capacity zero and creates none; `PlayerMonstersSpecialsV1` creates
/// the lights once, at load; `PlayerMonstersMoversV1` also creates movers during
/// play (M3h8 note §2.4–§2.6).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SectorThinker {
    /// `T_LightFlash`.
    Flash(LightFlash),
    /// `T_StrobeFlash`.
    Strobe(StrobeFlash),
    /// `T_Glow`.
    Glow(Glow),
    /// `T_FireFlicker`.
    Flicker(FireFlicker),
    /// `T_VerticalDoor`.
    Door(crate::Door),
    /// `T_PlatRaise`.
    Plat(crate::Plat),
    /// `T_MoveFloor`.
    Floor(crate::FloorMove),
}
impl SectorThinker {
    /// The sector this thinker changes.
    pub const fn sector(&self) -> u16 {
        match self {
            Self::Flash(t) => t.sector,
            Self::Strobe(t) => t.sector,
            Self::Glow(t) => t.sector,
            Self::Flicker(t) => t.sector,
            Self::Door(t) => t.sector,
            Self::Plat(t) => t.sector,
            Self::Floor(t) => t.sector,
        }
    }
    /// The sector of a mover (the thinkers that set `specialdata`); `None` for a
    /// light.
    pub const fn mover_sector(&self) -> Option<u16> {
        match self {
            Self::Door(_) | Self::Plat(_) | Self::Floor(_) => Some(self.sector()),
            _ => None,
        }
    }
    /// The DSIM kind: 1 flash, 2 strobe, 3 glow, 4 flicker, 5 door, 6 plat, 7 floor.
    pub const fn kind_code(&self) -> u8 {
        match self {
            Self::Flash(_) => 1,
            Self::Strobe(_) => 2,
            Self::Glow(_) => 3,
            Self::Flicker(_) => 4,
            Self::Door(_) => 5,
            Self::Plat(_) => 6,
            Self::Floor(_) => 7,
        }
    }
}

/// Runtime light level and special of one sector (`sector_t::lightlevel`,
/// `sector_t::special`); the map lumps keep the original values.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SectorSpecial {
    pub light: i16,
    pub special: i16,
}
/// The specials policy's mutable sector state plus its setup phase. Only
/// `sectors` changes during a tic; it is staged with the checked plan.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct LevelSpecials {
    pub sectors: Vec<SectorSpecial>,
    /// Whether [`Simulation::spawn_level_specials`] has run (exactly once).
    pub spawned: bool,
}

fn store_light(value: i32) -> Result<i16, SimError> {
    i16::try_from(value).map_err(|_| MapScaleError::Light.into())
}

/// One light thinker's turn in `P_RunThinkers`, with its `P_Random` draws at that
/// turn. Light changes and draws emit no event (M3h7 decision 5).
pub(crate) fn think(
    thinker: &mut SectorThinker,
    sectors: &mut [SectorSpecial],
    rng: &mut DoomRng,
) -> Result<(), SimError> {
    let sector = usize::from(thinker.sector());
    let s = sectors
        .get_mut(sector)
        .ok_or(StoreError::UnsupportedThinker)?;
    let light = i32::from(s.light);
    match thinker {
        // p_lights.c T_LightFlash: the toggle compares the current light with
        // maxlight; `P_Random() & 64` masks with the value 64 (1 or 65 tics).
        SectorThinker::Flash(f) => {
            f.count -= 1;
            if f.count != 0 {
                return Ok(());
            }
            if light == f.maxlight {
                s.light = store_light(f.minlight)?;
                f.count = (i32::from(rng.p_random()) & f.mintime) + 1;
            } else {
                s.light = store_light(f.maxlight)?;
                f.count = (i32::from(rng.p_random()) & f.maxtime) + 1;
            }
        }
        // T_StrobeFlash: no draw while running.
        SectorThinker::Strobe(f) => {
            f.count -= 1;
            if f.count != 0 {
                return Ok(());
            }
            if light == f.minlight {
                s.light = store_light(f.maxlight)?;
                f.count = f.brighttime;
            } else {
                s.light = store_light(f.minlight)?;
                f.count = f.darktime;
            }
        }
        // T_Glow: ±GLOWSPEED, bouncing at the bounds; no draw.
        SectorThinker::Glow(g) => match g.direction {
            -1 => {
                let mut next = light - GLOWSPEED;
                if next <= g.minlight {
                    next += GLOWSPEED;
                    g.direction = 1;
                }
                s.light = store_light(next)?;
            }
            1 => {
                let mut next = light + GLOWSPEED;
                if next >= g.maxlight {
                    next -= GLOWSPEED;
                    g.direction = -1;
                }
                s.light = store_light(next)?;
            }
            // The switch has no default: any other direction does nothing; only
            // −1 and 1 are ever stored.
            _ => {}
        },
        // T_FireFlicker: one `(P_Random() & 3) * 16` draw every four tics.
        SectorThinker::Flicker(f) => {
            f.count -= 1;
            if f.count != 0 {
                return Ok(());
            }
            let amount = (i32::from(rng.p_random()) & 3) * 16;
            s.light = store_light(if light - amount < f.minlight {
                f.minlight
            } else {
                f.maxlight - amount
            })?;
            f.count = 4;
        }
        // Movers run through `movers::think`, never as a light.
        SectorThinker::Door(_) | SectorThinker::Plat(_) | SectorThinker::Floor(_) => {
            return Err(StoreError::UnsupportedThinker.into());
        }
    }
    Ok(())
}

/// The thinker pass's sector-thinker hook. Without the specials policy's runtime
/// sectors a sector visit refuses explicitly ([`StoreError::UnsupportedThinker`])
/// instead of skipping; a retirement visit is an error too, because no admitted
/// policy removes a sector thinker.
pub(crate) fn visit<T>(
    context: &mut ThinkerContext<'_, T, SectorThinker>,
    visit: Visit,
    sectors: Option<&mut [SectorSpecial]>,
    rng: &mut DoomRng,
) -> Result<(), SimError> {
    match (visit, sectors) {
        (Visit::SectorThink(id), Some(sectors)) => {
            let thinker = context
                .sector_get_mut(id)
                .ok_or(StoreError::UnsupportedThinker)?;
            think(thinker, sectors, rng)
        }
        _ => Err(StoreError::UnsupportedThinker.into()),
    }
}

/// p_spec.c `P_FindMinSurroundingLight` over `getNextSector` (which requires
/// `ML_TWOSIDED`), reading the `P_GroupLines` line list of `sector`.
fn min_surrounding_light(
    sim: &Simulation,
    lights: &[SectorSpecial],
    sector: usize,
    max: i32,
) -> Result<i32, SimError> {
    let map = sim.world.map();
    let lists = sim
        .world
        .sector_lines
        .as_ref()
        .ok_or(MapSpawnError::Context)?;
    let mut min = max;
    for &line in lists.get(sector).ok_or(MapSpawnError::Context)? {
        let line = &map.linedefs()[usize::from(line)];
        // getNextSector's ML_TWOSIDED skip (p_spec.c:255–256), kept for source
        // fidelity but unreachable under admission: a line with a back side but no
        // ML_TWOSIDED is refused at load (`validate_diagonal_geometry`, which the
        // specials policy uses), so every line skipped here also has no back side
        // and the side lookup below would find no neighbor either.
        if line.flags & crate::player_map_scale::ML_TWOSIDED == 0 {
            continue;
        }
        let side =
            |i: usize| line.sides[i].map(|s| usize::from(map.sidedefs()[usize::from(s)].sector));
        let next = if side(0) == Some(sector) {
            side(1)
        } else {
            side(0)
        };
        if let Some(next) = next {
            min = min.min(i32::from(lights[next].light));
        }
    }
    Ok(min)
}

impl Simulation {
    /// `P_SpawnSpecials`' sector scan (p_spec.c:1270–1332), run once by the
    /// specials policy's explicit load operation (M3h7 decision 12) after every
    /// map THING and the player body, as `P_SetupLevel` does: sectors in index
    /// order; specials 1, 2, 3, 4, 8, 12, 13 and 17 spawn their light thinker
    /// (appended to the shared thinker order after every existing actor, with the
    /// flash and unsynchronized-strobe spawn draws in sector order) and clear the
    /// sector special, except special 4, which is set back to 4; special 9 adds one
    /// to `level.total_secrets`. Specials 5, 7 and 16 spawn nothing. Scrolling
    /// walls (line special 48) are presentation: their offset is `tic` units and
    /// is never stored. The spawn emits no event.
    ///
    /// Refused with the state unchanged ([`MapSpawnError::LevelSpecials`]) under
    /// any other policy, after a tic, a second time, or before the player body
    /// exists; [`Simulation::advance`] is refused until it has run, and
    /// [`Simulation::spawn_map_things`] and [`Simulation::spawn_player_body`] are
    /// refused after it, so no light thinker can precede a map actor or the body.
    pub fn spawn_level_specials(&mut self) -> Result<(), SimError> {
        let specials = self
            .specials
            .as_ref()
            .filter(|s| !s.spawned && self.tic == 0)
            .ok_or(MapSpawnError::LevelSpecials)?;
        let body = self
            .original_player
            .as_ref()
            .and_then(|p| p.mo)
            .filter(|id| self.actors.is_pending(*id) == Some(false));
        if body.is_none() {
            return Err(MapSpawnError::LevelSpecials.into());
        }
        let mut sectors = specials.sectors.clone();
        let mut statistics = self.statistics.ok_or(MapSpawnError::Context)?;
        let mut rng = self.rng;
        let mut store = self.actors.clone();
        for i in 0..sectors.len() {
            let sector = u16::try_from(i).map_err(|_| MapSpawnError::WorkLimit)?;
            let light = i32::from(sectors[i].light);
            let neighbor =
                |sectors: &[SectorSpecial]| min_surrounding_light(self, sectors, i, light);
            let thinker = match sectors[i].special {
                1 => {
                    sectors[i].special = 0;
                    let minlight = neighbor(&sectors)?;
                    let count = (i32::from(rng.p_random()) & 64) + 1;
                    Some(SectorThinker::Flash(LightFlash {
                        sector,
                        count,
                        minlight,
                        maxlight: light,
                        maxtime: 64,
                        mintime: 7,
                    }))
                }
                special @ (2 | 3 | 4 | 12 | 13) => {
                    let darktime = if matches!(special, 3 | 12) {
                        SLOWDARK
                    } else {
                        FASTDARK
                    };
                    let mut minlight = neighbor(&sectors)?;
                    if minlight == light {
                        minlight = 0;
                    }
                    // p_lights.c P_SpawnStrobeFlash clears the special; type 4 is
                    // set back to 4 by P_SpawnSpecials (strobe hurt).
                    sectors[i].special = if special == 4 { 4 } else { 0 };
                    let count = if matches!(special, 12 | 13) {
                        1
                    } else {
                        (i32::from(rng.p_random()) & 7) + 1
                    };
                    Some(SectorThinker::Strobe(StrobeFlash {
                        sector,
                        count,
                        minlight,
                        maxlight: light,
                        darktime,
                        brighttime: STROBEBRIGHT,
                    }))
                }
                8 => {
                    let minlight = neighbor(&sectors)?;
                    sectors[i].special = 0;
                    Some(SectorThinker::Glow(Glow {
                        sector,
                        minlight,
                        maxlight: light,
                        direction: -1,
                    }))
                }
                9 => {
                    statistics.total_secrets = statistics
                        .total_secrets
                        .checked_add(1)
                        .ok_or(MapSpawnError::StatisticsOverflow)?;
                    None
                }
                17 => {
                    sectors[i].special = 0;
                    let minlight = neighbor(&sectors)? + 16;
                    Some(SectorThinker::Flicker(FireFlicker {
                        sector,
                        count: 4,
                        minlight,
                        maxlight: light,
                    }))
                }
                0 | 5 | 7 | 16 => None,
                // Rejected at load (10, 11, 14 and unknown values).
                _ => return Err(PursuitError::Specials.into()),
            };
            if let Some(thinker) = thinker {
                store.spawn_sector(thinker)?;
            }
        }
        self.actors = store;
        self.rng = rng;
        self.statistics = Some(statistics);
        let specials = self.specials.as_mut().ok_or(MapSpawnError::Context)?;
        specials.sectors = sectors;
        specials.spawned = true;
        Ok(())
    }
    /// Runtime light levels and specials of every sector under the specials
    /// policy, in sector order (`None` under every other policy).
    pub fn sector_lights_and_specials(&self) -> Option<impl Iterator<Item = (i16, i16)> + '_> {
        self.specials
            .as_ref()
            .map(|s| s.sectors.iter().map(|s| (s.light, s.special)))
    }
    /// Whether the specials policy's load operation has run.
    pub fn level_specials_spawned(&self) -> bool {
        self.specials.as_ref().is_some_and(|s| s.spawned)
    }
    /// The player's `secretcount` under the specials policy.
    pub fn original_player_secret_count(&self) -> Result<i32, SimError> {
        match (&self.specials, &self.original_player) {
            (Some(_), Some(p)) => Ok(p.secret_count),
            _ => Err(PlayerBodyError::Profile.into()),
        }
    }
    /// The shared thinker order of actors and sector thinkers (`P_RunThinkers`).
    pub fn shared_thinker_order(&self) -> impl Iterator<Item = crate::ThinkerId> + '_ {
        self.actors.shared_order()
    }
    /// Read-only sector-thinker lookup.
    pub fn sector_thinker(&self, id: crate::SectorThinkerId) -> Option<&SectorThinker> {
        self.actors.sector_get(id)
    }
}

/// `pw_ironfeet`: no admitted roster has the radiation suit, so no player ever
/// carries the power and the `P_Random() < 5` of specials 4 and 16 is never drawn
/// (`!ironfeet || …` short-circuits, p_spec.c:1041).
const fn ironfeet(_player: &crate::original_player::PlayerRuntime) -> bool {
    false
}

/// p_spec.c `P_PlayerInSpecialSector`, at its `P_PlayerThink` position (after
/// `P_CalcHeight`, before the weapon change), for a live player whose center
/// subsector's sector has a nonzero runtime special. `leveltime` is the source
/// `leveltime` of this tic (the completed-tic count before it, `Simulation::tic`).
pub(crate) fn player_in_special_sector<A: crate::lifecycle::ActorAccess>(
    host: &mut LifeHost<'_, A>,
    sectors: &mut [SectorSpecial],
    leveltime: u64,
) -> Result<(), SimError> {
    let player = host.player.as_deref().ok_or(PlayerBodyError::Profile)?;
    if player.state != PlayerBodyState::Live {
        return Ok(());
    }
    let id: ActorId = player.mo.ok_or(PlayerBodyError::Binding)?;
    let actor = *host.access.get(id).ok_or(PlayerBodyError::Binding)?;
    let sector = actor.location.ok_or(PlayerBodyError::Invariant)?.sector;
    let index = usize::from(sector);
    let special = sectors
        .get(index)
        .ok_or(PlayerBodyError::Invariant)?
        .special;
    if special == 0 {
        return Ok(());
    }
    // "Falling, not all the way down yet?": the sector's floor, not mo->floorz.
    if actor.z != host.sectors[index].floor {
        return Ok(());
    }
    let gate = leveltime & 0x1f == 0;
    let damage = match special {
        5 => (!ironfeet(player) && gate).then_some(10),
        7 => (!ironfeet(player) && gate).then_some(5),
        4 | 16 => {
            // Without the suit the draw is short-circuited away; with it the
            // source would draw `P_Random() < 5`, which no admitted state reaches.
            if ironfeet(player) {
                return Err(PlayerBodyError::Unsupported.into());
            }
            gate.then_some(20)
        }
        9 => {
            let p = host.player.as_deref_mut().ok_or(PlayerBodyError::Profile)?;
            p.secret_count = p
                .secret_count
                .checked_add(1)
                .ok_or(MapSpawnError::StatisticsOverflow)?;
            let count = p.secret_count;
            sectors[index].special = 0;
            host.events
                .push(Event::Special(crate::SpecialEvent::Secret {
                    sector,
                    count,
                }));
            None
        }
        // 11 and every unknown value are rejected at load; the source I_Errors.
        _ => return Err(PursuitError::Specials.into()),
    };
    if let Some(amount) = damage {
        // P_DamageMobj(player->mo, NULL, NULL, amount): no inflictor, no source.
        let plan = crate::combat::DamagePlan::current_original(
            host,
            DamageRequest {
                target: id,
                inflictor: None,
                source: None,
                amount,
            },
        )?;
        host.access
            .reserve(plan.spawn_count())
            .map_err(|_| CombatError::WorkLimit)?;
        host.apply_damage(plan);
    }
    Ok(())
}

/// Specials-policy observations (DSIM34 event tag 25). Light changes, light draws
/// and sector-thinker runs have no event (M3h7 decision 5).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpecialEvent {
    /// `P_PlayerInSpecialSector` special 9: `secretcount` after the increment;
    /// the sector special is cleared.
    Secret {
        /// Sector index.
        sector: u16,
        /// `player->secretcount` after the increment.
        count: i32,
    },
    /// `P_XYMovement`: a refused fireball move whose `ceilingline` has a sky back
    /// ceiling removes the fireball (no explosion, no draw).
    SkyRemoval {
        /// The removed fireball.
        actor: ActorId,
        /// The refused check's `ceilingline`.
        line: u16,
    },
    /// `PTR_ShootTraverse`: a hitscan ended at a line whose front sector has a sky
    /// ceiling, without a puff.
    SkyShot {
        /// 0: `z` above the front ceiling; 1: the back sector is sky too.
        reason: u8,
        /// The hit line.
        line: u16,
        /// The would-be puff height (16.16).
        z: doom_types::Fixed,
    },
}
