# skill-icons.tsv: every skill's icon, as the client shows it.
#   python -I skillicons.py <cskill.xml> <CIconData_Skill.bin> <out.tsv>
# cskill.xml is the client's CSkillData.bin decoded (MiCSkillBase: ID, iconID). CIconData_Skill.bin is not encrypted:
# u32 count, then per icon u32 id + 36 bytes of name (the file in Interface\tga or Interface\tga2, without ".tga").
import sys, struct, xml.etree.ElementTree as ET
src, icons, out = sys.argv[1], sys.argv[2], sys.argv[3]
raw = open(icons, 'rb').read()
n = struct.unpack_from('<I', raw, 0)[0]
names = {}
for i in range(n):
    iid = struct.unpack_from('<I', raw, 4 + i * 40)[0]
    names[iid] = raw[8 + i * 40:44 + i * 40].split(b'\0')[0].decode('ascii', 'replace')
rows = []
for ev, el in ET.iterparse(src):
    if el.tag == 'object' and el.get('name') == 'MiCSkillBase':
        d = {m.get('name'): (m.text or '') for m in el.findall('member')}
        sid, iid = int(d.get('ID') or 0), int(d.get('iconID') or 0)
        if names.get(iid):
            rows.append((sid, iid, names[iid]))
    if el.tag == 'object' and el.get('name') == 'MiCSkillData':
        el.clear()
with open(out, 'w', encoding='utf-8', newline='\n') as o:
    o.write('#skill id\ticon id\ticon file (Interface\\tga or tga2, without .tga)\n')
    for r in sorted(set(rows)):
        o.write('%d\t%d\t%s\n' % r)
print(len(rows), 'skills with an icon;', n, 'icons')
