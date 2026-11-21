//! Flowlist binary entry point.
//!
//! The same executable is the GUI, the CLI and the MCP server:
//! `flowlist` opens the app, `flowlist <command>` runs a command, and
//! `flowlist mcp` speaks Model Context Protocol on stdio.

mod cli;
mod cli_install;
mod uninstall;

#[cfg(feature = "gui")]
mod gui;

mod mcp;
mod render;
mod serve;

fn main() {
    std::process::exit(cli::run());
}
