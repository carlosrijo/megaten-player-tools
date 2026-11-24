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
//! Counted here: SUMMONER-targeted tokusei of those parts, whatever they do (stats, damage, cost changes). Not counted:
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

/// What a summoned demon's name is on the page: "Alilat (summoned)".
pub fn source_name(d: &DemonRecord) -> String {
    format!("{} (summoned)", d.name)
}

impl Digitalize {
    /// The effects the demon `d` (the summoned partner) gives its summoner: counted ones are "always" (cond), the rest
    /// "situational" (a requirement the reader can't decide) or "inactive". Every effect is the character's own
    /// (`target` "self"), tagged `source` "summoned" and `item` [`source_name`]; its `cond_note` starts with where on
    /// the demon it comes from ("Demon Force", "passive Life Surge", ...).
    /// `mitama_boost`: the character has MITAMA_SET_BOOST (boosted Mitama set values).
    pub fn summoner_effects(&self, d: &DemonRecord, mitama_boost: bool) -> Result<Vec<Effect>> {
        let data = &*self.data;
        let t = data.demons()?;
        let mut out = Vec::new();
        let Some(devil) = t.devils.get(&d.r#type) else { return Ok(out) };
        let item = source_name(d);
        // (tokusei, where it comes from, the skill it belongs to for its icon)
        let mut found: Vec<(i32, String, i32)> = Vec::new();

        // Demon Force stack
        for &sid in d.force_stack.as_deref().unwrap_or(&[]) {
            if let Some(toks) = (sid > 0).then(|| t.force_stack_tokusei.get(&sid)).flatten() {
                found.extend(toks.iter().map(|&tid| (tid, "Demon Force".to_string(), 0)));
            }
        }
        // Mitama sets (the tokusei ones: bonus entries of -1)
        if d.mitama_type > 0 {
            for set in mitama_sets_of(d, t, false) {
                let b = if mitama_boost { &set.bonus_ex } else { &set.bonus };
                let mut i = 0;
                while i + 1 < b.len() {
                    if b[i] == -1 && b[i + 1] != 0 {
                        found.push((b[i + 1], format!("Mitama set {}", set_name(set.id)), 0));
                    }
                    i += 2;
                }
            }
        }
        // Passives of the learned skills, then the traits (a worn demon equipment's skill replaces its slot's trait)
        let skill_name = |id: i32| -> Result<String> { Ok(data.skill_names()?.get(&id).cloned().flatten().unwrap_or_else(|| format!("skill {id}"))) };
        let mut skills: Vec<(i32, &str)> = Vec::new();
        for &id in d.skills.iter().filter(|&&x| x > 0) {
            if t.skills.get(&id).is_some_and(|&(cat, _)| cat == 0) {
                skills.push((id, "passive"));
            }
        }
        for i in 0..4usize {
            let equip = d.equipment.as_ref().and_then(|e| e.get(i)).copied().unwrap_or(0);
            if equip > 0 {
                if let Some(&sk) = t.equipment_skill.get(&equip) {
                    skills.push((sk, "equipment"));
                }
            } else if i < devil.traits.len() && devil.traits[i] > 0 {
                skills.push((devil.traits[i], "trait"));
            }
        }
        for (id, kind) in skills {
            let Some(p) = data.passives()?.get(&id.to_string()).filter(|v| !v.is_null()) else { continue };
            let name = skill_name(id)?;
            for tn in json::arr(p.at("tokuseiIds")) {
                let tid = json::int(Some(tn));
                if tid != 0 {
                    found.push((tid, format!("{kind} {name}"), id));
                }
            }
        }

        for (tid, from, skill_id) in found {
            // only the ones the server aims at the summoner (a tokusei the data lacks can't be told)
            if !data.tokusei_full(tid)?.is_some_and(|def| def.target == "SUMMONER") {
                continue;
            }
            for mut e in self.fx.tokusei(tid, "summoned", &item)? {
                e.target = "self".into();
                e.source = "summoned".into();
                e.item = item.clone();
                e.skill_id = skill_id;
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
