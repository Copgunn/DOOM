//! Source actor/player field reader with explicit initial RNG and birth projection.
//! Historical zero-origin reader remains frozen; this reader is role-independent.
#![allow(dead_code)]
use doom_sim::*;
use doom_types::DoomRng;
#[derive(Default)]
pub struct SourceTrace {
    births: Vec<ActorId>,
    rng: DoomRng,
    pub counts: [usize; 5],
}
impl SourceTrace {
    pub fn seeded(seed: u8) -> Self {
        Self {
            rng: DoomRng::from_indices(seed, 0),
            ..Self::default()
        }
    }
    pub fn birth(&self, id: Option<ActorId>) -> i64 {
        id.map_or(-1, |id| {
            self.births.iter().position(|b| *b == id).unwrap() as i64
        })
    }
    pub fn check(
        &mut self,
        sim: &Simulation,
        which: usize,
        op: usize,
        events: &[Event],
        observations: &[(String, Vec<i64>)],
    ) {
        for e in events {
            if let Event::Spawned(id) = e {
                self.births.push(*id);
            }
        }
        for (tag, c) in observations {
            match tag.as_str() {
                "E" if c[0] == 8 => {
                    assert_eq!(i64::from(self.rng.p_random()), c[2]);
                    self.counts[3] += 1;
                }
                "P" => {
                    let p = sim.original_player(PlayerSlot::One).unwrap();
                    let fields = [
                        self.birth(p.mo),
                        i64::from(p.health),
                        i64::from(p.state == PlayerBodyState::Dead),
                        self.birth(p.attacker),
                        i64::from(p.damage_count),
                        i64::from(p.frags),
                    ];
                    assert_eq!(fields.as_slice(), c, "player case{which} op{op}");
                    self.counts[1] += 6;
                }
                "AP" => {
                    let actor = self.births[c[0] as usize];
                    let p = sim.original_player(PlayerSlot::One).unwrap();
                    assert_eq!(if p.mo == Some(actor) { 0 } else { -1 }, c[1]);
                    self.counts[2] += 1;
                }
                "LS" => {
                    let s = sim.level_statistics().unwrap();
                    assert_eq!(
                        [
                            s.total_kills,
                            s.total_items,
                            s.total_secrets,
                            s.single_player_kill_count
                        ]
                        .map(i64::from)
                        .as_slice(),
                        c
                    );
                    self.counts[4] += 4;
                }
                "A" => {
                    let id = self.births[c[0] as usize];
                    if c[1] == 0 {
                        assert!(sim.actor(id).is_none());
                        continue;
                    }
                    let a = sim.actor(id).unwrap();
                    let l = a.life().unwrap();
                    let got = [
                        i64::from(l.actor_type.0),
                        i64::from(a.position()[0].bits()),
                        i64::from(a.position()[1].bits()),
                        i64::from(a.z().bits()),
                        i64::from(a.momentum()[0].bits()),
                        i64::from(a.momentum()[1].bits()),
                        i64::from(a.vertical_momentum().bits()),
                        i64::from(l.health),
                        l.state.map_or(-1, |s| i64::from(s.0)),
                        i64::from(l.tics),
                        i64::from(l.flags),
                        i64::from(a.height().bits()),
                        i64::from(a.radius().bits()),
                        i64::from(l.reaction_time),
                        i64::from(l.threshold),
                        self.birth(l.target),
                        i64::from(a.lastlook()),
                        i64::from(a.constraints()[0].bits()),
                        i64::from(a.constraints()[1].bits()),
                        i64::from(a.location().unwrap().subsector),
                        i64::from(sim.rng().indices()[0]),
                        i64::from(a.angle()),
                        i64::from(l.sprite),
                        i64::from(l.frame),
                        i64::from(l.movedir),
                        i64::from(l.movecount),
                    ];
                    for (field, (actual, expected)) in got.iter().zip(&c[2..]).enumerate() {
                        assert_eq!(
                            actual, expected,
                            "case{which} op{op} birth{} field{field}",
                            c[0]
                        );
                        self.counts[0] += 1;
                    }
                }
                _ => {}
            }
        }
        assert_eq!(
            sim.rng().indices(),
            self.rng.indices(),
            "RNG case{which} op{op}"
        );
    }
}

pub fn check_motion(
    sim: &Simulation,
    body: ActorId,
    snapshot: &Snapshot,
    rows: &[(String, Vec<i64>)],
) -> [usize; 2] {
    let mut counts = [0, 0];
    for (tag, expected) in rows {
        if tag == "PM" {
            let actor = sim.actor(body).unwrap();
            let life = actor.life().unwrap();
            let p = sim.original_player(PlayerSlot::One).unwrap();
            let got = [
                i64::from(actor.angle()),
                i64::from(actor.position()[0].bits()),
                i64::from(actor.position()[1].bits()),
                i64::from(actor.momentum()[0].bits()),
                i64::from(actor.momentum()[1].bits()),
                i64::from(actor.z().bits()),
                i64::from(actor.vertical_momentum().bits()),
                i64::from(life.reaction_time),
                life.state.map_or(-1, |s| i64::from(s.0)),
                i64::from(life.tics),
                i64::from(life.health),
                i64::from(p.health),
                i64::from(p.state == PlayerBodyState::Dead),
            ];
            assert_eq!(got.as_slice(), expected);
            counts[0] += got.len();
        } else if tag == "UC" {
            let field = |path| {
                snapshot
                    .fields()
                    .find(|f| f.path() == path)
                    .unwrap()
                    .value()
            };
            let got = [
                i64::from(i8::from_le_bytes(
                    field("player.0.forward").try_into().unwrap(),
                )),
                i64::from(i8::from_le_bytes(
                    field("player.0.side").try_into().unwrap(),
                )),
                i64::from(i16::from_le_bytes(
                    field("player.0.angle_turn").try_into().unwrap(),
                )),
                i64::from(u8::from_le_bytes(
                    field("player.0.buttons").try_into().unwrap(),
                )),
            ];
            assert_eq!(got.as_slice(), expected);
            counts[1] += got.len();
        }
    }
    counts
}
