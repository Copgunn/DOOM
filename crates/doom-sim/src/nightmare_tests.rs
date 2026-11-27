//! Wire 29 (`PlayerMonstersEpisode1V7`, DSIM45): Rust-only tests of the profile,
//! the admission of skill 4 and the parameters, and the DSIM45 writer and reader.
//! No genuine engine runs here; `nightmare_genuine_tests` compares with it. The maps
//! are the M6b synthetic slots (`tests/reference/m6b_wads/m6b.wad`).
use crate::identity::{OriginalActorType, OriginalState};
use crate::nightmare_genuine_tests::level;
use crate::*;

const V6: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V6;
const V7: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V7;
const NONE: GameParameters = GameParameters {
    respawn: false,
    fast: false,
};

fn m6b_map(map: u8) -> doom_map::Map {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference/m6b_wads/m6b.wad");
    let bytes = std::fs::read(path).expect("wad");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("wad parse");
    doom_map::Map::from_wad(&wad, &format!("E1M{map}"), doom_map::MapLimits::default())
        .expect("map")
}

/// `level` at a load `gametic`.
fn level_at(map: u8, skill: Skill, parameters: GameParameters, gametic: u64) -> Simulation {
    Simulation::original_player_episode1_v7_level_at(
        m6b_map(map),
        map,
        skill,
        parameters,
        MapContextOptions::default(),
        LevelStart::NewGame,
        gametic,
    )
    .expect("the map loads")
}

fn idle(s: &mut Simulation, tics: usize) {
    for _ in 0..tics {
        s.advance([TicCommand::default(); 4]).expect("an idle tic");
    }
}

/// The DSIM framing of `fields` under `version`.
fn encode(version: u32, fields: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = b"DSIM".to_vec();
    out.extend(version.to_le_bytes());
    out.extend((fields.len() as u64).to_le_bytes());
    for (path, value) in fields {
        out.extend((path.len() as u32).to_le_bytes());
        out.extend(path.as_bytes());
        out.extend((value.len() as u64).to_le_bytes());
        out.extend(value);
    }
    out
}

fn fields(s: &Snapshot) -> Vec<(String, Vec<u8>)> {
    s.fields()
        .map(|f| (f.path().to_owned(), f.value().to_vec()))
        .collect()
}

/// `fields` with `path` set to `value` (`None` removes it).
fn with(fields: &[(String, Vec<u8>)], path: &str, value: Option<&[u8]>) -> Vec<(String, Vec<u8>)> {
    assert!(fields.iter().any(|(p, _)| p == path), "{path}");
    fields
        .iter()
        .filter_map(|(p, v)| match (p == path, value) {
            (false, _) => Some((p.clone(), v.clone())),
            (true, Some(value)) => Some((p.clone(), value.to_vec())),
            (true, None) => None,
        })
        .collect()
}

fn read45(fields: &[(String, Vec<u8>)]) -> Result<u32, DiagnosticError> {
    Diagnostic::decode_episode1_v7(&encode(45, fields), DiagnosticLimits::default())
        .map(|d| d.version())
}

/// DSIM45 is DSIM44 field for field under policy 29 and the wire-29 context, plus
/// `actors.parameters` right after `actors.options` and `level.gametic_lead` right
/// after `level.start.gametic`: a wire-28 and a wire-29 level of the same map,
/// skill and commands differ in exactly those four fields at every capture.
#[test]
fn dsim45_is_dsim44_plus_two_fields_under_policy_29() {
    let map = 3;
    let mut old = Simulation::original_player_episode1_v6_level_at(
        m6b_map(map),
        map,
        Skill::Hard,
        MapContextOptions::default(),
        LevelStart::NewGame,
        7,
    )
    .expect("wire 28 loads");
    let mut new = level_at(map, Skill::Hard, NONE, 7);
    for tic in 0..40 {
        let (a, b) = (old.snapshot(), new.snapshot());
        assert_eq!((a.version(), b.version()), (44, 45));
        let (a, b) = (fields(&a), fields(&b));
        let at = |p: &str| b.iter().position(|(q, _)| q == p).unwrap();
        assert_eq!(at("actors.parameters"), at("actors.options") + 1);
        assert_eq!(at("level.gametic_lead"), at("level.start.gametic") + 1);
        assert_eq!(b[at("actors.parameters")].1, [0, 0]);
        assert_eq!(b[at("level.gametic_lead")].1, 7_u64.to_le_bytes());
        let rest: Vec<_> = b
            .iter()
            .filter(|(p, _)| p != "actors.parameters" && p != "level.gametic_lead")
            .collect();
        assert_eq!(rest.len(), a.len());
        for ((pa, va), (pb, vb)) in a.iter().zip(rest) {
            assert_eq!(pa, pb);
            match pa.as_str() {
                "profile" => assert_eq!(
                    (&va[..], &vb[..]),
                    (&28_u32.to_le_bytes()[..], &29_u32.to_le_bytes()[..])
                ),
                "context.schema" => {
                    assert_eq!(
                        (&va[..], &vb[..]),
                        (
                            PLAYER_EPISODE1_V6_CONTEXT_SCHEMA,
                            PLAYER_EPISODE1_V7_CONTEXT_SCHEMA
                        )
                    )
                }
                _ => assert_eq!(va, vb, "{pa} at tic {tic}"),
            }
        }
        idle(&mut old, 1);
        idle(&mut new, 1);
    }
    assert_eq!((new.gametic_lead(), new.gametic()), (Some(7), 47));
}

