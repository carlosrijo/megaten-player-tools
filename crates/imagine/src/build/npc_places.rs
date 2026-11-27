//! Who opens an NPC exchange list, and where. A port of the event tracing in tools\dcmexport\Program.cs (the DCM
//! Locator's "Talk to Junkerman in Home III" lines), for the Skill Database and the Rebirth Planner.
//!
//! The client's NPCBarterGroupData is the list an NPC offers; a server event opens it with `EventOpenMenu`, menuType 808
//! (MENU_BARTER in constants.xml), `shopID` = the group. An event file does not say which zone it belongs to, so events
//! are traced back: zone files name the events their NPCs start, events name further events (any value of an event that
//! is another event's id). The NPC is the `ServerNPC` (or `ServerObject`) the starting event hangs on, named by the
//! client's hNPCData (oNPCData for objects, when the name is readable); the zone is the `ServerZone`'s ZoneData name,
//! or for a `ServerZonePartial` every zone whose DynamicMapID it lists. A partial under partials\<yyyymm_event>\ is a
//! seasonal event's: the opener says so ("Ordeal 2015 event"; the catch-all 999999_all is not one). An interior
//! (ZoneData type SHOP: a shop's own room, the Cathedral of Shadows) has no town of its own; the zones whose files send
//! you into it (`zoneID` members) follow in brackets, per zone id, so two rooms with one name keep their own towns:
//! "Cathedral of Shadows (Home III / Shinjuku Babel / Holy City Arcadia and 2 more)". When nothing leads to an opening
//! event, a zone id written inside its own id ("D920X_920106_TR01a") still places it, without an NPC.
//!
//! Inputs: the decoded client tables ZoneData.xml, hNPCData.xml, oNPCData.xml (smt-decoded\server) and the server
//! datastore's `zones`, `partials` and `events` folders.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::path::Path;

use regex::Regex;
use roxmltree::Node;

use super::demon_db::season_name;
use super::xml::{self, elements, m, mem, obj};
use crate::Result;
use crate::fmt;

/// One way an exchange list opens: the NPC to talk to (empty: the data names none), the zone as the client names it,
/// the towns that zone is entered from when it is an interior, and the seasonal event the file belongs to.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Opener {
    pub npc: String,
    pub zone: String,
    pub towns: Vec<String>,
    pub season: Option<String>,
}

/// A zone as the tracing labels it: its name and, for an interior, the towns it is entered from.
type Label = (String, Vec<String>);

#[derive(Default)]
pub struct NpcPlaces {
    /// barter group (the list an NPC offers) -> who opens it, where
    pub openers: HashMap<i32, Vec<Opener>>,
    /// how many exchange lists an event opens; the lists no event opens are the client's only
    pub lists_opened: usize,
}

fn int(s: Option<&str>) -> i32 {
    s.and_then(fmt::parse_i32).unwrap_or(0)
}

fn int_of(o: Option<Node>, name: &str) -> i32 {
    int(m(o, name).as_deref())
}

fn is_zone_top(o: Node) -> bool {
    matches!(o.attribute("name"), Some("ServerZone") | Some("ServerZonePartial"))
}

/// The comment just before a top object (`<!-- 邪教の館 -->`), as the C# `NodesBeforeSelf().OfType<XComment>().Last()`.
fn comment_before(o: Node) -> String {
    for s in o.prev_siblings() {
        if s.is_element() {
            break;
        }
        if s.is_comment() {
            return s.text().unwrap_or("").trim().to_string();
        }
    }
    String::new()
}

/// Every element under `o` with no element children, as the C# `Descendants().Where(e => !e.HasElements)`.
fn leaves<'a, 'i>(o: Node<'a, 'i>) -> impl Iterator<Item = Node<'a, 'i>> {
    o.descendants().filter(move |n| n.is_element() && *n != o && !n.children().any(|c| c.is_element()))
}

