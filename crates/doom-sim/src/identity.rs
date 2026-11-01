//! Canonical Linux DOOM 1.10 metadata identities, separate from registry slots.
//!
//! Canonical identities cannot be used implicitly as registry-local candidates:
//! ```compile_fail
//! use doom_sim::{identity::OriginalActorType, ActorTypeId};
//! let slot: ActorTypeId = OriginalActorType::MT_SERGEANT;
//! ```
//!
//! Recognition never grants execution. Registry conversions compare one complete
//! record at its original ordinal; they do not certify downstream records or game
//! context. All existing profile admission checks still apply.
use crate::lifecycle::{SOURCE_ACTIONS, SOURCE_SPRITES, SOURCE_STATES, SOURCE_TYPES};
use crate::{ActionId, ActorAction, ActorRegistry, ActorTypeId, StateId};
use std::{error::Error, fmt};
mod names;

/// Rejected canonical value or a registry record that is not the original record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityError {
    /// Outside the original enumeration (including NUM* count sentinels).
    OutOfRange,
    /// Missing, replaced, renamed or altered record at the canonical ordinal.
    RegistryMismatch,
}
impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "original identity: {self:?}")
    }
}
impl Error for IdentityError {}

macro_rules! identity {
    ($ty:ident, $table:ident, $name:expr) => {
        const _: () = assert!($table.len() <= u16::MAX as usize);
        #[doc = $name]
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $ty(u16);
        impl $ty {
            /// Number of original entries; this value itself is not an identity.
            pub const COUNT: u16 = $table.len() as u16;
            /// Original zero-based enum ordinal, never a registry-local alias.
            pub const fn ordinal(self) -> u16 {
                self.0
            }
            /// All original identities in source order, without allocation.
            pub fn all() -> impl ExactSizeIterator<Item = Self> + DoubleEndedIterator {
                (0..Self::COUNT).map(Self)
            }
            /// Exact original symbolic name; case sensitive, with no aliases.
            pub fn from_name(name: &str) -> Option<Self> {
                Self::all().find(|id| id.name() == name)
            }
        }
        impl TryFrom<u16> for $ty {
            type Error = IdentityError;
            fn try_from(value: u16) -> Result<Self, Self::Error> {
                if value < Self::COUNT {
                    Ok(Self(value))
                } else {
                    Err(IdentityError::OutOfRange)
                }
            }
        }
        impl TryFrom<i32> for $ty {
            type Error = IdentityError;
            fn try_from(value: i32) -> Result<Self, Self::Error> {
                Self::try_from(u16::try_from(value).map_err(|_| IdentityError::OutOfRange)?)
            }
        }
        impl From<$ty> for u16 {
            fn from(value: $ty) -> Self {
                value.0
            }
        }
    };
}
identity!(
    OriginalActorType,
    SOURCE_TYPES,
    "Validated original mobjtype_t; metadata recognition only."
);
identity!(
    OriginalState,
    SOURCE_STATES,
    "Validated original statenum_t; S_NULL is a valid sentinel identity."
);
identity!(
    OriginalSprite,
    SOURCE_SPRITES,
    "Validated original spritenum_t; no image resource is loaded."
);

/// Eight original actor-state relationships. S_NULL means absent where applicable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OriginalActorStates {
    /// Raw spawn-state assignment, without action execution.
    pub spawn: OriginalState,
    /// Wake-up/pursuit sequence.
    pub see: OriginalState,
    /// Pain response.
    pub pain: OriginalState,
    /// Close-range attack sequence.
    pub melee: OriginalState,
    /// Ranged attack sequence.
    pub missile: OriginalState,
    /// Ordinary death sequence.
    pub death: OriginalState,
    /// Extreme death sequence.
    pub extreme_death: OriginalState,
    /// Resurrection sequence; representation does not admit resurrection.
    pub raise: OriginalState,
}

/// Original actor information in source units; metadata does not authorize behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OriginalActorDef {
    /// DoomEd number; -1 excludes ordinary map lookup.
    pub doomed_number: i32,
    /// All eight state relationships, retaining S_NULL sentinels.
    pub states: OriginalActorStates,
    /// Initial health.
    pub spawn_health: i32,
    /// Initial reaction time outside Nightmare.
    pub reaction_time: i32,
    /// Pain probability threshold.
    pub pain_chance: i32,
    /// See, attack, pain, death and active sound ordinals, as inert metadata.
    pub sounds: [u16; 5],
    /// Source speed; units depend on type (not universally fixed-point).
    pub speed: i32,
    /// Raw 16.16 radius.
    pub radius: i32,
    /// Raw 16.16 height.
    pub height: i32,
    /// Source mass.
    pub mass: i32,
    /// Source damage factor.
    pub damage: i32,
    /// Original MF_* bits; recognition does not enable unsupported flags.
    pub flags: u32,
}

