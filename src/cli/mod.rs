//! The command-line interface: one module per command group, holding that
//! group's `clap` types and the functions that run its commands.

pub mod check;
pub mod footer;
pub mod generate;
pub mod image;
pub mod init;
pub mod menu;
pub mod navbar;
pub mod page;
pub mod prompt;
pub mod search;
pub mod social;
pub mod spare_tile;