/// The seasonal event a datastore path belongs to: partials\<yyyymm_name>\... (the catch-all 999999_all is none).
fn season_of(rel: &str) -> Option<String> {
    let parts: Vec<&str> = rel.split('/').collect();
    if parts.first() == Some(&"partials") && parts.len() > 2 && Regex::new(r"^20\d\d(0[1-9]|1[0-2])_").expect("season folder").is_match(parts[1]) {
        Some(season_name(parts[1]))
    } else {
        None
    }
}

impl NpcPlaces {
    /// Reads the tables and traces every exchange list an event opens back to its NPCs and zones.
    pub fn load(decoded: &Path, store: &Path) -> Result<Self> {
        // ---- the client's names ----
        let mut zone_name: HashMap<i32, String> = HashMap::new();
        let mut shop_zones: HashSet<i32> = HashSet::new(); // interiors: a shop's own room, a clinic, the Cathedral of Shadows
        {
            let text = xml::read_text(&decoded.join("ZoneData.xml"))?;
            let doc = xml::parse(&text)?;
            for z in xml::records(&doc) {
                let b = obj(Some(z), "basic");
                if b.is_none() {
                    continue;
                }
                let id = int_of(b, "id");
                if let Some(n) = m(b, "name").filter(|n| !n.trim().is_empty()) {
                    zone_name.insert(id, n);
                }
                if m(b, "type").as_deref() == Some("SHOP") {
                    shop_zones.insert(id);
                }
            }
        }
        let mut npc_name: HashMap<i32, String> = HashMap::new();
        {
            let text = xml::read_text(&decoded.join("hNPCData.xml"))?;
            let doc = xml::parse(&text)?;
            for n in xml::records(&doc) {
                let b = obj(Some(n), "basic");
                if let Some(name) = m(b, "name").filter(|n| !n.trim().is_empty()) {
                    npc_name.insert(int_of(b, "ID"), name);
                }
            }
        }
        let mut obj_name: HashMap<i32, String> = HashMap::new();
        if let Some(text) = xml::loadable_text(&decoded.join("oNPCData.xml"))? {
            let doc = xml::parse(&text)?;
            for n in xml::records(&doc) {
                if let Some(name) = m(Some(n), "name").filter(|n| !n.trim().is_empty() && n.chars().all(|c| (c as u32) < 128)) {
                    obj_name.insert(int_of(Some(n), "ID"), name);
                }
            }
        }

        // ---- every zone, partial and event file of the server data ----
        let mut docs: Vec<(String, String)> = Vec::new(); // (path under the store, text)
        for dir in ["zones", "partials", "events"] {
            let d = store.join(dir);
            if !d.is_dir() {
                continue;
            }
            for f in xml::enumerate_xml_files(&d, true)? {
                let rel = f.strip_prefix(store).unwrap_or(&f).to_string_lossy().replace('\\', "/");
                if rel.contains("tokusei") {
                    continue;
                }
                if let Some(text) = xml::loadable_text(&f)? {
                    docs.push((rel, text));
                }
            }
        }
        let parsed: Vec<(String, roxmltree::Document)> = docs.iter().filter_map(|(rel, text)| xml::parse(text).ok().map(|d| (rel.clone(), d))).collect();

        // ---- interiors: the towns whose zone files send you into them (by the interior's zone id) ----
        let mut entered_from: HashMap<i32, BTreeSet<String>> = HashMap::new();
        for (_, doc) in &parsed {
            for top in xml::records(doc).into_iter().filter(|o| o.attribute("name") == Some("ServerZone")) {
                let self_id = int_of(Some(top), "ID");
                let Some(self_name) = zone_name.get(&self_id) else { continue };
                if shop_zones.contains(&self_id) {
                    continue;
                }
                for mb in top.descendants().filter(|n| n.is_element() && n.tag_name().name() == "member" && n.attribute("name").is_some_and(|a| a.eq_ignore_ascii_case("zoneID"))) {
                    let to = int(Some(xml::value(mb).trim()));
                    if to == self_id || !shop_zones.contains(&to) {
                        continue;
                    }
                    let Some(to_name) = zone_name.get(&to) else { continue };
                    if to_name == self_name {
                        continue;
                    }
                    entered_from.entry(to).or_default().insert(self_name.clone());
                }
            }
        }
        let towns_of = |id: i32| -> Vec<String> { entered_from.get(&id).map(|s| s.iter().cloned().collect()).unwrap_or_default() };
        let zone_label = |top: Node| -> Label {
            let id = int_of(Some(top), "ID");
            match zone_name.get(&id) {
                Some(n) => (n.clone(), towns_of(id)),
                None => {
                    let c = comment_before(top);
                    (if c.is_empty() { format!("Zone {id}") } else { c }, Vec::new())
                }
            }
        };
        // a partial has no name: it is laid over every zone that uses one of its dynamic maps
        let mut dyn_zones: HashMap<String, BTreeSet<Label>> = HashMap::new();
        for (_, doc) in &parsed {
            for top in xml::records(doc).into_iter().filter(|o| o.attribute("name") == Some("ServerZone")) {
                let dm = m(Some(top), "DynamicMapID").unwrap_or_default();
                if dm.is_empty() {
                    continue;
                }
                dyn_zones.entry(dm).or_default().insert(zone_label(top));
            }
        }
        let labels_of_top = |top: Node| -> Vec<Label> {
            if top.attribute("name") == Some("ServerZone") {
                return vec![zone_label(top)];
            }
            let maps: Vec<String> = mem(Some(top), "DynamicMapIDs").map(|ms| elements(ms, "element").map(|e| xml::value(e).trim().to_string()).collect()).unwrap_or_default();
            let mut found: BTreeSet<Label> = BTreeSet::new();
            for mp in maps {
                if let Some(zs) = dyn_zones.get(&mp) {
                    found.extend(zs.iter().cloned());
                }
            }
            found.into_iter().collect()
        };

        // ---- the event graph: which zones (and NPCs) start an event, which events lead to it ----
        let mut event_ids: HashSet<String> = HashSet::new();
        for (_, doc) in &parsed {
            for top in xml::records(doc).into_iter().filter(|o| !is_zone_top(*o) && o.attribute("name") != Some("DropSet")) {
                if let Some(id) = m(Some(top), "ID").filter(|id| !id.is_empty() && fmt::parse_i32(id).is_none()) {
                    event_ids.insert(id);
                }
            }
        }
        let mut called_by: HashMap<String, HashSet<String>> = HashMap::new(); // event -> the events that lead to it
        let mut zones_of: HashMap<String, BTreeSet<Opener>> = HashMap::new(); // event -> the zones (and NPCs) that start it
        for (rel, doc) in &parsed {
            let season = season_of(rel);
            for top in xml::records(doc) {
                let zone_top = is_zone_top(top);
                let self_id = if zone_top { None } else { m(Some(top), "ID") };
                if !zone_top && !self_id.as_ref().is_some_and(|s| event_ids.contains(s)) {
                    continue;
                }
                let labels = if zone_top { labels_of_top(top) } else { Vec::new() };
                for leaf in leaves(top) {
                    let v = xml::value(leaf).trim().to_string();
                    if self_id.as_deref() == Some(v.as_str()) || !event_ids.contains(&v) {
                        continue;
                    }
                    if let Some(s) = &self_id {
                        called_by.entry(v).or_default().insert(s.clone());
                        continue;
                    }
                    // who starts it: the NPC or object the event hangs on
                    let mut npc = String::new();
                    if let Some(holder) = leaf.ancestors().find(|a| a.is_element() && a.tag_name().name() == "object" && matches!(a.attribute("name"), Some("ServerNPC") | Some("ServerObject"))) {
                        let actor = m(Some(holder), "ActorID").map(|a| int(Some(&a))).unwrap_or_else(|| int_of(Some(holder), "ID"));
                        let names = if holder.attribute("name") == Some("ServerNPC") { &npc_name } else { &obj_name };
                        npc = names.get(&actor).cloned().unwrap_or_default();
                    }
                    let set = zones_of.entry(v).or_default();
                    for (zone, towns) in &labels {
                        set.insert(Opener { npc: npc.clone(), zone: zone.clone(), towns: towns.clone(), season: season.clone() });
                    }
                }
            }
        }
        let zones_for = |ev: &str| -> BTreeSet<Opener> {
            let mut found: BTreeSet<Opener> = BTreeSet::new();
            if ev.is_empty() {
                return found;
            }
            let mut todo: VecDeque<String> = VecDeque::from([ev.to_string()]);
            let mut done: HashSet<String> = HashSet::from([ev.to_string()]);
            while let Some(e) = todo.pop_front() {
                if let Some(zs) = zones_of.get(&e) {
                    found.extend(zs.iter().cloned());
                }
                for c in called_by.get(&e).into_iter().flatten() {
                    if done.insert(c.clone()) {
                        todo.push_back(c.clone());
                    }
                }
            }
            found
        };
        // "D920X_920106_TR01a": when nothing leads to an event, the zone id inside its own name still says where it is
        let re_digits = Regex::new(r"\d{4,}").expect("digits");
        let zone_from_event_id = |ev: &str| -> Option<Opener> {
            for c in re_digits.find_iter(ev) {
                if let Some((id, zn)) = c.as_str().parse::<i32>().ok().and_then(|id| zone_name.get(&id).map(|n| (id, n))) {
                    return Some(Opener { npc: String::new(), zone: zn.clone(), towns: towns_of(id), season: None });
                }
            }
            None
        };

        // ---- the exchange lists: EventOpenMenu, menuType 808, shopID = the group ----
        let mut openers: HashMap<i32, Vec<Opener>> = HashMap::new();
        for (_, doc) in &parsed {
            for ev in xml::records(doc).into_iter().filter(|o| o.attribute("name") == Some("EventOpenMenu") && m(Some(*o), "menuType").as_deref() == Some("808")) {
                let group = int_of(Some(ev), "shopID");
                if group == 0 {
                    continue;
                }
                let id = m(Some(ev), "ID").unwrap_or_default();
                let mut who = zones_for(&id);
                if who.is_empty() {
                    if let Some(o) = zone_from_event_id(&id) {
                        who.insert(o);
                    }
                }
                let list = openers.entry(group).or_default();
                for o in who {
                    if !list.contains(&o) {
                        list.push(o);
                    }
                }
            }
        }
        openers.retain(|_, v| !v.is_empty());
        let lists_opened = openers.len();
        Ok(NpcPlaces { openers, lists_opened })
    }

