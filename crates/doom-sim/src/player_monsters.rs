//! Original former humans and bounded multi-monster encounters in the shared
//! player world. Every service (actors, thinkers, sight, pursuit, attacks,
//! damage, pickups, statistics and transactions) is the existing owner.
use crate::*;

/// Explicit original multi-monster execution context; historical contexts stay frozen.
pub const PLAYER_MONSTERS_CONTEXT_SCHEMA: &[u8] = b"PLAYER-MONSTERS-1-MEDIUM-GROUNDED-v1";

/// Bounded mixed roster of autonomous original monsters (Demon and former
/// humans). With one player, at most 16 world items, eight drops and bounded
/// impacts this stays well inside the 64 actor slots and per-tic sight budget.
pub const MAX_PLAYER_MONSTERS: usize = 8;

impl Simulation {
    /// Autonomous roster member: the native/original Demon owner or, under
    /// `PlayerMonstersV1`, an original former human.
    pub(crate) fn is_roster_monster(&self, ty: ActorTypeId) -> bool {
        ty == self.demon_owner_type()
            || (self.player_policy().has_monsters() && original_registry::is_former_human(ty))
    }
    /// Bonus world plus original MT_POSSESSED/MT_SHOTGUY/MT_CHAINGUY, a bounded
    /// mixed roster, source infighting and P_KillMobj item drops.
    pub fn original_player_monsters_map(
        map: Map,
        limit: u32,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        Self::with_original_registry(
            map,
            limit,
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersV1)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
}

use crate::lifecycle::{ActorAccess, LifeHost};

/// p_local.h MISSILERANGE (32*64 map units).
const MISSILE_RANGE_UNITS: i16 = 2048;
/// Shotgunner pellets per A_SPosAttack (p_enemy.c loop bound).
const SHOTGUNNER_PELLETS: u8 = 3;

/// Original former-human hitscan callbacks sharing one implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Hitscan {
    /// A_PosAttack: face, aim, pistol sound, one bullet.
    Pistol,
    /// A_SPosAttack: shotgun sound, face, aim, three complete bullets.
    Shotgun,
    /// A_CPosAttack: shotgun sound, face, aim, one bullet.
    Chaingun,
}

pub(crate) fn uses_current_monsters<A: ActorAccess>(h: &LifeHost<'_, A>) -> bool {
    h.player.as_ref().is_some_and(|p| p.policy.has_monsters())
}

/// Current-state original attack inside the bounded checked transaction. Any
/// late error is latched and discards the whole staged tic.
pub(crate) fn attack<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId, kind: Hitscan) {
    if let Err(error) = attack_checked(h, id, kind) {
        // Keep the first latched cause; the staged tic is discarded either way.
        if !h.sight_failed() {
            h.fail_sight(error);
        }
    }
}

fn readable_target<A: ActorAccess>(
    h: &LifeHost<'_, A>,
    id: ActorId,
) -> Result<Option<ActorId>, SimError> {
    let actor = h.access.get(id).ok_or(CombatError::Reference)?;
    let target = actor.life().ok_or(MonsterError::Definition)?.target;
    if let Some(t) = target {
        // Original targets stay allocated in this world (no corpse removal);
        // an unreadable handle is outside the admitted domain.
        h.access.get(t).ok_or(MonsterError::Target)?;
    }
    Ok(target)
}

fn attack_checked<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
    kind: Hitscan,
) -> Result<(), SimError> {
    if readable_target(h, id)?.is_none() {
        return Ok(());
    }
    if kind == Hitscan::Shotgun {
        h.events
            .push(Event::Shotgun(ShotgunEvent::Begin { actor: id }));
    }
    let sound = |name| lifecycle::source_sound(name).ok_or(MonsterError::Definition);
    if kind != Hitscan::Pistol {
        h.sound(Some(id), sound("sfx_shotgn")?);
    }
    crate::monsters::face(h, id);
    let range = Fixed::from_map_units(MISSILE_RANGE_UNITS);
    let angle = h.access.get(id).ok_or(CombatError::Reference)?.angle();
    let aim = {
        let lookup = |i| h.access.get(i).copied();
        let trace = combat_trace::collect_in(h.world, h.spatial, &lookup, id, angle, range)?;
        combat_trace::aim_trace(h.world, h.sectors, id, trace, range, &lookup)?
    };
    let slope = aim.slope;
    h.events.push(Event::Monster(MonsterEvent::Aim {
        actor: id,
        result: Box::new(aim),
    }));
    if kind == Hitscan::Pistol {
        h.sound(Some(id), sound("sfx_pistol")?);
    }
    let pellets = if kind == Hitscan::Shotgun {
        SHOTGUNNER_PELLETS
    } else {
        1
    };
    for index in 1..=pellets {
        if kind == Hitscan::Shotgun {
            h.events.push(Event::Shotgun(ShotgunEvent::PelletStart {
                actor: id,
                index,
            }));
        }
        let (shot, damage) = weapons::monster_shot(angle, &mut |k| crate::monsters::draw(h, id, k));
        if kind == Hitscan::Shotgun {
            h.events.push(Event::Shotgun(ShotgunEvent::Pellet {
                actor: id,
                index,
                angle: shot,
                slope,
                damage,
            }));
        }
        let result = bullet(h, id, shot, slope, damage)?;
        if kind == Hitscan::Shotgun {
            h.events.push(Event::Shotgun(ShotgunEvent::PelletDone {
                actor: id,
                index,
                rng: h.rng.indices()[0],
                effect: result.effect,
            }));
        }
    }
    if kind == Hitscan::Shotgun {
        h.events.push(Event::Shotgun(ShotgunEvent::Completed {
            actor: id,
            rng: h.rng.indices()[0],
        }));
    }
    Ok(())
}

