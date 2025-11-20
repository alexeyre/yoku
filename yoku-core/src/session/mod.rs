//! Session module for managing workout sessions and sets.
//!
//! This module provides the main `Session` struct that coordinates
//! workout tracking, set management, and LLM-based command processing.

mod commands;
mod context;
mod session;
mod sets;
mod summary;
mod workout;

pub use session::Session;
