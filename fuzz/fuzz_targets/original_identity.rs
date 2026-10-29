#![no_main]
use doom_sim::{identity::*, *};
use libfuzzer_sys::fuzz_target;
use std::sync::OnceLock;
static SOURCE: OnceLock<ActorRegistry> = OnceLock::new();
static NATIVE: OnceLock<ActorRegistry> = OnceLock::new();
fuzz_target!(|data: &[u8]| {
    let source = SOURCE.get_or_init(ActorRegistry::vanilla);
    let native = NATIVE.get_or_init(ActorRegistry::demon_map_context_fixture);
    if data.len() < 4 {
        return;
    }
    let n = u16::from_le_bytes([data[0], data[1]]);
    let signed = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let _ = OriginalActorType::try_from(signed);
    let _ = OriginalState::try_from(signed);
    let _ = OriginalSprite::try_from(signed);
    if let Ok(id) = OriginalActorType::try_from(n) {
        assert_eq!(id.ordinal(), n);
        assert_eq!(OriginalActorType::from_name(id.name()), Some(id));
        assert_eq!(
            source.original_actor_type(id.resolve(source).unwrap()),
            Ok(id)
        );
        if let Ok(slot) = id.resolve(native) {
            assert_eq!(native.original_actor_type(slot), Ok(id));
        }
        let _ = id.states();
        let mut types = source.types().to_vec();
        let field = usize::from(data[2]) % 23;
        types[usize::from(n)].properties[field] ^= 1;
        if let Ok(changed) = ActorRegistry::new(source.states().to_vec(), types) {
            assert_eq!(id.resolve(&changed), Err(IdentityError::RegistryMismatch));
        }
    } else {
        assert_eq!(
            source.original_actor_type(ActorTypeId(n)),
            Err(IdentityError::OutOfRange)
        );
    }
    if let Ok(id) = OriginalState::try_from(n) {
        assert_eq!(id.ordinal(), n);
        assert_eq!(OriginalState::from_name(id.name()), Some(id));
        assert_eq!(source.original_state(id.resolve(source).unwrap()), Ok(id));
        let _ = id.descriptor();
        if let Ok(slot) = id.resolve(native) {
            assert_eq!(native.original_state(slot), Ok(id));
        }
    } else {
        assert_eq!(
            source.original_state(StateId(n)),
            Err(IdentityError::OutOfRange)
        );
    }
    if let Ok(id) = OriginalSprite::try_from(n) {
        assert_eq!(id.ordinal(), n);
        assert_eq!(OriginalSprite::from_name(id.name()), Some(id));
        assert_eq!(id.resource_name().len(), 4);
    }
    if let Ok(name) = std::str::from_utf8(&data[4..data.len().min(68)]) {
        let _ = OriginalActorType::from_name(name);
        let _ = OriginalState::from_name(name);
        let _ = OriginalSprite::from_name(name);
    }
});
