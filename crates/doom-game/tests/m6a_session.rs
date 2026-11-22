//! M6a step 7: the session layer (`doom_game::session`) against the genuine engine, end to end.
//!
//! Every level-chain item of step 2 (the 18 of [`m6a::CHAIN_ITEMS`]: start maps 1, 2, 3 and 7,
//! registered) is run by the Rust session from its committed demo, the same session layer as
//! play (`Session::tic` per command: wire 26's levels, the intermission logic, the reborn, the
//! carried player, the session display with `DDSP` v3 and doom-sound's session with `DSND` v2),
//! at screenblocks 10 and 11, and compared tic for tic with the goldens of the genuine
//! linuxdoom-1.10 engine (`tools/reference/m6a_goldens.py`, `tools/reference/m6a_mixer.py`; no
//! Rust output feeds an expectation). Nothing is projected from a golden into the run: the only
//! input is the demo and the WADs.
//!
//! Compared per tic (the session rows are the genuine run's at screenblocks 10, compared in both
//! Rust runs):
//!
//! - **G** (session row): `gamestate`, `gameaction`, `map`, `leveltime`, `gametic`,
//!   `levelstarttic`, both random indices, the cumulative `M_Random` draw buckets (sound, status
//!   bar, wipe, intermission; the finale's and "other" are 0), `wipegamestate` (after the display
//!   it is `gamestate`, d_main.c:300), `secretexit`, the loads and the wipes;
//! - **C** (the console player): the simulation's player fields in a level; outside a level the
//!   finished level's player as `G_PlayerFinishLevel` leaves it (powers, cards, `damagecount`,
//!   `bonuscount` 0, g_game.c:779-792) with the latches the intermission rewrote; `viewheight`
//!   and `deltaviewheight` from the display's view in a level (`P_SpawnPlayer` resets
//!   `viewheight` only, p_mobj.c:683) and as carried outside it; `didsecret` from the session;
//! - **I** and **J** (`wminfo` and every wi_stuff.c static the single-player path writes,
//!   the ten animations included) on every intermission tic;
//! - **frames** at screenblocks 10 and 11: every level frame (whole digest, face-zeroed digest,
//!   renderer counters, `fuzzpos`, newly mapped lines, palette, palette set, face, `message_on`,
//!   `M_Random` index, cumulative sound and wipe draws) and every intermission frame (whole
//!   digest, palette, palette set, `M_Random` index), for every level of the item;
//! - **sound** (screenblocks 10 run): every S, T, K, L, C and R row of the genuine stream, actor
//!   origins matched to the genuine thinker serials by a bijection over the item (each level's
//!   handles tagged with the level's ordinal, since a handle is reused across loads), the
//!   `S_Start` stops of a load first; and **every mixer buffer**: the session's starts and buffers
//!   written as the genuine mixer driver writes them, drained after the last tic as it drains,
//!   row for row against `tests/reference/m6a_mixer/goldens`;
//! - **the end**: the demo's end, or the genuine stop (`I_Error`) as a typed refusal at the same
//!   tic with the session unchanged.
//!
//! Not compared here (no column in the goldens, or no counterpart): the per-level whole-state
//! dumps (the goldens have none, step 4's report), the HUD message texts beyond `message_on`
//! and the pixels (X rows), `levelstarttic`'s use (none in the C), the melt pictures (not built).
mod m6a;
#[path = "../../doom-sound/tests/common/rows.rs"]
mod rows;

use doom_game::session::demo::SessionDemo;
use doom_game::session::{
    PlantedFault, Refusal, Session, SessionMode, SessionSettings, SessionWitness, StartError,
    TicKind, TicReport,
};
use doom_render::{GameState, SCREENHEIGHT, SCREENWIDTH};
use doom_sim::{ActorId, PlayerBodyState, PlayerSlot, Simulation};
use doom_sound::chain::ChainRefusal;
use doom_sound::logic::{SoundOrigin, SoundRecord};
use doom_sound::mixer::buffer_bytes;
use doom_test_evidence::sha256;
use m6a::{GenuineSound, Item, Rows, int, read_frames, read_sound};
use rows::{Carry, GoldenWriter, drain, stream_rows_with};
use std::collections::BTreeMap;

/// What one run compared.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counts {
    tics: usize,
    level_tics: usize,
    intermission_tics: usize,
    loads: usize,
    wipes: usize,
    g_rows: usize,
    c_rows: usize,
    wi_rows: usize,
    level_frames: usize,
    other_frames: usize,
    sound_rows: usize,
    s_listener_unchecked: usize,
    buffers: u64,
    mixer_rows: usize,
    masked_frames: usize,
    uncomparable_frames: usize,
}

impl Counts {
    fn add(&mut self, o: &Counts) {
        self.tics += o.tics;
        self.level_tics += o.level_tics;
        self.intermission_tics += o.intermission_tics;
        self.loads += o.loads;
        self.wipes += o.wipes;
        self.g_rows += o.g_rows;
        self.c_rows += o.c_rows;
        self.wi_rows += o.wi_rows;
        self.level_frames += o.level_frames;
        self.other_frames += o.other_frames;
        self.sound_rows += o.sound_rows;
        self.s_listener_unchecked += o.s_listener_unchecked;
        self.buffers += o.buffers;
        self.mixer_rows += o.mixer_rows;
        self.masked_frames += o.masked_frames;
        self.uncomparable_frames += o.uncomparable_frames;
    }
}

