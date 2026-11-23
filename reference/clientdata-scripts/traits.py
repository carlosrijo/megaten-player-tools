"""Demon trait skills whose effect the server tables + comp_hack tokusei can't explain.
usage: traits.py <clientdata dir> <devils-game.tsv> <out tsv>"""
import sys, os
from collections import defaultdict
CD, DEV, OUT = sys.argv[1:4]
def rows(p):
    for line in open(p, encoding='utf-8'):
        if not line.startswith('#'): yield line.rstrip('\n').split('\t')
skill = {int(p[0]): (p[1], p[2]) for p in rows(os.path.join(CD, 'cskill.tsv'))}
sd = {int(p[0]): p for p in rows(os.path.join(CD, 'skilldata.tsv'))}
st = {}
for p in rows(os.path.join(CD, 'out', 'tokusei-refs.tsv')): st[int(p[0])] = p[1]
for p in rows(os.path.join(CD, 'out', 'charastic-text.tsv')): st.setdefault(int(p[0]), p[1])
demons = defaultdict(list)
for p in rows(DEV):
    for t in p[7].split(','):
        if t and t != '0': demons[int(t)].append(p[1])
SYNTH = {20007: 'Swordsmith', 20008: 'Swordsmith', 20009: 'Arms Maker', 20010: 'Arms Maker', 20011: 'Synthesis', 20012: 'Synthesis', 20013: 'crafting', 20014: 'crafting'}
with open(OUT, 'w', encoding='utf-8', newline='\n') as o:
    o.write('#skill id\tname\tstatus (no-data = no CorrectTbl and no tokusei; partial/missing = tokusei absent from comp_hack)\tmissing tokusei\tclient description\tdemons with this trait\tnote\n')
    for sid in sorted(demons):
        p = sd.get(sid)
        if not p: continue
        toks = [int(x) for x in p[3].split(',') if x]
        corr = p[4]
        miss = [t for t in toks if st.get(t, 'missing') in ('missing', 'empty')]
        if corr or (toks and not miss) or p[1] != '0': continue
        status = 'no-data' if not toks else ('missing' if len(miss) == len(toks) else 'partial')
        n, d = skill.get(sid, ('', ''))
        note = ''
        if sid in SYNTH:
            note = f'crafting assist ({SYNTH[sid]} expertise bonus from the demon INT/LUCK); no character stat effect; not implemented in comp_hack'
        o.write(f"{sid}\t{n}\t{status}\t{','.join(map(str, miss))}\t{d}\t{', '.join(sorted(set(demons[sid]))[:8])}{' ...' if len(set(demons[sid])) > 8 else ''}\t{note}\n")
