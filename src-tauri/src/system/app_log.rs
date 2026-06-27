use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use chrono::Local;

const LOG_FILE: &str = "keeper.log";

pub fn info(scope: &str, message: impl AsRef<str>) {
    write("INFO", scope, message.as_ref());
}

pub fn error(scope: &str, message: impl AsRef<str>) {
    write("ERROR", scope, message.as_ref());
}

pub fn path() -> PathBuf {
    base_dir().join(LOG_FILE)
}

fn write(level: &str, scope: &str, message: &str) {
    let path = path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let message = message.replace('\n', "\\n");
    let _ = writeln!(
        file,
        "{} [{level}] {scope}: {message}",
        Local::now().to_rfc3339()
    );
}

fn base_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("anyrouter-claude-keeper")
}
