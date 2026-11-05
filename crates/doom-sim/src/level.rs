//! The level policy's player-think additions (M3h9c, decision G4, wire 22):
//! `P_DeathThink`'s turn toward the attacker and the `damagecount` decays of
//! `P_PlayerThink` and `P_DeathThink`. Every older policy never reaches this code.
use crate::{lifecycle::LifeHost, original_player::PlayerBodyError, *};

/// p_user.c:180 `ANG5 (ANG90/18)`, integer division of the BAM right angle.
pub(crate) const ANG5: u32 = 0x4000_0000 / 18;
/// tables.h `ANG180`.
const ANG180: u32 = 0x8000_0000;

/// The part of `P_DeathThink` after `P_MovePsprites` and `P_CalcHeight`
/// (p_user.c:200–225) for a dead player under the level policy. `viewheight`,
/// `deltaviewheight`, `onground` and the view half of `P_CalcHeight` (190–198)
/// are presentation and not modelled; the bob is the caller's `CalcHeight`.
///
/// With an attacker other than the body, the body turns toward it:
/// `angle = R_PointToAngle2 (mo->x, mo->y, attacker->x, attacker->y)`,
/// `delta = angle - mo->angle` (unsigned BAM, wrapping); `delta < ANG5 ||
/// delta > (unsigned)-ANG5` sets the angle exactly and decrements a nonzero
/// `damagecount`, otherwise the angle steps `ANG5` toward it (`+` below
/// `ANG180`, `-` otherwise). Without one, a nonzero `damagecount` decrements.
/// The source reads the attacker through a raw pointer; an attacker handle that
/// no longer resolves to an actor that is not pending removal (only a
/// nightmare-respawned corpse can be one) is refused as
/// `PlayerBodyError::Invariant` inside the staged tic, state preserved.
pub(crate) fn death_turn(
    host: &mut LifeHost<'_, &mut ThinkerStore<Actor>>,
) -> Result<(), SimError> {
    let p = host.player.as_deref().ok_or(PlayerBodyError::Profile)?;
    let mo = p.mo.ok_or(PlayerBodyError::Binding)?;
    let attacker = p.attacker.filter(|a| *a != mo);
    let mut decay = true;
    if let Some(attacker) = attacker {
        if host.access.is_pending(attacker) != Some(false) {
            return Err(PlayerBodyError::Invariant.into());
        }
        let target = host
            .access
            .get(attacker)
            .ok_or(PlayerBodyError::Invariant)?
            .position();
        let body = host.access.get_mut(mo).ok_or(PlayerBodyError::Binding)?;
        let position = body.position();
        let angle = movement::point_angle([target[0] - position[0], target[1] - position[1]]);
        let delta = angle.wrapping_sub(body.spec.angle);
        if delta < ANG5 || delta > ANG5.wrapping_neg() {
            body.spec.angle = angle;
        } else {
            decay = false;
            body.spec.angle = if delta < ANG180 {
                body.spec.angle.wrapping_add(ANG5)
            } else {
                body.spec.angle.wrapping_sub(ANG5)
            };
        }
    }
    let p = host.player.as_deref_mut().ok_or(PlayerBodyError::Profile)?;
    if decay && p.damage_count > 0 {
        p.damage_count -= 1;
    }
    Ok(())
}
