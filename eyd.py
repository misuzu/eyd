import datetime
import json
import os
import pathlib
import shutil
import sys

import psutil


def walk_action(entry: str, keep: list[str]) -> str:
    for path in keep:
        if entry == path:
            return 'skip'
        if path.startswith(entry + '/'):
            return 'recurse'
    return 'yield'


def walk(root: str, keep: list[str]):
    for entry in pathlib.Path(root).iterdir():
        entry_path = entry.absolute().as_posix()
        action = walk_action(entry_path, keep)
        if action == 'recurse':
            if entry.is_dir():
                yield from walk(entry_path, keep)
        if action == 'yield':
            yield entry_path


def normalize_keep(root: str, items: set[str]) -> list[str]:
    result = []
    for item1 in items:
        for item2 in items:
            if item1 != item2 and item1.startswith(item2 + '/'):
                break
        else:
            result.append(os.path.join(root, item1[1:]))
    return sorted(result)


def move_dirty(root: str, target: str, keep: list[str]):
    target = os.path.join(root, target[1:])
    if os.path.isdir(target):
        shutil.rmtree(target)
    for path in walk(root, keep):
        to = os.path.join(target, path[len(root.rstrip('/')):][1:])
        parent = os.path.dirname(to)
        if not os.path.isdir(parent):
            os.makedirs(parent, 0o700)
        print('moving', path, 'to', to)
        shutil.move(path, to)


def main():
    timestamp = datetime.datetime.now(datetime.timezone.utc)
    target = '/oldroot'
    root, *args = sys.argv[1:]
    mountpoints = set(
        filter(
            lambda x: x.startswith(root + '/'),
            map(lambda x: x.mountpoint, psutil.disk_partitions()),
        )
    )
    keep = []
    for arg in args:
        keep = keep + json.loads(arg)
    keep = normalize_keep(
        '/',
        set(
            normalize_keep(
                root,
                set(keep) | {target},
            )
        )
        | mountpoints - {root},
    )
    move_dirty(
        root,
        os.path.join(target, timestamp.strftime('%Y-%m-%dT%H-%M-%S')),
        keep,
    )


main()
