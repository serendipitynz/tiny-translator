// Without this, a release build on Windows opens a console window beside the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tiny_translator_lib::run()
}