/// The outcome of one item at one screen size: the counts, or the first difference.
struct Outcome {
    counts: Counts,
    first_difference: Option<String>,
    end: Option<Refusal>,
}

/// The simulation's values of the compared `C` columns of a level tic (the same reading as
/// doom-sim's tests/wire26_chain.rs), by column name.
fn player_columns(s: &Simulation, intermission: bool) -> BTreeMap<&'static str, i64> {
    let p = s.original_player(PlayerSlot::One).expect("player");
    let w = s
        .original_player_weapons(PlayerSlot::One)
        .expect("weapons")
        .expect("pack");
    let armor = s.original_player_armor(PlayerSlot::One).expect("armor");
    let powers = s.original_player_powers(PlayerSlot::One).expect("powers");
    let pickups = s.original_player_pickups(PlayerSlot::One).expect("pickups");
    let finish = s.level_finish().expect("finish");
    let stats = s.level_statistics().expect("statistics");
    let state = match (p.state, finish.reborn) {
        (PlayerBodyState::Live, _) => 0,
        (PlayerBodyState::Dead, false) => 1,
        (PlayerBodyState::Dead, true) => 2,
    };
    let mut m = BTreeMap::new();
    m.insert("playerstate", state);
    m.insert("health", i64::from(p.health));
    m.insert("armorpoints", i64::from(armor.points));
    m.insert("armortype", i64::from(armor.kind.wire_id()));
    m.insert("readyweapon", w.ready as i64);
    m.insert("pendingweapon", w.pending.map_or(10, |x| x as i64));
    m.insert("weaponowned", i64::from(w.owned));
    m.insert("ammo0", i64::from(w.bullets));
    m.insert("ammo1", i64::from(w.shells));
    m.insert("ammo2", i64::from(w.cells));
    m.insert("ammo3", i64::from(w.rockets));
    let max = [200_i64, 50, 300, 50].map(|v| v << u32::from(w.backpack));
    for (k, v) in max.iter().enumerate() {
        m.insert(["maxammo0", "maxammo1", "maxammo2", "maxammo3"][k], *v);
    }
    m.insert("backpack", i64::from(w.backpack));
    // G_PlayerFinishLevel (g_game.c:785-791) clears cards, powers and both palette counts.
    let keep = i64::from(!intermission);
    let cards: i64 = powers
        .cards
        .iter()
        .enumerate()
        .map(|(k, c)| i64::from(*c) << k)
        .sum();
    m.insert("cards", cards * keep);
    for (k, v) in powers.powers.iter().enumerate() {
        m.insert(
            ["pw0", "pw1", "pw2", "pw3", "pw4", "pw5"][k],
            i64::from(*v) * keep,
        );
    }
    m.insert("cheats", 0);
    m.insert("killcount", i64::from(stats.single_player_kill_count));
    m.insert("itemcount", i64::from(pickups.item_count));
    m.insert(
        "secretcount",
        i64::from(s.original_player_secret_count().expect("secrets")),
    );
    m.insert("refire", i64::from(w.refire));
    m.insert("damagecount", i64::from(p.damage_count) * keep);
    m.insert("bonuscount", i64::from(pickups.bonus_count) * keep);
    if !intermission {
        m.insert("attackdown", i64::from(w.attack_down));
        // A living player's `usedown` is what it carries; a dead one's is read from the capture
        // (`P_PlayerThink` returns through `P_DeathThink` before its use block, p_user.c:249-262).
        let usedown = match finish.carried {
            Some(c) => c.use_down,
            None => s
                .snapshot()
                .fields()
                .find(|f| f.path() == "original_player.0.use_down")
                .is_some_and(|f| f.value() == [1]),
        };
        m.insert("usedown", i64::from(usedown));
    }
    m
}

/// For an S row whose Rust form has no placement and whose genuine form has no origin position,
/// the genuine row with its three listener columns replaced by `-` (doom-sound's session gate:
/// the record carries no listener for an unplaced start, and nothing reads it, s_sound.c:304-324).
fn unplaced_listener(rust: &str, genuine: &str) -> Option<String> {
    let r: Vec<&str> = rust.split(',').collect();
    let g: Vec<&str> = genuine.split(',').collect();
    if r.first() != Some(&"S") || g.first() != Some(&"S") || r.len() != 22 || g.len() != 22 {
        return None;
    }
    if r[17..22].iter().any(|f| *f != "-") || g[17] != "-" || g[18] != "-" {
        return None;
    }
    let mut out: Vec<&str> = g[..19].to_vec();
    out.extend(["-", "-", "-"]);
    Some(out.join(","))
}

/// The bijection of Rust actor handles (`A<level>/<slot>:<generation>`) and genuine serials.
#[derive(Default)]
struct Serials {
    to_rust: BTreeMap<String, String>,
    to_c: BTreeMap<String, String>,
}

impl Serials {
    fn compare(&mut self, rust: &str, genuine: &str) -> Result<(), String> {
        let r: Vec<&str> = rust.split(',').collect();
        let g: Vec<&str> = genuine.split(',').collect();
        if r.len() != g.len() {
            return Err("field count".into());
        }
        for (i, (a, b)) in r.iter().zip(&g).enumerate() {
            if let Some(h) = a.strip_prefix('A') {
                let (h, s) = (h.to_string(), b.to_string());
                match (self.to_c.get(&h), self.to_rust.get(&s)) {
                    (None, None) => {
                        self.to_c.insert(h.clone(), s.clone());
                        self.to_rust.insert(s, h);
                    }
                    (Some(c), Some(r)) if *c == s && *r == h => {}
                    _ => return Err(format!("field {i}: handle {h} against serial {s}")),
                }
            } else if a != b {
                return Err(format!("field {i}"));
            }
        }
        Ok(())
    }
}

