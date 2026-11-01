//! Shared built-in actor thinker pass; profile selection does not create a scheduler.
use crate::*;
pub(crate) struct Inputs<'a> {
    pub world: &'a StaticWorld,
    pub sectors: &'a [SectorState],
    pub profile: Profile,
    pub bindings: [Option<ActorId>; 4],
    pub live_bindings: &'a mut [Option<ActorId>; 4],
    pub registry: &'a Option<Arc<ActorRegistry>>,
    pub options: ActorOptions,
    pub player: Option<&'a mut original_player::PlayerRuntime>,
    pub statistics: &'a mut Option<LevelStatistics>,
    pub rng: &'a mut DoomRng,
    pub spatial: &'a mut movement::SpatialIndex,
    pub events: &'a mut Vec<Event>,
    pub commands: [TicCommand; 4],
    pub perception_inputs: Option<perception::PerceptionInputs<'a>>,
}
pub(crate) fn run(actors: &mut ThinkerStore<Actor>, input: Inputs<'_>) {
    let Inputs {
        world,
        sectors,
        profile,
        bindings,
        live_bindings,
        registry,
        options,
        statistics,
        mut player,
        rng,
        spatial,
        events,
        commands,
        perception_inputs,
    } = input;
    actors
        .run(
            actors.max_slots as usize,
            |visit, mut context| match visit {
                Visit::Retired(id) => events.push(Event::Retired(id)),
                Visit::Think(id) => {
                    if perception_inputs
                        .and_then(|p| p.checked)
                        .is_some_and(|c| c.failure.get().is_some())
                    {
                        return;
                    }
                    events.push(Event::Think(id));
                    let mut actor = *context.get(id).expect("live thinker handle");
                    if profile == Profile::OriginalDemon
                        && actor
                            .life
                            .is_some_and(|l| original_registry::is_fireball(l.actor_type))
                    {
                        // P_MobjThinker of a missile: its movement owns contact and
                        // explosion, then ordinary state countdown.
                        let mut host = lifecycle::LifeHost {
                            player: player.as_deref_mut(),
                            statistics: statistics.as_mut(),
                            perception: perception_inputs,
                            access: context,
                            registry: registry.as_deref().unwrap(),
                            world,
                            sectors,
                            options,
                            rng,
                            spatial,
                            bindings: live_bindings,
                            events,
                        };
                        projectiles::fly(&mut host, id);
                        host.tick(id);
                        return;
                    }
                    let had_xy_momentum = actor.momentum().iter().any(|m| *m != Fixed::ZERO);
                    let source_player = player.as_ref().is_some_and(|p| p.mo == Some(id));
                    if profile == Profile::Foundation {
                        events.extend(
                            world
                                .move_xy(&mut actor)
                                .into_iter()
                                .map(|attempt| Event::Movement { actor: id, attempt }),
                        );
                    } else {
                        let command = if source_player {
                            Some(commands[usize::from(PlayerSlot::One.ordinal())])
                        } else {
                            bindings
                                .iter()
                                .position(|binding| *binding == Some(id))
                                .map(|slot| commands[slot])
                        };
                        let motion = if source_player
                            && player
                                .as_ref()
                                .is_some_and(|p| p.policy.has_world_pickups())
                        {
                            let Some(p) = player.as_deref_mut() else {
                                return;
                            };
                            match movement::move_xy_contacts(
                                world,
                                id,
                                &mut actor,
                                spatial,
                                world_pickups::Contacts {
                                    access: &mut context,
                                    player: p,
                                },
                                command,
                                sectors,
                            ) {
                                Ok(events) => events,
                                Err(error) => {
                                    if let Some(control) = perception_inputs.and_then(|p| p.checked)
                                    {
                                        control.failure.set(Some(error));
                                    }
                                    return;
                                }
                            }
                        } else if profile.uses_sectors() {
                            movement::move_xy_multisector(
                                world,
                                id,
                                &mut actor,
                                spatial,
                                |id| context.get(id).copied(),
                                command,
                                sectors,
                            )
                        } else {
                            movement::move_xy(
                                world,
                                id,
                                &mut actor,
                                spatial,
                                |id| context.get(id).copied(),
                                command,
                            )
                        };
                        for event in motion {
                            events.push(match event {
                                movement::MotionEvent::Contact(event) => event.event(),
                                movement::MotionEvent::Attempt(attempt) => {
                                    Event::ActorMovement { actor: id, attempt }
                                }
                                movement::MotionEvent::Slide(line) => {
                                    Event::Slide { actor: id, line }
                                }
                                movement::MotionEvent::Sector(attempt) => {
                                    Event::SectorMovement { actor: id, attempt }
                                }
                            });
                        }
                    }
                    if (profile == Profile::PlayerWeapons || source_player)
                        && had_xy_momentum
                        && actor.z <= actor.floor
                        && actor.momentum() == [Fixed::ZERO; 2]
                        && (if source_player {
                            commands[usize::from(PlayerSlot::One.ordinal())].forward == 0
                                && commands[usize::from(PlayerSlot::One.ordinal())].side == 0
                        } else {
                            bindings
                                .iter()
                                .position(|b| *b == Some(id))
                                .is_some_and(|slot| {
                                    commands[slot].forward == 0 && commands[slot].side == 0
                                })
                        })
                    {
                        let run = identity::OriginalState::S_PLAY_RUN1.ordinal();
                        if actor
                            .life
                            .unwrap()
                            .state
                            .is_some_and(|s| s.0.wrapping_sub(run) < 4)
                        {
                            let play = StateId(identity::OriginalState::S_PLAY.ordinal());
                            *context.get_mut(id).unwrap() = actor;
                            lifecycle::LifeHost {
                                player: player.as_deref_mut(),
                                statistics: statistics.as_mut(),
                                perception: perception_inputs,
                                access: &mut *context,
                                registry: registry.as_deref().unwrap(),
                                world,
                                sectors,
                                options,
                                rng,
                                spatial,
                                bindings: live_bindings,
                                events,
                            }
                            .enter(id, play);
                            actor = *context.get(id).unwrap();
                        }
                    }
                    if profile.uses_sectors() {
                        if let Some(update) = multisector::move_z(&mut actor) {
                            events.push(Event::Vertical { actor: id, update });
                        }
                    }
                    *context.get_mut(id).expect("same live thinker handle") = actor;
                    if let Some(registry) = registry {
                        lifecycle::LifeHost {
                            player: player.as_deref_mut(),
                            statistics: statistics.as_mut(),
                            perception: perception_inputs,
                            access: context,
                            registry,
                            world,
                            sectors,
                            options,
                            rng,
                            spatial,
                            bindings: live_bindings,
                            events,
                        }
                        .tick(id);
                    }
                }
            },
        )
        .expect("built-in traversal preserves links and is bounded by max slots");
}
