//! Source player weapon/psprite interpreter and conservative preflight.
use crate::*;
use std::{collections::BTreeMap, error::Error, fmt};
// p_inter.c maxammo and p_user.c/p_pspr.c MAXBOB; immutable source policy data.
pub(crate) const AMMO_CAPACITIES: [i32; 4] = [200, 50, 300, 50];
pub(crate) const MAX_BOB_BITS: i32 = 0x100000;
/// Original command attack bit.
pub const BT_ATTACK: u8 = 1;
/// Original command weapon-change bit.
pub const BT_CHANGE: u8 = 4;
/// Original three-bit weapon selection mask.
pub const BT_WEAPONMASK: u8 = 56;
/// Original weapon identities; recognition does not grant execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Weapon {
    /// Unpowered source fallback fist.
    Fist = 0,
    /// Source pistol.
    Pistol = 1,
    /// Seven-pellet shotgun.
    Shotgun = 2,
    /// Source chaingun.
    Chaingun = 3,
    /// Deferred rocket launcher.
    Rocket = 4,
    /// Deferred plasma rifle.
    Plasma = 5,
    /// Deferred BFG.
    Bfg = 6,
    /// Deferred chainsaw.
    Chainsaw = 7,
    /// Deferred double-barrel shotgun.
    SuperShotgun = 8,
}
impl Weapon {
    pub(crate) fn from_number(v: u8) -> Result<Self, WeaponError> {
        match v {
            0 => Ok(Self::Fist),
            1 => Ok(Self::Pistol),
            2 => Ok(Self::Shotgun),
            3 => Ok(Self::Chaingun),
            _ => Err(WeaponError::UnsupportedWeapon),
        }
    }
    fn ammo(self) -> Option<usize> {
        match self {
            Self::Fist => None,
            Self::Shotgun => Some(1),
            _ => Some(0),
        }
    }
    pub(crate) fn states(self) -> [StateId; 5] {
        let names = match self {
            Self::Fist => ["S_PUNCHUP", "S_PUNCHDOWN", "S_PUNCH", "S_PUNCH1", "S_NULL"],
            Self::Pistol => [
                "S_PISTOLUP",
                "S_PISTOLDOWN",
                "S_PISTOL",
                "S_PISTOL1",
                "S_PISTOLFLASH",
            ],
            Self::Shotgun => [
                "S_SGUNUP",
                "S_SGUNDOWN",
                "S_SGUN",
                "S_SGUN1",
                "S_SGUNFLASH1",
            ],
            Self::Chaingun => [
                "S_CHAINUP",
                "S_CHAINDOWN",
                "S_CHAIN",
                "S_CHAIN1",
                "S_CHAINFLASH1",
            ],
            _ => unreachable!("weapon admission"),
        };
        names.map(state)
    }
}
fn state(name: &str) -> StateId {
    lifecycle::source_state_id(name).expect("source state")
}
/// A rejected profile, player, command or preflight prerequisite.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeaponError {
    /// Requires an explicitly admitted weapon capability.
    Profile,
    /// Only one initialized live player is admitted, in a validated slot.
    Player,
    /// Active original player pawn is required; stale/pending handles reject.
    Reference,
    /// Source weapon is recognized but unavailable in this slice.
    UnsupportedWeapon,
    /// Invalid ownership/ammo/setup values.
    Inventory,
    /// Modified source states/types are outside this profile's admission.
    Definition,
    /// Bounded recursion/entry/fire work would be exceeded.
    WorkLimit,
    /// Defined player bob arithmetic excludes extreme setup momentum.
    Momentum,
}
impl fmt::Display for WeaponError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "weapon: {self:?}")
    }
}
impl Error for WeaponError {}
/// Explicit test/level setup, not pickup or inventory gameplay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WeaponInventory {
    /// Initial supported owned ready weapon.
    pub ready: Weapon,
    /// Source weapon bits; fist and pistol must remain owned.
    pub owned: u16,
    /// Source clip/shell/cell/rocket counts, bounded by original capacities.
    pub ammo: [i32; 4],
}
impl Default for WeaponInventory {
    fn default() -> Self {
        Self {
            ready: Weapon::Pistol,
            owned: 3,
            ammo: [50, 0, 0, 0],
        }
    }
}
impl WeaponInventory {
    pub(crate) fn validate(self) -> Result<(), WeaponError> {
        Weapon::from_number(self.ready as u8)?;
        if self.owned & !15 != 0
            || self.owned & 3 != 3
            || self.owned & (1 << (self.ready as u8)) == 0
            || self
                .ammo
                .into_iter()
                .zip(AMMO_CAPACITIES)
                .any(|(a, m)| a < 0 || a > m)
            || self.ammo[2] != 0
            || self.ammo[3] != 0
        {
            return Err(WeaponError::Inventory);
        }
        Ok(())
    }
}
/// Player-owned source psprite state, never an actor/thinker.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Psprite {
    /// None is the original null state pointer.
    pub state: Option<StateId>,
    /// Current source countdown, including -1.
    pub tics: i32,
    /// Source 16.16 overlay coordinates.
    pub position: [Fixed; 2],
}
/// The admitted live player's source weapon-relevant state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeaponPlayer {
    /// Original ordered input slot.
    pub slot: u8,
    /// Generation-safe bound body.
    pub actor: ActorId,
    /// Historical persistent profile requires fixed live health. Genuine-player
    /// adapters borrow current authoritative health only for one operation.
    pub health: i32,
    /// Current source weapon.
    pub ready: Weapon,
    /// Pending source weapon, None for wp_nochange=10.
    pub pending: Option<Weapon>,
    /// Original weapon ownership bitset.
    pub owned: u16,
    /// Clip/shell/cell/rocket counts.
    pub ammo: [i32; 4],
    /// Original clip/shell/cell/rocket capacities.
    pub max_ammo: [i32; 4],
    /// Source attack-down latch.
    pub attack_down: bool,
    /// Source consecutive refire count.
    pub refire: i32,
    /// Source pre-friction momentum bob.
    pub bob: Fixed,
    /// Source weapon light actions' persistent value.
    pub extra_light: i32,
    /// Weapon then flash, in source processing order.
    pub psprites: [Psprite; 2],
}
/// Ordered weapon observations. Actor sounds retain existing sound events.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum WeaponEvent {
    Entered {
        slot: u8,
        position: u8,
        state: StateId,
    },
    Action {
        slot: u8,
        position: u8,
        action: ActionId,
    },
    Ammo {
        slot: u8,
        ammo: u8,
        before: i32,
        after: i32,
    },
    Light {
        slot: u8,
        value: i32,
    },
    Fire {
        slot: u8,
        weapon: Weapon,
        refire: i32,
    },
    Bullet {
        actor: ActorId,
        angle: u32,
        slope: Fixed,
        damage: i32,
    },
    Noise {
        sector: u16,
        blocks: u8,
        target: ActorId,
    },
    Initialized {
        slot: u8,
        actor: ActorId,
    },
    Random {
        kind: u8,
        value: u8,
    },
}
pub(crate) trait Host {
    fn actor(&self) -> Result<Actor, SimError>;
    fn health(&self) -> i32 {
        100
    }
    fn dead(&self) -> bool {
        false
    }
    fn admit_weapon(&self, _weapon: Weapon) -> Result<(), SimError> {
        Ok(())
    }
    fn motion(&mut self, command: TicCommand) -> Result<(), SimError>;
    fn body_state(&mut self, state: StateId) -> Result<(), SimError>;
    fn event(&mut self, event: WeaponEvent);
    fn sound(&mut self, sound: u16);
    fn fire(&mut self, weapon: Weapon, accurate: bool) -> Result<(), SimError>;
    fn noise(&mut self) -> Result<(), SimError>;
}
struct Machine<'a, H> {
    player: WeaponPlayer,
    registry: &'a ActorRegistry,
    host: H,
    buttons: u8,
    leveltime: u64,
    work: usize,
}
impl<H: Host> Machine<'_, H> {
    fn emit(&mut self, e: WeaponEvent) {
        self.host.event(e)
    }
    fn enter(&mut self, position: usize, mut id: StateId) -> Result<(), SimError> {
        loop {
            self.work += 1;
            if self.work > 128 {
                return Err(WeaponError::WorkLimit.into());
            }
            self.emit(WeaponEvent::Entered {
                slot: self.player.slot,
                position: position as u8,
                state: id,
            });
            if id.0 == 0 {
                self.player.psprites[position].state = None;
                return Ok(());
            }
            let s = self.registry.state(id).ok_or(WeaponError::Definition)?;
            let (tics, misc, action) = (s.tics, s.misc, s.action);
            let ps = &mut self.player.psprites[position];
            ps.state = Some(id);
            ps.tics = tics;
            if misc[0] != 0 {
                ps.position = misc.map(|v| Fixed::from_bits(v.wrapping_shl(16)));
            }
            match action {
                ActorAction::None => {}
                ActorAction::Original(action) => {
                    self.emit(WeaponEvent::Action {
                        slot: self.player.slot,
                        position: position as u8,
                        action,
                    });
                    self.act(position, action)?;
                }
                _ => return Err(WeaponError::Definition.into()),
            }
            let ps = self.player.psprites[position];
            if ps.state.is_none() || ps.tics != 0 {
                return Ok(());
            }
            id = self
                .registry
                .state(ps.state.ok_or(WeaponError::Definition)?)
                .ok_or(WeaponError::Definition)?
                .next;
        }
    }
    fn bring_up(&mut self) -> Result<(), SimError> {
        let w = self.player.pending.unwrap_or(self.player.ready);
        self.player.pending = None;
        self.player.psprites[0].position[1] = Fixed::from_map_units(128);
        self.enter(0, w.states()[0])
    }
    fn check_ammo(&mut self) -> Result<bool, SimError> {
        if self
            .player
            .ready
            .ammo()
            .is_none_or(|a| self.player.ammo[a] >= 1)
        {
            return Ok(true);
        }
        let fallback = if self.player.owned & 8 != 0 && self.player.ammo[0] != 0 {
            Weapon::Chaingun
        } else if self.player.owned & 4 != 0 && self.player.ammo[1] != 0 {
            Weapon::Shotgun
        } else if self.player.ammo[0] != 0 {
            Weapon::Pistol
        } else {
            Weapon::Fist
        };
        self.host.admit_weapon(fallback)?;
        self.player.pending = Some(fallback);
        self.enter(0, self.player.ready.states()[1])?;
        Ok(false)
    }
    fn fire_weapon(&mut self) -> Result<(), SimError> {
        if !self.check_ammo()? {
            return Ok(());
        }
        self.emit(WeaponEvent::Fire {
            slot: self.player.slot,
            weapon: self.player.ready,
            refire: self.player.refire,
        });
        self.host.body_state(state("S_PLAY_ATK1"))?;
        self.enter(0, self.player.ready.states()[3])?;
        self.host.noise()?;
        Ok(())
    }
    fn act(&mut self, position: usize, action: ActionId) -> Result<(), SimError> {
        match self
            .registry
            .action_name(action)
            .ok_or(WeaponError::Definition)?
        {
            "A_Raise" => {
                self.player.psprites[position].position[1] =
                    self.player.psprites[position].position[1] - Fixed::from_map_units(6);
                if self.player.psprites[position].position[1] <= Fixed::from_map_units(32) {
                    self.player.psprites[position].position[1] = Fixed::from_map_units(32);
                    self.enter(0, self.player.ready.states()[2])?;
                }
            }
            "A_Lower" => {
                self.player.psprites[position].position[1] =
                    self.player.psprites[position].position[1] + Fixed::from_map_units(6);
                if self.player.psprites[position].position[1] >= Fixed::from_map_units(128) {
                    if self.host.dead() {
                        self.player.psprites[position].position[1] = Fixed::from_map_units(128);
                        return Ok(());
                    }
                    if self.host.health() == 0 {
                        self.enter(0, StateId(identity::OriginalState::S_NULL.ordinal()))?;
                        return Ok(());
                    }
                    self.player.ready = self.player.pending.ok_or(WeaponError::Definition)?;
                    self.bring_up()?;
                }
            }
            "A_WeaponReady" => {
                if matches!(self.host.actor()?.life.ok_or(WeaponError::Definition)?.state,Some(s) if s==state("S_PLAY_ATK1")||s==state("S_PLAY_ATK2"))
                {
                    self.host.body_state(state("S_PLAY"))?;
                }
                if self.player.pending.is_some() || self.host.health() == 0 {
                    self.enter(0, self.player.ready.states()[1])?;
                    return Ok(());
                }
                if self.buttons & BT_ATTACK != 0 {
                    self.player.attack_down = true;
                    self.fire_weapon()?;
                    return Ok(());
                } else {
                    self.player.attack_down = false;
                }
                let phase = ((self.leveltime as u32).wrapping_mul(128) & 8191) as usize;
                self.player.psprites[position].position = [
                    Fixed::ONE + self.player.bob * Fixed::from_bits(FINE_SINE[phase + 2048]),
                    Fixed::from_map_units(32)
                        + self.player.bob * Fixed::from_bits(FINE_SINE[phase & 4095]),
                ];
            }
            "A_ReFire" => {
                if self.buttons & BT_ATTACK != 0
                    && self.player.pending.is_none()
                    && self.host.health() != 0
                {
                    self.player.refire = self
                        .player
                        .refire
                        .checked_add(1)
                        .ok_or(WeaponError::WorkLimit)?;
                    self.fire_weapon()?;
                } else {
                    self.player.refire = 0;
                    self.check_ammo()?;
                }
            }
            "A_FirePistol" | "A_FireShotgun" | "A_FireCGun" => {
                let w = self.player.ready;
                self.host.sound(
                    lifecycle::source_sound(if w == Weapon::Shotgun {
                        "sfx_shotgn"
                    } else {
                        "sfx_pistol"
                    })
                    .unwrap(),
                );
                let ammo = w.ammo().unwrap();
                if w == Weapon::Chaingun && self.player.ammo[ammo] == 0 {
                    return Ok(());
                }
                self.host.body_state(state("S_PLAY_ATK2"))?;
                let before = self.player.ammo[ammo];
                self.player.ammo[ammo] -= 1;
                self.emit(WeaponEvent::Ammo {
                    slot: self.player.slot,
                    ammo: ammo as u8,
                    before,
                    after: before - 1,
                });
                let mut flash = w.states()[4];
                if w == Weapon::Chaingun {
                    flash.0 +=
                        self.player.psprites[position].state.unwrap().0 - state("S_CHAIN1").0;
                }
                self.enter(1, flash)?;
                self.host.fire(w, self.player.refire == 0)?;
            }
            "A_Punch" => self.host.fire(Weapon::Fist, false)?,
            "A_Light0" | "A_Light1" | "A_Light2" => {
                let value = match self.registry.action_name(action).unwrap() {
                    "A_Light0" => 0,
                    "A_Light1" => 1,
                    _ => 2,
                };
                self.player.extra_light = value;
                self.emit(WeaponEvent::Light {
                    slot: self.player.slot,
                    value,
                });
            }
            _ => return Err(WeaponError::Definition.into()),
        }
        Ok(())
    }
    fn tick(&mut self, command: TicCommand) -> Result<(), SimError> {
        self.host.motion(command)?;
        self.after_motion(command)
    }
    fn after_motion(&mut self, command: TicCommand) -> Result<(), SimError> {
        let a = self.host.actor()?;
        let [x, y] = a.momentum();
        self.player.bob = Fixed::from_bits(((x * x + y * y).bits() >> 2).min(MAX_BOB_BITS));
        if command.buttons & BT_CHANGE != 0 {
            let w = Weapon::from_number((command.buttons & BT_WEAPONMASK) >> 3)?;
            if self.player.owned & (1 << (w as u8)) != 0 && w != self.player.ready {
                self.player.pending = Some(w);
            }
        }
        self.move_psprites()
    }
    fn move_psprites(&mut self) -> Result<(), SimError> {
        for i in 0..2 {
            if let Some(id) = self.player.psprites[i].state {
                if self.player.psprites[i].tics != -1 {
                    self.player.psprites[i].tics -= 1;
                    if self.player.psprites[i].tics == 0 {
                        self.enter(
                            i,
                            self.registry.state(id).ok_or(WeaponError::Definition)?.next,
                        )?;
                    }
                }
            }
        }
        self.player.psprites[1].position = self.player.psprites[0].position;
        Ok(())
    }
}
pub(crate) enum PlayerWeaponOperation {
    Setup,
    Advance,
    DeadAdvance,
    Drop,
}
pub(crate) fn original_player<H: Host>(
    registry: &ActorRegistry,
    player: WeaponPlayer,
    host: H,
    operation: PlayerWeaponOperation,
    command: TicCommand,
    leveltime: u64,
) -> Result<WeaponPlayer, SimError> {
    let mut machine = Machine {
        player,
        registry,
        host,
        buttons: command.buttons,
        leveltime,
        work: 0,
    };
    match operation {
        PlayerWeaponOperation::Setup => machine.bring_up()?,
        PlayerWeaponOperation::Advance => machine.after_motion(command)?,
        PlayerWeaponOperation::DeadAdvance => machine.move_psprites()?,
        PlayerWeaponOperation::Drop => machine.enter(0, machine.player.ready.states()[1])?,
    }
    Ok(machine.player)
}

