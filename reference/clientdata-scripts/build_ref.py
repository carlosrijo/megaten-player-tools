"""Build reference TSVs for newmoon-reader from decoded client tables.
usage: build_ref.py <clientdata dir (has out/)> <client translation.xml> <reference out dir>"""
import sys, os, re, xml.etree.ElementTree as ET
from collections import defaultdict, OrderedDict

CD, TRANS, REF = sys.argv[1:4]
BS = chr(92)
NL = BS + 'n'
os.makedirs(REF, exist_ok=True)

def rows(path):
    with open(path, encoding='utf-8') as f:
        for line in f:
            if line.startswith('#'): continue
            yield line.rstrip('\n').split('\t')

# ---------- A. client tokusei text (all)
chartext, charstat = {}, {}
for p in rows(os.path.join(CD, 'out', 'charastic-text.tsv')):
    chartext[int(p[0])] = p[2]; charstat[int(p[0])] = p[1]
with open(os.path.join(REF, 'tokusei-client-text.tsv'), 'w', encoding='utf-8', newline='\n') as o:
    o.write('#tokusei\tcompHack (ok = has effects, empty = id only, missing = absent)\tclient text (CMessageData_Charastic; newlines as ' + NL + ')\n')
    for t in sorted(chartext):
        o.write(f"{t}\t{charstat[t]}\t{chartext[t]}\n")

# ---------- B. missing / empty referenced tokusei, best-effort parse
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from tparse import parse
refs = defaultdict(list)
status = {}
for p in rows(os.path.join(CD, 'out', 'tokusei-refs.tsv')):
    t = int(p[0]); status[t] = p[1]
    refs[t].append(p)
rec_toks = defaultdict(set)   # (source family, record) -> tokusei ids sharing that record's text
def fam(src): return src.split('.cond')[0]
for t, rs in refs.items():
    for r in rs:
        rec_toks[(fam(r[4]), r[5])].add(t)
with open(os.path.join(REF, 'tokusei-missing.tsv'), 'w', encoding='utf-8', newline='\n') as o:
    o.write('#tokusei\tcompHack (missing = absent, empty = id only)\ttextSource (client = CMessageData_Charastic for this id; record = description of the referencing record, may cover several tokusei)\t'
            'text\ttarget\tcondition\tparsed (best-effort KEY=value; partner:/party: prefix = target)\tunparsed lines\t'
            'fullyParsed (yes = client text for this id, no condition, every line parsed)\treferenced by (source:record:name; ...)\n')
    n = 0
    for t in sorted(refs):
        if status[t] == 'ok': continue
        rs = refs[t]
        txt, src = chartext.get(t, ''), 'client'
        if not txt:
            # prefer the most specific record text: enchant / mitama / mod (one tokusei each) over skills/sets
            best = None
            for r in rs:
                if r[7]:
                    shared = len(rec_toks[(fam(r[4]), r[5])])
                    if best is None or shared < best[0]: best = (shared, r)
            if best:
                txt, src = best[1][7], ('record' if best[0] == 1 else f'record (text shared by {best[0]} tokusei of {fam(best[1][4])} {best[1][5]})')
            else:
                src = 'none'
        if src.startswith('record') and rs[0][4].startswith('MitamaReunionSetBonusData'):
            src = 'record (Mitama set description; this tokusei = the part not covered by the set\'s CorrectTbl pairs)'
        tg, cond, toks, rest = parse(txt)
        refl = '; '.join(OrderedDict.fromkeys(f"{r[4]}:{r[5]}:{r[6]}" for r in rs))
        safe = 'yes' if toks and not cond and not rest and src == 'client' else 'no'
        o.write(f"{t}\t{status[t]}\t{src}\t{txt}\t{tg}\t{cond}\t{toks}\t{rest}\t{safe}\t{refl}\n")
        n += 1
    print('missing/empty rows', n)

# ---------- C. expertise names + chains (NewMoon client)
names = {}
t = open(TRANS, encoding='utf-8').read()
addr = []
for m in re.finditer(r'<translation>(.*?)</translation>', t, re.S):
    b = m.group(1)
    jp = re.search(r'<!--\s*(.*?)\s*-->', b); en = re.search(r'CDATA\[(.*?)\]\]', b, re.S)
    for a in re.findall(r'<patch-addr>0x([0-9A-Fa-f]+)</patch-addr>', b):
        a = int(a, 16)
        if 0x6A1109 <= a <= 0x6A16CC and (a - 0x6A1109) % 0x19 == 0:
            names[(a - 0x6A1109) // 0x19] = (en.group(1), jp.group(1))
chains = {p[0]: p for p in rows(os.path.join(CD, 'out', 'expertise-chains.tsv'))}
with open(os.path.join(REF, 'expertise-newmoon.tsv'), 'w', encoding='utf-8', newline='\n') as o:
    o.write('#expertise id\tNewMoon name (client translation.xml)\tJapanese name\tmaxClass\tisChain\tchain (sourceId:minRank:share; minRank in points/10000, share of the source points)\twebsite name\twebsite chain (differs = website is stale)\n')
    for eid in sorted(int(k) for k in chains):
        p = chains[str(eid)]
        en, jp = names.get(eid, ('', ''))
        o.write(f"{eid}\t{en}\t{jp}\t{p[2]}\t{p[3]}\t{p[5]}\t{p[1]}\t{p[6]}\n")

# ---------- D. expertise rank skills
with open(os.path.join(REF, 'expertise-rank-skills.tsv'), 'w', encoding='utf-8', newline='\n') as o:
    o.write('#expertise id\tclass\trank\tskill ids\tskill names (CSkillData)\n')
    for p in rows(os.path.join(CD, 'out', 'expertise-class-skills.tsv')):
        o.write(f"{p[0]}\t{p[2]}\t{p[3]}\t{p[4]}\t{p[5]}\n")
print('expertise names', len(names))
