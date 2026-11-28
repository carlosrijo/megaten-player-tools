//! What the demon summoned beside the character gives the character itself.
//!
//! A tokusei is aimed by its TargetType. comp_hack's `TokuseiManager::Recalculate` collects every direct tokusei of an
//! entity; one aimed at the SUMMONER on a *partner demon* goes to the other entity of the pair, the character
//! (`otherEffects`), while on the character it is the digitalize path ("mostly affects digitalize skills"). A demon's
//! direct tokusei (`GetDirectTokusei`, PARTNER_DEMON) are: its force stack (`DemonState::UpdateDemonState`), the Mitama
//! set bonuses (`GetMitamaIndirectSetBonuses`), the passives of its learned skills and its traits with the worn demon
//! equipment's skill in place of the slot's trait (`DemonState::GetAllSkills`), and its compendium bonuses
//! (those are in `character_extras`). So with Alilat out, the game adds her force stack's "LNG-R +60" to the character.
//!
//! Counted here: SUMMONER-targeted tokusei of those parts, whatever they do (stats, damage, cost changes), and the
//! SELF-targeted ones as the partner's own (an epitaph's "Pursuit chance +20%" is the demon's). Not counted:
//! PARTY-targeted ones (they need the character to be in a party, which the reader can't see), switch skills (which
//! switches are on isn't read), and status effects. While the character is digitalized there is no partner, so none of
//! this applies then (the page leaves it out of the digitalized totals); `Digitalize::compute` keeps its own rule for
//! the digitalize demon's force stack.
//!
//! Checked: Rubito + Alilat Lv 93 summoned, LNG-R 544 = 137 + 45 + 177 + 125 + 60 (the 60 from force stack 10283,
//! tokusei 130199); with nothing summoned 525 against 585 with her out.

use super::digitalize::{Digitalize, mitama_sets_of, set_name};
use super::effect::Effect;
use crate::Result;
use crate::data::json::{self, At};
use crate::game::DemonRecord;

/// The digitalize window's picture for the Mitama set boost (the one the Digitalize page shows beside that boost): it stands
/// in for the summoned demon's Mitama set rows.
const MITAMA_PICTURE: &str = "digitalize_mitamaicon";
/// The digitalize window's Features boost picture: it stands in for a feature (trait), which has none of its own.
const FEATURE_PICTURE: &str = "digitalize_featureicon";

