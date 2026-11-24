//! Dumps everything the game readers find, in a fixed text format (the C# twin of this program prints the same lines
//! from the real C# sources, so `diff` shows any difference between the ports). Read-only.
//!
//!   cargo run --release -p imagine --example dump_game [out.txt]
//!
//! Timings go to stderr, the dump to the file (or stdout).

use std::time::Instant;

use imagine::game::{client_reader as cr, demon_reader as dr, progress_reader as pr};
use imagine::memory::{self, GameMemory};

fn q(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('\r', "\\r").replace('\n', "\\n"))
}

fn qo(s: Option<&str>) -> String {
    s.map_or("null".to_string(), q)
}

fn j<'a>(a: impl IntoIterator<Item = &'a i32>) -> String {
    a.into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

/// A C# `int[]` that can be null: an empty list stands for null (see GearGrid.tarot).
fn jn(a: &[i32]) -> String {
    if a.is_empty() { "null".to_string() } else { j(a) }
}

fn b(v: bool) -> &'static str {
    if v { "1" } else { "0" }
}

fn d(t: &chrono::NaiveDateTime) -> String {
    t.format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn m(map: &Option<indexmap::IndexMap<i32, i32>>) -> String {
    match map {
        None => "null".to_string(),
        Some(m) => m.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(" "),
    }
}

fn timed<T>(what: &str, f: impl FnOnce() -> T) -> T {
    let t = Instant::now();
    let r = f();
    eprintln!("TIME {what}: {} ms", t.elapsed().as_millis());
    r
}

fn main() -> imagine::Result<()> {
    let out_path = std::env::args().nth(1);
    let mut out = String::new();
    macro_rules! w {
        ($($arg:tt)*) => {{ out.push_str(&format!($($arg)*)); out.push('\n'); }};
    }

    w!("## clients");
    let list = timed("list_clients", cr::list_clients);
    for c in &list {
        w!("client pid={} folder={} started={} character={} level={}", c.pid, q(&c.folder), d(&c.started), qo(c.character.as_deref()), c.level);
    }

    for c in list.iter().filter(|c| c.character.is_some()) {
        let name = c.character.clone().unwrap_or_default();
        w!("## character {name}");
        let proc = timed("find_client", || cr::find_client(&name))?;
        w!("find: pid={} hwnd={}", proc.id(), proc.main_window_handle());
        match &proc.exe_path {
            Some(p) => w!("find: exe={} started={}", q(&p.to_string_lossy()), d(&proc.started)),
            None => w!("find: exe unreadable"),
        }
        let mem = GameMemory::new(proc.id())?;
        timed("regions", || w!("regions: {} total={}", mem.regions().len(), mem.regions().iter().map(|r| r.size).sum::<i64>()));
        timed("detect_character", || match cr::detect_character(&mem) {
            Some((n, l)) => w!("detect: {} {}", q(&n), l),
            None => w!("detect: null "),
        });

        {
            let _shared = memory::share_sweeps();
            timed("first shared sweep (nameplate)", || match cr::read_nameplate(&mem, &name) {
                None => w!("nameplate: null"),
                Some(np) => w!("nameplate: {} {}/{} {}/{}", np.level, np.hp, np.max_hp, np.mp, np.max_mp),
            });
            for dig in [false, true] {
                timed(&format!("own_status({dig})"), || match cr::read_own_status(&mem, dig) {
                    None => w!("ownstatus digitalized={}: null", b(dig)),
                    Some(os) => w!("ownstatus digitalized={}: {} {}/{} {}/{} pairs={}", b(dig), os.level, os.hp, os.max_hp, os.mp, os.max_mp, j(&os.pairs)),
                });
            }
            timed("lnc", || {
                let l = cr::read_lnc(&mem);
                w!("lnc: {} {}", l.map_or("null".to_string(), |v| v.to_string()), l.map_or("", cr::lnc_name));
            });
            timed("worn_items", || w!("worn: {}", cr::read_worn_items(&mem).map_or("null".to_string(), |v| j(&v))));
            timed("gear_grids", || {
                let grids = cr::read_gear_grids(&mem);
                w!("grids: {}", grids.len());
                for g in &grids {
                    w!("grid: addr={} ids={} tarot={} soul={} matched={} newest={}", g.address, j(&g.ids), jn(&g.tarot), jn(&g.soul), g.matched, g.newest_ms);
                }
            });
            timed("live_gear", || {
                let Some(g) = cr::live_gear(&mem) else {
                    w!("livegear: null");
                    return;
                };
                w!(
                    "livegear: addr={} ids={} tarot={} soul={} matched={} newest={} details={}",
                    g.address, j(&g.ids), jn(&g.tarot), jn(&g.soul), g.matched, g.newest_ms,
                    if g.details.is_none() { "null" } else { "yes" }
                );
                if let Some(det) = &g.details {
                    for (k, dt) in det.iter().enumerate() {
                        match dt {
                            None => w!("detail[{k}]: null"),
                            Some(x) => w!(
                                "detail[{k}]: id={} tarot={} soul={} basic={} special={} mods={} fuse={} dur={} maxdur={}",
                                x.id, x.tarot, x.soul, x.basic, x.special, j(&x.mods), j(&x.fuse), x.durability, x.max_durability
                            ),
                        }
                    }
                }
            });
            timed("info_window", || {
                let Some(iw) = cr::read_info_window(&mem, &name, |_| true) else {
                    w!("infowindow: null");
                    return;
                };
                w!(
                    "infowindow: addr={} level={} hp={}/{} mp={}/{} pairs={} items={}",
                    iw.address, iw.level, iw.hp, iw.max_hp, iw.mp, iw.max_mp, j(&iw.pairs), j(&iw.items)
                );
                for k in 0..15 {
                    w!("infotext[{k}]: {}", q(&iw.texts[k]));
                }
            });
            timed("expertise", || {
                let e = cr::read_expertise_points(&mem);
                w!("expertise: {} {}", e.len(), e.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(" "));
            });
            timed("set_table", || {
                let Some(t) = cr::read_set_table(&mem) else {
                    w!("settable: null");
                    return;
                };
                w!("settable: {}", t.len());
                for s in &t {
                    w!("set {} pieces={} tokusei={}", s.id, j(&s.pieces), j(&s.tokusei));
                }
            });
            let all = timed("demons(all)", || {
                let all = dr::read_demons(&mem, true);
                w!("demons(all): {}", all.len());
                for x in &all {
                    w!(
                        "demon addr={} flags={} name={} type={} level={} obj={} hp={}/{} mp={}/{} value={} bonus={} growth={} reunion={} mitama={}/{} mitamaReunion={} sets={} skills={} acquired={} soul={} force={} stack={} equip={} summoned={} loaded={}",
                        x.address, x.flags, q(&x.name), x.r#type, x.level, x.object_id, x.hp, x.max_hp, x.mp, x.max_mp,
                        j(&x.live_value), j(&x.live_bonus), x.growth_type, j(&x.reunion), x.mitama_type, x.mitama_rank,
                        x.mitama_reunion.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","),
                        x.mitama_sets_shown.iter().map(|&v| b(v)).collect::<String>(),
                        j(&x.skills), j(&x.acquired), x.soul_points,
                        x.force_values.as_ref().map_or("null".to_string(), |v| j(v)),
                        x.force_stack.as_ref().map_or("null".to_string(), |v| j(v)),
                        x.equipment.as_ref().map_or("null".to_string(), |v| j(v)),
                        b(x.summoned()), b(x.loaded())
                    );
                }
                all
            });
            timed("demons(loaded)", || {
                let l = dr::read_demons(&mem, false);
                w!("demons(loaded): {} {}", l.len(), l.iter().map(|x| x.address.to_string()).collect::<Vec<_>>().join(","));
            });
            timed("comp", || {
                match dr::read_comp(&mem, &all) {
                    None => w!("comp: null"),
                    Some(comp) => w!("comp: {}", comp.iter().map(|x| format!("{}:{}", x.object_id, x.name)).collect::<Vec<_>>().join(",")),
                }
                match all.iter().find(|x| x.summoned()) {
                    None => w!("summoned: null"),
                    Some(s) => w!("summoned: {}:{}", s.object_id, s.name),
                }
            });
            timed("assists", || w!("assists: {}", dr::read_assists(&mem).map_or("null".to_string(), |v| j(&v))));
            timed("progress", || {
                let Some(p) = dr::read_progress(&mem) else {
                    w!("progress: null");
                    return;
                };
                let set = |s: &Option<indexmap::IndexSet<i32>>, count: bool| match s {
                    None => "null".to_string(),
                    Some(s) if count => format!("{}:{}", s.len(), j(s)),
                    Some(s) => j(s),
                };
                w!("progress.assists={}", j(&p.assists));
                w!("progress.skills={}", set(&p.skills, true));
                w!("progress.digitalized={} demon={}", b(p.digitalized()), p.digitalized_demon);
                w!("progress.levels={}", m(&p.levels));
                w!("progress.points={}", m(&p.points));
                w!("progress.switched_on={}", set(&p.switched_on, false));
                w!("progress.quests={}", set(&p.completed_quests, true));
                w!(
                    "progress.level={} ext={}",
                    p.level.map_or("null".to_string(), |v| v.to_string()),
                    p.expertise_extension.map_or("null".to_string(), |v| v.to_string())
                );
                w!("progress.has_skill(5133)={}", b(p.has_skill(5133)));
            });
            timed("progress_state", || match pr::read(&mem, Some(7), Some(8), Some(true)) {
                None => w!("progressstate: null"),
                Some(st) => w!(
                    "progressstate: clan={} learned={} quest={} comp={} v2={}",
                    qo(st.clan_name.as_deref()),
                    st.learned_skills.as_ref().map_or("null".to_string(), |s| s.len().to_string()),
                    st.quest_bonus_count.map_or(String::new(), |v| v.to_string()),
                    st.compendium_count.map_or(String::new(), |v| v.to_string()),
                    st.has_compendium_v2.map_or(String::new(), |v| if v { "True" } else { "False" }.to_string())
                ),
            });
            timed("compendium", || w!("compendium: {}", pr::read_compendium_count(&mem).map_or("null".to_string(), |v| v.to_string())));
            timed("find_bytes", || {
                let mut pat = name.as_bytes().to_vec();
                pat.push(0);
                let h = mem.find_bytes(&pat, 20);
                w!("find_bytes: count20={} first={}", h.len(), h.iter().take(5).map(|a| a.to_string()).collect::<Vec<_>>().join(","));
            });
            timed("find_seq", || {
                w!("find_seq: {}", mem.find_seq(&[2021, 0, 0, 0, 48002, 37286, 24359], 4, 3).iter().map(|a| a.to_string()).collect::<Vec<_>>().join(","));
            });
        }
        // un-shared timings
        eprintln!("-- not shared --");
        timed("nameplate (alone)", || cr::read_nameplate(&mem, &name));
        timed("own_status (alone)", || cr::read_own_status(&mem, false));
        timed("live_gear (alone)", || cr::live_gear(&mem));
        timed("demons (alone)", || dr::read_demons(&mem, true));
        timed("progress (alone)", || dr::read_progress(&mem));
        timed("info_window (alone)", || cr::read_info_window(&mem, &name, |_| true));
        timed("whole scan (alone)", || {
            let n: usize = mem.scan().map(|(_, buf)| buf.len()).sum();
            eprintln!("  bytes {n}");
        });
    }

    match out_path {
        Some(p) => std::fs::write(p, out)?,
        None => print!("{out}"),
    }
    Ok(())
}