/// Original immutable state descriptor. A callback symbol is metadata, not dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OriginalStateDef {
    /// Canonical sprite identity.
    pub sprite: OriginalSprite,
    /// Frame including original fullbright bits.
    pub frame: u32,
    /// -1 infinite, 0 immediate entry, positive countdown; raw spawn differs.
    pub tics: i32,
    /// Original function symbol, or None for a NULL pointer. No original action ordinal exists.
    pub action: Option<&'static str>,
    /// Original next reference; infinite states need not follow it.
    pub next: OriginalState,
    /// Preserved miscellaneous fields.
    pub misc: [i32; 2],
}
impl OriginalActorType {
    /// Original MT_* symbol.
    pub fn name(self) -> &'static str {
        SOURCE_TYPES[usize::from(self.0)].0
    }
    /// All 23 original info.h fields, in declaration order and source units.
    /// This borrowed metadata has no registry allocation or execution capability.
    pub fn properties(self) -> &'static [i32; 23] {
        &SOURCE_TYPES[usize::from(self.0)].1
    }
    /// Named state references; every value is validated original metadata.
    pub fn states(self) -> OriginalActorStates {
        let p = self.properties();
        let state = |i| OriginalState::try_from(p[i]).expect("verified original state reference");
        OriginalActorStates {
            spawn: state(1),
            see: state(3),
            pain: state(7),
            melee: state(10),
            missile: state(11),
            death: state(12),
            extreme_death: state(13),
            raise: state(22),
        }
    }
    /// Named original actor information; all fields retain source units.
    pub fn descriptor(self) -> OriginalActorDef {
        let p = self.properties();
        OriginalActorDef {
            doomed_number: p[0],
            states: self.states(),
            spawn_health: p[2],
            reaction_time: p[5],
            pain_chance: p[8],
            sounds: [
                u16::try_from(p[4]).expect("verified original sound"),
                u16::try_from(p[6]).expect("verified original sound"),
                u16::try_from(p[9]).expect("verified original sound"),
                u16::try_from(p[14]).expect("verified original sound"),
                u16::try_from(p[20]).expect("verified original sound"),
            ],
            speed: p[15],
            radius: p[16],
            height: p[17],
            mass: p[18],
            damage: p[19],
            flags: u32::try_from(p[21]).expect("verified original flags"),
        }
    }
    /// Resolve one exact record in an owned registry. Execution admission is separate.
    pub fn resolve(self, registry: &ActorRegistry) -> Result<ActorTypeId, IdentityError> {
        let id = ActorTypeId(self.0);
        match registry.actor_type(id) {
            Some(t) if t.name == self.name() && &t.properties == self.properties() => Ok(id),
            _ => Err(IdentityError::RegistryMismatch),
        }
    }
}
impl OriginalState {
    /// Original S_* symbol.
    pub fn name(self) -> &'static str {
        SOURCE_STATES[usize::from(self.0)].0
    }
    /// Exact original descriptor. Private static source data is independently checked against C.
    pub fn descriptor(self) -> OriginalStateDef {
        let v = SOURCE_STATES[usize::from(self.0)].1;
        OriginalStateDef {
            sprite: OriginalSprite::try_from(v[0]).expect("verified original sprite reference"),
            frame: u32::try_from(v[1]).expect("verified original frame bits"),
            tics: v[2],
            action: if v[3] == 0 {
                None
            } else {
                Some(SOURCE_ACTIONS[usize::try_from(v[3]).expect("verified action reference")])
            },
            next: Self::try_from(v[4]).expect("verified original next reference"),
            misc: [v[5], v[6]],
        }
    }
    /// Resolve one complete state record, including next reference and callback.
    /// This does not certify the records reached by that next reference.
    pub fn resolve(self, registry: &ActorRegistry) -> Result<StateId, IdentityError> {
        let id = StateId(self.0);
        let d = self.descriptor();
        let v = SOURCE_STATES[usize::from(self.0)].1;
        let action = if v[3] == 0 {
            ActorAction::None
        } else {
            ActorAction::Original(ActionId(
                u16::try_from(v[3]).expect("verified action identity"),
            ))
        };
        match registry.state(id) {
            Some(s)
                if s.name == self.name()
                    && s.sprite == d.sprite.ordinal()
                    && s.frame == d.frame
                    && s.tics == d.tics
                    && s.action == action
                    && s.next.0 == d.next.ordinal()
                    && s.misc == d.misc =>
            {
                Ok(id)
            }
            _ => Err(IdentityError::RegistryMismatch),
        }
    }
}
impl OriginalSprite {
    /// Original SPR_* symbol (resource names are the suffix after SPR_).
    pub fn name(self) -> &'static str {
        SOURCE_SPRITES[usize::from(self.0)]
    }
    /// Original four-character sprite resource name; metadata only.
    pub fn resource_name(self) -> &'static str {
        &self.name()[4..]
    }
}
impl ActorRegistry {
    /// Explicitly identify one original actor record; native replacements reject.
    pub fn original_actor_type(&self, id: ActorTypeId) -> Result<OriginalActorType, IdentityError> {
        let original = OriginalActorType::try_from(id.0)?;
        original.resolve(self)?;
        Ok(original)
    }
    /// Explicitly identify one original state record, without granting execution.
    pub fn original_state(&self, id: StateId) -> Result<OriginalState, IdentityError> {
        let original = OriginalState::try_from(id.0)?;
        original.resolve(self)?;
        Ok(original)
    }
}
