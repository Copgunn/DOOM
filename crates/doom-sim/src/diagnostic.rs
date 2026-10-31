//! Bounded, read-only DSIM20–26 observations with explicit version readers. Never a world loader.
use crate::{identity::*, *};
use std::{error::Error, fmt, sync::OnceLock};

/// Explicit resource budgets, applied before allocation or untrusted length use.
#[derive(Clone, Copy, Debug)]
pub struct DiagnosticLimits {
    /// Maximum complete input/output bytes.
    pub bytes: usize,
    /// Maximum named fields.
    pub fields: usize,
    /// Maximum UTF-8 path bytes.
    pub path: usize,
    /// Maximum individual opaque value bytes.
    pub value: usize,
}
impl Default for DiagnosticLimits {
    fn default() -> Self {
        Self {
            bytes: 64 * 1024 * 1024,
            fields: 262_144,
            path: 192,
            value: 64 * 1024 * 1024,
        }
    }
}
/// Explicit rejection; diagnostics never cause implicit profile selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticError {
    /// Malformed framing, widths, UTF-8, duplicate paths or missing required fields.
    Structure,
    /// Unsupported diagnostic format.
    Version(u32),
    /// Unknown/incompatible execution profile or immutable registry/context schema.
    Schema,
    /// Invalid definition/state identity or live handle representation.
    Identity,
    /// Caller resource budget, arithmetic or fallible allocation failed.
    Limit,
}
impl fmt::Display for DiagnosticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "diagnostic: {self:?}")
    }
}
impl Error for DiagnosticError {}
/// Version-specific observations; native slots never acquire canonical meaning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticIdentity {
    /// Historical v20 runtime storage, interpreted only in that namespace.
    Native {
        /// Native type slot.
        actor: ActorTypeId,
        /// Native state slot or unavailable.
        state: Option<StateId>,
    },
    /// V21 actual source/context definition and source state.
    Original {
        /// Checked original/context identity.
        definition: DefinitionIdentity,
        /// Original state or unavailable.
        state: Option<OriginalState>,
    },
}
#[derive(Clone, Copy, Debug)]
struct Field<'a> {
    path: &'a str,
    value: &'a [u8],
}
/// Borrowed validated framing and identity observations, preserving encoded order.
///
/// Physical/map values and non-identity event payloads remain opaque. This is
/// neither a full gameplay validator nor a savegame restoration API.
#[derive(Debug)]
pub struct Diagnostic<'a> {
    bytes: &'a [u8],
    version: u32,
    fields: Vec<Field<'a>>,
    sorted: Vec<usize>,
}
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DiagnosticError> {
        let end = self.at.checked_add(n).ok_or(DiagnosticError::Limit)?;
        let out = self
            .bytes
            .get(self.at..end)
            .ok_or(DiagnosticError::Structure)?;
        self.at = end;
        Ok(out)
    }
    fn u32(&mut self) -> Result<u32, DiagnosticError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| DiagnosticError::Structure)?,
        ))
    }
    fn usize64(&mut self) -> Result<usize, DiagnosticError> {
        usize::try_from(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| DiagnosticError::Structure)?,
        ))
        .map_err(|_| DiagnosticError::Limit)
    }
}
fn number<const N: usize>(value: &[u8]) -> Result<[u8; N], DiagnosticError> {
    value.try_into().map_err(|_| DiagnosticError::Structure)
}
fn reserve<T>(n: usize) -> Result<Vec<T>, DiagnosticError> {
    let mut out = Vec::new();
    out.try_reserve_exact(n)
        .map_err(|_| DiagnosticError::Limit)?;
    Ok(out)
}
impl<'a> Diagnostic<'a> {
    /// Read only v20/v21 with explicit identity/schema validation and bounded allocation.
    pub fn decode(bytes: &'a [u8], limits: DiagnosticLimits) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, None)
    }
    /// Explicit DSIM-v22 player-body reader. Historical decode still rejects22.
    /// Shares bounded framing; observations never restore executable worlds.
    pub fn decode_player(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(22))
    }
    /// Explicit DSIM23 reader for player-owned pistol observations.
    /// Historical readers continue rejecting this separate field layout.
    pub fn decode_pistol(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(23))
    }
    /// Explicit DSIM25 world-pickup reader; older readers retain their version gate.
    pub fn decode_pickups(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(25))
    }
    /// Explicit DSIM24 reader for the original four-weapon capability.
    /// Decoding validates observations, never restores an executable world.
    pub fn decode_weapons(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(24))
    }
    /// Explicit DSIM26 original armor reader; historical entry points reject26.
    pub fn decode_armor(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
    ) -> Result<Self, DiagnosticError> {
        Self::decode_selected(bytes, limits, Some(26))
    }
    fn decode_selected(
        bytes: &'a [u8],
        limits: DiagnosticLimits,
        selected: Option<u32>,
    ) -> Result<Self, DiagnosticError> {
        if bytes.len() > limits.bytes {
            return Err(DiagnosticError::Limit);
        }
        let mut cursor = Cursor { bytes, at: 0 };
        if cursor.take(4)? != b"DSIM" {
            return Err(DiagnosticError::Structure);
        }
        let version = cursor.u32()?;
        if selected.map_or(!matches!(version, 20 | 21), |expected| version != expected) {
            return Err(DiagnosticError::Version(version));
        }
        let count = cursor.usize64()?;
        // Every field needs at least its 12 framing bytes, before reserving.
        if count > limits.fields || count > bytes.len().saturating_sub(cursor.at) / 12 {
            return Err(DiagnosticError::Limit);
        }
        let mut fields = reserve(count)?;
        for _ in 0..count {
            let len = usize::try_from(cursor.u32()?).map_err(|_| DiagnosticError::Limit)?;
            if len == 0 || len > limits.path {
                return Err(DiagnosticError::Limit);
            }
            let path =
                std::str::from_utf8(cursor.take(len)?).map_err(|_| DiagnosticError::Structure)?;
            let len = cursor.usize64()?;
            if len > limits.value {
                return Err(DiagnosticError::Limit);
            }
            fields.push(Field {
                path,
                value: cursor.take(len)?,
            });
        }
        if cursor.at != bytes.len() {
            return Err(DiagnosticError::Structure);
        }
        let mut sorted = reserve(count)?;
        sorted.extend(0..count);
        sorted.sort_unstable_by_key(|i| fields[*i].path);
        if sorted
            .windows(2)
            .any(|p| fields[p[0]].path == fields[p[1]].path)
        {
            return Err(DiagnosticError::Structure);
        }
        let result = Self {
            bytes,
            version,
            fields,
            sorted,
        };
        result.validate()?;
        Ok(result)
    }
    /// Historical-reader contract: a newer original-ID representation must reject.
    pub fn decode_v20(bytes: &'a [u8], limits: DiagnosticLimits) -> Result<Self, DiagnosticError> {
        let result = Self::decode(bytes, limits)?;
        if result.version != 20 {
            return Err(DiagnosticError::Version(result.version));
        }
        Ok(result)
    }
    /// Explicit format version, separate from the execution profile field.
    pub const fn version(&self) -> u32 {
        self.version
    }
    /// Borrow a field without reordering the original capture/event sequence.
    pub fn field(&self, path: &str) -> Option<&'a [u8]> {
        self.sorted
            .binary_search_by_key(&path, |i| self.fields[*i].path)
            .ok()
            .map(|i| self.fields[self.sorted[i]].value)
    }
    fn required(&self, path: &str) -> Result<&'a [u8], DiagnosticError> {
        self.field(path).ok_or(DiagnosticError::Structure)
    }
    fn slots(&self) -> Result<usize, DiagnosticError> {
        let count = u64::from_le_bytes(number(self.required("store.slot_count")?)?);
        if count > 64 {
            return Err(DiagnosticError::Limit);
        }
        usize::try_from(count).map_err(|_| DiagnosticError::Limit)
    }
    fn identity(&self, ty: u16, state: u16) -> Result<DiagnosticIdentity, DiagnosticError> {
        let actor = ActorTypeId(ty);
        let state = (state != u16::MAX).then_some(StateId(state));
        if matches!(self.version, 21..=26) {
            let policy = self.policy();
            original_registry::require_role_for(policy, actor)
                .map_err(|_| DiagnosticError::Identity)?;
            if let Some(s) = state {
                original_registry::state_role_for(policy, actor, s)
                    .map_err(|_| DiagnosticError::Identity)?;
            }
            Ok(DiagnosticIdentity::Original {
                definition: original_registry::identity(actor)
                    .map_err(|_| DiagnosticError::Identity)?,
                state: state
                    .map(|s| OriginalState::try_from(s.0).map_err(|_| DiagnosticError::Identity))
                    .transpose()?,
            })
        } else {
            let valid = match ty {
                136 => state.is_none_or(|s| NativeDemonState::checked(s).is_some()),
                135 => state.is_none_or(|s| (149..=173).contains(&s.0)),
                37 | 38 => state.is_none_or(|s| (90..=96).contains(&s.0)),
                _ => false,
            };
            if !valid {
                return Err(DiagnosticError::Identity);
            }
            Ok(DiagnosticIdentity::Native { actor, state })
        }
    }
    /// Resolve only a currently occupied slot with the exact generation.
    /// Stale references and retired/reused slots never redirect to another actor.
    pub fn actor(&self, handle: ActorId) -> Result<Option<DiagnosticIdentity>, DiagnosticError> {
        let slot = usize::try_from(handle.slot()).map_err(|_| DiagnosticError::Identity)?;
        if slot >= self.slots()? {
            return Ok(None);
        }
        let prefix = format!("actor.{slot}");
        let generation =
            u64::from_le_bytes(number(self.required(&format!("{prefix}.generation"))?)?);
        if generation != handle.generation() || self.required(&format!("{prefix}.occupied"))? == [0]
        {
            return Ok(None);
        }
        let ty = u16::from_le_bytes(number(self.required(&format!("{prefix}.type"))?)?);
        let state = u16::from_le_bytes(number(self.required(&format!("{prefix}.state"))?)?);
        self.identity(ty, state).map(Some)
    }
    fn policy(&self) -> OriginalExecutionProfile {
        if self.version == 26 {
            return OriginalExecutionProfile::PlayerDemonArmorV1;
        }
        if self.version == 25 {
            return OriginalExecutionProfile::PlayerDemonPickupsV1;
        }
        if self.version == 24 {
            return OriginalExecutionProfile::PlayerDemonWeaponsV1;
        }
        if self.version == 23 {
            return OriginalExecutionProfile::PlayerDemonPistolV1;
        }
        if matches!(self.version, 22..=26)
            && self.field("profile")
                == Some(
                    OriginalExecutionProfile::PlayerDemonMeleeV1
                        .wire_id()
                        .to_le_bytes()
                        .as_slice(),
                )
        {
            OriginalExecutionProfile::PlayerDemonMeleeV1
        } else if matches!(self.version, 22..=26)
            && self.field("profile")
                == Some(
                    OriginalExecutionProfile::PlayerDemonAcquisitionV1
                        .wire_id()
                        .to_le_bytes()
                        .as_slice(),
                )
        {
            OriginalExecutionProfile::PlayerDemonAcquisitionV1
        } else if matches!(self.version, 22..=26)
            && self.field("profile")
                == Some(
                    OriginalExecutionProfile::PlayerCommandMovementV1
                        .wire_id()
                        .to_le_bytes()
                        .as_slice(),
                )
        {
            OriginalExecutionProfile::PlayerCommandMovementV1
        } else if matches!(self.version, 22..=26)
            && self.field("profile")
                == Some(
                    OriginalExecutionProfile::PlayerBodyKnockbackV1
                        .wire_id()
                        .to_le_bytes()
                        .as_slice(),
                )
        {
            OriginalExecutionProfile::PlayerBodyKnockbackV1
        } else if matches!(self.version, 22..=26) {
            OriginalExecutionProfile::PlayerBodyLifecycleV1
        } else {
            OriginalExecutionProfile::DemonNormalV1
        }
    }
    fn validate(&self) -> Result<(), DiagnosticError> {
        let profile = u32::from_le_bytes(number(self.required("profile")?)?);
        if u32::from_le_bytes(number(self.required("tic_rate")?)?) != 35 {
            return Err(DiagnosticError::Schema);
        }
        number::<8>(self.required("tic")?)?;
        if matches!(self.version, 21..=26) {
            if profile != self.policy().wire_id()
                || self.required("registry.schema")? != ORIGINAL_REGISTRY_SCHEMA
                || self.required("context.schema")? != self.policy().context_schema()
                || self
                    .fields
                    .iter()
                    .any(|f| f.path.starts_with("registry.") && f.path != "registry.schema")
            {
                return Err(DiagnosticError::Schema);
            }
        } else {
            if profile != 20
                || self.field("registry.schema").is_some()
                || self.field("context.schema").is_some()
            {
                return Err(DiagnosticError::Schema);
            }
            // Old full metadata is meaningful only in its old namespace.
            static LEGACY: OnceLock<Vec<StateField>> = OnceLock::new();
            let expected = LEGACY.get_or_init(|| {
                let mut fields = Vec::new();
                snapshot::registry_fields(&mut fields, &ActorRegistry::demon_map_context_fixture());
                fields
            });
            for f in expected {
                if self.required(f.path())? != f.value() {
                    return Err(DiagnosticError::Schema);
                }
            }
        }
        if u32::from_le_bytes(number(self.required("store.max_slots")?)?) > 64 {
            return Err(DiagnosticError::Limit);
        }
        for slot in 0..self.slots()? {
            let prefix = format!("actor.{slot}");
            let generation =
                u64::from_le_bytes(number(self.required(&format!("{prefix}.generation"))?)?);
            if generation == 0 {
                return Err(DiagnosticError::Identity);
            }
            match self.required(&format!("{prefix}.occupied"))? {
                [0] => {}
                [1] => {
                    self.actor(ActorId::from_parts(
                        u32::try_from(slot).map_err(|_| DiagnosticError::Limit)?,
                        generation,
                    ))?;
                }
                _ => return Err(DiagnosticError::Structure),
            }
        }
        // Handles identify instances, never definitions. Unavailable target/roster
        // references may be stale, but their bounded slot and width remain checked.
        for f in &self.fields {
            let optional = f.path == "store.head"
                || f.path == "store.tail"
                || f.path.ends_with(".target")
                || f.path.ends_with(".soundtarget")
                || f.path.ends_with(".previous")
                || f.path.ends_with(".next")
                || matches!(self.version, 22..=26)
                    && matches!(
                        f.path,
                        "original_player.0.mo" | "original_player.0.attacker"
                    )
                || f.path.starts_with("player.") && f.path.ends_with(".actor")
                || f.path.starts_with("perception.player.") && f.path.ends_with(".actor");
            if optional {
                let value = number::<13>(f.value)?;
                match value[0] {
                    0 if value[1..].iter().all(|b| *b == 0) => {}
                    1 => validate_handle(&value[1..])?,
                    _ => return Err(DiagnosticError::Identity),
                }
            }
            if f.path == "thinker.order"
                || f.path == "spatial.link_order"
                || f.path.starts_with("sector.") && f.path.ends_with(".actors")
            {
                if f.value.len() % 12 != 0 || f.value.len() / 12 > 64 {
                    return Err(DiagnosticError::Identity);
                }
                for handle in f.value.chunks_exact(12) {
                    validate_handle(handle)?;
                }
            }
            if let Some(rest) = f.path.strip_prefix("actor.") {
                let (slot, _) = rest.split_once('.').ok_or(DiagnosticError::Structure)?;
                let slot: usize = slot.parse().map_err(|_| DiagnosticError::Structure)?;
                if slot >= self.slots()? {
                    return Err(DiagnosticError::Identity);
                }
            }
        }
        for prefix in ["events.last", "events.pending"] {
            let count = u64::from_le_bytes(number(self.required(&format!("{prefix}.count"))?)?);
            let count = usize::try_from(count).map_err(|_| DiagnosticError::Limit)?;
            if count > self.fields.len() {
                return Err(DiagnosticError::Limit);
            }
            for i in 0..count {
                let event = self.required(&format!("{prefix}.{i}"))?;
                let tag = *event.first().ok_or(DiagnosticError::Structure)?;
                if tag
                    > if self.version == 26 {
                        24
                    } else if self.version == 25 {
                        23
                    } else if matches!(self.version, 22..=26) {
                        22
                    } else {
                        21
                    }
                {
                    return Err(DiagnosticError::Structure);
                }
                if tag == 23 {
                    validate_pickup_event(event, self.version == 26)?;
                }
                if tag == 24 {
                    validate_armor_event(event)?;
                    let mo = self.required("original_player.0.mo")?;
                    if mo.len() != 13 || mo[0] != 1 || mo[1..] != event[1..13] {
                        return Err(DiagnosticError::Identity);
                    }
                }
                if tag == 22 {
                    validate_player_event(event, matches!(self.version, 24..=26))?;
                }
                if tag == 14 && matches!(self.version, 23..=26) {
                    validate_weapon_event(event, matches!(self.version, 24..=26))?;
                }
                // Validate identity-bearing payloads without interpreting nested
                // untrusted collision/sight counts or allocating from them.
                let reference = match event.get(..2) {
                    Some([12, 0]) => Some((14, true)),
                    Some([12, 1] | [16, 8] | [13, 6] | [13, 7]) => Some((14, false)),
                    Some([18, 3]) => Some((15, false)),
                    _ => None,
                };
                if let Some((offset, is_type)) = reference {
                    let value = u16::from_le_bytes(number(
                        event
                            .get(offset..offset + 2)
                            .ok_or(DiagnosticError::Structure)?,
                    )?);
                    if is_type {
                        self.identity(value, u16::MAX)?;
                    } else if matches!(self.version, 21..=26) {
                        let state = OriginalState::try_from(value)
                            .map_err(|_| DiagnosticError::Identity)?;
                        // Entering S_NULL is a source removal event, not a live
                        // executable state. Real pistol impact thinkers expose
                        // this sentinel in23; historical readers stay unchanged.
                        if matches!(self.version, 23..=26)
                            && state == OriginalState::S_NULL
                            && event.get(..2) == Some(&[12, 1])
                        {
                            continue;
                        }
                        // Any admitted normal role; never original alias interpretation.
                        if ![
                            OriginalActorType::MT_SERGEANT,
                            OriginalActorType::MT_PUFF,
                            OriginalActorType::MT_BLOOD,
                            OriginalActorType::MT_PLAYER,
                            OriginalActorType::MT_CLIP,
                            OriginalActorType::MT_MISC17,
                            OriginalActorType::MT_MISC22,
                            OriginalActorType::MT_MISC23,
                            OriginalActorType::MT_MISC10,
                            OriginalActorType::MT_MISC11,
                            OriginalActorType::MT_SHOTGUN,
                            OriginalActorType::MT_CHAINGUN,
                            OriginalActorType::MT_MISC0,
                            OriginalActorType::MT_MISC1,
                        ]
                        .into_iter()
                        .any(|ty| {
                            let slot = if ty == OriginalActorType::MT_PLAYER && self.version == 21 {
                                original_registry::body_slot()
                            } else {
                                ActorTypeId(ty.ordinal())
                            };
                            original_registry::state_role_for(
                                self.policy(),
                                slot,
                                StateId(state.ordinal()),
                            )
                            .is_ok()
                        }) {
                            return Err(DiagnosticError::Identity);
                        }
                    }
                }
            }
        }
        if matches!(self.version, 22..=26) {
            self.validate_player()?;
        }
        if self.version == 23 {
            self.validate_pistol()?;
        }
        if matches!(self.version, 24..=26) {
            self.validate_weapons()?;
        }
        if self.version == 26 {
            let kind = self.required("original_player.0.armor.type")?;
            if kind.len() != 1 {
                return Err(DiagnosticError::Structure);
            }
            diagnostic_armor(kind[0], self.required("original_player.0.armor.points")?)?;
        }
        if matches!(self.version, 25 | 26) {
            self.validate_pickup_items()?;
            if self.required("original_player.0.pickups.present")? != [1] {
                return Err(DiagnosticError::Structure);
            }
            let bonus = i32::from_le_bytes(number(
                self.required("original_player.0.pickups.bonus_count")?,
            )?);
            let items = i32::from_le_bytes(number(
                self.required("original_player.0.pickups.item_count")?,
            )?);
            let message = self.required("original_player.0.pickups.message")?;
            if !(0..=world_pickups::MAX_BONUS).contains(&bonus)
                || items != 0
                || !(matches!(message, [0..=6] | [8..=9])
                    || (self.version == 26 && matches!(message, [10..=11])))
            {
                return Err(DiagnosticError::Structure);
            }
        }
        Ok(())
    }
    fn validate_pickup_items(&self) -> Result<(), DiagnosticError> {
        let mut count = 0;
        for slot in 0..self.slots()? {
            let prefix = format!("actor.{slot}");
            if self.required(&format!("{prefix}.occupied"))? != [1] {
                continue;
            }
            let ty = ActorTypeId(u16::from_le_bytes(number(
                self.required(&format!("{prefix}.type"))?,
            )?));
            let Some(k) = world_pickups::kind_for(self.policy(), ty) else {
                continue;
            };
            count += 1;
            let state = OriginalState::try_from(u16::from_le_bytes(number(
                self.required(&format!("{prefix}.state"))?,
            )?))
            .map_err(|_| DiagnosticError::Identity)?;
            if !world_pickups::state_allowed(ty, state) {
                return Err(DiagnosticError::Identity);
            }
            let descriptor = state.descriptor();
            let i32field = |name: &str| -> Result<i32, DiagnosticError> {
                Ok(i32::from_le_bytes(number(
                    self.required(&format!("{prefix}.{name}"))?,
                )?))
            };
            let flags = u32::from_le_bytes(number(self.required(&format!("{prefix}.flags"))?)?);
            let spawn = self.required(&format!("{prefix}.spawnpoint"))?;
            let dropped = flags & world_pickups::MF_DROPPED != 0;
            if u16::from_le_bytes(number(self.required(&format!("{prefix}.state"))?)?)
                != state.ordinal()
                || u16::from_le_bytes(number(self.required(&format!("{prefix}.sprite"))?)?)
                    != descriptor.sprite.ordinal()
                || u32::from_le_bytes(number(self.required(&format!("{prefix}.frame"))?)?)
                    != descriptor.frame
                || (if k.is_armor() {
                    !(1..=descriptor.tics).contains(&i32field("tics")?)
                } else {
                    i32field("tics")? != -1
                })
                || i32field("health")? != 1000
                || i32field("radius")? != 20 * 65536
                || i32field("height")? != 16 * 65536
                || i32field("momx")? != 0
                || i32field("momy")? != 0
                || i32field("momz")? != 0
                || i32field("z")? != 0
                || i32field("floor")? != 0
                || i32field("ceiling")? != 128 * 65536
                || self.required(&format!("{prefix}.target"))? != [0; 13]
                || flags
                    & !(world_pickups::MF_SPECIAL
                        | world_pickups::MF_AMBUSH
                        | world_pickups::MF_DROPPED)
                    != 0
                || flags & world_pickups::MF_SPECIAL == 0
                || (dropped
                    && (spawn != [0]
                        || flags & world_pickups::MF_AMBUSH != 0
                        || !matches!(
                            k,
                            PickupKind::Clip | PickupKind::Shotgun | PickupKind::Chaingun
                        )))
                || (!dropped
                    && (spawn.len() != 11
                        || spawn[0] != 1
                        || i32::from(i16::from_le_bytes(number(&spawn[7..9])?))
                            != k.original_type().descriptor().doomed_number))
            {
                return Err(DiagnosticError::Identity);
            }
        }
        if count > MAX_WORLD_PICKUPS {
            return Err(DiagnosticError::Limit);
        }
        Ok(())
    }
    fn validate_weapons(&self) -> Result<(), DiagnosticError> {
        let prefix = "original_player.0.weapons";
        if self
            .fields
            .iter()
            .any(|f| f.path.starts_with("original_player.0.pistol"))
        {
            return Err(DiagnosticError::Schema);
        }
        match self.required(&format!("{prefix}.present"))? {
            [0] => {
                if self
                    .fields
                    .iter()
                    .any(|f| f.path.starts_with(prefix) && !f.path.ends_with(".present"))
                {
                    return Err(DiagnosticError::Schema);
                }
                return Ok(());
            }
            [1] => {}
            _ => return Err(DiagnosticError::Structure),
        }
        let byte = |key: &str| -> Result<u8, DiagnosticError> {
            Ok(number::<1>(self.required(&format!("{prefix}.{key}"))?)?[0])
        };
        let word = |key: &str| -> Result<u16, DiagnosticError> {
            Ok(u16::from_le_bytes(number(
                self.required(&format!("{prefix}.{key}"))?,
            )?))
        };
        let int = |key: &str| -> Result<i32, DiagnosticError> {
            Ok(i32::from_le_bytes(number(
                self.required(&format!("{prefix}.{key}"))?,
            )?))
        };
        let ready =
            crate::Weapon::from_number(byte("ready")?).map_err(|_| DiagnosticError::Identity)?;
        let pending = match byte("pending")? {
            crate::player_weapons::NO_CHANGE => None,
            w => Some(crate::Weapon::from_number(w).map_err(|_| DiagnosticError::Identity)?),
        };
        let latch = match byte("attack_down")? {
            0 => false,
            1 => true,
            _ => return Err(DiagnosticError::Schema),
        };
        let mut psprites = [crate::Psprite::default(); 2];
        for (i, ps) in psprites.iter_mut().enumerate() {
            let state = word(&format!("psprite.{i}.state"))?;
            ps.state = if state == u16::MAX {
                None
            } else {
                Some(crate::StateId(state))
            };
            ps.tics = int(&format!("psprite.{i}.tics"))?;
            ps.position = [
                crate::Fixed::from_bits(int(&format!("psprite.{i}.x"))?),
                crate::Fixed::from_bits(int(&format!("psprite.{i}.y"))?),
            ];
        }
        let state = crate::PlayerWeaponState {
            ready,
            pending,
            owned: word("owned")?,
            bullets: word("bullets")?,
            shells: word("shells")?,
            attack_down: latch,
            refire: int("refire")?,
            bob: crate::Fixed::from_bits(int("bob")?),
            extra_light: int("extra_light")?,
            psprites,
        };
        state.validate().map_err(|_| DiagnosticError::Schema)?;
        if self.required("original_player.0.state")? == [1]
            && state.psprites[0].state != Some(state.ready.states()[1])
        {
            return Err(DiagnosticError::Schema);
        }
        Ok(())
    }
    fn validate_pistol(&self) -> Result<(), DiagnosticError> {
        let prefix = "original_player.0.pistol";
        match self.required(&format!("{prefix}.present"))? {
            [0] => {
                if self
                    .fields
                    .iter()
                    .any(|f| f.path.starts_with(prefix) && !f.path.ends_with(".present"))
                {
                    return Err(DiagnosticError::Schema);
                }
                return Ok(());
            }
            [1] => {}
            _ => return Err(DiagnosticError::Structure),
        }
        let read = |key: &str| -> Result<i32, DiagnosticError> {
            Ok(i32::from_le_bytes(number(
                self.required(&format!("{prefix}.{key}"))?,
            )?))
        };
        let bullets = u16::from_le_bytes(number(self.required(&format!("{prefix}.bullets"))?)?);
        let light = read("extra_light")?;
        if bullets > 200
            || !matches!(self.required(&format!("{prefix}.attack_down"))?, [0] | [1])
            || !(0..=200).contains(&read("refire")?)
            || !(0..=crate::weapons::MAX_BOB_BITS).contains(&read("bob")?)
            || !(0..=1).contains(&light)
        {
            return Err(DiagnosticError::Schema);
        }
        for channel in 0..2 {
            let state = u16::from_le_bytes(number(
                self.required(&format!("{prefix}.psprite.{channel}.state"))?,
            )?);
            let tics = read(&format!("psprite.{channel}.tics"))?;
            let x = read(&format!("psprite.{channel}.x"))?;
            let y = read(&format!("psprite.{channel}.y"))?;
            if !(-15 * 65536..=17 * 65536).contains(&x) || !(0..=128 * 65536).contains(&y) {
                return Err(DiagnosticError::Schema);
            }
            let limit = if channel == 0 {
                let state =
                    OriginalState::try_from(state).map_err(|_| DiagnosticError::Identity)?;
                if !matches!(
                    state,
                    OriginalState::S_PISTOL
                        | OriginalState::S_PISTOLUP
                        | OriginalState::S_PISTOLDOWN
                        | OriginalState::S_PISTOL1
                        | OriginalState::S_PISTOL2
                        | OriginalState::S_PISTOL3
                        | OriginalState::S_PISTOL4
                ) {
                    return Err(DiagnosticError::Identity);
                }
                state.descriptor().tics
            } else if state == u16::MAX {
                if tics != 0 || light != 0 {
                    return Err(DiagnosticError::Schema);
                }
                continue;
            } else if state == OriginalState::S_PISTOLFLASH.ordinal() && light == 1 {
                OriginalState::S_PISTOLFLASH.descriptor().tics
            } else {
                return Err(DiagnosticError::Identity);
            };
            if !(1..=limit).contains(&tics) {
                return Err(DiagnosticError::Schema);
            }
            if channel == 0
                && self.required("original_player.0.state")? == [1]
                && state != OriginalState::S_PISTOLDOWN.ordinal()
            {
                return Err(DiagnosticError::Schema);
            }
        }
        Ok(())
    }
    fn validate_player(&self) -> Result<(), DiagnosticError> {
        let hp = i32::from_le_bytes(number(self.required("original_player.0.health")?)?);
        let damage = i32::from_le_bytes(number(self.required("original_player.0.damage_count")?)?);
        let frags = i32::from_le_bytes(number(self.required("original_player.0.frags")?)?);
        let state = self.required("original_player.0.state")?;
        if !(0..=100).contains(&hp)
            || !(0..=100).contains(&damage)
            || frags < 0
            || !matches!(state, [0] | [1])
            || (state == [1]) != (hp == 0)
        {
            return Err(DiagnosticError::Identity);
        }
        let raw = self.required("original_player.0.mo")?;
        let mo = if raw[0] == 1 {
            Some(ActorId::from_parts(
                u32::from_le_bytes(number(&raw[1..5])?),
                u64::from_le_bytes(number(&raw[5..13])?),
            ))
        } else {
            None
        };
        let attacker = self.required("original_player.0.attacker")?;
        if attacker[0] == 1 {
            let handle = ActorId::from_parts(
                u32::from_le_bytes(number(&attacker[1..5])?),
                u64::from_le_bytes(number(&attacker[5..13])?),
            );
            if let Some(DiagnosticIdentity::Original { definition, .. }) = self.actor(handle)? {
                if !matches!(
                    definition,
                    DefinitionIdentity::Original(
                        OriginalActorType::MT_PLAYER | OriginalActorType::MT_SERGEANT
                    )
                ) {
                    return Err(DiagnosticError::Identity);
                }
            }
        }
        for slot in 0..self.slots()? {
            let prefix = format!("actor.{slot}");
            if self.required(&format!("{prefix}.occupied"))? != [1] {
                continue;
            }
            let generation =
                u64::from_le_bytes(number(self.required(&format!("{prefix}.generation"))?)?);
            let current = mo
                == Some(ActorId::from_parts(
                    u32::try_from(slot).map_err(|_| DiagnosticError::Identity)?,
                    generation,
                ));
            let ty = u16::from_le_bytes(number(self.required(&format!("{prefix}.type"))?)?);
            if self.required(&format!("{prefix}.player"))? != [if current { 0 } else { u8::MAX }]
                || current && ty != OriginalActorType::MT_PLAYER.ordinal()
            {
                return Err(DiagnosticError::Identity);
            }
            if current {
                let actor_hp =
                    i32::from_le_bytes(number(self.required(&format!("{prefix}.health"))?)?);
                if actor_hp.max(0) != hp {
                    return Err(DiagnosticError::Identity);
                }
            }
        }
        Ok(())
    }
    /// Re-encode this read-only observation, preserving exact field/event order.
    pub fn encode(&self, limits: DiagnosticLimits) -> Result<Vec<u8>, DiagnosticError> {
        // Reapply caller budgets, including a smaller field/path/value policy.
        if self.bytes.len() > limits.bytes
            || self.fields.len() > limits.fields
            || self
                .fields
                .iter()
                .any(|f| f.path.len() > limits.path || f.value.len() > limits.value)
        {
            return Err(DiagnosticError::Limit);
        }
        let mut out = reserve(self.bytes.len())?;
        out.extend_from_slice(self.bytes);
        Ok(out)
    }
}
fn validate_handle(bytes: &[u8]) -> Result<(), DiagnosticError> {
    let raw = number::<12>(bytes)?;
    let slot = u32::from_le_bytes(number(&raw[..4])?);
    let generation = u64::from_le_bytes(number(&raw[4..])?);
    if slot >= 64 || generation == 0 {
        return Err(DiagnosticError::Identity);
    }
    Ok(())
}
impl Snapshot {
    /// Bounded Result-returning DSIM20/21/22/23/24 encoder. Trusted convenience APIs retain
    /// their old allocation policy; callers at input/output boundaries use this.
    pub fn diagnostic_bytes(&self, limits: DiagnosticLimits) -> Result<Vec<u8>, DiagnosticError> {
        if !matches!(self.version(), 20..=26) {
            return Err(DiagnosticError::Version(self.version()));
        }
        let mut size = 16_usize;
        let mut count = 0_usize;
        for field in self.fields() {
            count = count.checked_add(1).ok_or(DiagnosticError::Limit)?;
            if count > limits.fields
                || field.path().len() > limits.path
                || u32::try_from(field.path().len()).is_err()
                || field.value().len() > limits.value
            {
                return Err(DiagnosticError::Limit);
            }
            size = size
                .checked_add(12)
                .and_then(|s| s.checked_add(field.path().len()))
                .and_then(|s| s.checked_add(field.value().len()))
                .ok_or(DiagnosticError::Limit)?;
            if size > limits.bytes {
                return Err(DiagnosticError::Limit);
            }
        }
        let mut out = reserve(size)?;
        out.extend_from_slice(b"DSIM");
        out.extend_from_slice(&self.version().to_le_bytes());
        out.extend_from_slice(
            &u64::try_from(count)
                .map_err(|_| DiagnosticError::Limit)?
                .to_le_bytes(),
        );
        for f in self.fields() {
            out.extend_from_slice(
                &u32::try_from(f.path().len())
                    .map_err(|_| DiagnosticError::Limit)?
                    .to_le_bytes(),
            );
            out.extend_from_slice(f.path().as_bytes());
            out.extend_from_slice(
                &u64::try_from(f.value().len())
                    .map_err(|_| DiagnosticError::Limit)?
                    .to_le_bytes(),
            );
            out.extend_from_slice(f.value());
        }
        Ok(out)
    }
}

