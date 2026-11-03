//! Doors, lifts and floors of the movers policy (M3h8 note §2.4–§2.6): the
//! `SectorThinker::{Door, Plat, Floor}` records, their `EV_*` starters and their
//! `T_*` thinkers.
//!
//! A mover joins the shared thinker order at its tail (`P_AddThinker`) in the lowest
//! free sector-thinker slot. When it finishes it is marked (`P_RemoveThinker`): its
//! slot and its place in the order stay until the next pass reaches it, while the
//! sector's `specialdata` (derived here: the one live mover naming the sector) is
//! cleared at once, so a later thinker of the same pass may start a replacement. The
//! unlink frees the slot and grows its generation. Mover fields the source leaves
//! uninitialised and never reads for the admitted types start at 0 (decision N6).
//! Sounds are recorded as sector-origin sound requests; none draws `P_Random`.
use crate::{
    Actor, Event, SectorState, SectorThinker, SectorThinkerId, SimError, StaticWorld, StoreError,
    ThinkerContext, ThinkerStore,
    mover_rules::{self, LINE_DOOR_DR},
    planes::{MoveResult, Plane, PlaneHost},
    staging::Sectors,
};
use doom_types::Fixed;

/// p_spec.h `VDOORSPEED` (2 units) and `VDOORWAIT` (150 tics).
const VDOORSPEED: i32 = 2;
const VDOORWAIT: i32 = 150;
/// p_spec.h `PLATWAIT` (3 seconds) and `PLATSPEED` (1 unit): the `downWaitUpStay`
/// lift waits `35 * PLATWAIT` tics and moves at `PLATSPEED * 4`.
const PLAT_WAIT: i32 = 35 * 3;
const PLAT_SPEED: i32 = 4;
/// p_spec.h `FLOORSPEED` (1 unit); `turboLower` moves at four times it.
const FLOORSPEED: i32 = 1;

/// `vldoor_e` of the admitted doors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DoorKind {
    /// `normal` (0): DR (1) and SR (63) doors; speed 2, waits 150 tics at the top.
    Normal = 0,
    /// `blazeOpen` (6): the W1 109 door; speed 8, removed at the top.
    BlazeOpen = 6,
}
/// `vldoor_t` (p_spec.h:343); `speed` and `topwait` are implied by the kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Door {
    /// Sector index.
    pub sector: u16,
    /// `type`.
    pub kind: DoorKind,
    /// −1 down, 0 waiting, 1 up.
    pub direction: i32,
    /// `topheight` (16.16).
    pub topheight: Fixed,
    /// `topcountdown`: 0 until the door first waits (decision N6).
    pub topcountdown: i32,
}
impl Door {
    fn speed(self) -> Fixed {
        Fixed::from_map_units(match self.kind {
            DoorKind::Normal => VDOORSPEED as i16,
            DoorKind::BlazeOpen => (VDOORSPEED * 4) as i16,
        })
    }
}
/// `plat_e` of the admitted lift.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatStatus {
    /// `up` (0).
    Up = 0,
    /// `down` (1).
    Down = 1,
    /// `waiting` (2).
    Waiting = 2,
}
/// `plat_t` (p_spec.h:285) of a `downWaitUpStay` lift; `type`, `speed` (4), `wait`
/// (105), `crush` (false), `tag` and `oldstatus` (never written) are implied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Plat {
    /// Sector index.
    pub sector: u16,
    /// `status`.
    pub status: PlatStatus,
    /// `count`: tics left to wait; 0 until the lift first waits.
    pub count: i32,
    /// `low` (16.16).
    pub low: Fixed,
    /// `high` (16.16): the floor at activation.
    pub high: Fixed,
}
/// `floor_e` of the admitted floors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FloorKind {
    /// `lowerFloorToLowest` (1): W1 38, speed 1.
    LowerFloorToLowest = 1,
    /// `turboLower` (2): W1 36, speed 4.
    TurboLower = 2,
}
/// `floormove_t` (p_spec.h:584); `direction` (−1), `speed`, `crush` (false),
/// `newspecial` and `texture` (never read for these types) are implied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FloorMove {
    /// Sector index.
    pub sector: u16,
    /// `type`.
    pub kind: FloorKind,
    /// `floordestheight` (16.16).
    pub floordestheight: Fixed,
}
impl FloorMove {
    fn speed(self) -> Fixed {
        Fixed::from_map_units(match self.kind {
            FloorKind::LowerFloorToLowest => FLOORSPEED as i16,
            FloorKind::TurboLower => (FLOORSPEED * 4) as i16,
        })
    }
}

