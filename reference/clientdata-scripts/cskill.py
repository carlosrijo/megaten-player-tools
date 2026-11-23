import sys, xml.etree.ElementTree as ET
BS = chr(92)
def esc(s):
    return (s or '').replace('\r', '').replace('\n', BS + 'n').replace('\t', ' ')
src, out = sys.argv[1], sys.argv[2]
with open(out, 'w', encoding='utf-8', newline='\n') as o:
    o.write('#id\tname\tdesc (newlines as ' + BS + 'n)\n')
    for ev, el in ET.iterparse(src):
        if el.tag == 'object' and el.get('name') == 'MiCSkillBase':
            d = {m.get('name'): (m.text or '') for m in el.findall('member')}
            o.write(f"{d.get('ID')}\t{esc(d.get('name'))}\t{esc(d.get('desc'))}\n")
        if el.tag == 'object' and el.get('name') == 'MiCSkillData':
            el.clear()
