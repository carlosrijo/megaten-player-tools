"""Collect every tokusei id referenced by server/client tables, compare with comp_hack tokusei XML,
attach the client's own text (CMessageData_Charastic by tokusei id, plus the referencing record's text).
usage: tokrefs.py <clientdata dir> <sbin dir> <tokusei dir> <deploy datastore/data dir> <out dir>"""
import sys, os, glob, re, xml.etree.ElementTree as ET
from collections import defaultdict

CD, SB, TOK, DS, OUT = sys.argv[1:6]
BS = chr(92)
def esc(s):
    return (s or '').replace('\r', '').replace('\n', BS + 'n').replace('\t', ' ').strip()

def iter_records(path, objname):
    for ev, el in ET.iterparse(path):
        if el.tag == 'object' and el.get('name') == objname:
            yield el
            el.clear()

def mval(el, name):
    m = el.find(f"member[@name='{name}']")
    return m.text if m is not None else None

def ints(el):
    return [int(e.text) for e in el.iter('element') if e.text and e.text.lstrip('-').isdigit() and int(e.text) > 0]

# ---- known tokusei (comp_hack)
known = {}
for f in glob.glob(os.path.join(TOK, '*.xml')):
    root = ET.parse(f).getroot()
    for t in root.findall("object[@name='Tokusei']"):
        tid = int(mval(t, 'ID'))
        parts = []
        for k in ('CorrectValues', 'TokuseiCorrectValues', 'Aspects'):
            m = t.find(f"member[@name='{k}']")
            if m is not None and len(list(m)) > 0:
                parts.append(k)
        known[tid] = '+'.join(parts) if parts else 'empty'

# ---- client tokusei text
chartext = {}
for o in iter_records(os.path.join(CD, 'CMessageData_Charastic.xml'), 'MiCMessageData'):
    chartext[int(mval(o, 'ID'))] = esc(mval(o, 'message'))

# ---- client text for records
skilltext = {}
with open(os.path.join(CD, 'cskill.tsv'), encoding='utf-8') as f:
    for line in f:
        if line.startswith('#'): continue
        p = line.rstrip('\n').split('\t')
        if len(p) >= 3: skilltext[int(p[0])] = (p[1], p[2])
itemtext = {}
for o in iter_records(os.path.join(CD, 'CItemData.xml'), 'MiCItemBaseData'):
    itemtext[int(mval(o, 'ID'))] = (esc(mval(o, 'name')), esc(mval(o, 'desc')))
statustext = {}
for o in iter_records(os.path.join(CD, 'CStatusData.xml'), 'MiCStatusData'):
    statustext[int(mval(o, 'ID'))] = (esc(mval(o, 'name')), esc(mval(o, 'desc')))
cmodtext = {}
for o in iter_records(os.path.join(CD, 'CModifiedEffectData.xml'), 'MiCModifiedEffectData'):
    cmodtext[int(mval(o, 'ID'))] = (esc(mval(o, 'name')), esc(mval(o, 'desc')))

refs = defaultdict(list)   # tokusei id -> [(source, record, name, text)]
def add(tid, src, rec, name='', text=''):
    refs[tid].append((src, str(rec), name, text))

# SkillData charastic
for o in iter_records(os.path.join(SB, 'SkillData.xml'), 'MiSkillData'):
    sid = int(o.find(".//object[@name='MiSkillItemStatusCommonData']/member[@name='id']").text)
    ch = o.find(".//object[@name='MiSkillCharasticData']/member[@name='charastic']")
    if ch is None: continue
    n, d = skilltext.get(sid, ('', ''))
    for t in ints(ch): add(t, 'SkillData', sid, n, d)

# EnchantData
for o in iter_records(os.path.join(CD, 'EnchantData.xml'), 'MiEnchantData'):
    eid = int(mval(o, 'ID'))
    for side in ('tarot', 'soul'):
        c = o.find(f".//member[@name='{side}']/object[@name='MiEnchantCharasticData']")
        if c is None: continue
        nm, ds = esc(mval(c, 'name')), esc(mval(c, 'desc'))
        tk = c.find("member[@name='tokusei']")
        for t in (ints(tk) if tk is not None else []): add(t, 'EnchantData.' + side, eid, nm, ds)
        for i, cond in enumerate(c.findall("member[@name='conditions']/element/object")):
            ct = cond.find("member[@name='tokusei']")
            for t in (ints(ct) if ct is not None else []): add(t, f'EnchantData.{side}.cond{i}', eid, nm, ds)
    # enchant may also carry set/other tokusei lists
    for m in o.findall("member[@name='tokusei']"):
        for t in ints(m): add(t, 'EnchantData.other', eid)

# EquipmentSetData
for o in iter_records(os.path.join(CD, 'EquipmentSetData.xml'), 'MiEquipmentSetData'):
    sid = int(mval(o, 'ID'))
    eq = [str(x) for x in ints(o.find("member[@name='equipment']"))]
    names = ' + '.join(itemtext.get(int(x), ('?',))[0] for x in eq)
    for t in ints(o.find("member[@name='tokusei']")): add(t, 'EquipmentSetData', sid, names, '')

# SItemData (item special effects)
for o in iter_records(os.path.join(CD, 'SItemData.xml'), 'MiSItemData'):
    iid = int(mval(o, 'ID'))
    tk = o.find("member[@name='tokusei']")
    if tk is None: continue
    n, d = itemtext.get(iid, ('', ''))
    for t in ints(tk): add(t, 'SItemData', iid, n, d)

