"""Measure the text parser against comp_hack tokusei that have both values and client text.
usage: validate.py <tokusei dir> <reference/tokusei-client-text.tsv>"""
import sys, os, glob, xml.etree.ElementTree as ET
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from tparse import parse
TOK, TXT = sys.argv[1:3]
truth = {}
for f in glob.glob(os.path.join(TOK, '*.xml')):
    for t in ET.parse(f).getroot().findall("object[@name='Tokusei']"):
        tid = int(t.find("member[@name='ID']").text)
        tt = t.find("member[@name='TargetType']")
        tgt = {None: '', 'PARTNER': 'partner:', 'PARTY': 'party:', 'SUMMONER': 'summoner:'}.get(tt.text if tt is not None else None, '?:')
        cond = any(t.find(f"member[@name='{k}']") is not None and len(list(t.find(f"member[@name='{k}']"))) for k in ('Conditions', 'SkillConditions', 'SkillTargetConditions'))
        toks = set()
        for c in t.findall("member[@name='CorrectValues']/element/object"):
            d = {m.get('name'): m.text for m in c.findall('member')}
            v = int(d.get('Value', 0)); typ = d.get('Type', '0')
            pct = typ == '1'
            toks.add(f"{tgt}{d['ID']}{'%' if pct and d['ID'] in ('HP_MAX','MP_MAX') else ''}={'+' if v >= 0 else '-'}{abs(v)}")
        for a in t.findall("member[@name='Aspects']/element/object"):
            d = {m.get('name'): m.text for m in a.findall('member')}
            v = int(d.get('Value', 0))
            toks.add(f"{tgt}{d['Type']}={'+' if v >= 0 else '-'}{abs(v)}")
        truth[tid] = (cond, toks)
sys.stdout.reconfigure(encoding='utf-8')
ok = bad = 0; bads = []
with open(TXT, encoding='utf-8') as f:
    for line in f:
        if line.startswith('#'): continue
        tid, st, text = line.rstrip('\n').split('\t')
        tid = int(tid)
        if st != 'ok' or not text: continue
        tg, cond, toks, rest = parse(text)
        if not toks or cond or rest: continue
        tc, tt = truth[tid]
        if tc: continue
        mine = set(x.replace('%', '') for x in toks.split())
        mine = set(m.replace('HP_MAX=', 'HP_MAX=') for m in mine)
        th = set(x.replace('%', '') for x in tt)
        if mine == th: ok += 1
        else:
            bad += 1
            if len(bads) < 40: bads.append((tid, text[:90], sorted(mine), sorted(th)))
print('fully-parsed unconditional texts compared:', ok + bad, 'match:', ok, 'differ:', bad)
for b in bads: print(*b, sep='\t')