// p_pspr.c P_GunShot/A_Punch: bounded i32 difference, explicit BAM wrapping.
// Replaces the source's undefined negative signed left shift by multiplication.
const SOURCE_SPREAD_SCALE: i32 = 1 << 18;
fn source_spread(left: u8, right: u8) -> i32 {
    (i32::from(left) - i32::from(right)) * SOURCE_SPREAD_SCALE
}
fn bullet_damage(value: u8) -> i32 {
    5 * (i32::from(value) % 3 + 1)
}
fn punch_damage(value: u8) -> i32 {
    2 * (i32::from(value) % 10 + 1)
}
fn spread_angle(angle: u32, random: &mut impl FnMut(u8) -> u8) -> u32 {
    let left = random(1);
    let right = random(2);
    angle.wrapping_add_signed(source_spread(left, right))
}
/// Shared source P_GunShot ordering; caller supplies the existing gameplay RNG.
pub(crate) fn gun_shot(
    angle: u32,
    accurate: bool,
    random: &mut impl FnMut(u8) -> u8,
) -> (u32, i32) {
    let damage = bullet_damage(random(0));
    let angle = if accurate {
        angle
    } else {
        spread_angle(angle, random)
    };
    (angle, damage)
}
// p_enemy.c A_PosAttack/A_SPosAttack/A_CPosAttack: (P_Random()-P_Random())<<20
// spread, then ((P_Random()%5)+1)*3 damage. Multiplication replaces the
// source's negative signed shift; BAM addition wraps explicitly.
const MONSTER_SPREAD_SCALE: i32 = 1 << 20;
/// Shared former-human bullet: left (kind2) then right (kind3) spread draw, then
/// damage (kind4). The caller owns the RNG/event adapter and the trace.
pub(crate) fn monster_shot(angle: u32, random: &mut impl FnMut(u8) -> u8) -> (u32, i32) {
    let left = i32::from(random(2));
    let right = i32::from(random(3));
    let angle = angle.wrapping_add_signed((left - right) * MONSTER_SPREAD_SCALE);
    (angle, (i32::from(random(4)) % 5 + 1) * 3)
}
/// Unpowered A_Punch component; aim/impact remains the caller's source action.
pub(crate) fn punch_shot(angle: u32, random: &mut impl FnMut(u8) -> u8) -> (u32, i32) {
    let damage = punch_damage(random(0));
    (spread_angle(angle, random), damage)
}
struct Preview<'a> {
    sim: &'a Simulation,
    actor: Actor,
    id: ActorId,
    spawns: usize,
    rng: DoomRng,
    damaged: BTreeMap<ActorId, Actor>,
}
impl Preview<'_> {
    fn lookup(&self, id: ActorId) -> Option<Actor> {
        if id == self.id {
            Some(self.actor)
        } else {
            self.damaged
                .get(&id)
                .copied()
                .or_else(|| self.sim.actor(id).copied())
        }
    }
}
impl Host for Preview<'_> {
    fn actor(&self) -> Result<Actor, SimError> {
        Ok(self.actor)
    }
    fn motion(&mut self, command: TicCommand) -> Result<(), SimError> {
        apply_player_motion(&mut self.actor, command).map(|_| ())
    }
    fn body_state(&mut self, s: StateId) -> Result<(), SimError> {
        self.actor
            .life
            .as_mut()
            .ok_or(WeaponError::Definition)?
            .state = Some(s);
        Ok(())
    }
    fn event(&mut self, _: WeaponEvent) {}
    fn sound(&mut self, _: u16) {}
    fn noise(&mut self) -> Result<(), SimError> {
        Ok(())
    }
    fn fire(&mut self, w: Weapon, accurate: bool) -> Result<(), SimError> {
        self.spawns += if w == Weapon::Shotgun { 7 } else { 1 };
        if self.spawns > 7 {
            return Err(WeaponError::WorkLimit.into());
        }
        let angle = self.actor.angle();
        let range = Fixed::from_map_units(if w == Weapon::Fist { 64 } else { 2048 });
        let mut slope = Fixed::ZERO;
        if w != Weapon::Fist {
            for an in [
                angle,
                angle.wrapping_add(1 << 26),
                angle.wrapping_sub(1 << 26),
            ] {
                let aim = crate::combat_trace::aim_with(
                    self.sim,
                    self.id,
                    an,
                    Fixed::from_map_units(1024),
                    &|id| self.lookup(id),
                )?;
                slope = aim.slope;
                if aim.target.is_some() {
                    break;
                }
            }
        }
        for _ in 0..if w == Weapon::Shotgun { 7 } else { 1 } {
            let (an, damage) = if w == Weapon::Fist {
                punch_shot(angle, &mut |_| self.rng.p_random())
            } else {
                gun_shot(
                    angle,
                    matches!(w, Weapon::Pistol | Weapon::Chaingun) && accurate,
                    &mut |_| self.rng.p_random(),
                )
            };
            if w == Weapon::Fist {
                slope = crate::combat_trace::aim_with(self.sim, self.id, an, range, &|id| {
                    self.lookup(id)
                })?
                .slope;
            }
            let trace = crate::combat_trace::collect(self.sim, self.id, an, range)?;
            let (hit, _, _) =
                crate::combat_trace::hit_with(self.sim, self.id, &trace, range, slope, &|id| {
                    self.lookup(id)
                });
            if hit.is_some() {
                for _ in 0..4 {
                    self.rng.p_random();
                }
            }
            if let Some(TraceObject::Actor(target)) = hit {
                self.sim.plan_weapon_damage(
                    DamageRequest {
                        target,
                        inflictor: Some(self.id),
                        source: Some(self.id),
                        amount: damage,
                    },
                    self.id,
                )?;
                let mut a = self.lookup(target).unwrap();
                let life = a.life.as_mut().unwrap();
                if damage < 40
                    && damage > life.health
                    && a.z - self.actor.z > Fixed::from_map_units(64)
                {
                    self.rng.p_random();
                }
                self.rng.p_random();
                life.health -= damage;
                if life.health <= 0 {
                    life.flags &= !4;
                    a.spec.height = Fixed::from_bits(a.spec.height.bits() >> 2);
                }
                self.damaged.insert(target, a);
                if w == Weapon::Fist {
                    let delta = [
                        a.position()[0] - self.actor.position()[0],
                        a.position()[1] - self.actor.position()[1],
                    ];
                    self.actor.spec.angle = movement::point_angle(delta);
                }
            }
        }
        Ok(())
    }
}
struct Commit<'a> {
    sim: &'a mut Simulation,
    id: ActorId,
}
impl Host for Commit<'_> {
    fn actor(&self) -> Result<Actor, SimError> {
        self.sim
            .actor(self.id)
            .copied()
            .ok_or(WeaponError::Reference.into())
    }
    fn motion(&mut self, command: TicCommand) -> Result<(), SimError> {
        let old = self.actor()?;
        let a = self.sim.actors.get_mut(self.id).unwrap();
        let moved = apply_player_motion(a, command)?;
        if moved
            && (command.forward != 0 || command.side != 0)
            && old
                .life
                .is_some_and(|life| life.state == Some(state("S_PLAY")))
        {
            self.body_state(state("S_PLAY_RUN1"))?;
        }
        Ok(())
    }
    fn body_state(&mut self, s: StateId) -> Result<(), SimError> {
        self.sim.enter_validated(self.id, s);
        Ok(())
    }
    fn event(&mut self, e: WeaponEvent) {
        self.sim.pending.push(Event::Weapon(e));
    }
    fn sound(&mut self, sound: u16) {
        self.sim
            .pending
            .push(Event::ActorState(ActorStateEvent::Sound {
                actor: Some(self.id),
                sound,
            }));
    }
    fn noise(&mut self) -> Result<(), SimError> {
        self.sim.weapon_noise(self.id);
        Ok(())
    }
    fn fire(&mut self, w: Weapon, accurate: bool) -> Result<(), SimError> {
        let mut slope = Fixed::ZERO;
        let angle = self.actor()?.angle();
        let range = Fixed::from_map_units(if w == Weapon::Fist { 64 } else { 2048 });
        if w != Weapon::Fist {
            for an in [
                angle,
                angle.wrapping_add(1 << 26),
                angle.wrapping_sub(1 << 26),
            ] {
                let aim =
                    crate::combat_trace::aim(self.sim, self.id, an, Fixed::from_map_units(1024))
                        .expect("preflight aim");
                slope = aim.slope;
                if aim.target.is_some() {
                    break;
                }
            }
        }
        for _ in 0..if w == Weapon::Shotgun { 7 } else { 1 } {
            let (an, damage) = if w == Weapon::Fist {
                punch_shot(angle, &mut |kind| self.sim.weapon_random(kind))
            } else {
                gun_shot(
                    angle,
                    matches!(w, Weapon::Pistol | Weapon::Chaingun) && accurate,
                    &mut |kind| self.sim.weapon_random(kind),
                )
            };
            if w == Weapon::Fist {
                slope = crate::combat_trace::aim(self.sim, self.id, an, range)
                    .expect("preflight fist aim")
                    .slope;
            }
            self.event(WeaponEvent::Bullet {
                actor: self.id,
                angle: an,
                slope,
                damage,
            });
            let result = self
                .sim
                .weapon_line_attack(
                    self.id,
                    LineAttack {
                        angle: an,
                        range,
                        slope,
                        damage,
                    },
                )
                .expect("preflight weapon attack");
            if w == Weapon::Fist {
                if let Some(TraceObject::Actor(target)) = result.hit {
                    self.sound(lifecycle::source_sound("sfx_punch").unwrap());
                    let to = self.sim.actor(target).unwrap().position();
                    let from = self.actor()?.position();
                    self.sim.actors.get_mut(self.id).unwrap().spec.angle =
                        movement::point_angle([to[0] - from[0], to[1] - from[1]]);
                }
            }
        }
        Ok(())
    }
}
impl Simulation {
    /// Explicit source weapon profile; at most 64 actors and one live player.
    pub fn player_weapons(map: Map) -> Result<Self, SimError> {
        Self::with_weapon_registry(map, 64, ActorRegistry::vanilla(), ActorOptions::default())
    }
    /// Source states are immutable; type 136 may be an explicit player-state fixture.
    pub fn with_weapon_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        if limit > 64 {
            return Err(WeaponError::WorkLimit.into());
        }
        let source = ActorRegistry::vanilla();
        if registry.states() != source.states()
            || registry.types().len() != source.types().len()
            || registry
                .types()
                .iter()
                .enumerate()
                .any(|(i, t)| i != 136 && t != &source.types()[i])
        {
            return Err(WeaponError::Definition.into());
        }
        let mut sim = Self::with_combat_registry(map, limit, registry, options)?;
        sim.profile = Profile::PlayerWeapons;
        sim.sound_targets = vec![None; sim.sectors.len()];
        Ok(sim)
    }
    pub(crate) fn validate_weapon_type(&self, ty: ActorTypeId) -> Result<(), SimError> {
        let registry = self.registry.as_deref().unwrap();
        let info = registry.actor_type(ty).unwrap();
        if info.flags() & 4 != 0
            && (![0, 136].contains(&ty.0) || info.states() != lifecycle::source_player_states())
        {
            return Err(WeaponError::Definition.into());
        }
        Ok(())
    }
    /// Read-only initialized player record, independent of rendering.
    pub fn weapon_player(&self) -> Option<&WeaponPlayer> {
        self.weapon_player.as_ref()
    }
    /// Persistent source noise targets, retained generation-safely after removal.
    pub fn sound_targets(&self) -> &[Option<ActorId>] {
        &self.sound_targets
    }
    /// Initialize the declared LIVE spawn subset on an existing original player pawn.
    pub fn initialize_weapon_player(
        &mut self,
        slot: usize,
        id: ActorId,
        inventory: WeaponInventory,
    ) -> Result<(), SimError> {
        if self.profile != Profile::PlayerWeapons {
            return Err(WeaponError::Profile.into());
        }
        inventory.validate()?;
        if slot >= 4 || self.weapon_player.is_some() || self.bindings.iter().any(Option::is_some) {
            return Err(WeaponError::Player.into());
        }
        let life = self.active_typed(id)?;
        if life.actor_type != ActorTypeId(0)
            || life.health != 100
            || life.state != Some(state("S_PLAY"))
        {
            return Err(WeaponError::Reference.into());
        }
        let player = WeaponPlayer {
            slot: slot as u8,
            actor: id,
            health: 100,
            ready: inventory.ready,
            pending: Some(inventory.ready),
            owned: inventory.owned,
            ammo: inventory.ammo,
            max_ammo: AMMO_CAPACITIES,
            attack_down: true,
            refire: 0,
            bob: Fixed::ZERO,
            extra_light: 0,
            psprites: [Psprite::default(); 2],
        };
        let registry = std::sync::Arc::clone(self.registry.as_ref().unwrap());
        let mut preview = Machine {
            player: player.clone(),
            registry: &registry,
            host: Preview {
                sim: self,
                actor: *self.actor(id).unwrap(),
                id,
                spawns: 0,
                rng: self.rng,
                damaged: BTreeMap::new(),
            },
            buttons: 0,
            leveltime: self.tic,
            work: 0,
        };
        preview.bring_up()?;
        self.bindings[slot] = Some(id);
        self.actors
            .get_mut(id)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .flags |= (slot as u32) << 26;
        self.pending.push(Event::Weapon(WeaponEvent::Initialized {
            slot: slot as u8,
            actor: id,
        }));
        let time = self.tic;
        let mut m = Machine {
            player,
            registry: &registry,
            host: Commit { sim: self, id },
            buttons: 0,
            leveltime: time,
            work: 0,
        };
        m.bring_up().expect("initialization preflight");
        let player = m.player;
        self.weapon_player = Some(player);
        Ok(())
    }
    /// Detach only the weapon/input record; the actor is preserved.
    pub fn detach_weapon_player(&mut self) -> Result<(), SimError> {
        if self.profile != Profile::PlayerWeapons {
            return Err(WeaponError::Profile.into());
        }
        if let Some(p) = self.weapon_player.take() {
            self.bindings[usize::from(p.slot)] = None;
        }
        Ok(())
    }
    pub(crate) fn preflight_weapon_step(
        &mut self,
        commands: &[TicCommand; 4],
    ) -> Result<(), SimError> {
        for (slot, c) in commands.iter().enumerate() {
            if c.buttons & !(BT_ATTACK | BT_CHANGE | BT_WEAPONMASK) != 0
                || c.buttons & BT_WEAPONMASK != 0 && c.buttons & BT_CHANGE == 0
            {
                return Err(SimError::UnsupportedButtons);
            }
            if c.buttons & BT_CHANGE != 0 {
                Weapon::from_number((c.buttons & BT_WEAPONMASK) >> 3)?;
            }
            if c.buttons != 0
                && self
                    .weapon_player
                    .as_ref()
                    .is_none_or(|p| usize::from(p.slot) != slot)
            {
                return Err(WeaponError::Player.into());
            }
        }
        let Some(p) = self.weapon_player.as_ref() else {
            return Ok(());
        };
        let id = p.actor;
        let slot = usize::from(p.slot);
        if self.bindings[slot] != Some(id) || self.actors.is_pending(id) != Some(false) {
            return Err(WeaponError::Reference.into());
        }
        let actor = *self.actor(id).unwrap();
        if actor.life.unwrap().health != 100 || actor.life.unwrap().flags & 4 == 0 {
            return Err(WeaponError::Reference.into());
        }
        if actor
            .momentum()
            .into_iter()
            .any(|v| v.bits().unsigned_abs() > 64 * 65536)
        {
            return Err(WeaponError::Momentum.into());
        }
        let mut m = Machine {
            player: p.clone(),
            registry: self.registry.as_deref().unwrap(),
            host: Preview {
                sim: self,
                actor,
                id,
                spawns: 0,
                rng: self.rng,
                damaged: BTreeMap::new(),
            },
            buttons: commands[slot].buttons,
            leveltime: self.tic,
            work: 0,
        };
        m.tick(commands[slot])?;
        let spawns = m.host.spawns;
        self.reserve_actor_spawns(spawns)?;
        Ok(())
    }
    pub(crate) fn weapon_tick(&mut self, commands: [TicCommand; 4]) {
        let Some(p) = self.weapon_player.take() else {
            return;
        };
        let id = p.actor;
        let slot = usize::from(p.slot);
        let time = self.tic;
        let registry = std::sync::Arc::clone(self.registry.as_ref().unwrap());
        self.pending.push(Event::Player {
            slot: p.slot,
            actor: id,
        });
        let mut m = Machine {
            player: p,
            registry: &registry,
            host: Commit { sim: self, id },
            buttons: commands[slot].buttons,
            leveltime: time,
            work: 0,
        };
        m.tick(commands[slot]).expect("weapon preflight");
        let p = m.player;
        self.weapon_player = Some(p);
    }
    fn weapon_random(&mut self, kind: u8) -> u8 {
        let value = self.rng.p_random();
        self.pending
            .push(Event::Weapon(WeaponEvent::Random { kind, value }));
        value
    }
    fn weapon_noise(&mut self, target: ActorId) {
        self.emit_noise(target, target);
    }
    pub(crate) fn emit_noise(&mut self, target: ActorId, emitter: ActorId) {
        let sector = self.actor(emitter).unwrap().location.unwrap().sector;
        let mut visited = vec![u8::MAX; self.sectors.len()];
        let mut stack = vec![(sector, 0_u8)];
        while let Some((sector, blocks)) = stack.pop() {
            let i = usize::from(sector);
            if visited[i] <= blocks {
                continue;
            }
            visited[i] = blocks;
            self.sound_targets[i] = Some(target);
            self.pending.push(Event::Weapon(WeaponEvent::Noise {
                sector,
                blocks,
                target,
            }));
            let mut children = Vec::new();
            for (line, l) in self.world.map().linedefs().iter().enumerate() {
                if l.flags & 4 == 0 {
                    continue;
                }
                let sectors = l
                    .sides
                    .map(|s| self.world.map().sidedefs()[usize::from(s.unwrap())].sector);
                if !sectors.contains(&sector) {
                    continue;
                }
                if multisector::opening(&self.world, &self.sectors, line)
                    .is_none_or(|o| o.range <= Fixed::ZERO)
                {
                    continue;
                }
                let next = if sectors[0] == sector {
                    sectors[1]
                } else {
                    sectors[0]
                };
                let blocked = u8::from(l.flags & 64 != 0);
                if blocks + blocked <= 1 {
                    children.push((next, blocks + blocked));
                }
            }
            stack.extend(children.into_iter().rev());
        }
    }
}

#[cfg(test)]
#[path = "weapons_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "formula_tests.rs"]
mod formula_tests;
#[cfg(test)]
#[path = "weapon_stage_cost_tests.rs"]
mod weapon_stage_cost_tests;
