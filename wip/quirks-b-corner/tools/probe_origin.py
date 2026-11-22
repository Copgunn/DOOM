import sys
sys.path.insert(0, '/home/copgunn/src/DOOM/tools/reference')
import m4b_wad as B
p = B.Plan('t')
s = p.sector('ROOM', [(0, 0, 2304, 2304)])
p.void(s, 1952, 1952, 2016, 2016)
p.thing(583, 582, 45, B.PLAYER1)
m = B.build_map(p)
print(m['blocks'][:4], len(m['lines']))
