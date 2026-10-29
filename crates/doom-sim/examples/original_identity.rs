//! Canonical metadata is separate from executable registry admission.
use doom_sim::{identity::*, *};
fn main() {
    let registry = ActorRegistry::vanilla();
    let demon = OriginalActorType::MT_SERGEANT;
    assert_eq!(demon.ordinal(), 12);
    assert_eq!(demon.states().spawn, OriginalState::S_SARG_STND);
    let stand = demon.states().spawn.descriptor();
    assert_eq!(stand.sprite, OriginalSprite::SPR_SARG);
    assert_eq!(stand.action, Some("A_Look"));
    assert_eq!(
        registry.original_actor_type(demon.resolve(&registry).unwrap()),
        Ok(demon)
    );
    let native = ActorRegistry::demon_map_context_fixture();
    assert_eq!(
        native.original_actor_type(ActorTypeId(136)),
        Err(IdentityError::RegistryMismatch)
    );
    assert_eq!(
        OriginalState::try_from(OriginalState::COUNT),
        Err(IdentityError::OutOfRange)
    );
    println!(
        "Original identity metadata: {} actors, {} states, {} sprites; MT_SERGEANT=12; native aliases remain explicit.",
        OriginalActorType::COUNT,
        OriginalState::COUNT,
        OriginalSprite::COUNT
    );
}