/// The DSIM45 reader reads every capture of a wire-29 run back field for field;
/// every earlier reader refuses version 45, and it refuses version 44.
#[test]
fn dsim45_reader_round_trip_and_version_refusals() {
    let limits = DiagnosticLimits::default();
    let mut s = level_at(2, Skill::Nightmare, NONE, 0);
    for _ in 0..21 {
        let snap = s.snapshot();
        let bytes = snap.canonical_bytes();
        let d = Diagnostic::decode_episode1_v7(&bytes, limits).expect("DSIM45");
        assert_eq!(d.version(), 45);
        for f in snap.fields() {
            assert_eq!(d.field(f.path()), Some(f.value()), "{}", f.path());
        }
        for old in [
            Diagnostic::decode_episode1_v6,
            Diagnostic::decode_episode1_v5,
            Diagnostic::decode_episode1_v4,
        ] {
            assert_eq!(
                old(&bytes, limits).err(),
                Some(DiagnosticError::Version(45))
            );
        }
        idle(&mut s, 1);
    }
    let old = Simulation::original_player_episode1_v6_level(
        m6b_map(2),
        2,
        Skill::Hard,
        MapContextOptions::default(),
        LevelStart::NewGame,
    )
    .expect("wire 28 loads")
    .snapshot()
    .canonical_bytes();
    assert_eq!(
        Diagnostic::decode_episode1_v7(&old, limits).err(),
        Some(DiagnosticError::Version(44))
    );
}

/// The DSIM45 rules beyond DSIM44's: the parameters' bytes, the gametic lead's
/// range, skill 4 (refused by DSIM44), kills past the total (refused by DSIM44)
/// and the durations of the thirteen fast-table states, halved iff skill 4 or
/// `fastparm`.
#[test]
fn dsim45_reader_rules() {
    let f = fields(&level_at(2, Skill::Nightmare, NONE, 100).snapshot());
    assert_eq!(read45(&f), Ok(45));
    for bad in [[2, 0], [0, 2]] {
        assert_eq!(
            read45(&with(&f, "actors.parameters", Some(&bad))),
            Err(DiagnosticError::Structure)
        );
    }
    assert_eq!(
        read45(&with(&f, "actors.parameters", Some(&[1]))),
        Err(DiagnosticError::Structure)
    );
    assert_eq!(
        read45(&with(&f, "actors.parameters", None)),
        Err(DiagnosticError::Structure)
    );
    assert_eq!(
        read45(&with(&f, "actors.parameters", Some(&[1, 1]))),
        Ok(45)
    );
    for (lead, ok) in [
        (99_u64, false),
        (100, true),
        (5000, true),
        (i32::MAX as u64, true),
        (1 << 31, false),
    ] {
        let got = read45(&with(&f, "level.gametic_lead", Some(&lead.to_le_bytes())));
        assert_eq!(got.is_ok(), ok, "lead {lead}: {got:?}");
    }
    assert_eq!(
        read45(&with(&f, "level.gametic_lead", None)),
        Err(DiagnosticError::Structure)
    );
    assert_eq!(
        read45(&with(&f, "actors.options", Some(&[5, 0, 0]))),
        Err(DiagnosticError::Identity)
    );
    // The same skill-4 capture as DSIM44 (version, profile, context, no new fields).
    let as44: Vec<_> = f
        .iter()
        .filter(|(p, _)| p != "actors.parameters" && p != "level.gametic_lead")
        .map(|(p, v)| match p.as_str() {
            "profile" => (p.clone(), 28_u32.to_le_bytes().to_vec()),
            "context.schema" => (p.clone(), PLAYER_EPISODE1_V6_CONTEXT_SCHEMA.to_vec()),
            _ => (p.clone(), v.clone()),
        })
        .collect();
    let read44 = |f: &[(String, Vec<u8>)]| {
        Diagnostic::decode_episode1_v6(&encode(44, f), DiagnosticLimits::default())
            .map(|d| d.version())
    };
    assert_eq!(read44(&as44), Err(DiagnosticError::Identity));
    assert_eq!(
        read44(&with(&as44, "actors.options", Some(&[3, 0, 0]))),
        Ok(44)
    );
    // One placed monster: a credit of 2 passes the total only in DSIM45.
    let two = 2_i32.to_le_bytes();
    assert_eq!(
        read45(&with(&f, "level.single_player_kill_count", Some(&two))),
        Ok(45)
    );
    let hard44 = with(&as44, "actors.options", Some(&[3, 0, 0]));
    assert_eq!(
        read44(&with(&hard44, "level.single_player_kill_count", Some(&two))),
        Err(DiagnosticError::Identity)
    );
    assert_eq!(
        read45(&with(
            &f,
            "level.single_player_kill_count",
            Some(&(-1_i32).to_le_bytes())
        )),
        Err(DiagnosticError::Identity)
    );

    // A Demon (E1M4, skill 3) set into S_SARG_RUN1 (2 tics) and S_SARG_ATK1 (8).
    let f = fields(&level_at(4, Skill::Hard, NONE, 0).snapshot());
    let demon = (0..)
        .map(|k| format!("actor.{k}"))
        .find(|p| {
            f.iter().any(|(q, v)| {
                *q == format!("{p}.type")
                    && v[..] == OriginalActorType::MT_SERGEANT.ordinal().to_le_bytes()
            })
        })
        .unwrap();
    for (state, slow) in [
        (OriginalState::S_SARG_RUN1, 2),
        (OriginalState::S_SARG_ATK1, 8),
    ] {
        let set = |tics: i32, parameters: [u8; 2], skill: u8| {
            let g = with(
                &f,
                &format!("{demon}.state"),
                Some(&state.ordinal().to_le_bytes()),
            );
            let g = with(&g, &format!("{demon}.tics"), Some(&tics.to_le_bytes()));
            let g = with(&g, "actors.parameters", Some(&parameters));
            read45(&with(&g, "actors.options", Some(&[skill, 0, 0])))
        };
        assert_eq!(set(slow, [0, 0], 3), Ok(45), "{state:?}");
        assert_eq!(
            set(slow + 1, [0, 0], 3),
            Err(DiagnosticError::Identity),
            "{state:?}"
        );
        assert_eq!(
            set(0, [0, 0], 3),
            Err(DiagnosticError::Identity),
            "{state:?}"
        );
        assert_eq!(set(slow, [1, 0], 3), Ok(45), "{state:?} respawn");
        for fast in [([0, 1], 3), ([0, 0], 4), ([1, 1], 4)] {
            assert_eq!(set(slow >> 1, fast.0, fast.1), Ok(45), "{state:?} {fast:?}");
            assert_eq!(
                set(slow, fast.0, fast.1),
                Err(DiagnosticError::Identity),
                "{state:?} {fast:?}"
            );
        }
    }
}