/// What stands for a summoned demon's bonus line on the page: a skill and an item (a worn equipment's Epitaph Parts, or the
/// item a Demon Force stack entry is drawn with; their picture is used when the skill has none) or a client interface picture.
enum Pic {
    Skill(i32, i32),
    Client(&'static str),
}

/// What a summoned demon's name is on the page: "Alilat (summoned)".
pub fn source_name(d: &DemonRecord) -> String {
    format!("{} (summoned)", d.name)
}

impl Digitalize {
    /// The effects the demon `d` (the summoned partner) gives its summoner (`target` "self"), and what its own skills,
    /// equipment, Demon Force and Mitama sets give the demon itself (`target` "partner": the partner's totals). Counted
    /// ones are "always" (cond), the rest "situational" (a requirement the reader can't decide) or "inactive". Every
    /// effect is tagged `source` "summoned" and `item` [`source_name`]; its `cond_note` starts with where on the demon it
    /// comes from ("Demon Force", "passive Life Surge", ...).
    /// `mitama_boost`: the character has MITAMA_SET_BOOST (boosted Mitama set values).
    pub fn summoner_effects(&self, d: &DemonRecord, mitama_boost: bool) -> Result<Vec<Effect>> {
        let data = &*self.data;
        let t = data.demons()?;
        let mut out = Vec::new();
        let Some(devil) = t.devils.get(&d.r#type) else { return Ok(out) };
        let item = source_name(d);
        // (tokusei, where it comes from, its picture)
        let mut found: Vec<(i32, String, Pic)> = Vec::new();

        // Demon Force stack
        for &sid in d.force_stack.as_deref().unwrap_or(&[]) {
            if let Some(toks) = (sid > 0).then(|| t.force_stack_tokusei.get(&sid)).flatten() {
                let item = t.force_stack_item.get(&sid).copied().unwrap_or(0);
                found.extend(toks.iter().map(|&tid| (tid, "Demon Force".to_string(), Pic::Skill(0, item))));
            }
        }
        // Mitama sets (the tokusei ones: bonus entries of -1)
        if d.mitama_type > 0 {
            for set in mitama_sets_of(d, t, false) {
                let b = if mitama_boost { &set.bonus_ex } else { &set.bonus };
                let mut i = 0;
                while i + 1 < b.len() {
                    if b[i] == -1 && b[i + 1] != 0 {
                        found.push((b[i + 1], format!("Mitama set {}", set_name(set.id)), Pic::Client(MITAMA_PICTURE)));
                    }
                    i += 2;
                }
            }
        }
        // Passives of the learned skills, then the traits (a worn demon equipment's skill replaces its slot's trait)
        let skill_name = |id: i32| -> Result<String> { Ok(data.skill_names()?.get(&id).cloned().flatten().unwrap_or_else(|| format!("skill {id}"))) };
        // (skill, where it comes from, the worn equipment item it stands for)
        let mut skills: Vec<(i32, &str, i32)> = Vec::new();
        for &id in d.skills.iter().filter(|&&x| x > 0) {
            if t.skills.get(&id).is_some_and(|&(cat, _)| cat == 0) {
                skills.push((id, "passive", 0));
            }
        }
        for i in 0..4usize {
            let equip = d.equipment.as_ref().and_then(|e| e.get(i)).copied().unwrap_or(0);
            if equip > 0 {
                if let Some(&sk) = t.equipment_skill.get(&equip) {
                    skills.push((sk, "equipment", equip));
                }
            } else if i < devil.traits.len() && devil.traits[i] > 0 {
                skills.push((devil.traits[i], "trait", 0));
            }
        }
        for (id, kind, item_id) in skills {
            let Some(p) = data.passives()?.get(&id.to_string()).filter(|v| !v.is_null()) else { continue };
            let name = skill_name(id)?;
            for tn in json::arr(p.at("tokuseiIds")) {
                let tid = json::int(Some(tn));
                if tid != 0 {
                    found.push((tid, format!("{kind} {name}"), Pic::Skill(id, item_id)));
                }
            }
        }

        for (tid, from, pic) in found {
            // the ones the server aims at the summoner go to the character; the ones aimed at the demon itself (TargetType
            // SELF, the default: an epitaph's "Pursuit chance +20%") are the partner's own; the rest (PARTY, ...) aren't
            // counted. A tokusei the data lacks can't be told.
            let Some(def) = data.tokusei_full(tid)? else { continue };
            let target = match def.target.as_str() {
                "SUMMONER" => "self",
                "SELF" => "partner",
                _ => continue,
            };
            for mut e in self.fx.tokusei(tid, "summoned", &item)? {
                e.target = target.into();
                e.source = "summoned".into();
                e.item = item.clone();
                match pic {
                    Pic::Skill(skill_id, item_id) => {
                        e.skill_id = skill_id;
                        e.item_id = item_id;
                        // a feature has no picture of its own: the digitalize window's Features boost picture stands in
                        if from.starts_with("trait ") {
                            e.picture = FEATURE_PICTURE.to_string();
                        }
                    }
                    Pic::Client(name) => e.picture = name.to_string(),
                }
                // a tokusei that needs the owner digitalized has nothing to do beside a partner
                if e.cond == "digitalized" {
                    e.cond = "inactive".into();
                }
                e.cond_note = if e.cond_note.is_empty() { from.clone() } else { format!("{from}; {}", e.cond_note) };
                out.push(e);
            }
        }
        Ok(out)
    }
}
