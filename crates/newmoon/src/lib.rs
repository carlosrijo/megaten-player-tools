//! newmoon: the read-only NewMoon (ImagineClient) reader, ported from the C# `src\` folder.
//!
//! Module map (one Rust file per C# file; see PORTING.md at the repo root):
//!   memory  <- src\Memory\GameMemory.cs       read-only process access
//!   game    <- src\Game\*.cs                  structure layouts in client memory
//!   data    <- src\Data\*.cs                  reference tables, saved files, config, NIF/TGA readers
//!   calc    <- src\Calc\*.cs                  effects, gear bonuses, digitalize, HP/MP
//!   build   <- src\Build\*.cs                 snapshot, CSV, page data, database builders, Simulator
//!   context <- src\Context.cs

pub mod build;
pub mod calc;
pub mod console;
pub mod context;
pub mod data;
pub mod diag;
pub mod error;
pub mod fmt;
pub mod game;
pub mod memory;
pub mod text;

pub use error::{Result, UserError, user_error};