/// `S_StartSound` names of the admitted sector sounds.
fn sound(name: &str) -> u16 {
    crate::lifecycle::source_sound_id(name)
}

/// The sector-thinker store as the `EV_*` starters see it: the committed store
/// between tics and in the players phase, the traversal context in the pass.
pub(crate) trait MoverAccess {
    fn mover_store(&self) -> &ThinkerStore<Actor>;
    fn spawn_mover(&mut self, mover: SectorThinker) -> Result<SectorThinkerId, StoreError>;
    fn mover_mut(&mut self, id: SectorThinkerId) -> Option<&mut SectorThinker>;
}
impl MoverAccess for ThinkerStore<Actor> {
    fn mover_store(&self) -> &ThinkerStore<Actor> {
        self
    }
    fn spawn_mover(&mut self, mover: SectorThinker) -> Result<SectorThinkerId, StoreError> {
        self.spawn_sector(mover)
    }
    fn mover_mut(&mut self, id: SectorThinkerId) -> Option<&mut SectorThinker> {
        self.sector_get_mut(id)
    }
}
impl MoverAccess for ThinkerContext<'_, Actor> {
    fn mover_store(&self) -> &ThinkerStore<Actor> {
        self.store()
    }
    fn spawn_mover(&mut self, mover: SectorThinker) -> Result<SectorThinkerId, StoreError> {
        self.spawn_sector(mover)
    }
    fn mover_mut(&mut self, id: SectorThinkerId) -> Option<&mut SectorThinker> {
        self.sector_get_mut(id)
    }
}

/// `sector->specialdata`: the live (not removed) door, plat or floor naming
/// `sector`; lights never set it. At most one exists (the starters skip a sector
/// that has one).
pub(crate) fn special_data(store: &ThinkerStore<Actor>, sector: u16) -> Option<SectorThinkerId> {
    store.sector_slots.iter().enumerate().find_map(|(slot, s)| {
        let entry = s.entry.as_ref()?;
        (!entry.pending && entry.value.mover_sector() == Some(sector))
            .then(|| SectorThinkerId::from_parts(slot as u32, s.generation))
    })
}
/// The live `downWaitUpStay` plats, the occupied `activeplats` slots.
pub(crate) fn active_plats(store: &ThinkerStore<Actor>) -> usize {
    store
        .sector_slots
        .iter()
        .filter_map(|s| s.entry.as_ref())
        .filter(|e| !e.pending && matches!(e.value, SectorThinker::Plat(_)))
        .count()
}

/// The `p_spec.c` neighbour searches over `getNextSector`, reading `sectors`.
struct Surrounding<'a> {
    world: &'a StaticWorld,
    sectors: Sectors<'a>,
}
impl Surrounding<'_> {
    fn neighbors(&self, sector: u16) -> Result<impl Iterator<Item = SectorState> + '_, SimError> {
        let lines = self
            .world
            .sector_lines
            .as_ref()
            .and_then(|l| l.get(usize::from(sector)))
            .ok_or(crate::MapSpawnError::Context)?;
        let map = self.world.map();
        Ok(lines.iter().filter_map(move |&line| {
            mover_rules::next_sector(map, usize::from(line), usize::from(sector))
                .and_then(|n| self.sectors.get(n).copied())
        }))
    }
    fn own(&self, sector: u16) -> Result<SectorState, SimError> {
        self.sectors
            .get(usize::from(sector))
            .copied()
            .ok_or_else(|| crate::GeometryError::InvalidReference.into())
    }
    /// `P_FindLowestCeilingSurrounding` (starts at `MAXINT`; rule 3 guarantees a
    /// neighbour).
    fn lowest_ceiling(&self, sector: u16) -> Result<Fixed, SimError> {
        self.neighbors(sector)?
            .map(|s| s.ceiling)
            .min()
            .ok_or_else(|| crate::MapScaleError::MoverNeighbor.into())
    }
    /// `P_FindLowestFloorSurrounding` (starts at the sector's own floor).
    fn lowest_floor(&self, sector: u16) -> Result<Fixed, SimError> {
        let own = self.own(sector)?.floor;
        Ok(self
            .neighbors(sector)?
            .map(|s| s.floor)
            .fold(own, Fixed::min))
    }
    /// `P_FindHighestFloorSurrounding` (starts at −500; rule 3 guarantees a
    /// neighbour).
    fn highest_floor(&self, sector: u16) -> Result<Fixed, SimError> {
        self.neighbors(sector)?
            .map(|s| s.floor)
            .max()
            .ok_or_else(|| crate::MapScaleError::MoverNeighbor.into())
    }
}

