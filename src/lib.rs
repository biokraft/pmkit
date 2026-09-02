//! pmkit — installs a safe, human-in-the-loop agentic workflow into the agent a
//! product manager already uses.

#![forbid(unsafe_code)]

pub mod capabilities;
pub mod commands;
pub mod doctor;
pub mod emit;
pub mod error;
pub mod forge;
pub mod preamble;
pub mod skills;
pub mod state;
pub mod target;
pub mod wizard;
