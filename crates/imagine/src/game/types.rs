//! Records shared by the game readers and the rest of the library (the record types declared at the top of
//! `src\Game\ClientReader.cs` and `src\Game\DemonReader.cs`). Defined here first so the data layer (Store) and the
//! readers can be written at the same time. C# `int[]` -> `Vec<i32>`, nullable -> `Option`.

use serde::{Deserialize, Serialize};

/// C# `record Nameplate(int Level, int Hp, int MaxHp, int Mp, int MaxMp)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Nameplate {
    pub level: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
}

/// One worn item as the game stores it (comp_hack objects::Item). basic = the item whose stats it carries (its own id
/// when not fused), special = the item whose special effects it carries (0 = its own), mods = 5 modification slot
/// effect ids (0 = no slot, 255 = open slot), fuse = the 3 fusion great-success bonuses. durability -1 = not known.
/// C# `record WornItem(int Id, int Tarot, int Soul, int Basic, int Special, int[] Mods, int[] Fuse, int Durability, int MaxDurability)`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WornItem {
    pub id: i32,
    pub tarot: i32,
    pub soul: i32,
    pub basic: i32,
    pub special: i32,
    pub mods: Vec<i32>,
    pub fuse: Vec<i32>,
    pub durability: i32,
    pub max_durability: i32,
}

/// Equipped items with each slot's tarot and soul enchant ids (0 = none). details (None when not read) is each slot's
/// full item record (an entry is None where C# had a null element).
/// C# `record GearGrid(long Address, int[] Ids, int[] Tarot, int[] Soul, int Matched, long NewestMs, WornItem[] Details = null)`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GearGrid {
    pub address: i64,
    pub ids: Vec<i32>,
    pub tarot: Vec<i32>,
    pub soul: Vec<i32>,
    pub matched: i32,
    pub newest_ms: i64,
    pub details: Option<Vec<Option<WornItem>>>,
}

/// C# `record SetRecord(int Id, int[] Pieces, int[] Tokusei)`: one equipment set (pieces) and the effects it gives.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SetRecord {
    pub id: i32,
    pub pieces: Vec<i32>,
    pub tokusei: Vec<i32>,
}

/// Information (status) window block: present only while that window is open. C# `class InfoWindow`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InfoWindow {
    pub address: i64,
    pub level: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    /// 12 stats x (base, bonus)
    pub pairs: Vec<i32>,
    /// 15 item ids, slot order
    pub items: Vec<i32>,
    /// 15 tooltip texts as shown in game ("" for empty slots)
    pub texts: Vec<String>,
}

/// One COMP demon as the client holds it (C# `record DemonRecord`, Game\DemonReader.cs; see its comments for the
/// memory layout). live_* come from its status object (what the demon window shows); the rest is the demon's own data.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DemonRecord {
    pub address: i64,
    pub flags: i32,
    pub name: String,
    pub r#type: i32,
    pub level: i32,
    pub object_id: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    /// CorrectTbl 0..20 (STR..MDEF): the window's number
    pub live_value: Vec<i32>,
    /// ... and its "+N" part
    pub live_bonus: Vec<i32>,
    pub growth_type: i32,
    /// 12 rebirth ranks (Tiwaz ... Wyrd order of the server)
    pub reunion: Vec<i32>,
    /// 0 = not Mitama-enhanced; 1 Ara, 2 Nigi, 3 Kusi, 4 Saki
    pub mitama_type: i32,
    pub mitama_rank: i32,
    /// 96 Mitama slot bonus ids, 8 per rune (0 = empty)
    pub mitama_reunion: Vec<u8>,
    /// 48 Mitama sets the client shows as active
    pub mitama_sets_shown: Vec<bool>,
    /// the 8 skill slots (0 = empty): what digitalizing uses
    pub skills: Vec<i32>,
    /// every learned skill (the Acquire Skills list)
    pub acquired: Vec<i32>,
    pub soul_points: i32,
    /// Demon Force: 20 values, 100000 = +1 (None where C# had null)
    pub force_values: Option<Vec<i32>>,
    /// Demon Force stack: up to 8 DevilBoostExtraData stack ids (0 = empty)
    pub force_stack: Option<Vec<i32>>,
    /// demon equipment: item type per trait slot 0-3 (0 = empty)
    pub equipment: Option<Vec<i32>>,
}

impl DemonRecord {
    /// Summoned right now (the client marks the partner's record).
    pub fn summoned(&self) -> bool {
        (self.flags & 0x02) != 0
    }

    /// Whether the client has this demon's details (its stats, skills, growth and Mitama). C# `Loaded`.
    pub fn loaded(&self) -> bool {
        !(self.live_value.iter().take(6).all(|&v| v == 0) && self.acquired.is_empty())
    }
}