/// What an `EV_*` starter needs: the map, the current sector heights, the store and
/// the event stream.
pub(crate) struct Starter<'a, M: ?Sized> {
    pub world: &'a StaticWorld,
    pub sectors: Sectors<'a>,
    pub store: &'a mut M,
    pub events: &'a mut Vec<Event>,
}
impl<M: MoverAccess + ?Sized> Starter<'_, M> {
    fn surrounding(&self) -> Surrounding<'_> {
        Surrounding {
            world: self.world,
            sectors: self.sectors,
        }
    }
    fn sector_sound(&mut self, sector: u16, name: &str) {
        self.events
            .push(Event::Mover(crate::MoverEvent::SectorSound {
                origin: crate::planes::SoundOrigin::Sector,
                index: sector,
                sound: sound(name),
            }));
    }
    fn tagged(&self, line: u16) -> Result<impl Iterator<Item = u16> + '_, SimError> {
        let map = self.world.map();
        let tag = map
            .linedefs()
            .get(usize::from(line))
            .ok_or(crate::GeometryError::InvalidReference)?
            .tag;
        Ok(mover_rules::tagged(map, tag).map(|i| i as u16))
    }
    /// Appends `mover` to the shared order (`P_AddThinker`).
    fn spawn(&mut self, mover: SectorThinker) -> Result<SectorThinkerId, SimError> {
        Ok(self.store.spawn_mover(mover)?)
    }
    fn finish(
        &mut self,
        function: crate::planes::MoverFunction,
        line: u16,
        argument: i32,
        result: i32,
        created: &[(SectorThinkerId, SectorThinker)],
    ) {
        for (id, mover) in created {
            self.events.push(Event::Mover(crate::MoverEvent::Started {
                mover: *id,
                kind: mover.kind_code(),
                sector: mover.mover_sector().unwrap_or(u16::MAX),
            }));
        }
        self.events.push(Event::Mover(crate::MoverEvent::Call {
            function,
            line,
            argument,
            result,
            created: created.len() as u8,
        }));
    }

    /// p_doors.c `EV_DoDoor` for the admitted `normal` (63) and `blazeOpen` (109)
    /// doors: every tagged sector without a live mover gets a door rising at speed 2
    /// (8 for `blazeOpen`) to `P_FindLowestCeilingSurrounding − 4`, with `sfx_doropn`
    /// (`sfx_bdopn`) only when that top differs from the ceiling. Returns whether a
    /// door was created.
    pub(crate) fn do_door(&mut self, line: u16, kind: DoorKind) -> Result<bool, SimError> {
        let targets: Vec<u16> = self.tagged(line)?.collect();
        let mut created = Vec::new();
        for sector in targets {
            if special_data(self.store.mover_store(), sector).is_some() {
                continue;
            }
            let ceiling = self.surrounding().own(sector)?.ceiling;
            let topheight = self.surrounding().lowest_ceiling(sector)? - Fixed::from_map_units(4);
            let door = SectorThinker::Door(Door {
                sector,
                kind,
                direction: 1,
                topheight,
                topcountdown: 0,
            });
            let id = self.spawn(door)?;
            if topheight != ceiling {
                self.sector_sound(
                    sector,
                    match kind {
                        DoorKind::Normal => "sfx_doropn",
                        DoorKind::BlazeOpen => "sfx_bdopn",
                    },
                );
            }
            created.push((id, door));
        }
        let result = i32::from(!created.is_empty());
        self.finish(
            crate::planes::MoverFunction::DoDoor,
            line,
            kind as i32,
            result,
            &created,
        );
        Ok(result != 0)
    }

    /// p_doors.c `EV_VerticalDoor` for the DR line (special 1): the back sector's
    /// live door is reversed (a closing door goes back up for anyone; otherwise a
    /// player sends it down and a monster does nothing); without one, `sfx_doropn`
    /// plays unconditionally and a `normal` door rises to
    /// `P_FindLowestCeilingSurrounding − 4`. `player` is whether the user is a
    /// player (`thing->player`).
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "called by the use bodies (M3h8 plan steps 10-11)")
    )]
    pub(crate) fn vertical_door(&mut self, line: u16, player: bool) -> Result<(), SimError> {
        let map = self.world.map();
        let l = map
            .linedefs()
            .get(usize::from(line))
            .ok_or(crate::GeometryError::InvalidReference)?;
        if l.special != LINE_DOOR_DR {
            return Err(crate::PursuitError::Specials.into());
        }
        // Rule 1: every DR line has a back side.
        let back = l.sides[1].ok_or(crate::MapScaleError::DoorLine)?;
        let sector = map.sidedefs()[usize::from(back)].sector;
        let mut created = Vec::new();
        if let Some(id) = special_data(self.store.mover_store(), sector) {
            // Rule 2: the cast of specialdata to a door reads a door.
            let Some(SectorThinker::Door(door)) = self.store.mover_mut(id) else {
                return Err(crate::MapScaleError::MoverFamily.into());
            };
            if door.direction == -1 {
                door.direction = 1;
            } else if player {
                door.direction = -1;
            }
        } else {
            self.sector_sound(sector, "sfx_doropn");
            let topheight = self.surrounding().lowest_ceiling(sector)? - Fixed::from_map_units(4);
            let door = SectorThinker::Door(Door {
                sector,
                kind: DoorKind::Normal,
                direction: 1,
                topheight,
                topcountdown: 0,
            });
            created.push((self.spawn(door)?, door));
        }
        self.finish(
            crate::planes::MoverFunction::VerticalDoor,
            line,
            i32::from(LINE_DOOR_DR),
            -1,
            &created,
        );
        Ok(())
    }

    /// p_plats.c `EV_DoPlat(downWaitUpStay)` (the WR 88 lift): every tagged sector
    /// without a live mover gets a lift going down at speed 4 to
    /// `P_FindLowestFloorSurrounding` (at most its floor), `high` = its floor, with
    /// `sfx_pstart`, and takes an `activeplats` slot (rule 5 keeps one free).
    pub(crate) fn do_plat(&mut self, line: u16) -> Result<bool, SimError> {
        let targets: Vec<u16> = self.tagged(line)?.collect();
        let mut created = Vec::new();
        for sector in targets {
            if special_data(self.store.mover_store(), sector).is_some() {
                continue;
            }
            let floor = self.surrounding().own(sector)?.floor;
            let low = self.surrounding().lowest_floor(sector)?.min(floor);
            let plat = SectorThinker::Plat(Plat {
                sector,
                status: PlatStatus::Down,
                count: 0,
                low,
                high: floor,
            });
            let id = self.spawn(plat)?;
            self.sector_sound(sector, "sfx_pstart");
            // P_AddActivePlat: I_Error without a free slot, unreachable under rule 5.
            if active_plats(self.store.mover_store()) > mover_rules::MAX_PLATS {
                return Err(crate::MapScaleError::MoverCount.into());
            }
            created.push((id, plat));
        }
        let result = i32::from(!created.is_empty());
        // plattype_e downWaitUpStay = 1.
        self.finish(
            crate::planes::MoverFunction::DoPlat,
            line,
            1,
            result,
            &created,
        );
        Ok(result != 0)
    }

    /// p_floor.c `EV_DoFloor` for `lowerFloorToLowest` (38: speed 1 to
    /// `P_FindLowestFloorSurrounding`) and `turboLower` (36: speed 4 to
    /// `P_FindHighestFloorSurrounding`, plus 8 unless it equals the floor). No
    /// sound at the start.
    pub(crate) fn do_floor(&mut self, line: u16, kind: FloorKind) -> Result<bool, SimError> {
        let targets: Vec<u16> = self.tagged(line)?.collect();
        let mut created = Vec::new();
        for sector in targets {
            if special_data(self.store.mover_store(), sector).is_some() {
                continue;
            }
            let floordestheight = match kind {
                FloorKind::LowerFloorToLowest => self.surrounding().lowest_floor(sector)?,
                FloorKind::TurboLower => {
                    let dest = self.surrounding().highest_floor(sector)?;
                    if dest != self.surrounding().own(sector)?.floor {
                        dest + Fixed::from_map_units(8)
                    } else {
                        dest
                    }
                }
            };
            let floor = SectorThinker::Floor(FloorMove {
                sector,
                kind,
                floordestheight,
            });
            created.push((self.spawn(floor)?, floor));
        }
        let result = i32::from(!created.is_empty());
        self.finish(
            crate::planes::MoverFunction::DoFloor,
            line,
            kind as i32,
            result,
            &created,
        );
        Ok(result != 0)
    }
}

