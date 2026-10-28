//! Bounded source sight and explicit perception profile composition.
use crate::*;
use std::{error::Error, fmt};
/// Admission/identity/work rejection before authoritative perception changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PerceptionError {
    /// Explicit profile 8 required.
    Profile,
    /// Invalid, pending, retired or stale actor.
    Reference,
    /// Unsupported source/fixture metadata or action handoff.
    Definition,
    /// Source-compatible arithmetic slice exceeded.
    Arithmetic,
    /// Actor or expanded traversal bound exceeded.
    WorkLimit,
    /// Perception roster configuration is invalid.
    Roster,
    /// Retained sound target is pending, retired or stale.
    SoundTarget,
}
impl fmt::Display for PerceptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "perception: {self:?}")
    }
}
impl Error for PerceptionError {}
impl Simulation {
    /// Explicit profile 8; no roster, weapon player, chase or attack is implicit.
    pub fn monster_perception(map: Map) -> Result<Self, SimError> {
        Self::with_perception_registry(map, 64, ActorRegistry::vanilla(), ActorOptions::default())
    }
    /// Immutable original definitions; fixture admission is explicitly constrained.
    pub fn with_perception_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        if limit > 64 {
            return Err(PerceptionError::WorkLimit.into());
        }
        crate::sight::validate_map(&map)?;
        let source = ActorRegistry::vanilla();
        if registry.states() != source.states() || registry.types() != source.types() {
            return Err(PerceptionError::Definition.into());
        }
        let mut sim = Self::with_actor_registry(map, limit, registry, options)?;
        sim.profile = Profile::Perception;
        sim.sound_targets = vec![None; sim.sectors.len()];
        Ok(sim)
    }
    pub(crate) fn perception_actor(&self, id: ActorId) -> Result<Actor, SimError> {
        if self.profile != Profile::Perception {
            return Err(PerceptionError::Profile.into());
        }
        if self.actors.is_pending(id) != Some(false) {
            return Err(PerceptionError::Reference.into());
        }
        let a = *self.actor(id).ok_or(PerceptionError::Reference)?;
        crate::sight::actor_bounds(&a)?;
        Ok(a)
    }
    /// Original read-only REJECT/BSP sight, independent of roster/RNG/presentation.
    pub fn check_sight(&self, observer: ActorId, target: ActorId) -> Result<SightResult, SimError> {
        let observer = self.perception_actor(observer)?;
        let target = self.perception_actor(target)?;
        Ok(crate::sight::query(
            &self.world,
            &self.sectors,
            observer,
            target,
        ))
    }
}

