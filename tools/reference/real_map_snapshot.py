"""Independent DSIM33 projection from compiled original-C real-map-scale rows.

Extends the frozen DSIM32 projection without editing it: the same field layout
and event vocabulary under version 33, policy 17 and the real-map context, with
the 320-slot store and the 208 live-fireball bound. Raising the static-geometry
caps adds no actor or sector state, so no field is added; the sector and line
counts of these rooms simply appear as larger `sectors.count`, `map.*` lumps and
`sector.N.*` tables, all already derived from the frame. The `TI` observer rows
(intercept count on entry to P_TraverseIntercepts) are not events in any DSIM
vocabulary and are dropped before the chain, as the heights layer drops slide
traverse rows."""
import diagonal_snapshot as inherited
CONTEXT=b'PLAYER-MONSTERS-REALMAP-1-MEDIUM-v1'
VERSION=33;POLICY=17;MAX_SLOTS=inherited.MAX_SLOTS;MAX_LIVE_PROJECTILES=inherited.MAX_LIVE_PROJECTILES
def build(cases,rows,maps,prefix,full):
    rows=[r for r in rows if r[0]!='TI']
    old=(inherited.VERSION,inherited.POLICY,inherited.CONTEXT)
    inherited.VERSION,inherited.POLICY,inherited.CONTEXT=VERSION,POLICY,CONTEXT
    try:return inherited.build(cases,rows,maps,prefix,full)
    finally:inherited.VERSION,inherited.POLICY,inherited.CONTEXT=old