fn validate_weapon_event(event: &[u8], pack: bool) -> Result<(), DiagnosticError> {
    let invalid = DiagnosticError::Identity;
    match event.get(1) {
        Some(0 | 1) if event.len() == 6 && event[2] == 0 && event[3] <= 1 => {
            let id = u16::from_le_bytes(number(&event[4..6])?);
            let valid = if event[1] == 0 {
                let state = OriginalState::try_from(id).map_err(|_| invalid)?;
                if event[3] == 0 {
                    (pack
                        && [
                            crate::Weapon::Fist,
                            crate::Weapon::Shotgun,
                            crate::Weapon::Chaingun,
                        ]
                        .into_iter()
                        .any(|w| crate::player_weapons::weapon_state(w, state)))
                        || (pack && state == OriginalState::S_CHAIN3)
                        || matches!(
                            state,
                            OriginalState::S_PISTOL
                                | OriginalState::S_PISTOLUP
                                | OriginalState::S_PISTOLDOWN
                                | OriginalState::S_PISTOL1
                                | OriginalState::S_PISTOL2
                                | OriginalState::S_PISTOL3
                                | OriginalState::S_PISTOL4
                        )
                } else {
                    (pack && crate::player_weapons::flash_light(state).is_some())
                        || matches!(
                            state,
                            OriginalState::S_NULL
                                | OriginalState::S_LIGHTDONE
                                | OriginalState::S_PISTOLFLASH
                        )
                }
            } else {
                let name = crate::lifecycle::SOURCE_ACTIONS
                    .get(usize::from(id))
                    .copied();
                if event[3] == 0 {
                    (pack && matches!(name, Some("A_Punch" | "A_FireShotgun" | "A_FireCGun")))
                        || matches!(
                            name,
                            Some(
                                "A_Raise"
                                    | "A_Lower"
                                    | "A_WeaponReady"
                                    | "A_ReFire"
                                    | "A_FirePistol"
                            )
                        )
                } else {
                    matches!(name, Some("A_Light0" | "A_Light1"))
                        || (pack && name == Some("A_Light2"))
                }
            };
            if valid { Ok(()) } else { Err(invalid) }
        }
        Some(2)
            if event.len() == 12 && event[2] == 0 && (event[3] == 0 || (pack && event[3] == 1)) =>
        {
            let before = i32::from_le_bytes(number(&event[4..8])?);
            let after = i32::from_le_bytes(number(&event[8..12])?);
            if (1..=if event[3] == 1 { 50 } else { 200 }).contains(&before) && after == before - 1 {
                Ok(())
            } else {
                Err(invalid)
            }
        }
        Some(3) if event.len() == 7 && event[2] == 0 => {
            if (0..=if pack { 2 } else { 1 }).contains(&i32::from_le_bytes(number(&event[3..7])?)) {
                Ok(())
            } else {
                Err(invalid)
            }
        }
        Some(4)
            if event.len() == 8 && event[2] == 0 && (event[3] == 1 || (pack && event[3] <= 3)) =>
        {
            if (0..=if pack { i32::MAX } else { 200 })
                .contains(&i32::from_le_bytes(number(&event[4..8])?))
            {
                Ok(())
            } else {
                Err(invalid)
            }
        }
        Some(5) if event.len() == 26 => {
            validate_handle(&event[2..14])?;
            let slope = i32::from_le_bytes(number(&event[18..22])?);
            let damage = i32::from_le_bytes(number(&event[22..26])?);
            if (-40960..=40960).contains(&slope)
                && ([5, 10, 15].contains(&damage)
                    || (pack && (2..=20).contains(&damage) && damage % 2 == 0))
            {
                Ok(())
            } else {
                Err(invalid)
            }
        }
        Some(6) if event.len() == 17 && event[2..5] == [0, 0, 0] => validate_handle(&event[5..17]),
        Some(7) if event.len() == 15 && event[2] == 0 => validate_handle(&event[3..15]),
        Some(8) if event.len() == 4 && event[2] <= 2 => Ok(()),
        _ => Err(DiagnosticError::Structure),
    }
}

