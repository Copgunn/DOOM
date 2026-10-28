//! Original finite direction selection.
use super::*;
pub(crate) fn new_direction<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) {
    start(h, id, PursuitOperation::NewChaseDir);
    new_direction_body(h, id);
    end(h, id, PursuitOperation::NewChaseDir, false);
}
fn new_direction_body<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) {
    let a = *h.access.get(id).unwrap();
    let life = a.life.unwrap();
    let old = life.movedir;
    let opposite = MoveDirection::from_raw(old).unwrap().opposite() as u8;
    let target = h.access.get(life.target.unwrap()).unwrap();
    let dx = (target.position()[0] - a.position()[0]).bits();
    let dy = (target.position()[1] - a.position()[1]).bits();
    let mut axes = [
        if dx > 10 * 65536 {
            0
        } else if dx < -10 * 65536 {
            4
        } else {
            NODIR
        },
        if dy < -10 * 65536 {
            6
        } else if dy > 10 * 65536 {
            2
        } else {
            NODIR
        },
    ];
    if axes[0] != NODIR && axes[1] != NODIR {
        let d = [3, 1, 5, 7][usize::from(dy < 0) * 2 + usize::from(dx > 0)];
        direction(h, id, d);
        if d != opposite && try_walk(h, id) {
            return;
        }
    }
    if draw(h, id, 1) > 200 || dy.unsigned_abs() > dx.unsigned_abs() {
        axes.swap(0, 1);
    }
    for d in &mut axes {
        if *d == opposite {
            *d = NODIR;
        }
    }
    for d in axes {
        if d != NODIR {
            direction(h, id, d);
            if try_walk(h, id) {
                return;
            }
        }
    }
    if old != NODIR {
        direction(h, id, old);
        if try_walk(h, id) {
            return;
        }
    }
    let forward = draw(h, id, 2) & 1 != 0;
    for n in 0..8 {
        let d = if forward { n } else { 7 - n };
        if d != opposite {
            direction(h, id, d);
            if try_walk(h, id) {
                return;
            }
        }
    }
    if opposite != NODIR {
        direction(h, id, opposite);
        if try_walk(h, id) {
            return;
        }
    }
    direction(h, id, NODIR);
}
