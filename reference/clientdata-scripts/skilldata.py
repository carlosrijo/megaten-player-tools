import sys, xml.etree.ElementTree as ET
# dump server SkillData: id, family, mainCategory, charastic (tokusei ids), correctTbl, specialParams
src, out = sys.argv[1], sys.argv[2]
with open(out, 'w', encoding='utf-8', newline='\n') as o:
    o.write('#id\tmain\tfamily\ttokusei\tcorrectTbl(id:type:value)\tspecialParams\n')
    for ev, el in ET.iterparse(src):
        if el.tag == 'object' and el.get('name') == 'MiSkillData':
            sid = el.find(".//object[@name='MiSkillItemStatusCommonData']/member[@name='id']").text
            main = el.find(".//object[@name='MiCategoryData']/member[@name='mainCategory']")
            fam = el.find(".//object[@name='MiSkillBasicData']/member[@name='family']")
            ch = [e.text for e in el.findall(".//object[@name='MiSkillCharasticData']/member[@name='charastic']/element") if e.text and e.text != '0']
            sp = [e.text for e in el.findall(".//object[@name='MiSkillSpecialParams']/member[@name='specialParams']/element")]
            ct = []
            for c in el.findall(".//object[@name='MiSkillItemStatusCommonData']/member[@name='correctTbl']/element/object"):
                d = {m.get('name'): m.text for m in c.findall('member')}
                if d.get('ID') and d.get('ID') != 'NONE':
                    ct.append(f"{d.get('ID')}:{d.get('Type')}:{d.get('Value')}")
            o.write(f"{sid}\t{main.text if main is not None else ''}\t{fam.text if fam is not None else ''}\t{','.join(ch)}\t{','.join(ct)}\t{','.join(sp)}\n")
            el.clear()
