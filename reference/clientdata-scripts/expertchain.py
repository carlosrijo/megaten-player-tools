import sys, os, json, xml.etree.ElementTree as ET
CD, WEB, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
web = json.load(open(WEB, encoding='utf-8'))['expertises']
root = ET.parse(os.path.join(CD, 'ExpertClassData.xml')).getroot()
with open(OUT, 'w', encoding='utf-8', newline='\n') as o:
    o.write('#expertise\twebsiteName\tmaxClass\tisChain\tchainCount\tchain(sourceId:rankRequired:percent;...)\twebsiteChain\n')
    for e in root.findall("object[@name='MiExpertData']"):
        g = lambda n: (e.find(f"member[@name='{n}']").text if e.find(f"member[@name='{n}']") is not None else '')
        eid = g('ID')
        ch = []
        for c in e.findall("member[@name='chainData']/element/object"):
            d = {m.get('name'): m.text for m in c.findall('member')}
            if d['ID'] != '0' or d['rankRequired'] != '0':
                ch.append(f"{d['ID']}:{d['rankRequired']}:{round(float(d['chainPercent']), 4)}")
        w = web.get(eid, {})
        wc = ';'.join(f"{a}:{b}:{round(c, 4)}" for a, b, c in w.get('chain', []))
        o.write(f"{eid}\t{w.get('name','')}\t{g('maxClass')}\t{g('isChain')}\t{g('chainCount')}\t{';'.join(ch)}\t{wc}\n")
