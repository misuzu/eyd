use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

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

fn path_file_name_to_number(path: &Path) -> Option<usize> {
    path.file_name()
        .and_then(|x| x.to_str().and_then(|x| x.parse::<usize>().ok()))
}

fn find_target_path_number(target_path: &Path) -> usize {
    let number = fs::read_dir(target_path)
        .ok()
        .and_then(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter_map(|x| path_file_name_to_number(&x))
                .max()
        })
        .unwrap_or(0);
    number + 1
}

fn move_dirty(root: &Path, target: &Path, keep: &BTreeSet<PathBuf>) {
    let target_path = root.join(target.strip_prefix("/").unwrap_or(target));
    let target_path = target_path.join(format!("{:016}", find_target_path_number(&target_path)));

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
            let mut paths = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect::<Vec<_>>();
            if paths.len() > retain {
                paths.sort_by_cached_key(|x| path_file_name_to_number(&x).unwrap_or(0));
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

    let keep = normalize_keep(
        root,
        target,
        mountpaths().unwrap(),
        serde_json::from_str(&args[4]).unwrap(),
    );

    move_dirty(root, target, &keep);

    cleanup_old(root, target, retain);
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

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
        let target_1 = Path::new("/oldroot/0000000000000001");
        let path_1 = Path::new("/var/lib/nixos");
        let target_path_1 = Path::new("/oldroot/0000000000000001/var/lib/nixos");

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
                Path::new("/oldroot/0000000000000001"),
                Path::new("/oldroot/0000000000000001/var"),
                Path::new("/oldroot/0000000000000001/var/lib"),
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
        let target_2 = Path::new("/sysroot/oldroot/0000000000000001");
        let path_2 = Path::new("/sysroot/var/lib/nixos");
        let target_path_2 = Path::new("/sysroot/oldroot/0000000000000001/var/lib/nixos");

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
                Path::new("/sysroot/oldroot/0000000000000001"),
                Path::new("/sysroot/oldroot/0000000000000001/var"),
                Path::new("/sysroot/oldroot/0000000000000001/var/lib"),
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
    fn test_find_target_path_number() {
        let tmpdir = tempdir().unwrap();
        let root = tmpdir.path();
        let target = Path::new("/oldroot");
        let target_path = root.join(target.strip_prefix("/").unwrap_or(&target));

        let target_path_unrelated = target_path.join("random_123");
        let target_path_1 = target_path.join(format!("{:016}", 1));
        let target_path_2 = target_path.join(format!("{:016}", 2));
        let target_path_3 = target_path.join(format!("{:016}", 3));

        assert_eq!(target_path_1.exists(), false);
        assert_eq!(find_target_path_number(&target_path), 1);

        fs::DirBuilder::new()
            .recursive(true)
            .create(&target_path_1)
            .unwrap();
        assert_eq!(target_path_1.exists(), true);
        assert_eq!(target_path_2.exists(), false);
        assert_eq!(find_target_path_number(&target_path), 2);

        fs::DirBuilder::new()
            .recursive(true)
            .create(&target_path_unrelated)
            .unwrap();

        assert_eq!(target_path_unrelated.exists(), true);
        assert_eq!(find_target_path_number(&target_path), 2);

        fs::DirBuilder::new()
            .recursive(true)
            .create(&target_path_2)
            .unwrap();
        assert_eq!(target_path_2.exists(), true);

        cleanup_old(&root, &target, 1);
        assert_eq!(target_path_1.exists(), false);
        assert_eq!(target_path_2.exists(), true);
        assert_eq!(target_path_3.exists(), false);
        assert_eq!(find_target_path_number(&target_path), 3);
    }

    #[test]
    fn test_eyd() {
        let tmpdir = tempdir().unwrap();
        let root = tmpdir.path();
        let target = Path::new("/oldroot");
        let target_path = root.join(target.strip_prefix("/").unwrap_or(&target));
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

        let target_path_1 = target_path.join("0000000000000001");
        assert_eq!(target_path_1.exists(), false);

        move_dirty(root, &target, &keep);

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

        assert_eq!(fs::read_dir(&target_path).unwrap().count(), 1);

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

        let target_path_2 = target_path.join("0000000000000002");
        assert_eq!(target_path_2.exists(), false);

        move_dirty(root, &target, &keep);

        cleanup_old(root, target, 2);

        assert_eq!(target_path_1.exists(), true);
        assert_eq!(target_path_2.exists(), true);

        assert_eq!(root.join("var").exists(), true);
        assert_eq!(target_path_2.join("var").exists(), true);

        assert_eq!(root.join("var/lib").exists(), false);
        assert_eq!(target_path_2.join("var/lib").exists(), true);

        assert_eq!(root.join("var/log").exists(), true);
        assert_eq!(target_path_2.join("var/log").exists(), false);

        assert_eq!(fs::read_dir(&target_path).unwrap().count(), 2);

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

        let target_path_3 = target_path.join("0000000000000003");
        assert_eq!(target_path_3.exists(), false);

        move_dirty(root, &target, &keep);

        cleanup_old(root, target, 2);

        assert_eq!(target_path_1.exists(), false);
        assert_eq!(target_path_2.exists(), true);
        assert_eq!(target_path_3.exists(), true);

        assert_eq!(fs::read_dir(&target_path).unwrap().count(), 2);

        let target_path_unrelated = target_path.join("random_123");
        fs::DirBuilder::new()
            .recursive(true)
            .create(&target_path_unrelated)
            .unwrap();

        assert_eq!(target_path_unrelated.exists(), true);
        assert_eq!(fs::read_dir(&target_path).unwrap().count(), 3);

        cleanup_old(root, target, 2);

        assert_eq!(target_path_unrelated.exists(), false);
        assert_eq!(target_path_2.exists(), true);
        assert_eq!(target_path_3.exists(), true);

        assert_eq!(fs::read_dir(&target_path).unwrap().count(), 2);
    }
}