/// Perception-only player record; activity, health and actor identity are distinct.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PerceptionPlayer {
    /// Corresponds to playeringame[slot], not a network connection.
    pub active: bool,
    /// Source player.health used by scanning, independent of mobj health.
    pub health: i32,
    /// A stable admitted unbound pawn/fixture; inactive records may retain old IDs.
    pub actor: Option<ActorId>,
}
/// Source perception decisions, in the order actually evaluated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PerceptionEvent {
    /// Start at current lastlook; no nearest-target sorting.
    ScanStart {
        /// Observer.
        actor: ActorId,
        /// Source allaround request.
        allaround: bool,
        /// Initial slot.
        lastlook: u8,
        /// Source (lastlook-1)&3.
        stop: u8,
    },
    /// Slot before the active filter and c++ stopping condition.
    Slot {
        /// Observer.
        actor: ActorId,
        /// Slot considered.
        slot: u8,
        /// playeringame value.
        active: bool,
        /// Active encounters before this slot.
        count: u8,
    },
    /// Player-record health before sight; not the actor's health.
    Health {
        /// Observer.
        actor: ActorId,
        /// Active slot.
        slot: u8,
        /// Signed source health.
        health: i32,
    },
    /// Directional test after successful sight; distance evaluated only in rear.
    Facing {
        /// Observer.
        actor: ActorId,
        /// Candidate slot.
        slot: u8,
        /// Wrapping BAM angle relative to observer.
        angle: u32,
        /// Approximate rear distance, absent for front candidates.
        distance: Option<Fixed>,
        /// Source directional/close-range decision.
        accepted: bool,
    },
    /// Safe empty-roster termination where original C would loop forever.
    EmptyRoster(ActorId),
    /// Return with lastlook at the exact current source slot.
    ScanEnd {
        /// Observer.
        actor: ActorId,
        /// Whether target assignment occurred.
        found: bool,
        /// Final slot.
        lastlook: u8,
    },
    /// Local sight evidence for a mutating source perception operation.
    Sight {
        /// Observer.
        observer: ActorId,
        /// Candidate or sound target.
        target: ActorId,
        /// Complete read-only result/decision order.
        result: SightResult,
    },
}
pub(crate) struct ScanPlan {
    pub lastlook: u8,
    pub target: Option<ActorId>,
    pub events: Vec<Event>,
}
fn perception_event(events: &mut Vec<Event>, e: PerceptionEvent) {
    events.push(Event::Perception(e));
}
pub(crate) fn scan(
    world: &StaticWorld,
    sectors: &[SectorState],
    roster: &[PerceptionPlayer; 4],
    get: &impl Fn(ActorId) -> Option<Actor>,
    id: ActorId,
    allaround: bool,
) -> ScanPlan {
    let observer = get(id).expect("preflight observer");
    let mut lastlook = observer.lastlook;
    let stop = lastlook.wrapping_sub(1) & 3;
    let mut plan = ScanPlan {
        lastlook,
        target: None,
        events: Vec::new(),
    };
    perception_event(
        &mut plan.events,
        PerceptionEvent::ScanStart {
            actor: id,
            allaround,
            lastlook,
            stop,
        },
    );
    if !roster.iter().any(|p| p.active) {
        perception_event(&mut plan.events, PerceptionEvent::EmptyRoster(id));
        return plan;
    }
    let mut count = 0_u8;
    for _ in 0..12 {
        let p = roster[usize::from(lastlook)];
        perception_event(
            &mut plan.events,
            PerceptionEvent::Slot {
                actor: id,
                slot: lastlook,
                active: p.active,
                count,
            },
        );
        if p.active {
            let prior = count;
            count += 1;
            if prior == 2 || lastlook == stop {
                plan.lastlook = lastlook;
                return plan;
            }
            perception_event(
                &mut plan.events,
                PerceptionEvent::Health {
                    actor: id,
                    slot: lastlook,
                    health: p.health,
                },
            );
            if p.health > 0 {
                let target = p.actor.expect("preflight active roster");
                let a = get(target).expect("preflight stable target");
                let result = crate::sight::query(world, sectors, observer, a);
                let visible = result.visible;
                perception_event(
                    &mut plan.events,
                    PerceptionEvent::Sight {
                        observer: id,
                        target,
                        result,
                    },
                );
                if visible {
                    let mut accepted = true;
                    if !allaround {
                        let delta = [
                            a.position()[0] - observer.position()[0],
                            a.position()[1] - observer.position()[1],
                        ];
                        let angle = movement::point_angle(delta).wrapping_sub(observer.angle());
                        let rear = angle > 0x40000000 && angle < 0xc0000000;
                        let distance = rear.then(|| {
                            let x = delta[0].bits().unsigned_abs() as i32;
                            let y = delta[1].bits().unsigned_abs() as i32;
                            Fixed::from_bits(x + y - (x.min(y) >> 1))
                        });
                        accepted = distance.is_none_or(|d| d <= Fixed::from_map_units(64));
                        perception_event(
                            &mut plan.events,
                            PerceptionEvent::Facing {
                                actor: id,
                                slot: lastlook,
                                angle,
                                distance,
                                accepted,
                            },
                        );
                    }
                    if accepted {
                        plan.lastlook = lastlook;
                        plan.target = Some(target);
                        return plan;
                    }
                }
            }
        }
        lastlook = (lastlook + 1) & 3;
    }
    unreachable!("nonempty four-slot source scan terminates within 12 visits")
}
pub(crate) fn commit_scan<A: crate::lifecycle::ActorAccess>(
    host: &mut crate::lifecycle::LifeHost<'_, A>,
    id: ActorId,
    plan: ScanPlan,
) -> bool {
    host.events.extend(plan.events);
    host.access.get_mut(id).unwrap().lastlook = plan.lastlook;
    if let Some(target) = plan.target {
        host.target(id, Some(target));
    }
    let found = plan.target.is_some();
    host.events
        .push(Event::Perception(PerceptionEvent::ScanEnd {
            actor: id,
            found,
            lastlook: plan.lastlook,
        }));
    found
}
impl Simulation {
    /// Read the distinct four-slot perception roster.
    pub fn perception_players(&self) -> &[PerceptionPlayer; 4] {
        &self.perception_players
    }
    fn validate_perception_reference(&self, id: ActorId) -> Result<Actor, SimError> {
        let a = self.perception_actor(id)?;
        let life = a.life.unwrap();
        let registry = self.registry.as_deref().unwrap();
        let fixture = [19, 21, 136].contains(&life.actor_type.0)
            && registry.actor_type(life.actor_type).unwrap().states()[..2]
                == [StateId(1), StateId(2)];
        if life.actor_type != ActorTypeId(0) && !fixture {
            return Err(PerceptionError::Roster.into());
        }
        let valid = life
            .state
            .is_some_and(|s| (149..=173).contains(&s.0) || fixture && [1, 2].contains(&s.0));
        if !valid {
            return Err(PerceptionError::Roster.into());
        }
        Ok(a)
    }
    pub(crate) fn preflight_perception_roster(&self) -> Result<(), SimError> {
        for p in self.perception_players {
            if p.active {
                self.validate_perception_reference(p.actor.ok_or(PerceptionError::Roster)?)?;
            }
        }
        Ok(())
    }
    /// Explicit fixture/player registration; no full player gameplay or damage.
    pub fn set_perception_player(
        &mut self,
        slot: usize,
        p: PerceptionPlayer,
    ) -> Result<(), SimError> {
        if self.profile != Profile::Perception {
            return Err(PerceptionError::Profile.into());
        }
        if slot >= 4 || p.active && p.actor.is_none() {
            return Err(PerceptionError::Roster.into());
        }
        if let Some(id) = p.actor {
            self.validate_perception_reference(id)?;
        }
        self.perception_players[slot] = p;
        Ok(())
    }
    /// Between-tic source observer angle/threshold setup, leaving lastlook untouched.
    pub fn configure_perception_observer(
        &mut self,
        id: ActorId,
        angle: u32,
        threshold: i32,
    ) -> Result<(), SimError> {
        self.perception_actor(id)?;
        let a = self.actors.get_mut(id).unwrap();
        a.spec.angle = angle;
        a.life.as_mut().unwrap().threshold = threshold;
        Ok(())
    }
    /// Exact P_LookForPlayers selection. Ok(false) may still advance lastlook.
    pub fn look_for_players(&mut self, id: ActorId, allaround: bool) -> Result<bool, SimError> {
        self.perception_actor(id)?;
        self.preflight_perception_roster()?;
        let plan = scan(
            &self.world,
            &self.sectors,
            &self.perception_players,
            &|id| self.actor(id).copied(),
            id,
            allaround,
        );
        Ok(commit_scan(
            &mut crate::lifecycle::LifeHost {
                access: &mut self.actors,
                registry: self.registry.as_deref().unwrap(),
                world: &self.world,
                sectors: &self.sectors,
                options: self.actor_options,
                rng: &mut self.rng,
                spatial: &mut self.spatial,
                bindings: &mut self.bindings,
                events: &mut self.pending,
            },
            id,
            plan,
        ))
    }
    pub(crate) fn preflight_perception_step(&self) -> Result<(), SimError> {
        self.preflight_perception_roster()?;
        for id in self.actors.order() {
            if self.actors.is_pending(id) != Some(false) {
                continue;
            }
            let a = self.actor(id).unwrap();
            crate::sight::actor_bounds(a)?;
            for axis in 0..2 {
                if a.momentum()[axis] != Fixed::ZERO
                    && !crate::sight::within(a.position()[axis], 994)
                {
                    return Err(PerceptionError::Arithmetic.into());
                }
            }
            if !crate::sight::within(a.momz, 64) || !crate::sight::within(a.z + a.momz, 1024) {
                return Err(PerceptionError::Arithmetic.into());
            }
        }
        Ok(())
    }
}

