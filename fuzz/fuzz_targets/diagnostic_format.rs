#![no_main]
use doom_sim::*;
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if data.len() > 131_072 {
        return;
    }
    let limits = DiagnosticLimits {
        bytes: 131_072,
        fields: 4096,
        path: 192,
        value: 131_072,
    };
    if let Ok(view) = Diagnostic::decode(data, limits) {
        assert_eq!(view.encode(limits).unwrap(), data);
        let _ = view.actor(ActorId::from_parts(0, 1));
        assert_eq!(
            Diagnostic::decode_v20(data, limits).is_ok(),
            view.version() == 20
        );
    }
    // The explicit DSIM28 reader accepts only28 and never overlaps older readers.
    if let Ok(view) = Diagnostic::decode_monsters(data, limits) {
        assert_eq!(view.encode(limits).unwrap(), data);
        assert_eq!(view.version(), 28);
        assert!(Diagnostic::decode(data, limits).is_err());
        assert!(Diagnostic::decode_bonuses(data, limits).is_err());
    }
    // The explicit DSIM29 reader accepts only29 and never overlaps older readers.
    if let Ok(view) = Diagnostic::decode_imps(data, limits) {
        assert_eq!(view.encode(limits).unwrap(), data);
        assert_eq!(view.version(), 29);
        assert!(Diagnostic::decode(data, limits).is_err());
        assert!(Diagnostic::decode_monsters(data, limits).is_err());
    }
    // Also attack framing/schema/identity fields inside an otherwise valid stream.
    if (1..=64).contains(&data.len()) {
        let mut bytes = include_bytes!("../../tests/reference/m3f7_state_0_0.bin").to_vec();
        for chunk in data.chunks(3) {
            if chunk.len() == 3 {
                let offset = usize::from(u16::from_le_bytes([chunk[0], chunk[1]])) % bytes.len();
                bytes[offset] = chunk[2];
            }
        }
        if let Ok(view) = Diagnostic::decode(&bytes, limits) {
            assert_eq!(view.encode(limits).unwrap(), bytes);
        }
    }
});
