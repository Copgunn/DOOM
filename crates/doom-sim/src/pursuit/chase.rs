//! Supported original non-attacking A_Chase.
use super::*;
pub(crate) fn chase<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId, work: &mut usize) {
    start(h, id, PursuitOperation::Chase);
    chase_body(h, id, work);
    end(h, id, PursuitOperation::Chase, false);
}
fn chase_body<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId, work: &mut usize) {
    let before = *h.access.get(id).unwrap();
    let life = before.life.unwrap();
    let target = life.target.map(|t| *h.access.get(t).unwrap());
    let actor = h.access.get_mut(id).unwrap();
    let l = actor.life.as_mut().unwrap();
    if l.reaction_time != 0 {
        l.reaction_time -= 1;
    }
    if l.threshold != 0 {
        if target.is_none_or(|a| a.life.unwrap().health <= 0) {
            l.threshold = 0;
        } else {
            l.threshold -= 1;
        }
    }
    if l.movedir < 8 {
        actor.spec.angle &= 7 << 29;
        let delta = actor.spec.angle.wrapping_sub(u32::from(l.movedir) << 29) as i32;
        if delta > 0 {
            actor.spec.angle = actor.spec.angle.wrapping_sub(0x20000000);
        } else if delta < 0 {
            actor.spec.angle = actor.spec.angle.wrapping_add(0x20000000);
        }
    }
    if target.is_none_or(|a| a.life.unwrap().flags & 4 == 0) {
        let inputs = h.perception.unwrap();
        let plan = crate::perception::scan(
            h.world,
            h.sectors,
            inputs.players,
            &|id| h.access.get(id).copied(),
            id,
            true,
            inputs.checked,
        );
        let plan = match plan {
            Ok(plan) => plan,
            Err(error) => {
                h.fail_sight(error);
                return;
            }
        };
        if crate::perception::commit_scan(h, id, plan) {
            return;
        }
        let spawn = h.registry.actor_type(life.actor_type).unwrap().states()[0];
        h.enter_counted(id, spawn, work);
        return;
    }
    if h.access.get(id).unwrap().life.unwrap().flags & 128 != 0 {
        h.access.get_mut(id).unwrap().life.as_mut().unwrap().flags &= !128;
        if h.options.skill != Skill::Nightmare {
            new_direction(h, id);
        }
        return;
    }
    if (h.perception.is_some_and(|p| p.monsters) && crate::monsters::select(h, id, work))
        || h.sight_failed()
    {
        return;
    }
    // netgame=false. Historical profile 9 has no attack metadata.
    let l = h.access.get_mut(id).unwrap().life.as_mut().unwrap();
    l.movecount -= 1;
    if l.movecount < 0 || !move_actor(h, id) {
        new_direction(h, id);
    }
    let sound = h.registry.actor_type(life.actor_type).unwrap().sounds()[4];
    if sound != 0 && draw(h, id, 3) < 3 {
        h.sound(Some(id), sound);
    }
}
