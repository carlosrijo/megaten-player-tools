import re
BS = chr(92)
NL = BS + "n"
STAT = [  # (regex, key, is_pct)  value group 'v'
    (r'(?:limit break|lb) cap(?: damage)?\s*(?:\+|by \+?)\s*(?P<v>[\d,]+)|\+(?P<v2>[\d,]+) limit break cap', 'LIMIT_BREAK_MAX', False),
    (r'(?:limit break|lb) chance\s*\+(?P<v>\d+)%|\+(?P<v2>\d+)% limit break chance', 'LB_CHANCE', True),
    (r'(?:limit break|lb) (?:power|damage)\s*\+(?P<v>\d+)%|\+?(?P<v2>\d+)% limit break power', 'LB_DAMAGE', True),
    (r'final crit(?:ical)?(?: rate)?(?: correction)?\s*\+\s*(?P<v>\d+)%', 'FINAL_CRIT_CHANCE', True),
    (r'^(?:critical(?: rate)?|crit)\s*\+(?P<v>\d+)$', 'CRITICAL', False),
    (r'critical defense\s*\+(?P<v>\d+)', 'CRIT_DEF', False),
    (r'ta chance and power\s*\+(?P<v>\d+)%', 'TECH_ATTACK_RATE+TECH_ATTACK_POWER', True),
    (r'pursuit chance and power\s*\+(?P<v>\d+)%', 'PURSUIT_RATE+PURSUIT_POWER', True),
    (r'(?:ta|technical attack) chance\s*\+(?P<v>\d+)%', 'TECH_ATTACK_RATE', True),
    (r'(?:ta|technical attack|tehnical attack) power\s*\+(?P<v>\d+)%|(?P<v2>\d+)% te(?:c)?hnical attack power', 'TECH_ATTACK_POWER', True),
    (r'pursuit chance\s*\+(?P<v>\d+)%', 'PURSUIT_RATE', True),
    (r'pursuit power\s*\+(?P<v>\d+)%', 'PURSUIT_POWER', True),
    (r'close-?range damage taken\s*-(?P<v>\d+)%', 'RATE_CLSR_TAKEN-', True),
    (r'long-?range damage taken\s*-(?P<v>\d+)%', 'RATE_LNGR_TAKEN-', True),
    (r'(?:spell|magic) damage taken\s*-(?P<v>\d+)%', 'RATE_SPELL_TAKEN-', True),
    (r'damage taken\s*-(?P<v>\d+)%', 'RATE_ALL_TAKEN-', True),
    (r'close-?range damage\s*\+?(?P<v>\d+)%', 'RATE_CLSR', True),
    (r'long-?range damage\s*\+?(?P<v>\d+)%', 'RATE_LNGR', True),
    (r'support magic damage\s*\+?(?P<v>\d+)%', 'RATE_SUPPORT', True),
    (r'(?:spell|magic) damage\s*\+?(?P<v>\d+)%', 'RATE_SPELL', True),
    (r'support damage\s*\+?(?P<v>\d+)%', 'RATE_SUPPORT', True),
    (r'^(?:close-?range|cls-?r)\s*\+(?P<v>\d+)$', 'CLSR', False),
    (r'^(?:long-?range|lng-?r)\s*\+(?P<v>\d+)$', 'LNGR', False),
    (r'^spell\s*\+(?P<v>\d+)$', 'SPELL', False),
    (r'^support\s*\+(?P<v>\d+)$', 'SUPPORT', False),
    (r'max(?:imum)? hp\s*\+(?P<v>\d+)$', 'HP_MAX', False),
    (r'max(?:imum)? mp\s*\+(?P<v>\d+)$', 'MP_MAX', False),
    (r'max(?:imum)? hp\s*\+(?P<v>\d+)%', 'HP_MAX%', True),
    (r'max(?:imum)? mp\s*\+(?P<v>\d+)%', 'MP_MAX%', True),
    (r'^(?:strength|str)\s*\+\s*(?P<v>\d+)$', 'STR', False),
    (r'^(?:magic|mag)\s*\+\s*(?P<v>\d+)$', 'MAGIC', False),
    (r'^(?:vitality|vit)\s*\+\s*(?P<v>\d+)$', 'VIT', False),
    (r'^(?:intelligence|int)\s*\+\s*(?P<v>\d+)$', 'INT', False),
    (r'^(?:speed|spd)\s*\+\s*(?P<v>\d+)$', 'SPEED', False),
    (r'^(?:luck|lck)\s*\+\s*(?P<v>\d+)$', 'LUCK', False),
    (r'physical defense\s*\+(?P<v>\d+)', 'PDEF', False),
    (r'magic(?:al)? defense\s*\+(?P<v>\d+)', 'MDEF', False),
    (r'cooldown(?: time)?\s*-(?P<v>\d+)%', 'COOLDOWN_TIME-', True),
    (r'ailment resistance\s*\+(?P<v>\d+)', 'RES_STATUS-', False),  # comp_hack stores 'Ailment Resistance +N' as RES_STATUS -N
    (r'move(?:ment)? speed\s*\+(?P<v>\d+)%', 'MOVE2', True),
    (r'digitalize duration\s*\+(?P<v>\d+)%', 'DIGITALIZE_LENGTH', True),
]
AFF = {'slash': 'SLASH', 'thrust': 'THRUST', 'blunt': 'STRIKE', 'penetrate': 'PIERCE', 'spread': 'SPREAD', 'handgun': 'LNGR',
       'fire': 'FIRE', 'ice': 'ICE', 'electric': 'ELEC', 'elec': 'ELEC', 'force': 'FORCE', 'almighty': 'ALMIGHTY', 'expel': 'EXPEL',
       'death': 'CURSE', 'curse': 'CURSE', 'mystic': 'MAGICFORCE', 'nerve': 'NERVE', 'mind': 'MIND', 'curative': 'HEAL'}
