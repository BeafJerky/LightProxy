//! LightProxy core.
//!
//! This crate owns every write to the Windows system proxy. The Tauri layer
//! only assembles: it forwards commands to the single-writer actor.

pub mod actor;
pub mod backend;
pub mod error;
pub mod health;
pub mod model;
pub mod profile;
pub mod recovery;
pub mod repair;
pub mod service;
pub mod store;
pub mod watcher;
pub mod wininet;

pub use actor::ActorHandle;
pub use backend::{MemoryBackend, ProxyBackend};
pub use error::CoreError;
pub use model::*;
pub use repair::{Diagnostic, RepairResult};
pub use service::{Command, CommandReply, ProxyService};
pub use store::{LoadResult, StateStore};
pub use watcher::Watcher;
pub use wininet::WinInetBackend;