/// One complete current P_LineAttack: collect, hit, plan current damage, then
/// the shared impact/damage executor. Earlier pellets' effects are visible.
fn bullet<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
    angle: u32,
    slope: Fixed,
    damage: i32,
) -> Result<AttackResult, SimError> {
    let attack = LineAttack {
        angle,
        range: Fixed::from_map_units(MISSILE_RANGE_UNITS),
        slope,
        damage,
    };
    let (trace, (hit, position, traversed)) = {
        let lookup = |i| h.access.get(i).copied();
        let trace = combat_trace::collect_in(h.world, h.spatial, &lookup, id, angle, attack.range)?;
        let hit =
            combat_trace::hit_trace(h.world, h.sectors, id, &trace, attack.range, slope, &lookup)?;
        (trace, hit)
    };
    let plan = match hit {
        Some(TraceObject::Actor(target)) if damage != 0 => {
            Some(combat::DamagePlan::current_original(
                h,
                DamageRequest {
                    target,
                    inflictor: Some(id),
                    source: Some(id),
                    amount: damage,
                },
            )?)
        }
        _ => None,
    };
    let prepared = combat::PreparedAttack::current(
        id,
        attack,
        AttackResult {
            trace,
            hit,
            position,
            traversed,
            effect: None,
            damage: None,
        },
        plan,
    );
    h.access.reserve(prepared.spawn_count())?;
    let result = combat::execute_attack(h, prepared);
    if h.sight_failed() {
        return Err(PlayerBodyError::Unsupported.into());
    }
    Ok(result)
}

/// A_CPosRefire with the checked expanded sight query; decision order is the
/// shared source short-circuit in `chaingunner::decide`.
pub(crate) fn refire<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId, work: &mut usize) {
    if let Err(error) = refire_checked(h, id, work) {
        // Keep the first latched cause; the staged tic is discarded either way.
        if !h.sight_failed() {
            h.fail_sight(error);
        }
    }
}
fn refire_checked<A: ActorAccess>(
    h: &mut LifeHost<'_, A>,
    id: ActorId,
    work: &mut usize,
) -> Result<(), SimError> {
    let target = readable_target(h, id)?;
    crate::monsters::face(h, id);
    let roll = crate::monsters::draw(h, id, 6);
    let a = *h.access.get(id).ok_or(CombatError::Reference)?;
    let checked = h.perception.and_then(|p| p.checked);
    let mut failure = None;
    let (decision, sight) = crate::chaingunner::decide(
        roll,
        || target.and_then(|t| h.access.get(t).and_then(Actor::life).map(|l| l.health)),
        || {
            let t = *h
                .access
                .get(target.expect("health branch proved target"))
                .unwrap();
            crate::demon_expanded_map::action_sight(checked, h.world, h.sectors, a, t)
                .unwrap_or_else(|error| {
                    failure = Some(error);
                    SightResult {
                        visible: false,
                        reject_index: 0,
                        rejected: true,
                        eye: None,
                        slopes: None,
                        visits: Vec::new(),
                    }
                })
        },
    );
    if let Some(error) = failure {
        return Err(error);
    }
    if let Some(result) = sight {
        h.events.push(Event::Perception(PerceptionEvent::Sight {
            observer: id,
            target: target.ok_or(MonsterError::Target)?,
            result,
        }));
    }
    h.events.push(Event::Refire(RefireEvent {
        actor: id,
        decision,
    }));
    if decision.exits() {
        let see = h
            .registry
            .actor_type(a.life().ok_or(MonsterError::Definition)?.actor_type)
            .ok_or(MonsterError::Definition)?
            .states()[1];
        h.enter_counted(id, see, work);
    }
    Ok(())
}
