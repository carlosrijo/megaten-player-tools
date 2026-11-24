//! Troubleshooting: which build this is, the local log, crash reports, taking personal details out of text, and the
//! file digest that identifies the game client. The self-check is `build::doctor`, the support bundle `build::bundle`.
pub mod about;
pub mod crash;
pub mod log;
pub mod redact;
pub mod sha256;
pub mod symbolize;