ACT = ('attack', 'spin', 'rush', 'shot', 'rapid', 'counter', 'guard', 'dodge')
COND_RE = re.compile(r'^(when|while|if|during|for (?:all|every)|against|at |to all|except|on |after|upon)', re.I)
JP_RE = re.compile(r'[぀-ヿ一-鿿！-～]')

def norm(s):
    return s.replace('　', ' ').replace('＋', '+').replace('－', '-').strip(' .・*-•')

def parse(text):
    """-> (target, condition, parsed tokens, unparsed lines)"""
    if not text: return ('', '', '', '')
    if JP_RE.search(text) and not re.search(r'[A-Za-z]{4,}', text):
        return ('', '', '', 'japanese text, not parsed')
    raw = [l.strip(' 　') for l in text.split(NL)]
    lines = [norm(l) for l in raw if l and not re.fullmatch(r'-+ ?[\w ]+ ?-+', l)]
    lines = [l for l in lines if l and not l.startswith('※')]
    target, cond, toks, rest = 'self', [], [], []
    # split "While Digitalized, X +5%," into a condition and a stat line
    split = []
    for l in lines:
        m = re.match(r'^((?:when|while|if|during|against)[^,:]*)[,:]\s*(.+)$', l, re.I)
        if m and re.search(r'[+-]\s*\d', m.group(2)):
            split += [m.group(1) + ',', m.group(2).rstrip(',')]
        else:
            split.append(l)
    for l in split:
        ll = l.lower()
        if re.match(r"^(to )?partner('s)?[:,]?$", ll) or ll.startswith('to partner') or ll == "partner's":
            target = 'partner'; continue
        if ll.startswith('to all party') or ll.startswith('to party') or 'party members' in ll and len(ll) < 40:
            target = 'party'; continue
        if (COND_RE.match(ll) and (ll.endswith(',') or ll.endswith(':') or not re.search(r'[+-]\d', ll))) or \
           (not re.search(r'\d', ll) and re.search(r"(:|,|'|skills?)$", ll)):
            cond.append(l.rstrip(',:')); continue
        hit = False
        tgt = target
        sub = ll
        if re.search(r'\bto (?:your )?partner\.?$', sub):
            tgt, sub = 'partner', re.sub(r'\s*\bto (?:your )?partner\.?$', '', sub)
        if re.search(r'\bto all party members\.?$', sub):
            tgt, sub = 'party', re.sub(r'\s*\bto all party members\.?$', '', sub)
        for pre, tname in (("partner's", 'partner'), ("summoner's", 'summoner'), ("summoner", 'summoner'), ("party's", 'party')):
            if sub.startswith(pre + ' '):
                tgt, sub = tname, sub[len(pre) + 1:]
        pfx = '' if tgt == 'self' else tgt + ':'
        for part in re.split(r',\s*(?=[a-z])', sub):
            part = part.strip()
            found = False
            for rx, key, pct in STAT:
                m = re.search(rx, part)
                if m:
                    v = (m.groupdict().get('v') or m.groupdict().get('v2') or '').replace(',', '')
                    neg = key.endswith('-')
                    k = key.rstrip('-')
                    for kk in k.split('+'):
                        toks.append(f"{pfx}{kk}={'-' if neg else '+'}{v}{'%' if pct and not kk.endswith('%') else ''}")
                    found = True; break
            if not found:
                m = re.match(r'^([a-z/ ]+?) affinity\s*\+(\d+)%', part)
                if m and all(w.strip() in AFF for w in m.group(1).split('/')):
                    for w in m.group(1).split('/'):
                        toks.append(f"{pfx}BOOST_{AFF[w.strip()]}=+{m.group(2)}%")
                    found = True
            if not found:
                # "Spin Damage +N%" = comp_hack SkillCondition ACTION_TYPE + aspect EFFECT_POWER (not a stat)
                m = re.match(r'^(%s) damage\s*\+(\d+)%%' % '|'.join(ACT), part)
                if m:
                    toks.append(f"{pfx}EFFECT_POWER[action={m.group(1).upper()}]=+{m.group(2)}%")
                    found = True
            hit = hit or found
            if not found: rest.append(part)
    return (target, ' / '.join(cond), ' '.join(toks), ' | '.join(rest))