fn validate_player_event(event: &[u8], pack: bool) -> Result<(), DiagnosticError> {
    match event.get(1) {
        Some(0) if event.len() == 14 => validate_handle(&event[2..]),
        Some(1) if event.len() == 27 => {
            let hp = i32::from_le_bytes(number(&event[2..6])?);
            let count = i32::from_le_bytes(number(&event[19..23])?);
            let tactile = i32::from_le_bytes(number(&event[23..27])?);
            if !(0..=100).contains(&hp)
                || !(0..=100).contains(&count)
                || !(40..=240).contains(&tactile)
                || tactile % 2 != 0
            {
                return Err(DiagnosticError::Identity);
            }
            match event[6] {
                0 if event[7..19].iter().all(|b| *b == 0) => Ok(()),
                1 => validate_handle(&event[7..19]),
                _ => Err(DiagnosticError::Identity),
            }
        }
        Some(2) if event.len() == 8 => {
            if i32::from_le_bytes(number(&event[2..6])?) < 0
                || !([OriginalState::S_PISTOLDOWN]
                    .into_iter()
                    .chain(
                        if pack {
                            Some([
                                OriginalState::S_PUNCHDOWN,
                                OriginalState::S_SGUNDOWN,
                                OriginalState::S_CHAINDOWN,
                            ])
                        } else {
                            None
                        }
                        .into_iter()
                        .flatten(),
                    )
                    .any(|s| s.ordinal() == u16::from_le_bytes([event[6], event[7]])))
            {
                return Err(DiagnosticError::Identity);
            }
            Ok(())
        }
        _ => Err(DiagnosticError::Structure),
    }
}

