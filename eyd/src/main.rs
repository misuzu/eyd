use std::collections::BTreeSet;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use mountpoints::mountpaths;
use serde_json;

#[derive(Debug, PartialEq)]
enum WalkAction {
    Skip,
    Recurse,
    Yield,
}

fn walk_action(entry: &Path, keep: &HashSet<PathBuf>) -> WalkAction {
    for path in keep {
        if path == entry {
            return WalkAction::Skip;
        }
        if path.starts_with(entry) {
            return WalkAction::Recurse;
        }
    }
    WalkAction::Yield
}

fn walk(root: &Path, keep: &HashSet<PathBuf>) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            match walk_action(&entry_path, keep) {
                WalkAction::Recurse => {
                    if entry_path.is_dir() {
                        result.extend(walk(&entry_path, keep));
                    }
                }
                WalkAction::Yield => {
                    result.push(entry_path);
                }
                WalkAction::Skip => continue,
            }
        }
    }
    result
}

fn swith_root(root: &Path, items: &HashSet<PathBuf>) -> HashSet<PathBuf> {
    items
        .iter()
        .map(|entry| root.join(entry.strip_prefix("/").unwrap_or(entry)))
        .collect()
}

fn normalize_keep(items: &HashSet<PathBuf>) -> HashSet<PathBuf> {
    items
        .iter()
        .filter(|item1| {
            !items
                .iter()
                .any(|item2| *item1 != item2 && item1.starts_with(item2))
        })
        .cloned()
        .collect()
}

fn move_dirty(root: &Path, target: &Path, keep: &HashSet<PathBuf>) {
    let target_path = root.join(target.strip_prefix("/").unwrap_or(target));

    if target_path.is_dir() {
        fs::remove_dir_all(&target_path).unwrap();
    }

    for path in walk(root, keep) {
        let to = target_path.join(path.strip_prefix(root).unwrap_or(&path));

        if let Some(parent) = to.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).unwrap();
            }
        }

        if let Err(e) = fs::rename(&path, &to) {
            println!(
                "moving {} -> {} error! {:?}",
                path.display(),
                to.display(),
                e
            );
        } else {
            println!("moving {} -> {} ok!", path.display(), to.display());
        }
    }
}

fn cleanup_old(root: &Path, target: &Path, retain: usize) {
    if retain > 0 {
        let target_path = root.join(target.strip_prefix("/").unwrap_or(target));
        if let Ok(entries) = fs::read_dir(target_path) {
            let paths = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect::<BTreeSet<_>>();
            if paths.len() > retain {
                for path in paths.iter().take(paths.len() - retain) {
                    println!("removing {}", path.display());
                    fs::remove_dir_all(&path).unwrap();
                }
                return;
            }
        }
    }
    println!("not cleaning up");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 5 {
        eprintln!("Usage: {} <root> <target> <retain> <keep_json>", args[0]);
        return;
    }

    let root = Path::new(&args[1]);
    let target = Path::new(&args[2]);
    let retain = args[3].parse::<usize>().unwrap();
    let target_with_timestamp = target.join(Utc::now().format("%Y-%m-%dT%H-%M-%S").to_string());

    let mut keep: HashSet<PathBuf> = serde_json::from_str(&args[4]).unwrap();
    keep.insert(target.into());

    let mut keep = swith_root(root, &keep);

    let mountpoints: HashSet<PathBuf> = mountpaths()
        .unwrap()
        .iter()
        .filter(|x| *x != root && x.starts_with(root))
        .cloned()
        .collect();

    keep.extend(mountpoints);

    let keep = normalize_keep(&keep);

    move_dirty(root, &target_with_timestamp, &keep);

    cleanup_old(root, target, retain);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_walk_action() {
        assert_eq!(
            walk_action(
                Path::new("/var/log"),
                &HashSet::from([Path::new("/var/log").into()]),
            ),
            WalkAction::Skip
        );

        assert_eq!(
            walk_action(
                Path::new("/var/log"),
                &HashSet::from([Path::new("/var/log/journal").into()]),
            ),
            WalkAction::Recurse
        );

        assert_eq!(
            walk_action(
                Path::new("/var/log"),
                &HashSet::from([Path::new("/var/logaa").into()]),
            ),
            WalkAction::Yield
        );

        assert_eq!(
            walk_action(
                Path::new("/var/logaa"),
                &HashSet::from([Path::new("/var/log").into()]),
            ),
            WalkAction::Yield
        );
    }

    #[test]
    fn test_swith_root() {
        assert_eq!(
            swith_root(
                Path::new("/"),
                &HashSet::from([Path::new("/var/log").into(), Path::new("/var").into()])
            ),
            HashSet::from([Path::new("/var/log").into(), Path::new("/var").into()])
        );

        assert_eq!(
            swith_root(
                Path::new("/sysroot"),
                &HashSet::from([Path::new("/var/log").into(), Path::new("/var").into()])
            ),
            HashSet::from([
                Path::new("/sysroot/var/log").into(),
                Path::new("/sysroot/var").into()
            ])
        );
    }

    #[test]
    fn test_normalize_keep() {
        assert_eq!(
            normalize_keep(&HashSet::from([
                Path::new("/var/log").into(),
                Path::new("/var").into()
            ])),
            HashSet::from([Path::new("/var").into(),])
        );

        assert_eq!(
            normalize_keep(&HashSet::from([
                Path::new("/var/log").into(),
                Path::new("/var/loga").into()
            ])),
            HashSet::from([Path::new("/var/log").into(), Path::new("/var/loga").into()])
        );
    }

    #[test]
    fn test_move_dirty() {
        let root_1 = Path::new("/");
        let target_1 = Path::new("/oldroot");

        let root_2 = Path::new("/sysroot/");
        let target_2 = Path::new("/sysroot/oldroot");

        assert_eq!(
            root_1.join(target_1.strip_prefix("/").unwrap_or(target_1)),
            Path::new("/oldroot")
        );

        assert_eq!(
            Path::new("/sysroot").join(target_1.strip_prefix("/").unwrap_or(target_1)),
            Path::new("/sysroot/oldroot")
        );

        assert_eq!(
            target_1.join(
                Path::new("/home")
                    .strip_prefix(root_1)
                    .unwrap_or(&Path::new("/home"))
            ),
            Path::new("/oldroot/home")
        );

        assert_eq!(
            target_2.join(
                Path::new("/sysroot/home")
                    .strip_prefix(root_2)
                    .unwrap_or(&Path::new("/sysroot/home"))
            ),
            Path::new("/sysroot/oldroot/home")
        );
    }
}