/// Tags every actor handle of `rows` with the level ordinal `n` (`A<n>/<slot>:<generation>`).
fn tag(rows: Vec<String>, n: usize) -> Vec<String> {
    rows.into_iter()
        .map(|r| {
            r.split(',')
                .map(|f| match f.strip_prefix('A') {
                    Some(h) => format!("A{n}/{h}"),
                    None => f.to_string(),
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect()
}

/// The sound comparison of one run: the genuine stream, the row state and the mixer writer.
struct SoundGate {
    genuine: GenuineSound,
    serials: Serials,
    carry: Carry,
    golden: GoldenWriter,
}

/// The run of one item at one screen size, with an optional planted fault; stops at the first
/// difference.
struct Run<'a> {
    item: &'a Item,
    sb: u8,
    session_rows: Rows,
    frames: BTreeMap<i64, m6a::GoldenFrame>,
    sound: Option<SoundGate>,
    counts: Counts,
    level_ordinal: usize,
    st_draws: i64,
    wipe_draws: i64,
    snd_draws: i64,
    wi_draws: i64,
    listener: Option<ActorId>,
}

fn diff(item: &str, sb: u8, tic: i64, what: impl std::fmt::Display) -> String {
    format!("{item} sb{sb} tic {tic}: {what}")
}

impl Run<'_> {
    fn g_row(&mut self, s: &Session, tic: i64) -> Result<(), String> {
        let g = self.session_rows.at("G", tic).ok_or_else(|| {
            diff(
                &self.item.name,
                self.sb,
                tic,
                "a tic the genuine run does not have",
            )
        })?;
        let gamestate = match s.gamestate() {
            GameState::Level => 0,
            GameState::Intermission => 1,
            GameState::Finale => 2,
        };
        let got: [(&str, i64); 19] = [
            ("gamestate", gamestate),
            ("gameaction", i64::from(s.gameaction())),
            ("episode", 1),
            ("map", i64::from(s.map())),
            ("leveltime", s.level().tic() as i64),
            ("gametic", s.gametic() as i64),
            ("levelstarttic", s.levelstarttic() as i64),
            ("prndindex", i64::from(s.level().rng().indices()[0])),
            ("rndindex", i64::from(s.display().random_index())),
            ("snd_draws", self.snd_draws),
            ("st_draws", self.st_draws),
            ("wipe_draws", self.wipe_draws),
            ("wi_draws", self.wi_draws),
            ("fin_draws", 0),
            ("other_draws", 0),
            ("wipegamestate", gamestate),
            ("secretexit", i64::from(s.secretexit())),
            ("loads", self.counts.loads as i64 + 1),
            ("wipes", self.counts.wipes as i64),
        ];
        for (name, v) in got {
            if int(g, name) != v {
                return Err(diff(
                    &self.item.name,
                    self.sb,
                    tic,
                    format!("G {name} {v} genuine {}", g[name]),
                ));
            }
        }
        self.counts.g_rows += 1;
        Ok(())
    }

    fn c_row(&mut self, s: &Session, tic: i64) -> Result<(), String> {
        let Some(c) = self.session_rows.at("C", tic) else {
            return Ok(());
        };
        let outside = s.gamestate() != GameState::Level;
        let mut m = player_columns(s.level(), outside);
        if outside {
            let carried = s.carried().expect("a carried player outside a level");
            m.insert("attackdown", i64::from(carried.latches.attackdown));
            m.insert("usedown", i64::from(carried.latches.usedown));
            m.insert("viewheight", i64::from(carried.viewheight));
            m.insert("deltaviewheight", i64::from(carried.deltaviewheight));
        } else {
            let v = s.display().level_view().expect("a level view");
            m.insert("viewheight", i64::from(v.viewheight));
            m.insert("deltaviewheight", i64::from(v.deltaviewheight));
        }
        m.insert("didsecret", i64::from(s.didsecret()));
        for (name, v) in m {
            if int(c, name) != v {
                return Err(diff(
                    &self.item.name,
                    self.sb,
                    tic,
                    format!("C {name} {v} genuine {}", c[name]),
                ));
            }
        }
        self.counts.c_rows += 1;
        Ok(())
    }

    fn wi_rows(&mut self, s: &Session, tic: i64) -> Result<(), String> {
        let (Some(i), Some(j)) = (
            self.session_rows.at("I", tic),
            self.session_rows.at("J", tic),
        ) else {
            if s.gamestate() == GameState::Intermission {
                return Err(diff(
                    &self.item.name,
                    self.sb,
                    tic,
                    "no genuine I or J row in the intermission",
                ));
            }
            return Ok(());
        };
        let w = s.wminfo();
        let wi = s.intermission();
        let got: [(&str, i64, &BTreeMap<String, String>); 30] = [
            ("epsd", i64::from(w.epsd), i),
            ("didsecret", i64::from(w.didsecret), i),
            ("last", i64::from(w.last), i),
            ("next", i64::from(w.next), i),
            ("maxkills", i64::from(w.maxkills), i),
            ("maxitems", i64::from(w.maxitems), i),
            ("maxsecret", i64::from(w.maxsecret), i),
            ("maxfrags", 0, i),
            ("partime", i64::from(w.partime), i),
            ("pnum", 0, i),
            ("in", 1, i),
            ("skills", i64::from(w.plyr.skills), i),
            ("sitems", i64::from(w.plyr.sitems), i),
            ("ssecret", i64::from(w.plyr.ssecret), i),
            ("stime", i64::from(w.plyr.stime), i),
            ("score", 0, i),
            ("state", i64::from(wi.state), j),
            ("acceleratestage", i64::from(wi.acceleratestage), j),
            ("me", 0, j),
            ("cnt", i64::from(wi.cnt), j),
            ("bcnt", i64::from(wi.bcnt), j),
            ("firstrefresh", i64::from(wi.firstrefresh), j),
            ("sp_state", i64::from(wi.sp_state), j),
            ("cnt_kills", i64::from(wi.cnt_kills), j),
            ("cnt_items", i64::from(wi.cnt_items), j),
            ("cnt_secret", i64::from(wi.cnt_secret), j),
            ("cnt_time", i64::from(wi.cnt_time), j),
            ("cnt_par", i64::from(wi.cnt_par), j),
            ("cnt_pause", i64::from(wi.cnt_pause), j),
            ("snl_pointeron", i64::from(wi.snl_pointeron), j),
        ];
        for (name, v, row) in got {
            if int(row, name) != v {
                return Err(diff(
                    &self.item.name,
                    self.sb,
                    tic,
                    format!("{name} {v} genuine {}", row[name]),
                ));
            }
        }
        let ctr: Vec<String> = wi.anims.iter().map(|a| a.ctr.to_string()).collect();
        let next: Vec<String> = wi.anims.iter().map(|a| a.nexttic.to_string()).collect();
        if j["anim_ctr"] != ctr.join(" ")
            || j["anim_nexttic"] != next.join(" ")
            || j["nanims"] != "10"
            || j["wbs_is_wminfo"] != "1"
        {
            return Err(diff(
                &self.item.name,
                self.sb,
                tic,
                format!(
                    "animations {ctr:?} {next:?} genuine {} / {}",
                    j["anim_ctr"], j["anim_nexttic"]
                ),
            ));
        }
        self.counts.wi_rows += 2;
        Ok(())
    }

    fn frame(&mut self, s: &Session, r: &TicReport, tic: i64) -> Result<(), String> {
        let g = self.frames.get(&tic).ok_or_else(|| {
            diff(
                &self.item.name,
                self.sb,
                tic,
                "a frame the genuine run does not have",
            )
        })?;
        let px = s.frame().pixels();
        let d = |w: String| diff(&self.item.name, self.sb, tic, w);
        match &r.level_frame {
            Some(f) => {
                if g.gamestate != 0 || g.f.is_empty() {
                    return Err(d("a level frame where the genuine frame is not one".into()));
                }
                let q = g.q;
                // Real maps only (the synthetic goldens have neither, asserted by read_frames,
                // and their frames keep the comparison below unchanged): a frame whose switch
                // parity the Rust renderer reports unknown, or that the genuine runner excludes
                // whole, is counted and not compared by digest; a frame with tracker flagged
                // columns (undefined reads) on either side is compared by its masked digest, as
                // tools/local/m5c_real_full.py compares them (decision R4).
                let real = m6a::real_maps();
                if real && (f.view.switch_parity_unknown || g.whole_excluded) {
                    self.counts.uncomparable_frames += 1;
                } else if real && (!g.flagged.is_empty() || !f.view.flagged_columns.is_empty()) {
                    let mut m = px.to_vec();
                    for x in &f.view.flagged_columns {
                        for y in 0..SCREENHEIGHT {
                            m[y * SCREENWIDTH + usize::from(*x)] = 0;
                        }
                    }
                    if g.sha256 != "undefined" || sha256(&m) != g.masked {
                        return Err(d("masked digest or flag differs".into()));
                    }
                    self.counts.masked_frames += 1;
                } else {
                    if sha256(px) != g.sha256 {
                        return Err(d("level frame differs".into()));
                    }
                    let mut nf = px.to_vec();
                    for y in q[3] as usize..(q[5] as usize).min(SCREENHEIGHT) {
                        for x in q[2] as usize..(q[4] as usize).min(SCREENWIDTH) {
                            nf[y * SCREENWIDTH + x] = 0;
                        }
                    }
                    if sha256(&nf) != g.noface {
                        return Err(d("face-zeroed digest differs".into()));
                    }
                }
                // Real maps: the flagged columns themselves, in every frame (m5c_real_full.py
                // compares the U rows of every frame).
                if real && f.view.flagged_columns != g.flagged {
                    return Err(d(format!(
                        "flagged columns {:?} genuine {:?}",
                        f.view.flagged_columns, g.flagged
                    )));
                }
                if [q[0], q[1]] != [self.snd_draws, self.wipe_draws] {
                    return Err(d(format!(
                        "Q draws {:?} genuine {:?}",
                        [self.snd_draws, self.wipe_draws],
                        [q[0], q[1]]
                    )));
                }
                let c = f.view.counters;
                let got = vec![
                    i64::from(f.view.fuzzpos),
                    i64::from(c.mapped),
                    i64::from(c.visplanes),
                    i64::from(c.drawsegs),
                    i64::from(c.vissprites),
                    i64::from(c.openings),
                    i64::from(c.solidsegs_peak),
                    i64::from(c.drawseg_drops),
                    i64::from(c.vissprite_drop),
                ];
                let unexpected =
                    !real && (f.view.switch_parity_unknown || !f.view.flagged_columns.is_empty());
                if got != g.f || f.view.newly_mapped != g.mapped || unexpected {
                    return Err(d(format!("F {got:?} genuine {:?}", g.f)));
                }
                let status = vec![
                    i64::from(f.palette),
                    f.palette_set.map_or(-1, i64::from),
                    i64::from(f.face),
                    i64::from(f.message_on),
                    i64::from(s.display().random_index()),
                ];
                if status != g.status {
                    return Err(d(format!("P {status:?} genuine {:?}", g.status)));
                }
                self.counts.level_frames += 1;
            }
            None => {
                if g.gamestate == 0 {
                    return Err(d(
                        "a frame outside a level where the genuine frame is a level frame".into(),
                    ));
                }
                if sha256(px) != g.sha256 {
                    return Err(d("intermission frame differs".into()));
                }
                let status = vec![
                    i64::from(r.palette),
                    r.palette_set.map_or(-1, i64::from),
                    i64::from(s.display().random_index()),
                ];
                if status != g.status {
                    return Err(d(format!("G {status:?} genuine {:?}", g.status)));
                }
                self.counts.other_frames += 1;
            }
        }
        Ok(())
    }

    /// The sound rows of the tic and the mixer rows (screenblocks 10 only).
    fn sound(
        &mut self,
        s: &Session,
        r: &TicReport,
        tic: i64,
        old_listener: Option<ActorId>,
        new_level: bool,
    ) -> Result<(), String> {
        let level = r.level_frame.is_some();
        let listener = s.level().sound_listener().map(|l| l.actor);
        let Some(gate) = self.sound.as_mut() else {
            return Ok(());
        };
        let records: Vec<SoundRecord> = if level {
            s.level().sound_records().to_vec()
        } else {
            r.wi_starts
                .iter()
                .map(|w| SoundRecord::Start {
                    sfx: w.sfx(),
                    origin: SoundOrigin::None,
                    at: None,
                })
                .collect()
        };
        let first_seq = gate.carry.seq;
        let after_p = r.sound.sound.rndindex;
        let after_display = s.display().random_index();
        let rows = stream_rows_with(
            &r.sound.sound,
            &records,
            s.sound().sound(),
            listener,
            &mut gate.carry,
            after_p.wrapping_add(1),
            after_display,
        );
        let origin = |o: SoundOrigin| match o {
            SoundOrigin::None => "none,-1".to_string(),
            SoundOrigin::Actor(a) => format!(
                "{},A{}:{}",
                if Some(a) == old_listener {
                    "player"
                } else {
                    "mobj"
                },
                a.slot(),
                a.generation()
            ),
            SoundOrigin::Sector(x) => format!("sector,{x}"),
            SoundOrigin::Button(b) => format!("button,{b}"),
        };
        let k_rows: Vec<String> = r
            .sound
            .level_start
            .iter()
            .map(|k| {
                format!(
                    "K,{tic},0,{},{},{},{},level_start,{}",
                    k.cnum,
                    k.entry.sfx.get(),
                    origin(k.entry.origin),
                    k.entry.handle,
                    u8::from(k.playing)
                )
            })
            .collect();
        let before = if new_level {
            self.level_ordinal - 1
        } else {
            self.level_ordinal
        };
        let mut rows = tag(rows, self.level_ordinal);
        rows.splice(0..0, tag(k_rows, before));
        let last = rows.last_mut().expect("an R row");
        let mut f: Vec<String> = last.split(',').map(str::to_string).collect();
        f[6] = self.st_draws.to_string();
        f[7] = self.wipe_draws.to_string();
        if !level {
            f[2] = "-1".into();
            f[3] = "-1".into();
        }
        *last = f.join(",");
        let genuine = gate.genuine.by_tic.get(&tic).cloned().unwrap_or_default();
        for i in 0..rows.len().max(genuine.len()) {
            let rr = rows.get(i).map_or("<none>", String::as_str);
            let gg = genuine.get(i).map_or("<none>", String::as_str);
            let patched;
            let gg = match unplaced_listener(rr, gg) {
                Some(p) => {
                    self.counts.s_listener_unchecked += 1;
                    patched = p;
                    patched.as_str()
                }
                None => gg,
            };
            gate.serials.compare(rr, gg).map_err(|e| {
                diff(
                    &self.item.name,
                    self.sb,
                    tic,
                    format!("sound row {i} ({e}):\n  genuine: {gg}\n  rust:    {rr}"),
                )
            })?;
            self.counts.sound_rows += 1;
        }
        gate.golden.starts(&r.sound.sound, first_seq);
        if let Some(b) = &r.sound.sound.buffer {
            gate.golden
                .buffer(r.sound.sound.tic, b, s.sound().sound().mixer().mask());
            let _ = buffer_bytes(b);
        }
        Ok(())
    }
}

/// The session settings of an item.
fn settings(item: &Item, sb: u8) -> SessionSettings {
    SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: sb,
    }
}

/// The genuine stop of an item as the session's refusal: (the refused session tic, whether the
/// witness is the expected missing music).
fn expected_stop(item: &Item, end: &Refusal) -> bool {
    if item.end == "victory" {
        // A real-map run cut at the finale's first tic (tools/local/m6a_session_real.py): the
        // session's typed refusal there (the finale is built after the hand test).
        return end.tic == item.session_tics as u64
            && matches!(
                end.witness,
                SessionWitness::NotYet(doom_game::session::NotYet::Victory)
            );
    }
    let music = item
        .end
        .strip_prefix("error:W_GetNumForName: ")
        .and_then(|m| m.strip_suffix(" not found!"));
    let Some(music) = music else { return false };
    end.tic == item.session_tics as u64
        && matches!(&end.witness, SessionWitness::Sound(doom_game::session::sound::SoundWitness::Chain(ChainRefusal::MusicMissing { music: m, tic: Some(t) })) if m.lump_name() == music && *t == end.tic)
}

/// Runs one item at screenblocks `sb` from its demo, with an optional planted fault.
fn run_item(item: &Item, sb: u8, fault: Option<PlantedFault>) -> Outcome {
    run_item_in(item, sb, fault, None)
}

/// [`run_item`] in the game mode `mode` (None: the item's own).
fn run_item_in(
    item: &Item,
    sb: u8,
    fault: Option<PlantedFault>,
    mode: Option<SessionMode>,
) -> Outcome {
    let data = m6a::data(item, mode);
    if let Some(m) = mode {
        assert_eq!(data.mode().mode, m);
    }
    let lmp = std::fs::read(m6a::root(&item.lmp)).expect("demo");
    let demo = SessionDemo::parse(&lmp).expect("session demo");
    assert_eq!((demo.map, demo.skill), (item.map, item.skill));
    let mut session = match Session::new(data.clone(), settings(item, sb)) {
        Ok(s) => s,
        Err(StartError::Sound(w)) => {
            // NOE1M2AT: the first map's music at the session's load, before any tic.
            let ok = item.session_tics == 0
                && matches!(&w, doom_game::session::sound::SoundWitness::Chain(ChainRefusal::MusicMissing { music, tic: None }) if item.end.contains(&music.lump_name()));
            return Outcome {
                counts: Counts::default(),
                first_difference: (!ok).then(|| format!("{}: start refused {w:?}", item.name)),
                end: None,
            };
        }
        Err(e) => panic!("{}: start {e:?}", item.name),
    };
    if let Some(f) = fault {
        session.plant_fault(f);
    }
    let (frames, _) = read_frames(&item.name, sb);
    let sound = (sb == 10).then(|| SoundGate {
        genuine: read_sound(&item.name),
        serials: Serials::default(),
        carry: Carry::default(),
        golden: GoldenWriter::new(session.sound().sound().mixer().sound_set()),
    });
    let mut run = Run {
        item,
        sb,
        session_rows: Rows::read(&format!("m6a_session/{}.session", item.name)),
        frames,
        sound,
        counts: Counts::default(),
        level_ordinal: 0,
        st_draws: 0,
        wipe_draws: 0,
        snd_draws: 0,
        wi_draws: 0,
        listener: session.level().sound_listener().map(|l| l.actor),
    };
    let mut end = None;
    let mut first_difference = None;
    for (t, &cmd) in demo.commands.iter().enumerate() {
        let tic = t as i64;
        let before = session.gametic();
        match session.tic(cmd) {
            Ok(r) => {
                let old_listener = run.listener;
                let new_level = matches!(r.kind, TicKind::Load { .. });
                if new_level {
                    run.counts.loads += 1;
                    run.level_ordinal += 1;
                }
                run.counts.tics += 1;
                if r.level_frame.is_some() {
                    run.counts.level_tics += 1;
                    run.st_draws += 1;
                } else {
                    run.counts.intermission_tics += 1;
                }
                if r.wiped {
                    run.counts.wipes += 1;
                    run.wipe_draws += 320;
                }
                run.snd_draws += i64::from(r.sound.sound.draws);
                run.wi_draws += i64::from(r.wi_draws);
                run.listener = session.level().sound_listener().map(|l| l.actor);
                let checks = run
                    .g_row(&session, tic)
                    .and_then(|()| run.c_row(&session, tic))
                    .and_then(|()| run.wi_rows(&session, tic))
                    .and_then(|()| run.frame(&session, &r, tic))
                    .and_then(|()| run.sound(&session, &r, tic, old_listener, new_level));
                if let Err(e) = checks {
                    first_difference = Some(e);
                    break;
                }
            }
            Err(e) => {
                assert_eq!(
                    session.gametic(),
                    before,
                    "{}: a refusal keeps the session",
                    item.name
                );
                end = Some(e);
                break;
            }
        }
    }
    if first_difference.is_none() {
        let genuine_end_ok = match &end {
            None => item.end == "marker" && run.counts.tics == item.session_tics,
            Some(e) => expected_stop(item, e),
        };
        if !genuine_end_ok {
            first_difference = Some(format!(
                "{} sb{sb}: ended {end:?} after {} tics; the genuine run: {} after {} tics",
                item.name, run.counts.tics, item.end, item.session_tics
            ));
        }
    }
    if first_difference.is_none() {
        if let Some(mut gate) = run.sound.take() {
            drain(session.sound().sound(), &mut gate.golden);
            run.counts.buffers = gate.golden.buffers();
            let got = gate.golden.finish();
            let path = format!("m6a_mixer/goldens/{}.golden", item.name);
            let want: Vec<String> = std::fs::read_to_string(m6a::reference(&path))
                .expect("mixer golden")
                .lines()
                .filter(|l| !l.starts_with("d ") && !l.starts_with("q "))
                .map(str::to_string)
                .collect();
            let n = want.len().max(got.len());
            if let Some(i) = (0..n).find(|i| want.get(*i) != got.get(*i)) {
                first_difference = Some(format!(
                    "{} mixer row {i}:\n  golden: {:?}\n  rust:   {:?}",
                    item.name,
                    want.get(i),
                    got.get(i)
                ));
            }
            run.counts.mixer_rows = want.len();
        }
    }
    Outcome {
        counts: run.counts,
        first_difference,
        end,
    }
}

fn chain_items() -> Vec<Item> {
    m6a::CHAIN_ITEMS.iter().map(|n| m6a::item(n)).collect()
}

/// The level-chain gate: every one of the 18 items at both screen sizes, everything above equal.
#[test]
fn the_level_chain_against_the_genuine_goldens() {
    let mut total = Counts::default();
    let mut failures = Vec::new();
    for item in chain_items() {
        for sb in [10, 11] {
            let o = run_item(&item, sb, None);
            eprintln!(
                "{} sb{sb}: {:?} end {:?}",
                item.name,
                o.counts,
                o.end.as_ref().map(ToString::to_string)
            );
            if let Some(d) = o.first_difference {
                failures.push(d);
            }
            total.add(&o.counts);
        }
    }
    eprintln!("level chain, 18 items x 2 screen sizes: {total:?}");
    assert!(
        failures.is_empty(),
        "{} differences:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The comparison can fail: each planted fault of the session is caught, at the first place it
/// shows. A carried start one point of health short (CHAIN: the C row of the first carried load),
/// the exit checked before the reborn (USEAT131: tic 132 completes the level instead of reloading
/// it), the tenth animation draw of `WI_Start` skipped (INTFAST: the `M_Random` index of the
/// first intermission tic), and the tally's kill count one too high (INTSTA: the J row).
#[test]
fn the_comparisons_can_fail() {
    let cases = [
        ("CHAIN", PlantedFault::CarriedHealth, "tic 142: C health"),
        (
            "USEAT131",
            PlantedFault::ExitBeforeReborn,
            "tic 132: G gamestate 1 genuine 0",
        ),
        (
            "INTFAST",
            PlantedFault::SkippedAnimationDraw,
            "tic 56: G rndindex",
        ),
        ("INTSTA", PlantedFault::TallyOffByOne, ": cnt_kills"),
    ];
    for (name, fault, at) in cases {
        let o = run_item(&m6a::item(name), 10, Some(fault));
        let d = o
            .first_difference
            .unwrap_or_else(|| panic!("{name}: {fault:?} not caught"));
        eprintln!("{name} {fault:?}: {}", d.lines().next().unwrap_or(""));
        assert!(d.contains(at), "{name} {fault:?}: caught elsewhere: {d}");
    }
}

/// Local only (never in CI): every run of the real-map goldens that
/// `tools/local/m6a_session_real.py` wrote from the genuine engine on a scratch copy of the
/// owner's IWAD, at screenblocks 10 and 11, through this same gate. Run with
/// `M6A_REFERENCE_DIR=<its --out> cargo test --release -p doom-game --test m6a_session --
/// --ignored real_maps`; without the variable it does nothing.
#[test]
#[ignore = "local only: needs M6A_REFERENCE_DIR from tools/local/m6a_session_real.py"]
fn real_maps_session_local() {
    if !m6a::real_maps() {
        eprintln!("skip: M6A_REFERENCE_DIR is not set");
        return;
    }
    let mut total = Counts::default();
    let mut failures = Vec::new();
    let items = m6a::real_items();
    for item in &items {
        for sb in [10, 11] {
            let o = run_item(item, sb, None);
            eprintln!(
                "{} sb{sb}: {:?} end {:?}{}",
                item.name,
                o.counts,
                o.end.as_ref().map(ToString::to_string),
                o.first_difference
                    .as_deref()
                    .map_or(String::new(), |d| format!(" DIFFERENCE {d}"))
            );
            if let Some(d) = o.first_difference {
                failures.push(d);
            }
            total.add(&o.counts);
        }
    }
    eprintln!(
        "real maps, {} runs x 2 screen sizes: {total:?}",
        items.len()
    );
    assert!(
        failures.is_empty(),
        "{} differences:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Local only (never in CI): the real-map gate can fail. A carried start one point of health
/// short (`PlantedFault::CarriedHealth`) is caught, at screenblocks 10, in every real run whose
/// state golden has a carried start, and there is at least one. Run as
/// [`real_maps_session_local`]; `M6A_REAL_ONLY` narrows it.
#[test]
#[ignore = "local only: needs M6A_REFERENCE_DIR from tools/local/m6a_session_real.py"]
fn real_maps_session_local_catches_a_planted_fault() {
    if !m6a::real_maps() {
        eprintln!("skip: M6A_REFERENCE_DIR is not set");
        return;
    }
    let carried = m6a::carried_items();
    let mut caught = 0;
    let mut missed = Vec::new();
    for item in m6a::real_items() {
        if !carried.contains(&item.name) {
            continue;
        }
        match run_item(&item, 10, Some(PlantedFault::CarriedHealth)).first_difference {
            Some(d) => {
                eprintln!("{} CarriedHealth: {d}", item.name);
                caught += 1;
            }
            None => missed.push(item.name.clone()),
        }
    }
    eprintln!("real maps, CarriedHealth: caught in {caught} runs, missed in {missed:?}");
    assert!(caught > 0, "no real run with a carried start");
    assert!(missed.is_empty(), "not caught: {missed:?}");
}

/// A carried start's `bob` one unit off passes every comparison of this gate (rows, frames at
/// screenblocks 10, sound rows and the mixer): `P_CalcHeight` rewrites the player's `bob` in tic
/// 0 (p_user.c:83-87), so only the `DSIM` 42 start group keeps it, and `tests/m6a_state.rs`
/// (`the_state_comparison_catches_what_the_rows_miss`) catches it there. This is the step-8
/// proof that the whole-state comparison sees what the rows cannot.
#[test]
fn a_carried_bob_fault_passes_the_row_gate() {
    let o = run_item(&m6a::item("CHAIN"), 10, Some(PlantedFault::CarriedBob));
    assert_eq!(o.first_difference, None);
    assert_eq!((o.counts.tics, o.counts.loads), (831, 7));
}

/// CHAIN in retail (ruling U4): the genuine retail run of CHAIN is identical to the registered
/// goldens apart from the mode lines (`tests/reference/m6a_results.json`, `other_mode`:
/// `identical_but_the_mode_lines`), so the Rust session forced retail is compared with the same
/// goldens, at both screen sizes. Nothing relies on retail beyond this comparison until the
/// owner's condition on the 61 real runs is met.
#[test]
fn chain_in_retail_against_the_genuine_goldens() {
    let item = m6a::item("CHAIN");
    let results: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(m6a::root("tests/reference/m6a_results.json")).expect("results"),
    )
    .expect("JSON");
    let other = &results["items"]["CHAIN"]["other_mode"];
    assert_eq!(other["mode"], "retail");
    assert_eq!(other["identical_but_the_mode_lines"], true);
    for sb in [10, 11] {
        let o = run_item_in(&item, sb, None, Some(SessionMode::Retail));
        eprintln!("CHAIN retail sb{sb}: {:?}", o.counts);
        assert!(o.first_difference.is_none(), "{:?}", o.first_difference);
        assert_eq!(o.counts.tics, 831);
    }
}

/// The items of the owner's rulings of 2026-11-22 ([`m6a::RULING_ITEMS`]) through the same gate
/// as the level chain, at both screen sizes: every G, C, I and J row, every level and
/// intermission frame (whole digest; `message_on` in the P row, so the turbo message's tics are
/// compared), every sound row and mixer buffer, and the demo's end.
///
/// - `TURBO`: a forward move of 100 on E1M2's load tic (session tic 192, `gametic` mod 128 = 64,
///   so no message), around its leveltime 128 (no message) and around `gametic` 384 (the
///   message); the same after the reborn reload on tic 672 (`gametic` mod 128 = 32: no message on
///   the reload tic or around leveltime 128, the message at `gametic` 896); and E1M3's load tic
///   1152 with the message, set after `P_SpawnPlayer`'s clear (p_mobj.c:676) in the load's own
///   `G_Ticker` (g_game.c:617-650, then 671-679).
/// - `INTCMD`: every tic of the intermission after the E1M7 exit carries a command outside wire
///   24's level domain (sides of -128, turns, forward moves up to 127, buttons with bit 0x40 and
///   weapon bits without `BT_CHANGE`; forward 100 across `gametic` 128, whose turbo message no
///   frame shows), and its presses are rising edges of bits 0 and 1 only.
#[test]
fn the_ruling_items_against_the_genuine_goldens() {
    let mut total = Counts::default();
    for name in m6a::RULING_ITEMS {
        let item = m6a::item(name);
        for sb in [10, 11] {
            let o = run_item(&item, sb, None);
            eprintln!("{name} sb{sb}: {:?}", o.counts);
            assert!(o.first_difference.is_none(), "{:?}", o.first_difference);
            assert_eq!(o.counts.tics, item.session_tics, "{name}: every tic");
            total.add(&o.counts);
        }
    }
    eprintln!("ruling items, 2 items x 2 screen sizes: {total:?}");
}

/// The rulings' comparisons can fail, each at the first place the old behaviour shows:
/// `PlantedFault::TurboLeveltime` (every load at `gametic` 0, so the turbo test reads
/// `leveltime`, step 7's model without its refusal) shows the message on TURBO's E1M2 load tic
/// 192, where the genuine frame has none; `PlantedFault::IntermissionDomain` (wire 24's domain in
/// every game state, step 7's rule) refuses INTCMD's first intermission tic 56, which the genuine
/// engine runs.
#[test]
fn the_ruling_faults_are_caught() {
    let o = run_item(&m6a::item("TURBO"), 10, Some(PlantedFault::TurboLeveltime));
    let d = o.first_difference.expect("TurboLeveltime caught");
    eprintln!("TURBO TurboLeveltime: {}", d.lines().next().unwrap_or(""));
    assert!(
        d.starts_with("TURBO sb10 tic 192:"),
        "caught elsewhere: {d}"
    );
    let o = run_item(
        &m6a::item("INTCMD"),
        10,
        Some(PlantedFault::IntermissionDomain),
    );
    let end = o.end.expect("refused");
    eprintln!("INTCMD IntermissionDomain: {end}");
    assert_eq!((end.tic, end.witness), (56, SessionWitness::Command));
    assert!(
        o.first_difference.is_some(),
        "the early end is a difference"
    );
}
