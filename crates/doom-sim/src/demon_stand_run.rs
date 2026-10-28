//! Original normal-timing SARG stand/run closure in an explicit native fixture.
use crate::*;

impl ActorRegistry {
    /// Source Demon stand/run, melee, pain and ordinary corpse fixture.
    /// State IDs are remapped; scoring, raise, fast timing and full original
    /// Demon/Spectre execution remain outside this explicit admission.
    pub fn demon_stand_run_fixture() -> Self {
        let source = Self::vanilla();
        let fixture = Self::demon_lifecycle_fixture();
        let mut states = fixture.states().to_vec();
        for (id, original, next) in [
            (1, "S_SARG_STND", 3),
            (3, "S_SARG_STND2", 1),
            (2, "S_SARG_RUN1", 15),
            (15, "S_SARG_RUN2", 16),
            (16, "S_SARG_RUN3", 17),
            (17, "S_SARG_RUN4", 18),
            (18, "S_SARG_RUN5", 19),
            (19, "S_SARG_RUN6", 20),
            (20, "S_SARG_RUN7", 21),
            (21, "S_SARG_RUN8", 2),
        ] {
            let mut state = source
                .states()
                .iter()
                .find(|s| s.name == original)
                .unwrap()
                .clone();
            state.name = format!("FIXTURE_{}", &original[2..]);
            state.next = StateId(next);
            states[id] = state;
        }
        let mut types = fixture.types().to_vec();
        types[136].name = "FIXTURE_DEMON_STAND_RUN_136".into();
        Self::new(states, types).expect("original SARG state closure")
    }
}
