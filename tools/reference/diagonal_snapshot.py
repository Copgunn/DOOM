"""Independent DSIM32 projection from compiled original-C diagonal-wall rows.

Extends the frozen DSIM31 projection without editing it: the same field layout
and event vocabulary under version 32, policy 16 and the diagonal context, with
the 320-slot store and the 208 live-fireball bound of the heights policy. Diagonal
walls add no actor or sector state: a slide's outcome is already in the mover's
position, momentum and location fields and the slide event carries its line, so
no field is added. The `corner_jump` rows after the push are projected like every
other row; the Rust replay refuses that tic (`MapScaleError::Escape`) and never
reaches them, which its test states explicitly."""
import heights_snapshot as inherited
CONTEXT=b'PLAYER-MONSTERS-DIAGONAL-1-MEDIUM-v1'
VERSION=32;POLICY=16;MAX_SLOTS=inherited.MAX_SLOTS;MAX_LIVE_PROJECTILES=inherited.MAX_LIVE_PROJECTILES
def build(cases,rows,maps,prefix,full):
    old=(inherited.VERSION,inherited.POLICY,inherited.CONTEXT)
    inherited.VERSION,inherited.POLICY,inherited.CONTEXT=VERSION,POLICY,CONTEXT
    try:return inherited.build(cases,rows,maps,prefix,full)
    finally:inherited.VERSION,inherited.POLICY,inherited.CONTEXT=old
