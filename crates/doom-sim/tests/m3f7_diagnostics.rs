//! Untrusted diagnostic framing and identity policy, not original-C behavior proof.
#[path = "../../../tests/support/m3f7.rs"]
mod fixtures;
use doom_sim::*;
use fixtures::historical as frozen;
fn limits() -> DiagnosticLimits {
    DiagnosticLimits::default()
}
fn bytes() -> Vec<u8> {
    fixtures::setup(&fixtures::cases()[0])
        .snapshot()
        .diagnostic_bytes(limits())
        .unwrap()
}
fn edit(input: &[u8], path: &str, replacement: &[u8]) -> Vec<u8> {
    let mut out = input[..16].to_vec();
    let mut at = 16;
    while at < input.len() {
        let n = u32::from_le_bytes(input[at..at + 4].try_into().unwrap()) as usize;
        let start = at;
        at += 4;
        let name = std::str::from_utf8(&input[at..at + n]).unwrap();
        at += n;
        let len =
            usize::try_from(u64::from_le_bytes(input[at..at + 8].try_into().unwrap())).unwrap();
        at += 8;
        let value = if name == path {
            replacement
        } else {
            &input[at..at + len]
        };
        out.extend_from_slice(&input[start..start + 4 + n]);
        out.extend_from_slice(&(value.len() as u64).to_le_bytes());
        out.extend_from_slice(value);
        at += len;
    }
    out
}
#[test]
fn both_versions_roundtrip_and_native_values_keep_legacy_meaning() {
    for (bytes, version) in [
        (bytes(), 21),
        (
            frozen::setup(&frozen::cases()[0])
                .snapshot()
                .canonical_bytes(),
            20,
        ),
    ] {
        let read = Diagnostic::decode(&bytes, limits()).unwrap();
        assert_eq!(read.version(), version);
        assert_eq!(read.encode(limits()).unwrap(), bytes);
        match read.actor(ActorId::from_parts(0, 1)).unwrap().unwrap() {
            DiagnosticIdentity::Native { actor, state } => {
                assert_eq!(version, 20);
                assert_eq!(actor, ActorTypeId(136));
                assert_eq!(state, Some(StateId(1)));
            }
            DiagnosticIdentity::Original { definition, state } => {
                assert_eq!(version, 21);
                assert_eq!(
                    definition,
                    DefinitionIdentity::Original(identity::OriginalActorType::MT_SERGEANT)
                );
                assert_eq!(state, Some(identity::OriginalState::S_SARG_STND));
            }
        }
        assert_eq!(read.actor(ActorId::from_parts(0, 2)), Ok(None));
        assert_eq!(read.actor(ActorId::from_parts(u32::MAX, 1)), Ok(None));
        assert_eq!(
            Diagnostic::decode_v20(&bytes, limits()).is_ok(),
            version == 20
        );
    }
}
#[test]
fn format_profile_registry_and_context_are_independent_explicit_contracts() {
    let b = bytes();
    for version in [0, 1, 19, 22, u32::MAX] {
        let mut altered = b.clone();
        altered[4..8].copy_from_slice(&version.to_le_bytes());
        assert_eq!(
            Diagnostic::decode(&altered, limits()).unwrap_err(),
            DiagnosticError::Version(version)
        );
    }
    for (path, value) in [
        ("profile", 20_u32.to_le_bytes().to_vec()),
        ("registry.schema", b"unknown".to_vec()),
        ("context.schema", b"unknown".to_vec()),
        ("tic_rate", 36_u32.to_le_bytes().to_vec()),
    ] {
        assert!(Diagnostic::decode(&edit(&b, path, &value), limits()).is_err());
    }
    let old = frozen::setup(&frozen::cases()[0])
        .snapshot()
        .canonical_bytes();
    assert!(Diagnostic::decode(&edit(&old, "registry.type.136", b"forged"), limits()).is_err());
}
#[test]
fn original_ids_cannot_be_native_aliases_or_unsupported_metadata() {
    let b = bytes();
    for ty in [0_u16, 2, 13, 135, 136, 138, u16::MAX] {
        assert!(
            Diagnostic::decode(&edit(&b, "actor.0.type", &ty.to_le_bytes()), limits()).is_err(),
            "type{ty}"
        );
    }
    for state in [0_u16, 1, 21, 474, 496, 501, 967] {
        assert!(
            Diagnostic::decode(&edit(&b, "actor.0.state", &state.to_le_bytes()), limits()).is_err(),
            "state{state}"
        );
    }
    assert!(
        Diagnostic::decode(
            &edit(&b, "actor.0.state", &u16::MAX.to_le_bytes()),
            limits()
        )
        .is_ok()
    );
    let mut event = Diagnostic::decode(&b, limits())
        .unwrap()
        .field("events.pending.1")
        .unwrap()
        .to_vec();
    assert_eq!(&event[..2], &[12, 0]);
    event[14..16].copy_from_slice(&136_u16.to_le_bytes());
    assert!(Diagnostic::decode(&edit(&b, "events.pending.1", &event), limits()).is_err());
}
#[test]
fn resource_budgets_apply_before_allocations_and_to_both_encoders() {
    let s = fixtures::setup(&fixtures::cases()[0]).snapshot();
    let b = s.canonical_bytes();
    let read = Diagnostic::decode(&b, limits()).unwrap();
    for l in [
        DiagnosticLimits {
            bytes: b.len() - 1,
            ..limits()
        },
        DiagnosticLimits {
            fields: 0,
            ..limits()
        },
        DiagnosticLimits {
            path: 1,
            ..limits()
        },
        DiagnosticLimits {
            value: 1,
            ..limits()
        },
    ] {
        assert!(Diagnostic::decode(&b, l).is_err());
        assert_eq!(read.encode(l), Err(DiagnosticError::Limit));
        assert_eq!(s.diagnostic_bytes(l), Err(DiagnosticError::Limit));
    }
    let mut huge = b.clone();
    huge[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
    assert_eq!(
        Diagnostic::decode(&huge, limits()).unwrap_err(),
        DiagnosticError::Limit
    );
    assert!(
        Diagnostic::decode(
            &edit(&b, "store.slot_count", &65_u64.to_le_bytes()),
            limits()
        )
        .is_err()
    );
    assert!(
        Diagnostic::decode(
            &edit(&b, "events.pending.count", &u64::MAX.to_le_bytes()),
            limits()
        )
        .is_err()
    );
}
#[test]
fn framing_rejects_truncation_overflow_duplicates_utf8_and_trailing_data() {
    let b = bytes();
    for n in 0..b.len() {
        assert!(Diagnostic::decode(&b[..n], limits()).is_err(), "prefix{n}");
    }
    let mut changed = b.clone();
    changed.push(0);
    assert!(Diagnostic::decode(&changed, limits()).is_err());
    let mut changed = b.clone();
    changed[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Diagnostic::decode(&changed, limits()).is_err());
    let mut changed = b.clone();
    changed[20] = 255;
    assert!(Diagnostic::decode(&changed, limits()).is_err());
    let mut changed = b.clone();
    changed[23..31].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(Diagnostic::decode(&changed, limits()).is_err());
    let mut changed = b.clone();
    let count = u64::from_le_bytes(b[8..16].try_into().unwrap());
    changed[8..16].copy_from_slice(&(count + 1).to_le_bytes());
    changed.extend_from_slice(&b[16..39]);
    assert!(Diagnostic::decode(&changed, limits()).is_err());
    assert!(Diagnostic::decode(&edit(&b, "events.pending.0", &[255]), limits()).is_err());
}
#[test]
fn independently_encoded_v21_full_fixtures_decode_exactly() {
    for b in [
        include_bytes!("../../../tests/reference/m3f7_state_0_0.bin").as_slice(),
        include_bytes!("../../../tests/reference/m3f7_state_0_80.bin").as_slice(),
    ] {
        let read = Diagnostic::decode(b, limits()).unwrap();
        assert_eq!(read.encode(limits()).unwrap(), b);
    }
}
#[test]
fn complete_trace_diagnostics_are_observations_and_preserve_order() {
    let c = &fixtures::cases()[1];
    let mut sim = fixtures::setup(c);
    for op in 0..=130 {
        if op != 0 {
            fixtures::advance(&mut sim, c, op);
        }
        let s = sim.snapshot();
        let b = s.diagnostic_bytes(limits()).unwrap();
        assert_eq!(b, s.canonical_bytes());
        let read = Diagnostic::decode(&b, limits()).unwrap();
        assert_eq!(read.encode(limits()).unwrap(), b);
        assert_eq!(sim.snapshot(), s);
    }
}
