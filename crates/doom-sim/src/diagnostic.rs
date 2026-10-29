//! Bounded, read-only DSIM20/21 observations. Never an executable world loader.
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
        if bytes.len() > limits.bytes {
            return Err(DiagnosticError::Limit);
        }
        let mut cursor = Cursor { bytes, at: 0 };
        if cursor.take(4)? != b"DSIM" {
            return Err(DiagnosticError::Structure);
        }
        let version = cursor.u32()?;
        if !matches!(version, 20 | 21) {
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
        if self.version == 21 {
            original_registry::require_role(actor).map_err(|_| DiagnosticError::Identity)?;
            if let Some(s) = state {
                original_registry::state_role(actor, s).map_err(|_| DiagnosticError::Identity)?;
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
    fn validate(&self) -> Result<(), DiagnosticError> {
        let profile = u32::from_le_bytes(number(self.required("profile")?)?);
        if u32::from_le_bytes(number(self.required("tic_rate")?)?) != 35 {
            return Err(DiagnosticError::Schema);
        }
        number::<8>(self.required("tic")?)?;
        if self.version == 21 {
            if profile != OriginalExecutionProfile::DemonNormalV1.wire_id()
                || self.required("registry.schema")? != ORIGINAL_REGISTRY_SCHEMA
                || self.required("context.schema")? != ORIGINAL_CONTEXT_SCHEMA
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
                if tag > 21 {
                    return Err(DiagnosticError::Structure);
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
                    } else if self.version == 21 {
                        let state = OriginalState::try_from(value)
                            .map_err(|_| DiagnosticError::Identity)?;
                        // Any admitted normal role; never original alias interpretation.
                        if ![
                            OriginalActorType::MT_SERGEANT,
                            OriginalActorType::MT_PUFF,
                            OriginalActorType::MT_BLOOD,
                            OriginalActorType::MT_PLAYER,
                        ]
                        .into_iter()
                        .any(|ty| {
                            let slot = if ty == OriginalActorType::MT_PLAYER {
                                original_registry::body_slot()
                            } else {
                                ActorTypeId(ty.ordinal())
                            };
                            original_registry::state_role(slot, StateId(state.ordinal())).is_ok()
                        }) {
                            return Err(DiagnosticError::Identity);
                        }
                    }
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
    /// Bounded Result-returning DSIM20/21 encoder. Trusted convenience APIs retain
    /// their old allocation policy; callers at input/output boundaries use this.
    pub fn diagnostic_bytes(&self, limits: DiagnosticLimits) -> Result<Vec<u8>, DiagnosticError> {
        if !matches!(self.version(), 20 | 21) {
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
