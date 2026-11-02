"""Independent DSIM31 projection from compiled original-C heights rows.

Extends the frozen DSIM30 projection without editing it: the same field layout
and event vocabulary under version 31, policy 15, the heights context, the
320-slot store and the 208 live-fireball bound of the new policy. The source
state that heights need (z, momz, floorz, ceilingz, subsector, sector and the
per-sector sound target) has been projected since DSIM3 and DSIM6; no field is
added.

Slide-traverse rows (a `TR` row with role 2 followed by D/B/U/I/T rows) are not
events in any DSIM vocabulary and the inherited bonus layer drops them. They are
dropped here before the chain because a scenario without a single aim keeps the
C harness's trace stage at 0, and the inherited monster layer would otherwise
read such a slide trace as an unterminated monster aim."""
import map_scale_snapshot as inherited
CONTEXT=b'PLAYER-MONSTERS-HEIGHTS-1-MEDIUM-v1'
VERSION=31;POLICY=15;MAX_SLOTS=inherited.MAX_SLOTS;MAX_LIVE_PROJECTILES=inherited.MAX_LIVE_PROJECTILES
def events(rows):
    filtered=[];role=None
    for r in rows:
        if r[0]=='TR':role=r[4];assert role in(0,1,2)
        if role==2 and r[0]in{'D','B','U','I','T'}:continue
        filtered.append(r)
    return INHERITED_EVENTS(filtered)
INHERITED_EVENTS=inherited.events
def build(cases,rows,maps,prefix,full):
    old=(inherited.VERSION,inherited.POLICY,inherited.CONTEXT,inherited.events)
    inherited.VERSION,inherited.POLICY,inherited.CONTEXT,inherited.events=VERSION,POLICY,CONTEXT,events
    try:return inherited.build(cases,rows,maps,prefix,full)
    finally:inherited.VERSION,inherited.POLICY,inherited.CONTEXT,inherited.events=old