/// One mover's turn in `P_RunThinkers`: `T_VerticalDoor`, `T_PlatRaise` or
/// `T_MoveFloor`. `leveltime` is the source `leveltime` of this tic.
pub(crate) fn think(
    host: &mut PlaneHost<'_, '_>,
    id: SectorThinkerId,
    leveltime: u64,
) -> Result<(), SimError> {
    let mover = *host
        .access
        .sector_get(id)
        .ok_or(StoreError::UnsupportedThinker)?;
    let (next, removed) = match mover {
        SectorThinker::Door(door) => vertical_door(host, door)?,
        SectorThinker::Plat(plat) => plat_raise(host, plat)?,
        SectorThinker::Floor(floor) => move_floor(host, floor, leveltime)?,
        _ => return Err(StoreError::UnsupportedThinker.into()),
    };
    *host
        .access
        .sector_get_mut(id)
        .ok_or(StoreError::UnsupportedThinker)? = next;
    if removed {
        // P_RemoveThinker: marked, unlinked when the next pass reaches it.
        host.access.mark_remove_sector(id)?;
        host.events
            .push(Event::Mover(crate::MoverEvent::Removed { mover: id }));
    }
    if let Some(sound) = removed_sound(mover, removed) {
        sector_sound(host, mover.mover_sector().unwrap_or(u16::MAX), sound);
    }
    Ok(())
}
/// `T_MoveFloor` plays `sfx_pstop` after `P_RemoveThinker`.
fn removed_sound(mover: SectorThinker, removed: bool) -> Option<&'static str> {
    (removed && matches!(mover, SectorThinker::Floor(_))).then_some("sfx_pstop")
}
fn sector_sound(host: &mut PlaneHost<'_, '_>, sector: u16, name: &str) {
    host.events
        .push(Event::Mover(crate::MoverEvent::SectorSound {
            origin: crate::planes::SoundOrigin::Sector,
            index: sector,
            sound: sound(name),
        }));
}

