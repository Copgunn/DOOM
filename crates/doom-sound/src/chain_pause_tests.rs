//! `Pause::commit`'s music calls on the tics that change the music, from the rule the module
//! documentation cites (`S_StopMusic`, s_sound.c:689-696: a music change while `mus_paused` is
//! set calls `I_ResumeSong` once more and leaves the flag; the intermission's first `WI_Ticker`
//! changes it after the toggle, wi_stuff.c:1508-1515, and `F_StartFinale` before it).
//! Below the public API because `Pause::commit` is `pub(crate)`: a session reaches these two
//! tics with a pausing command on the completion tic, which no committed genuine item has (the
//! mutation of step 9 that deletes `Pause::stop_music` survives every genuine item and every
//! doom-game test).
use super::*;

fn paused(next: TicPause) -> Pause {
    Pause {
        fields: PauseFields {
            mus_paused: true,
            level_paused: 7,
        },
        next,
        calls: vec![],
    }
}

#[test]
fn a_music_change_while_paused_resumes_once_more_and_keeps_the_flag() {
    let mut p = paused(TicPause::default());
    p.commit(PauseTic::EnterIntermission { music: true });
    assert_eq!(p.calls, [MusicCall::Resume]);
    assert!(p.fields.mus_paused, "S_StopMusic leaves the flag");
    for tic in [
        PauseTic::EnterIntermission { music: false },
        PauseTic::EnterFinale { music: false },
        PauseTic::Other,
    ] {
        let mut p = paused(TicPause::default());
        p.commit(tic);
        assert_eq!(p.calls, [], "{tic:?}");
    }
    let mut p = Pause::default();
    p.commit(PauseTic::EnterFinale { music: true });
    assert_eq!(
        p.calls,
        [],
        "nothing to resume when the music is not paused"
    );
}

#[test]
fn the_toggle_and_the_music_change_come_in_each_tics_order() {
    let unpause = TicPause {
        toggle: true,
        paused: false,
    };
    // The finale's change is first: it resumes, then the toggle (now unpausing) resumes again.
    let mut p = paused(unpause);
    p.commit(PauseTic::EnterFinale { music: true });
    assert_eq!(p.calls, [MusicCall::Resume, MusicCall::Resume]);
    assert!(!p.fields.mus_paused);
    // The intermission's toggle is first: it resumes and clears the flag, so the change finds
    // nothing paused.
    let mut p = paused(unpause);
    p.commit(PauseTic::EnterIntermission { music: true });
    assert_eq!(p.calls, [MusicCall::Resume]);
    assert!(!p.fields.mus_paused);
}
