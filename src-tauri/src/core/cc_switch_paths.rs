use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::env;

pub fn settings_path() -> Option<PathBuf> {
    resolve_settings_path_from_candidate_dirs(&candidate_dirs())
}

pub fn db_path() -> Option<PathBuf> {
    resolve_db_path_from_candidate_dirs(&candidate_dirs())
}

pub fn resolve_settings_path_from_candidate_dirs(candidates: &[PathBuf]) -> Option<PathBuf> {
    choose_candidate_dir(candidates, "settings.json").map(|dir| dir.join("settings.json"))
}

pub fn resolve_db_path_from_candidate_dirs(candidates: &[PathBuf]) -> Option<PathBuf> {
    choose_candidate_dir(candidates, "cc-switch.db").map(|dir| dir.join("cc-switch.db"))
}

fn candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut seen = HashSet::new();

    if let Some(home) = dirs::home_dir() {
        push_candidate(&mut dirs, &mut seen, home.join(".cc-switch"));
    }

    #[cfg(windows)]
    {
        if let Some(userprofile) = env::var_os("USERPROFILE").map(PathBuf::from) {
            push_candidate(&mut dirs, &mut seen, userprofile.join(".cc-switch"));
        }
        for var in ["APPDATA", "LOCALAPPDATA"] {
            if let Some(root) = env::var_os(var).map(PathBuf::from) {
                push_candidate(&mut dirs, &mut seen, root.join(".cc-switch"));
                push_candidate(&mut dirs, &mut seen, root.join("cc-switch"));
            }
        }
    }

    dirs
}

fn push_candidate(dirs: &mut Vec<PathBuf>, seen: &mut HashSet<OsString>, path: PathBuf) {
    let key = normalized_path_key(&path);
    if seen.insert(key) {
        dirs.push(path);
    }
}

fn choose_candidate_dir(candidates: &[PathBuf], required_file: &str) -> Option<PathBuf> {
    candidates
        .iter()
        .enumerate()
        .filter_map(|(index, dir)| {
            let required_path = dir.join(required_file);
            if !required_path.is_file() {
                return None;
            }
            Some((candidate_score(dir), index, dir.clone()))
        })
        .max_by(
            |(left_score, left_index, _), (right_score, right_index, _)| {
                left_score
                    .cmp(right_score)
                    .then_with(|| right_index.cmp(left_index))
            },
        )
        .map(|(_, _, dir)| dir)
}

fn candidate_score(dir: &Path) -> i32 {
    let mut score = 0;
    if dir.join("settings.json").is_file() {
        score += 10;
    }
    if dir.join("cc-switch.db").is_file() {
        score += 20;
    }
    score
}

fn normalized_path_key(path: &Path) -> OsString {
    #[cfg(windows)]
    {
        path.to_string_lossy().to_ascii_lowercase().into()
    }

    #[cfg(not(windows))]
    {
        path.as_os_str().to_os_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn prefers_candidate_with_settings_and_db_over_home_default() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("home").join(".cc-switch");
        let appdata = dir.path().join("appdata").join("cc-switch");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&appdata).unwrap();
        std::fs::write(home.join("settings.json"), "{}").unwrap();
        std::fs::write(appdata.join("settings.json"), "{}").unwrap();
        std::fs::write(appdata.join("cc-switch.db"), "").unwrap();

        let candidates = vec![home.clone(), appdata.clone()];

        assert_eq!(
            resolve_settings_path_from_candidate_dirs(&candidates).as_deref(),
            Some(appdata.join("settings.json").as_path())
        );
        assert_eq!(
            resolve_db_path_from_candidate_dirs(&candidates).as_deref(),
            Some(appdata.join("cc-switch.db").as_path())
        );
    }

    #[test]
    fn can_use_different_existing_dirs_when_settings_and_db_are_split() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("home").join(".cc-switch");
        let local_appdata = dir.path().join("local").join(".cc-switch");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&local_appdata).unwrap();
        std::fs::write(home.join("settings.json"), "{}").unwrap();
        std::fs::write(local_appdata.join("cc-switch.db"), "").unwrap();

        let candidates = vec![home.clone(), local_appdata.clone()];

        assert_eq!(
            resolve_settings_path_from_candidate_dirs(&candidates).as_deref(),
            Some(home.join("settings.json").as_path())
        );
        assert_eq!(
            resolve_db_path_from_candidate_dirs(&candidates).as_deref(),
            Some(local_appdata.join("cc-switch.db").as_path())
        );
    }
}