/// p_doors.c `T_VerticalDoor` (63–200) for `normal` and `blazeOpen`, ceiling plane,
/// `crush` false. Returns the updated record and whether it removed itself.
fn vertical_door(
    host: &mut PlaneHost<'_, '_>,
    mut door: Door,
) -> Result<(SectorThinker, bool), SimError> {
    let mut removed = false;
    match door.direction {
        0 => {
            // Waiting: `if (!--door->topcountdown)`; only `normal` turns down.
            door.topcountdown = door.topcountdown.wrapping_sub(1);
            if door.topcountdown == 0 && door.kind == DoorKind::Normal {
                door.direction = -1;
                sector_sound(host, door.sector, "sfx_dorcls");
            }
        }
        -1 => {
            let floor = host.sector(door.sector)?.floor;
            match host.move_plane(door.sector, door.speed(), floor, Plane::Ceiling, -1)? {
                // A normal door is removed at the floor; a reversed blazeOpen door
                // has no case (default) and stays at the floor as a live thinker.
                MoveResult::PastDest => removed = door.kind == DoorKind::Normal,
                // Every admitted door type reopens on an obstruction.
                MoveResult::Crushed => {
                    door.direction = 1;
                    sector_sound(host, door.sector, "sfx_doropn");
                }
                MoveResult::Ok => {}
            }
        }
        1 => {
            let result =
                host.move_plane(door.sector, door.speed(), door.topheight, Plane::Ceiling, 1)?;
            if result == MoveResult::PastDest {
                match door.kind {
                    DoorKind::Normal => {
                        door.direction = 0;
                        door.topcountdown = VDOORWAIT;
                    }
                    DoorKind::BlazeOpen => removed = true,
                }
            }
        }
        // The switch has no default: nothing for any other direction (never stored).
        _ => {}
    }
    Ok((SectorThinker::Door(door), removed))
}