# ModificationExtEffectData
for o in iter_records(os.path.join(CD, 'ModificationExtEffectData.xml'), 'MiModificationExtEffectData'):
    t = int(mval(o, 'tokusei') or 0)
    if t > 0: add(t, 'ModificationExtEffectData', f"{mval(o,'groupID')}/{mval(o,'slot')}/{mval(o,'subID')}", esc(mval(o, 'shortDesc')), esc(mval(o, 'fullDesc')))

# ModifiedEffectData (+ client text)
for o in iter_records(os.path.join(CD, 'ModifiedEffectData.xml'), 'MiModifiedEffectData'):
    t = int(mval(o, 'tokusei') or 0); mid = int(mval(o, 'ID'))
    if t > 0:
        n, d = cmodtext.get(mid, ('', ''))
        add(t, 'ModifiedEffectData', mid, n, d)

# ModificationData
for o in iter_records(os.path.join(CD, 'ModificationData.xml'), 'MiModificationData'):
    for m in o.iter('member'):
        if m.get('name') == 'tokusei' and m.text and m.text.isdigit() and int(m.text) > 0:
            add(int(m.text), 'ModificationData', mval(o, 'ID'))

# DevilBoostData / DevilBoostExtraData (any member named tokusei)
for fn, on in (('DevilBoostData.xml', 'MiDevilBoostData'), ):
    p = os.path.join(CD, fn)
    if os.path.exists(p):
        for o in iter_records(p, on):
            rid = o.find('.//member[@name="ID"]')
            for m in o.iter('member'):
                if m.get('name') == 'tokusei':
                    for t in ints(m): add(t, 'DevilBoostData', rid.text if rid is not None else '')
p = os.path.join(SB, 'DevilBoostExtraData.xml')
if os.path.exists(p):
    for o in iter_records(p, 'MiDevilBoostExtraData'):
        sid, iid = mval(o, 'stackID'), int(mval(o, 'itemID') or 0)
        n, d = itemtext.get(iid, ('', ''))
        for m in o.iter('member'):
            if m.get('name') == 'tokusei':
                for t in ints(m): add(t, 'DevilBoostExtraData(stack/item)', f"{sid}/{iid}", n, d)

# Mitama set bonus: type -1 = tokusei
for o in iter_records(os.path.join(SB, 'MitamaReunionSetBonusData.xml'), 'MiMitamaReunionSetBonusData'):
    mid = mval(o, 'ID')
    for m in o.findall('member'):
        vals = [int(e.text) for e in m.findall('element') if e.text and e.text.lstrip('-').isdigit()]
        for i in range(0, len(vals) - 1, 2):
            if vals[i] == -1 and vals[i + 1] > 0:
                add(vals[i + 1], 'MitamaReunionSetBonusData.' + m.get('name'), mid, '', esc(mval(o, m.get('name') + 'Description')))

# server-side datastore XMLs
for fn, on, src in (('enchantset.xml', 'EnchantSetData', 'datastore/enchantset'), ('sstatus.xml', 'MiSStatusData', 'datastore/sstatus')):
    root = ET.parse(os.path.join(DS, fn)).getroot()
    for o in root.findall(f"object[@name='{on}']"):
        rid = int(mval(o, 'ID'))
        n, d = statustext.get(rid, ('', '')) if 'sstatus' in src else ('', '')
        for m in o.findall("member[@name='tokusei']"):
            for t in ints(m): add(t, src, rid, n, d)
for f in glob.glob(os.path.join(DS, 'sitemextended', '*.xml')):
    root = ET.parse(f).getroot()
    for o in root:
        rid = mval(o, 'ID')
        for m in o.iter('member'):
            if m.get('name') and 'okusei' in m.get('name'):
                for t in ints(m) or ([int(m.text)] if m.text and m.text.isdigit() else []):
                    n, d = itemtext.get(int(rid), ('', '')) if rid and rid.isdigit() else ('', '')
                    add(t, 'datastore/sitemextended', rid, n, d)

# ---- write outputs
os.makedirs(OUT, exist_ok=True)
with open(os.path.join(OUT, 'tokusei-refs.tsv'), 'w', encoding='utf-8', newline='\n') as o:
    o.write('#tokusei\tstatus(ok/empty/missing)\tcompHackFields\tclientCharasticText\tsource\trecord\trecordName\trecordText\n')
    for t in sorted(refs):
        st = 'missing' if t not in known else ('empty' if known[t] == 'empty' else 'ok')
        for (src, rec, n, d) in refs[t]:
            o.write(f"{t}\t{st}\t{known.get(t,'')}\t{chartext.get(t,'')}\t{src}\t{rec}\t{n}\t{d}\n")
with open(os.path.join(OUT, 'charastic-text.tsv'), 'w', encoding='utf-8', newline='\n') as o:
    o.write('#tokusei\tinCompHack(ok/empty/missing)\tclientText\n')
    for t in sorted(chartext):
        st = 'missing' if t not in known else ('empty' if known[t] == 'empty' else 'ok')
        o.write(f"{t}\t{st}\t{chartext[t]}\n")
miss = [t for t in refs if t not in known]
emp = [t for t in refs if t in known and known[t] == 'empty']
print('referenced', len(refs), 'missing', len(miss), 'empty', len(emp), 'charastic texts', len(chartext))
by = defaultdict(int)
for t in miss:
    for r in refs[t]: by[r[0]] += 1
print(dict(by))
