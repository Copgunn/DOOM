//! Bounded original-player command stage, sharing motion, ownership and lifetime.
use crate::{identity::*, lifecycle::ActorAccess, *};

/// DSIM22 command/gate capability; historical body contexts remain distinct.
pub const PLAYER_COMMAND_CONTEXT_SCHEMA: &[u8] = b"PLAYER-COMMAND-1-MEDIUM-GROUNDED-v1";
// p_mobj.h: P_PlayerThink would rewrite commands for this excluded branch.
pub(crate) const MF_JUSTATTACKED: u32 = 0x80;
// g_game.c MAXPLMOVE=forwardmove[1], not arbitrary signed command bytes.
const MAX_COMMAND_MOVE: i8 = 50;
// M3G4_REFERENCE.md: conservative whole axis-slide pass, in raw fixed bits.
const ATTEMPT_MOVE_MULTIPLIER: i64 = 4;
const ATTEMPT_ROUNDING_MARGIN: i64 = 8;

impl Simulation {
    /// One genuine original player with grounded commands and reaction-time gate.
    /// Only slot One, zero buttons, flat static axis walls and checked travel.
    pub fn original_player_command_map(
        map: Map,
        limit: u32,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        let registry =
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerCommandMovementV1)?;
        Self::with_original_registry(map, limit, registry, ActorOptions::default(), context)
            .map_err(Into::into)
    }

    /// Controlled initial source counter setup, before the first successful tic.
    /// Default spawning remains pristine. This does not implement teleportation.
    /// The existing bound live body must still be current and nonpending.
    pub fn set_initial_player_reaction_time(
        &mut self,
        _slot: PlayerSlot,
        delay: u8,
    ) -> Result<(), SimError> {
        if !self.player_policy().has_player_commands() {
            return Err(PlayerBodyError::Profile.into());
        }
        if self.tic != 0 {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.preflight_player_body(false)?;
        let p = self
            .original_player
            .as_ref()
            .ok_or(PlayerBodyError::Profile)?;
        let id = p.mo.ok_or(PlayerBodyError::Binding)?;
        if p.state != PlayerBodyState::Live || self.actors.is_pending(id) != Some(false) {
            return Err(PlayerBodyError::Binding.into());
        }
        let actor = self.actors.get_mut(id).ok_or(PlayerBodyError::Binding)?;
        let life = actor.life.as_mut().ok_or(PlayerBodyError::Invariant)?;
        if life.actor_type.0 != OriginalActorType::MT_PLAYER.ordinal() || life.health <= 0 {
            return Err(PlayerBodyError::Binding.into());
        }
        life.reaction_time = i32::from(delay);
        Ok(())
    }

    pub(crate) fn preflight_player_commands(
        &self,
        commands: &[TicCommand; 4],
    ) -> Result<Option<ActorId>, SimError> {
        let p = self
            .original_player
            .as_ref()
            .ok_or(PlayerBodyError::Profile)?;
        if !p.policy.has_player_commands() {
            return Err(PlayerBodyError::Profile.into());
        }
        let slot = usize::from(PlayerSlot::One.ordinal());
        if commands
            .iter()
            .enumerate()
            .any(|(index, c)| index != slot && *c != TicCommand::default())
        {
            return Err(PlayerBodyError::Slot.into());
        }
        let command = commands[slot];
        if command.buttons
            & !(if p.policy == OriginalExecutionProfile::PlayerDemonPistolV1 {
                BT_ATTACK
            } else {
                0
            })
            != 0
        {
            return Err(SimError::UnsupportedButtons);
        }
        if !(-MAX_COMMAND_MOVE..=MAX_COMMAND_MOVE).contains(&command.forward)
            || !(-MAX_COMMAND_MOVE..=MAX_COMMAND_MOVE).contains(&command.side)
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        let current = p.mo.filter(|id| self.actors.is_pending(*id) == Some(false));
        let Some(id) = current else {
            return if command == TicCommand::default() {
                Ok(None)
            } else {
                Err(PlayerBodyError::Binding.into())
            };
        };
        let actor = self.actor(id).ok_or(PlayerBodyError::Invariant)?;
        let life = actor.life().ok_or(PlayerBodyError::Invariant)?;
        if life.actor_type.0 != OriginalActorType::MT_PLAYER.ordinal() || life.state.is_none() {
            return Err(PlayerBodyError::Invariant.into());
        }
        if p.state == PlayerBodyState::Dead {
            return if command == TicCommand::default() {
                Ok(None)
            } else {
                Err(PlayerBodyError::Unsupported.into())
            };
        }
        Ok(Some(id))
    }
}

fn attempt_bounds(actor: &Actor) -> Result<(), SimError> {
    let [x, y] = actor.momentum().map(|m| m.bits().unsigned_abs());
    let maximum = x.max(y);
    if maximum > original_player::MAX_MOVE_BITS {
        return Err(PlayerBodyError::Unsupported.into());
    }
    let margin = if maximum == 0 {
        0
    } else {
        ATTEMPT_MOVE_MULTIPLIER * i64::from(maximum) + ATTEMPT_ROUNDING_MARGIN
    };
    let limit =
        i64::from(original_player::PLAYER_BODY_COORDINATE_LIMIT) * i64::from(Fixed::ONE.bits());
    if actor
        .position()
        .into_iter()
        .any(|p| i64::from(p.bits()).abs() + margin > limit)
    {
        return Err(PlayerBodyError::Unsupported.into());
    }
    Ok(())
}

pub(crate) fn stage<A: ActorAccess>(
    host: &mut lifecycle::LifeHost<'_, A>,
    id: ActorId,
    command: TicCommand,
) -> Result<(), SimError> {
    let player = host.player.as_deref().ok_or(PlayerBodyError::Profile)?;
    if player.mo != Some(id) || player.state != PlayerBodyState::Live {
        return Err(PlayerBodyError::Binding.into());
    }
    // The caller admitted this current nonpending handle before cloning the
    // existing bounded actor plan; no thinker/removal occurs before this stage.
    let actor = host.access.get_mut(id).ok_or(PlayerBodyError::Binding)?;
    let life = actor.life().ok_or(PlayerBodyError::Invariant)?;
    if life.actor_type.0 != OriginalActorType::MT_PLAYER.ordinal() || life.health <= 0 {
        return Err(PlayerBodyError::Invariant.into());
    }
    let moved = apply_player_motion(actor, command)?;
    attempt_bounds(actor)?;
    let running = moved
        && (command.forward != 0 || command.side != 0)
        && actor
            .life()
            .is_some_and(|l| l.state == Some(StateId(OriginalState::S_PLAY.ordinal())));
    if running {
        host.enter(id, StateId(OriginalState::S_PLAY_RUN1.ordinal()));
    }
    if moved {
        host.events.push(Event::Player {
            slot: PlayerSlot::One.ordinal(),
            actor: id,
        });
    }
    Ok(())
}

pub(crate) fn validate_result(
    actors: &ThinkerStore<Actor>,
    player: Option<&OriginalPlayer>,
) -> Result<(), SimError> {
    if let Some(id) = player.and_then(|p| p.mo) {
        if actors.is_pending(id) == Some(false) {
            let actor = actors.get(id).ok_or(PlayerBodyError::Invariant)?;
            // Actual movement is executed once. This postcondition preserves
            // the source-zero-command continuation instead of predicting it.
            original_player::travel_bounds(actor.position(), actor.momentum())?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "player_command_tests.rs"]
mod tests;