/// p_plats.c `T_PlatRaise` (52–129) for `downWaitUpStay`, floor plane, `crush`
/// false; reaching the top calls `P_RemoveActivePlat`.
fn plat_raise(
    host: &mut PlaneHost<'_, '_>,
    mut plat: Plat,
) -> Result<(SectorThinker, bool), SimError> {
    let speed = Fixed::from_map_units(PLAT_SPEED as i16);
    let mut removed = false;
    match plat.status {
        PlatStatus::Up => match host.move_plane(plat.sector, speed, plat.high, Plane::Floor, 1)? {
            MoveResult::Crushed => {
                plat.count = PLAT_WAIT;
                plat.status = PlatStatus::Down;
                sector_sound(host, plat.sector, "sfx_pstart");
            }
            MoveResult::PastDest => {
                plat.count = PLAT_WAIT;
                plat.status = PlatStatus::Waiting;
                sector_sound(host, plat.sector, "sfx_pstop");
                removed = true;
            }
            MoveResult::Ok => {}
        },
        PlatStatus::Down => {
            if host.move_plane(plat.sector, speed, plat.low, Plane::Floor, -1)?
                == MoveResult::PastDest
            {
                plat.count = PLAT_WAIT;
                plat.status = PlatStatus::Waiting;
                sector_sound(host, plat.sector, "sfx_pstop");
            }
        }
        PlatStatus::Waiting => {
            plat.count = plat.count.wrapping_sub(1);
            if plat.count == 0 {
                plat.status = if host.sector(plat.sector)?.floor == plat.low {
                    PlatStatus::Up
                } else {
                    PlatStatus::Down
                };
                sector_sound(host, plat.sector, "sfx_pstart");
            }
        }
    }
    Ok((SectorThinker::Plat(plat), removed))
}

/// p_floor.c `T_MoveFloor` (209–253) for the admitted lowering types: one
/// downward plane step, `sfx_stnmov` when `leveltime & 7 == 0`, and at `pastdest`
/// the removal followed by `sfx_pstop`.
fn move_floor(
    host: &mut PlaneHost<'_, '_>,
    floor: FloorMove,
    leveltime: u64,
) -> Result<(SectorThinker, bool), SimError> {
    let result = host.move_plane(
        floor.sector,
        floor.speed(),
        floor.floordestheight,
        Plane::Floor,
        -1,
    )?;
    if leveltime & 7 == 0 {
        sector_sound(host, floor.sector, "sfx_stnmov");
    }
    Ok((SectorThinker::Floor(floor), result == MoveResult::PastDest))
}

#[cfg(test)]
impl crate::Simulation {
    /// Runs `EV_*` starters on the committed state between tics, as a use in the
    /// players phase would before the pass: the movers join the order's tail and
    /// step in the next tic. Their events are kept for that tic.
    pub(crate) fn start_movers<R>(
        &mut self,
        f: impl FnOnce(&mut Starter<'_, ThinkerStore<Actor>>) -> Result<R, SimError>,
    ) -> Result<R, SimError> {
        let mut events = Vec::new();
        let result = f(&mut Starter {
            world: &self.world,
            sectors: Sectors::committed(&self.sectors),
            store: &mut self.actors,
            events: &mut events,
        })?;
        self.pending.extend(events);
        Ok(result)
    }
}

#[cfg(test)]
#[path = "movers_tests.rs"]
mod tests;
