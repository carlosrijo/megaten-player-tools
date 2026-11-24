//! Port of `src/Context.cs`: shared state for one run: files, reference data, and the parser/set table built from them.
//!
//! The reference data is shared (`Arc<GameData>`): a `Context` can be used from worker threads, each call building its own
//! calculator.

use std::sync::Arc;

use indexmap::IndexMap;

use crate::Result;
use crate::calc::digitalize::Digitalize;
use crate::calc::gear_effects::GearEffects;
use crate::calc::item_effects::ItemEffects;
use crate::data::game_data::GameData;
use crate::data::store::Store;

pub struct Context {
    pub store: Store,
    pub data: Arc<GameData>,
}

impl Context {
    /// `new Context(store, data)`; `data` is a `GameData` or an `Arc<GameData>` already shared.
    pub fn new(store: Store, data: impl Into<Arc<GameData>>) -> Context {
        Context { store, data: data.into() }
    }

    /// Gear calculator with the current alignment, expertise classes and set table (re-read each call: files may change mid-run).
    pub fn gear(&self) -> Result<GearEffects> {
        Ok(GearEffects::new(
            &self.store.alignment()?,
            &self.store.expertise_classes()?,
            Arc::clone(&self.data),
            self.store.load_set_table()?,
            Some(self.store.expertise_ranks()?),
        ))
    }

    /// Digitalize calculator; level = the character's (for level-scaled effects).
    pub fn digitalize(&self, level: i32) -> Result<Digitalize> {
        let fx = ItemEffects::new(Arc::clone(&self.data), &self.store.alignment()?, level, self.store.expertise_ranks()?);
        Ok(Digitalize::new(Arc::clone(&self.data), fx))
    }

    /// Digitalize calculator that also knows the character's final stats while digitalized (CorrectTbl id -> value),
    /// for the effects that scale with one ("Strength +100 ÷ Luck").
    pub fn digitalize_with_stats(&self, level: i32, stats: IndexMap<String, f64>) -> Result<Digitalize> {
        let fx = ItemEffects::new(Arc::clone(&self.data), &self.store.alignment()?, level, self.store.expertise_ranks()?).with_stats(stats);
        Ok(Digitalize::new(Arc::clone(&self.data), fx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_is_shareable_between_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Context>();
        assert_send_sync::<ItemEffects>();
        assert_send_sync::<GearEffects>();
        assert_send_sync::<Digitalize>();
    }
}