fn diagnostic_armor(kind: u8, points: &[u8]) -> Result<PlayerArmorState, DiagnosticError> {
    let value = PlayerArmorState {
        kind: ArmorType::try_from(kind).map_err(|_| DiagnosticError::Identity)?,
        points: i32::from_le_bytes(number(points)?),
    };
    value.validate().map_err(|_| DiagnosticError::Identity)?;
    Ok(value)
}
fn validate_armor_event(event: &[u8]) -> Result<(), DiagnosticError> {
    if event.len() != 35 {
        return Err(DiagnosticError::Structure);
    }
    validate_handle(&event[1..13])?;
    let incoming = i32::from_le_bytes(number(&event[13..17])?);
    let saved = i32::from_le_bytes(number(&event[17..21])?);
    let effective = i32::from_le_bytes(number(&event[21..25])?);
    let before = diagnostic_armor(event[25], &event[26..30])?;
    let after = diagnostic_armor(event[30], &event[31..35])?;
    let expected = ArmorEvent::plan(
        ActorId::from_parts(
            u32::from_le_bytes(number(&event[1..5])?),
            u64::from_le_bytes(number(&event[5..13])?),
        ),
        incoming,
        before,
    )
    .map_err(|_| DiagnosticError::Identity)?;
    if saved != expected.saved || effective != expected.effective || after != expected.after {
        return Err(DiagnosticError::Identity);
    }
    Ok(())
}
// Independent DSIM25/26 widths and bounded source domains; no world restoration.
fn validate_pickup_event(event: &[u8], armor: bool) -> Result<(), DiagnosticError> {
    let get = |a: usize, b: usize| event.get(a..b).ok_or(DiagnosticError::Structure);
    let ok = match event.get(1) {
        Some(0) => {
            if event.len() != 40 {
                return Err(DiagnosticError::Structure);
            }
            validate_handle(get(2, 14)?)?;
            validate_handle(get(14, 26)?)?;
            let sprite = u16::from_le_bytes(number(get(26, 28)?)?);
            [
                OriginalSprite::SPR_STIM,
                OriginalSprite::SPR_MEDI,
                OriginalSprite::SPR_CLIP,
                OriginalSprite::SPR_AMMO,
                OriginalSprite::SPR_SHEL,
                OriginalSprite::SPR_SBOX,
                OriginalSprite::SPR_MGUN,
                OriginalSprite::SPR_SHOT,
            ]
            .map(OriginalSprite::ordinal)
            .contains(&sprite)
                || (armor
                    && [
                        OriginalSprite::SPR_ARM1.ordinal(),
                        OriginalSprite::SPR_ARM2.ordinal(),
                    ]
                    .contains(&sprite))
        }
        Some(1) => {
            event.len() == 14
                && event[2] <= 1
                && event[3] <= 1
                && event[12] <= 3
                && matches!(event[13], 0..=3 | 10)
                && {
                    let before = i32::from_le_bytes(number(get(4, 8)?)?);
                    let after = i32::from_le_bytes(number(get(8, 12)?)?);
                    let max = if event[2] == 0 { 200 } else { 50 };
                    (0..=max).contains(&before) && (before..=max).contains(&after)
                }
        }
        Some(2) => {
            event.len() == 15 && event[2] <= 1 && {
                let before = i32::from_le_bytes(number(get(3, 7)?)?);
                let after = i32::from_le_bytes(number(get(7, 11)?)?);
                let body = i32::from_le_bytes(number(get(11, 15)?)?);
                (1..=100).contains(&before) && (before..=100).contains(&after) && after == body
            }
        }
        Some(3) => {
            event.len() == 10
                && matches!(event[2], 2 | 3)
                && event[3] <= 1
                && u16::from_le_bytes(number(get(4, 6)?)?) <= 15
                && u16::from_le_bytes(number(get(6, 8)?)?) <= 15
                && matches!(event[8], 0..=3 | 10)
                && matches!(event[9], 0..=3 | 10)
        }
        Some(4) => {
            event.len() == 3
                && (matches!(event[2],1..=6|8..=9) || (armor && matches!(event[2], 10..=11)))
        }
        Some(5) => {
            event.len() == 10
                && (0..=world_pickups::MAX_BONUS).contains(&i32::from_le_bytes(number(get(2, 6)?)?))
                && get(6, 10)? == [0; 4]
        }
        Some(6) => {
            if event.len() != 15 {
                return Err(DiagnosticError::Structure);
            }
            validate_handle(get(2, 14)?)?;
            event[14] <= 3
        }
        Some(7) if armor => {
            if event.len() != 14 || event[3] > 1 {
                return Err(DiagnosticError::Structure);
            }
            let requested = ArmorType::try_from(event[2]).map_err(|_| DiagnosticError::Identity)?;
            if requested == ArmorType::None {
                return Err(DiagnosticError::Identity);
            }
            let before = diagnostic_armor(event[4], get(5, 9)?)?;
            let after = diagnostic_armor(event[9], get(10, 14)?)?;
            let accepted = before.points < requested.capacity();
            accepted == (event[3] == 1)
                && after
                    == (if accepted {
                        PlayerArmorState {
                            kind: requested,
                            points: requested.capacity(),
                        }
                    } else {
                        before
                    })
        }
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(DiagnosticError::Structure)
    }
}
