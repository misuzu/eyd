use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

use chrono::Utc;
use mountpoints::mountpaths;

#[derive(Debug, PartialEq)]
enum WalkAction {
    Skip,
    Recurse,
    Yield,
}

fn walk_action(entry: &Path, keep: &BTreeSet<PathBuf>) -> WalkAction {
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

fn walk(root: &Path, keep: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
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

fn normalize_keep(
    root: &Path,
    target: &Path,
    mountpoints: Vec<PathBuf>,
    mut keep: BTreeSet<PathBuf>,
) -> BTreeSet<PathBuf> {
    keep.insert(target.into());

    let mut keep = keep
        .iter()
        .map(|entry| root.join(entry.strip_prefix("/").unwrap_or(entry)))
        .collect::<BTreeSet<_>>();

    keep.extend(
        mountpoints
            .iter()
            .filter(|x| *x != root && x.starts_with(root))
            .cloned(),
    );

    keep.iter()
        .filter(|item1| {
            !keep
                .iter()
                .any(|item2| *item1 != item2 && item1.starts_with(item2))
        })
        .cloned()
        .collect()
}

fn root_path_to_target_path(root: &Path, target: &Path, path: &Path) -> PathBuf {
    target.join(path.strip_prefix(root).unwrap_or(path))
}

fn target_path_to_root_path(root: &Path, target: &Path, path: &Path) -> Option<PathBuf> {
    path.strip_prefix(target).ok().map(|path| root.join(path))
}

fn create_target_parents(root: &Path, target: &Path, path: &Path) {
    for target_parent in root_path_to_target_path(root, target, path)
        .parent()
        .unwrap()
        .ancestors()
        .collect::<Vec<_>>()
        .iter()
        .rev()
    {
        if !target_parent.exists() {
            let parent =
                target_path_to_root_path(root, target, target_parent).unwrap_or(root.into());
            let parent_mode = parent
                .metadata()
                .ok()
                .map(|x| x.permissions().mode())
                .unwrap_or(0o700);
            println!(
                "creating directory {} with mode {:#o}",
                target_parent.display(),
                parent_mode
            );
            fs::DirBuilder::new()
                .mode(parent_mode)
                .create(target_parent)
                .unwrap();
        }
    }
}

fn move_dirty(root: &Path, target: &Path, keep: &BTreeSet<PathBuf>) {
    let target_path = root.join(target.strip_prefix("/").unwrap_or(target));

    if target_path.is_dir() {
        println!(
            "target path {} already exists, not moving anything",
            target_path.display()
        );
        return;
    }

    for path in walk(root, keep) {
        create_target_parents(root, &target_path, &path);

        let to = root_path_to_target_path(root, &target_path, &path);
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
                    fs::remove_dir_all(path).unwrap();
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

    let keep = normalize_keep(
        root,
        target,
        mountpaths().unwrap(),
        serde_json::from_str(&args[4]).unwrap(),
    );

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
                &BTreeSet::from([Path::new("/var/log").into()]),
            ),
            WalkAction::Skip
        );

        assert_eq!(
            walk_action(
                Path::new("/var/log"),
                &BTreeSet::from([Path::new("/var/log/journal").into()]),
            ),
            WalkAction::Recurse
        );

        assert_eq!(
            walk_action(
                Path::new("/var/log"),
                &BTreeSet::from([Path::new("/var/logaa").into()]),
            ),
            WalkAction::Yield
        );

        assert_eq!(
            walk_action(
                Path::new("/var/logaa"),
                &BTreeSet::from([Path::new("/var/log").into()]),
            ),
            WalkAction::Yield
        );
    }

    #[test]
    fn test_normalize_keep() {
        assert_eq!(
            normalize_keep(
                &Path::new("/"),
                &Path::new("/oldroot"),
                vec!["/".into(), "/run".into()],
                BTreeSet::from([Path::new("/var/log").into(), Path::new("/var").into()])
            ),
            BTreeSet::from([
                Path::new("/oldroot").into(),
                Path::new("/run").into(),
                Path::new("/var").into()
            ])
        );

        assert_eq!(
            normalize_keep(
                &Path::new("/sysroot"),
                &Path::new("/oldroot"),
                vec![
                    "/".into(),
                    "/run".into(),
                    "/sysroot".into(),
                    "/sysroot/run".into()
                ],
                BTreeSet::from([
                    Path::new("/var").into(),
                    Path::new("/var/log").into(),
                    Path::new("/var/loga").into()
                ])
            ),
            BTreeSet::from([
                Path::new("/sysroot/oldroot").into(),
                Path::new("/sysroot/run").into(),
                Path::new("/sysroot/var").into()
            ])
        );
    }

    #[test]
    fn test_create_target_parents() {
        let root_1 = Path::new("/");
        let target_1 = Path::new("/oldroot/2025-05-31T16-05-25");
        let path_1 = Path::new("/var/lib/nixos");
        let target_path_1 = Path::new("/oldroot/2025-05-31T16-05-25/var/lib/nixos");

        assert_eq!(
            root_path_to_target_path(root_1, target_1, path_1),
            target_path_1,
        );

        assert_eq!(
            target_path_to_root_path(root_1, target_1, target_path_1),
            Some(path_1.into()),
        );

        let ancestors_1 = target_path_1
            .parent()
            .unwrap()
            .ancestors()
            .collect::<Vec<_>>()
            .iter()
            .rev()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            ancestors_1,
            vec![
                Path::new("/"),
                Path::new("/oldroot"),
                Path::new("/oldroot/2025-05-31T16-05-25"),
                Path::new("/oldroot/2025-05-31T16-05-25/var"),
                Path::new("/oldroot/2025-05-31T16-05-25/var/lib"),
            ],
        );

        assert_eq!(
            ancestors_1
                .iter()
                .map(|parent| target_path_to_root_path(root_1, target_1, parent))
                .collect::<Vec<_>>(),
            vec![
                None,
                None,
                Some(Path::new("/").into()),
                Some(Path::new("/var").into()),
                Some(Path::new("/var/lib").into()),
            ],
        );

        let root_2 = Path::new("/sysroot/");
        let target_2 = Path::new("/sysroot/oldroot/2025-05-31T16-05-25");
        let path_2 = Path::new("/sysroot/var/lib/nixos");
        let target_path_2 = Path::new("/sysroot/oldroot/2025-05-31T16-05-25/var/lib/nixos");

        assert_eq!(
            root_path_to_target_path(root_2, target_2, path_2),
            target_path_2,
        );

        assert_eq!(
            target_path_to_root_path(root_2, target_2, target_path_2),
            Some(path_2.into()),
        );

        let ancestors_2 = target_path_2
            .parent()
            .unwrap()
            .ancestors()
            .collect::<Vec<_>>()
            .iter()
            .rev()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            ancestors_2,
            vec![
                Path::new("/"),
                Path::new("/sysroot"),
                Path::new("/sysroot/oldroot"),
                Path::new("/sysroot/oldroot/2025-05-31T16-05-25"),
                Path::new("/sysroot/oldroot/2025-05-31T16-05-25/var"),
                Path::new("/sysroot/oldroot/2025-05-31T16-05-25/var/lib"),
            ],
        );

        assert_eq!(
            ancestors_2
                .iter()
                .map(|parent| target_path_to_root_path(root_2, target_2, parent))
                .collect::<Vec<_>>(),
            vec![
                None,
                None,
                None,
                Some(Path::new("/sysroot").into()),
                Some(Path::new("/sysroot/var").into()),
                Some(Path::new("/sysroot/var/lib").into()),
            ],
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
                    .unwrap_or(Path::new("/home"))
            ),
            Path::new("/oldroot/home")
        );

        assert_eq!(
            target_2.join(
                Path::new("/sysroot/home")
                    .strip_prefix(root_2)
                    .unwrap_or(Path::new("/sysroot/home"))
            ),
            Path::new("/sysroot/oldroot/home")
        );
    }

    #[test]
    fn test_eyd() {
        use tempdir::TempDir;

        let tmpdir = TempDir::new("eyd-test").unwrap();
        let root = tmpdir.path();
        let target = Path::new("/oldroot");
        let mountpoints = vec![root.into(), root.join("run"), root.join("home")];
        let keep = normalize_keep(
            root,
            target,
            mountpoints,
            BTreeSet::from([
                Path::new("/etc/ssh/ssh_host_ed25519_key").into(),
                Path::new("/etc/ssh/ssh_host_ed25519_key.pub").into(),
                Path::new("/var/log").into(),
            ]),
        );

        fs::DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(root.join("etc/ssh"))
            .unwrap();
        fs::File::create(root.join("etc/ssh/config")).unwrap();
        fs::File::create(root.join("etc/ssh/ssh_host_ed25519_key")).unwrap();

        let target_1 = target.join("2025-05-31T16-00-00");
        let target_path_1 = root.join(target_1.strip_prefix("/").unwrap_or(&target_1));
        assert_eq!(target_path_1.exists(), false);

        move_dirty(root, &target_1, &keep);

        cleanup_old(root, target, 2);

        assert_eq!(target_path_1.exists(), true);
        assert_eq!(root.join("etc/ssh").exists(), true);
        assert_eq!(target_path_1.join("etc/ssh").exists(), true);

        assert_eq!(
            target_path_1
                .metadata()
                .ok()
                .map(|x| x.permissions().mode())
                .unwrap(),
            0o40755
        );
        assert_eq!(
            target_path_1
                .join("etc")
                .metadata()
                .ok()
                .map(|x| x.permissions().mode())
                .unwrap(),
            0o40700
        );
        assert_eq!(
            target_path_1
                .join("etc/ssh")
                .metadata()
                .ok()
                .map(|x| x.permissions().mode())
                .unwrap(),
            0o40700
        );

        assert_eq!(root.join("etc/ssh/config").exists(), false);
        assert_eq!(target_path_1.join("etc/ssh/config").exists(), true);

        assert_eq!(root.join("etc/ssh/ssh_host_ed25519_key").exists(), true);
        assert_eq!(
            target_path_1.join("etc/ssh/ssh_host_ed25519_key").exists(),
            false
        );

        assert_eq!(
            fs::read_dir(root.join(target.strip_prefix("/").unwrap_or(&target)))
                .unwrap()
                .count(),
            1,
        );

        fs::DirBuilder::new()
            .recursive(true)
            .create(root.join("var/lib/acme"))
            .unwrap();
        fs::DirBuilder::new()
            .recursive(true)
            .create(root.join("var/log"))
            .unwrap();
        fs::File::create(root.join("var/lib/cert")).unwrap();
        fs::File::create(root.join("var/log/somelog")).unwrap();

        let target_2 = target.join("2025-05-31T16-01-00");
        let target_path_2 = root.join(target_2.strip_prefix("/").unwrap_or(&target_2));
        assert_eq!(target_path_2.exists(), false);

        move_dirty(root, &target_2, &keep);

        cleanup_old(root, target, 2);

        assert_eq!(target_path_1.exists(), true);
        assert_eq!(target_path_2.exists(), true);

        assert_eq!(root.join("var").exists(), true);
        assert_eq!(target_path_2.join("var").exists(), true);

        assert_eq!(root.join("var/lib").exists(), false);
        assert_eq!(target_path_2.join("var/lib").exists(), true);

        assert_eq!(root.join("var/log").exists(), true);
        assert_eq!(target_path_2.join("var/log").exists(), false);

        assert_eq!(
            fs::read_dir(root.join(target.strip_prefix("/").unwrap_or(&target)))
                .unwrap()
                .count(),
            2,
        );

        fs::DirBuilder::new()
            .recursive(true)
            .create(root.join("var/lib/acme"))
            .unwrap();
        fs::DirBuilder::new()
            .recursive(true)
            .create(root.join("var/log"))
            .unwrap();
        fs::File::create(root.join("var/lib/cert")).unwrap();
        fs::File::create(root.join("var/log/somelog")).unwrap();

        let target_3 = target.join("2025-05-31T16-02-00");
        let target_path_3 = root.join(target_3.strip_prefix("/").unwrap_or(&target_3));
        assert_eq!(target_path_3.exists(), false);

        move_dirty(root, &target_3, &keep);

        cleanup_old(root, target, 2);

        assert_eq!(target_path_1.exists(), false);
        assert_eq!(target_path_2.exists(), true);
        assert_eq!(target_path_3.exists(), true);

        assert_eq!(
            fs::read_dir(root.join(target.strip_prefix("/").unwrap_or(&target)))
                .unwrap()
                .count(),
            2,
        );
    }
}