/// Skill 4 is admitted under wire 29 alone; wire 28 refuses it at the load as
/// before. Wire 29 takes the parameters at any skill and keeps them in the
/// capture; `with_original_registry` refuses wire 29 as it refuses wires 26 to
/// 28, and `MapContextOptions::respawn_monsters` stays refused under wire 29 (the
/// respawn parameter is `GameParameters::respawn`).
#[test]
fn skill_4_and_the_parameters_are_admitted_under_wire_29_alone() {
    assert!(level("BITS4", 2, Skill::Nightmare, NONE).is_ok());
    let wire28 = Simulation::original_player_episode1_v6_level(
        m6b_map(2),
        2,
        Skill::Nightmare,
        MapContextOptions::default(),
        LevelStart::NewGame,
    );
    assert_eq!(
        wire28.map(|_| ()),
        Err(LevelStartError::Registry(OriginalRegistryError::Runtime(
            PlayerBodyError::Unsupported.into()
        )))
    );
    for (skill, respawn, fast) in [
        (Skill::Baby, true, false),
        (Skill::Medium, false, true),
        (Skill::Nightmare, true, true),
    ] {
        let p = GameParameters { respawn, fast };
        let s = level_at(2, skill, p, 0);
        let snap = s.snapshot();
        let get = |path: &str| {
            snap.fields()
                .find(|f| f.path() == path)
                .unwrap()
                .value()
                .to_vec()
        };
        assert_eq!(get("actors.options"), [skill as u8, 0, 0]);
        assert_eq!(
            get("actors.parameters"),
            [u8::from(respawn), u8::from(fast)]
        );
    }
    let registry = OriginalExecutionRegistry::new(V7).expect("registry");
    assert_eq!(
        Simulation::with_original_registry(
            m6b_map(2),
            player_map_scale::actor_slots(V7),
            registry,
            ActorOptions::default(),
            MapContextOptions::default(),
        )
        .map(|_| ()),
        Err(PlayerBodyError::Unsupported.into())
    );
    let respawning = Simulation::original_player_episode1_v7_level_at(
        m6b_map(2),
        2,
        Skill::Medium,
        NONE,
        MapContextOptions {
            respawn_monsters: true,
            ..MapContextOptions::default()
        },
        LevelStart::NewGame,
        0,
    );
    assert_eq!(
        respawning.map(|_| ()),
        Err(LevelStartError::Registry(OriginalRegistryError::Runtime(
            MapSpawnError::Respawn.into()
        )))
    );
    assert!(!V6.has_nightmare() && V7.has_nightmare() && V7.has_easy_skills());
    assert_eq!(
        (V7.wire_id(), V7.context_schema()),
        (29, PLAYER_EPISODE1_V7_CONTEXT_SCHEMA)
    );
}
