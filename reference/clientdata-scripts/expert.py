import sys, os, xml.etree.ElementTree as ET
CD, OUT = sys.argv[1], sys.argv[2]
BS = chr(92)
def esc(s): return (s or '').replace('\r', '').replace('\n', BS + 'n').replace('\t', ' ').strip()
skill = {}
with open(os.path.join(CD, 'cskill.tsv'), encoding='utf-8') as f:
    for line in f:
        if line.startswith('#'): continue
        p = line.rstrip('\n').split('\t')
        skill[int(p[0])] = p[1]
root = ET.parse(os.path.join(CD, 'ExpertClassData.xml')).getroot()
with open(OUT, 'w', encoding='utf-8', newline='\n') as o:
    o.write('#expertise\tmaxClass\tclass\trank\tskillIds\tskillNames\n')
    for e in root.findall("object[@name='MiExpertData']"):
        eid = e.find("member[@name='ID']").text
        mc = e.find("member[@name='maxClass']").text
        for ci, c in enumerate(e.findall("member[@name='classData']/element/object")):
            for ri, r in enumerate(c.findall("member[@name='rankData']/element/object")):
                ids = [int(x.text) for x in r.findall("member[@name='skill']/element") if x.text and int(x.text) > 0]
                if ids:
                    o.write(f"{eid}\t{mc}\t{ci}\t{ri}\t{','.join(map(str,ids))}\t{' | '.join(skill.get(i,'?') for i in ids)}\n")