    /// The openers of every list among `groups`, each once.
    pub fn of_groups(&self, groups: &[i32]) -> Vec<Opener> {
        let mut out: Vec<Opener> = Vec::new();
        for g in groups {
            for o in self.openers.get(g).into_iter().flatten() {
                if !out.contains(o) {
                    out.push(o.clone());
                }
            }
        }
        out
    }
}

/// "Demonic Researcher in Cathedral of Shadows (Home III / Shinjuku Babel / Holy City Arcadia and 2 more)";
/// "Demon God Thoth in Home III, Shinjuku Babel (Ordeal 2015 event)"; "an NPC in Ginza" when the data names none.
/// Several NPCs are separated by "; ". Empty when there is nothing to say.
pub fn say(openers: &[Opener]) -> String {
    struct Who {
        npc: String,
        zones: Vec<(String, Vec<String>)>,
        seasons: Vec<String>,
    }
    let mut people: Vec<Who> = Vec::new(); // in first-seen order
    for o in openers {
        if o.zone.is_empty() && o.npc.is_empty() {
            continue;
        }
        let w = match people.iter_mut().find(|w| w.npc == o.npc) {
            Some(w) => w,
            None => {
                people.push(Who { npc: o.npc.clone(), zones: Vec::new(), seasons: Vec::new() });
                people.last_mut().expect("just pushed")
            }
        };
        if !o.zone.is_empty() {
            match w.zones.iter_mut().find(|(z, _)| *z == o.zone) {
                Some((_, towns)) => {
                    for t in &o.towns {
                        if !towns.contains(t) {
                            towns.push(t.clone());
                        }
                    }
                }
                None => w.zones.push((o.zone.clone(), o.towns.clone())),
            }
        }
        if let Some(s) = &o.season {
            let t = format!("{s} event");
            if !w.seasons.contains(&t) {
                w.seasons.push(t);
            }
        }
    }
    let more = |n: usize| if n > 0 { format!(" and {n} more") } else { String::new() };
    let mut parts: Vec<String> = Vec::new();
    for w in people {
        let mut s = if w.npc.is_empty() { "an NPC".to_string() } else { w.npc };
        if !w.zones.is_empty() {
            let shown: Vec<String> = w.zones.iter().take(4).map(|(z, towns)| {
                if towns.is_empty() {
                    z.clone()
                } else {
                    let t: Vec<&str> = towns.iter().take(4).map(String::as_str).collect();
                    format!("{z} ({}{})", t.join(" / "), more(towns.len().saturating_sub(4)))
                }
            }).collect();
            s.push_str(&format!(" in {}{}", shown.join(", "), more(w.zones.len().saturating_sub(4))));
        }
        if !w.seasons.is_empty() {
            s.push_str(&format!(" ({})", w.seasons.join(", ")));
        }
        parts.push(s);
    }
    parts.join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn o(npc: &str, zone: &str, towns: &[&str], season: Option<&str>) -> Opener {
        Opener { npc: npc.into(), zone: zone.into(), towns: towns.iter().map(|t| t.to_string()).collect(), season: season.map(String::from) }
    }

    #[test]
    fn says_who_and_where() {
        assert_eq!(say(&[o("Incubus", "Home II", &[], None)]), "Incubus in Home II");
        // one room's name in several towns: the towns merge behind the name
        assert_eq!(
            say(&[o("Demonic Researcher", "Cathedral of Shadows", &["Home III"], None), o("Demonic Researcher", "Cathedral of Shadows", &["Shinjuku Babel"], None)]),
            "Demonic Researcher in Cathedral of Shadows (Home III / Shinjuku Babel)"
        );
        assert_eq!(
            say(&[o("Demon God Thoth", "Shinjuku Babel", &[], Some("Ordeal 2015")), o("Demon God Thoth", "Home III", &[], Some("Ordeal 2015"))]),
            "Demon God Thoth in Shinjuku Babel, Home III (Ordeal 2015 event)"
        );
        assert_eq!(say(&[o("", "Ginza", &[], None), o("Rag", "Rag's Shop", &["Kakyo Castle"], None)]), "an NPC in Ginza; Rag in Rag's Shop (Kakyo Castle)");
        let many: Vec<Opener> = (1..=6).map(|i| o("A", &format!("Z{i}"), &[], None)).collect();
        assert_eq!(say(&many), "A in Z1, Z2, Z3, Z4 and 2 more");
        let towns: Vec<Opener> = (1..=6).map(|i| o("A", "Room", &[&format!("T{i}")], None)).collect();
        assert_eq!(say(&towns), "A in Room (T1 / T2 / T3 / T4 and 2 more)");
        assert_eq!(say(&[]), "");
    }

    #[test]
    fn seasons_are_dated_partials_only() {
        assert_eq!(season_of("partials/201501_ordeal/events/extra/201501_ordeal.xml"), Some("Ordeal 2015".to_string()));
        assert_eq!(season_of("partials/999999_all/zones/partial/extra/999999_all.xml"), None);
        assert_eq!(season_of("events/zone_events-241001.xml"), None);
    }

    #[test]
    fn of_groups_dedupes() {
        let mut p = NpcPlaces::default();
        p.openers.insert(1, vec![o("Rag", "Rag's Shop", &["Home III"], None)]);
        p.openers.insert(2, vec![o("Rag", "Rag's Shop", &["Home III"], None), o("Rag", "Rag's Shop", &["Kakyo Castle"], None)]);
        assert_eq!(p.of_groups(&[1, 2, 3]).len(), 2);
    }
}
