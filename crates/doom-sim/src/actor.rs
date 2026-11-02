use doom_types::Fixed;

/// Signed per-tic movement command; buttons are reserved for later gameplay.
#[derive(Clone, Copy, Default, Debug, Eq, PartialEq)]
pub struct TicCommand {
    /// Signed forward thrust units, multiplied by 2048 as in P_MovePlayer.
    pub forward: i8,
    /// Signed sideways thrust units, relative to angle minus 90 degrees.
    pub side: i8,
    /// Signed angle delta in units of 1/65536 revolution.
    pub angle_turn: i16,
    /// Currently must be zero; unsupported actions are rejected, not ignored.
    pub buttons: u8,
}

/// Explicit grounded XY parameters, not a full mobj type. Solidity is supplied
/// separately at spawn; player response follows the selected profile/binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActorSpec {
    /// Horizontal position in raw 16.16 coordinates.
    pub position: [Fixed; 2],
    /// Horizontal momentum before per-tic clamping/friction.
    pub momentum: [Fixed; 2],
    /// Radius, from zero through 32 map units in the supported profile.
    pub radius: Fixed,
    /// Positive height that fits the static sector.
    pub height: Fixed,
    /// Unsigned binary angle, modulo one revolution.
    pub angle: u32,
}
impl Default for ActorSpec {
    fn default() -> Self {
        Self {
            position: [Fixed::ZERO; 2],
            momentum: [Fixed::ZERO; 2],
            radius: Fixed::from_map_units(8),
            height: Fixed::from_map_units(56),
            angle: 0,
        }
    }
}

/// Owned runtime state. Read-only public access prevents unsupported mutations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Actor {
    pub(crate) spec: ActorSpec,
    pub(crate) floor: Fixed,
    pub(crate) ceiling: Fixed,
    pub(crate) z: Fixed,
    pub(crate) lastlook: u8,
    pub(crate) solidity: crate::Solidity,
    pub(crate) momz: Fixed,
    pub(crate) gravity: crate::Gravity,
    pub(crate) allow_dropoff: bool,
    pub(crate) location: Option<crate::Location>,
    pub(crate) life: Option<crate::ActorLife>,
}
impl Actor {
    /// Typed actor-state data, absent in earlier physics-only profiles.
    pub const fn life(&self) -> Option<crate::ActorLife> {
        self.life
    }
    /// Vertical momentum in the M3b profile.
    pub const fn vertical_momentum(&self) -> Fixed {
        self.momz
    }
    /// Current source BSP membership; None in older profiles.
    pub const fn location(&self) -> Option<crate::Location> {
        self.location
    }
    /// Current contacted floor/ceiling constraints, which can differ from the
    /// center sector while straddling an opening.
    pub const fn constraints(&self) -> [Fixed; 2] {
        [self.floor, self.ceiling]
    }
    /// Supported gravity exception.
    pub const fn gravity(&self) -> crate::Gravity {
        self.gravity
    }
    /// MF_DROPOFF permission independent of player binding.
    pub const fn allows_dropoff(&self) -> bool {
        self.allow_dropoff
    }

    /// Explicit collision participation, independent of input binding.
    pub const fn solidity(&self) -> crate::Solidity {
        self.solidity
    }
    /// Current XY position.
    pub const fn position(&self) -> [Fixed; 2] {
        self.spec.position
    }
    /// Current XY momentum.
    pub const fn momentum(&self) -> [Fixed; 2] {
        self.spec.momentum
    }
    /// Unsigned binary angle.
    pub const fn angle(&self) -> u32 {
        self.spec.angle
    }
    /// Vertical coordinate; grounded-only in older profiles, simulated in M3b.
    pub const fn z(&self) -> Fixed {
        self.z
    }
    /// Current floor constraint (`floorz`), from the last position check.
    pub const fn floor(&self) -> Fixed {
        self.floor
    }
    /// Current ceiling constraint (`ceilingz`), from the last position check.
    pub const fn ceiling(&self) -> Fixed {
        self.ceiling
    }
    /// Radius used for destination-box collision.
    pub const fn radius(&self) -> Fixed {
        self.spec.radius
    }
    /// Collision height.
    pub const fn height(&self) -> Fixed {
        self.spec.height
    }
    /// Spawn's gameplay RNG lookup modulo four, matching the reference foundation.
    pub const fn lastlook(&self) -> u8 {
        self.lastlook
    }
}
