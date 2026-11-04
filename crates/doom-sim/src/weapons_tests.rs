//! Internal bounded interpreter edge cases; public admission rejects these graphs.
use super::*;
#[path = "../../../tests/support/m3d2.rs"]
pub(super) mod fixtures;
#[test]
fn psprite_zero_chain_infinite_null_and_coordinate_rules() {
    let sim = fixtures::setup(&fixtures::cases()[0]);
    let source = ActorRegistry::vanilla();
    let mut states = source.states().to_vec();
    states[1].tics = 0;
    states[1].next = StateId(2);
    states[1].action = ActorAction::None;
    states[1].misc = [0, 99];
    states[2].tics = -1;
    states[2].next = StateId(0);
    states[2].action = ActorAction::None;
    states[2].misc = [3, -7];
    let r = ActorRegistry::new(states, source.types().to_vec()).unwrap();
    let p = sim.weapon_player().unwrap().clone();
    let id = p.actor;
    let mut m = Machine {
        player: p,
        registry: &r,
        host: Preview {
            sim: &sim,
            actor: *sim.actor(id).unwrap(),
            id,
            spawns: 0,
            rng: sim.rng,
            damaged: BTreeMap::new(),
        },
        buttons: 0,
        leveltime: 0,
        work: 0,
    };
    m.enter(0, StateId(1)).unwrap();
    assert_eq!(m.player.psprites[0].state, Some(StateId(2)));
    assert_eq!(
        m.player.psprites[0].position,
        [Fixed::from_map_units(3), Fixed::from_map_units(-7)]
    );
    m.tick(TicCommand::default()).unwrap();
    assert_eq!(m.player.psprites[0].tics, -1);
    let before = m.player.psprites[0];
    m.enter(0, StateId(0)).unwrap();
    assert_eq!(m.player.psprites[0].state, None);
    assert_eq!(m.player.psprites[0].position, before.position);
    assert_eq!(m.player.psprites[0].tics, before.tics);
    assert_eq!(
        m.enter(0, StateId(u16::MAX)),
        Err(SimError::Weapon(WeaponError::Definition))
    );
}
#[test]
fn pathological_zero_chain_is_bounded() {
    let sim = fixtures::setup(&fixtures::cases()[0]);
    let source = ActorRegistry::vanilla();
    let mut states = source.states().to_vec();
    states[1].tics = 0;
    states[1].next = StateId(1);
    states[1].action = ActorAction::None;
    let r = ActorRegistry::new(states, source.types().to_vec()).unwrap();
    let p = sim.weapon_player().unwrap().clone();
    let id = p.actor;
    let mut m = Machine {
        player: p,
        registry: &r,
        host: Preview {
            sim: &sim,
            actor: *sim.actor(id).unwrap(),
            id,
            spawns: 0,
            rng: sim.rng,
            damaged: BTreeMap::new(),
        },
        buttons: 0,
        leveltime: 0,
        work: 0,
    };
    assert_eq!(
        m.enter(0, StateId(1)),
        Err(SimError::Weapon(WeaponError::WorkLimit))
    );
    assert_eq!(m.work, 129);
}
#[test]
fn source_state_constants_are_the_named_source_states() {
    // The constants replace a name search; each must be the index that search finds.
    for (id, name) in [
        (STATE_CHAIN1, "S_CHAIN1"),
        (STATE_PLAY, "S_PLAY"),
        (STATE_PLAY_ATK1, "S_PLAY_ATK1"),
        (STATE_PLAY_ATK2, "S_PLAY_ATK2"),
        (STATE_PLAY_RUN1, "S_PLAY_RUN1"),
    ] {
        assert_eq!(lifecycle::source_state_id(name), Some(id), "{name}");
    }
    let names = [
        ["S_PUNCHUP", "S_PUNCHDOWN", "S_PUNCH", "S_PUNCH1", "S_NULL"],
        [
            "S_PISTOLUP",
            "S_PISTOLDOWN",
            "S_PISTOL",
            "S_PISTOL1",
            "S_PISTOLFLASH",
        ],
        [
            "S_SGUNUP",
            "S_SGUNDOWN",
            "S_SGUN",
            "S_SGUN1",
            "S_SGUNFLASH1",
        ],
        [
            "S_CHAINUP",
            "S_CHAINDOWN",
            "S_CHAIN",
            "S_CHAIN1",
            "S_CHAINFLASH1",
        ],
    ];
    for (weapon, names) in [
        Weapon::Fist,
        Weapon::Pistol,
        Weapon::Shotgun,
        Weapon::Chaingun,
    ]
    .into_iter()
    .zip(names)
    {
        assert_eq!(
            weapon.states().map(Some),
            names.map(lifecycle::source_state_id),
            "{weapon:?}"
        );
    }
}