fn identity(b: &mut Vec<u8>, id: ActorId) {
    b.extend(id.slot().to_le_bytes());
    b.extend(id.generation().to_le_bytes());
}
pub(crate) fn event_bytes(b: &mut Vec<u8>, e: &PerceptionEvent) {
    match e {
        PerceptionEvent::ScanStart {
            actor,
            allaround,
            lastlook,
            stop,
        } => {
            b.push(2);
            identity(b, *actor);
            b.extend([u8::from(*allaround), *lastlook, *stop]);
        }
        PerceptionEvent::Slot {
            actor,
            slot,
            active,
            count,
        } => {
            b.push(3);
            identity(b, *actor);
            b.extend([*slot, u8::from(*active), *count]);
        }
        PerceptionEvent::Health {
            actor,
            slot,
            health,
        } => {
            b.push(4);
            identity(b, *actor);
            b.push(*slot);
            b.extend(health.to_le_bytes());
        }
        PerceptionEvent::Facing {
            actor,
            slot,
            angle,
            distance,
            accepted,
        } => {
            b.push(5);
            identity(b, *actor);
            b.push(*slot);
            b.extend(angle.to_le_bytes());
            b.push(u8::from(distance.is_some()));
            if let Some(d) = distance {
                b.extend(d.bits().to_le_bytes());
            }
            b.push(u8::from(*accepted));
        }
        PerceptionEvent::EmptyRoster(actor) => {
            b.push(6);
            identity(b, *actor);
        }
        PerceptionEvent::ScanEnd {
            actor,
            found,
            lastlook,
        } => {
            b.push(7);
            identity(b, *actor);
            b.extend([u8::from(*found), *lastlook]);
        }
        PerceptionEvent::Sight {
            observer,
            target,
            result: r,
        } => {
            b.push(10);
            identity(b, *observer);
            identity(b, *target);
            b.push(u8::from(r.visible));
            b.extend(r.reject_index.to_le_bytes());
            b.extend([u8::from(r.rejected), u8::from(r.eye.is_some())]);
            if let Some(eye) = r.eye {
                b.extend(eye.bits().to_le_bytes());
                for s in r.slopes.unwrap() {
                    b.extend(s.bits().to_le_bytes());
                }
            }
            b.extend((r.visits.len() as u32).to_le_bytes());
            for v in &r.visits {
                match v {
                    SightVisit::NodeStart { node, side } | SightVisit::NodeEnd { node, side } => {
                        b.push(if matches!(v, SightVisit::NodeStart { .. }) {
                            1
                        } else {
                            2
                        });
                        b.extend(node.to_le_bytes());
                        b.push(*side);
                    }
                    SightVisit::Subsector(i) => {
                        b.push(3);
                        b.extend(i.to_le_bytes());
                    }
                    SightVisit::Seg {
                        subsector,
                        seg,
                        line,
                        duplicate,
                    } => {
                        b.push(4);
                        b.extend(subsector.to_le_bytes());
                        b.extend(seg.to_le_bytes());
                        b.extend(line.to_le_bytes());
                        b.push(u8::from(*duplicate));
                    }
                    SightVisit::Sides { line, stage, sides } => {
                        b.push(5);
                        b.extend(line.to_le_bytes());
                        b.extend([*stage, sides[0], sides[1]]);
                    }
                    SightVisit::Opening { line, heights } => {
                        b.push(6);
                        b.extend(line.to_le_bytes());
                        for h in heights {
                            b.extend(h.bits().to_le_bytes());
                        }
                    }
                    SightVisit::Clip {
                        line,
                        fraction,
                        slopes,
                    } => {
                        b.push(7);
                        b.extend(line.to_le_bytes());
                        for v in [*fraction, slopes[0], slopes[1]] {
                            b.extend(v.bits().to_le_bytes());
                        }
                    }
                    SightVisit::Blocked { line, reason } => {
                        b.push(8);
                        b.extend(line.to_le_bytes());
                        b.push(*reason);
                    }
                }
            }
        }
    }
}

impl Simulation {
    /// Original P_NoiseAlert(target, emitter), sharing profile-6 propagation.
    /// This does not fire a weapon or require the target to be a roster player.
    pub fn noise_alert(&mut self, target: ActorId, emitter: ActorId) -> Result<(), SimError> {
        self.validate_perception_reference(target)?;
        self.perception_actor(emitter)?;
        self.emit_noise(target, emitter);
        Ok(())
    }
}
