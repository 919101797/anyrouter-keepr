#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

fn main() {
    anyrouter_claude_keeper_lib::run();
}
