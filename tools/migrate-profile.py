#!/usr/bin/env python3
"""Copy selected local preferences and a verified galaxy DB into a new Praetorium profile."""

import argparse
import json
import os
from pathlib import Path
import shutil
import sqlite3
import stat
import tempfile


PREFERENCES = {
    "tts_enabled": bool,
    "tts_min_priority": int,
    "tts_rate": (float, int),
    "tts_volume": (float, int),
    "toast_seconds": (float, int),
    "ui_scale": (float, int),
    "high_value_threshold": int,
    "auto_page": bool,
    "auto_page_altitude_km": (float, int),
}
LEGACY_NAMES = {"nexus-data", "elitenexus", "elite nexus", "nexus-gen2-data", "elitenexus-gen2"}


def fingerprint(path):
    try:
        info = path.lstat()
    except FileNotFoundError:
        return None
    if not stat.S_ISREG(info.st_mode) or getattr(info, "st_file_attributes", 0) & 0x400:
        raise ValueError(f"Expected a regular file at {path}")
    return info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns


def migrate(source, requested_destination):
    source = source.resolve(strict=True)
    if not source.is_dir() or not requested_destination.name:
        raise ValueError("Select a source directory and a new destination directory")
    parent = requested_destination.parent.resolve(strict=True)
    destination = parent / requested_destination.name
    if destination.is_relative_to(source) or source.is_relative_to(destination):
        raise ValueError("Keep the source and destination profiles separate")
    if destination.exists() or destination.is_symlink() or any(p.casefold() in LEGACY_NAMES for p in destination.parts):
        raise ValueError("The destination must be a new Praetorium profile directory")
    files = ("settings.json", "galaxy.db", "galaxy.db-wal", "galaxy.db-journal")
    before = {name: fingerprint(source / name) for name in files}
    if before["settings.json"] is None:
        raise ValueError("The source must contain a Gen2 settings.json")
    if before["galaxy.db"] is None and any(before[name] for name in files[2:]):
        raise ValueError("The source database is missing while its sidecars are present")
    with tempfile.TemporaryDirectory(prefix=".praetorium-migration-", dir=parent) as tmp:
        staged = Path(tmp) / "profile"
        staged.mkdir()
        settings = json.loads((source / "settings.json").read_text(encoding="utf-8-sig"))
        if not isinstance(settings, dict):
            raise ValueError("The source settings must contain a JSON object")
        allowed = {}
        for name, kind in PREFERENCES.items():
            value = settings.get(name)
            if name in settings and type(value) in (kind if isinstance(kind, tuple) else (kind,)):
                allowed[name] = value
        (staged / "settings.json").write_text(json.dumps(allowed, allow_nan=False, indent=2) + "\n", encoding="utf-8")
        for name in files[1:]:
            if before[name] is not None:
                shutil.copyfile(source / name, staged / name)
        if before != {name: fingerprint(source / name) for name in files}:
            raise ValueError("The old profile changed during copying. Close its app and try again")
        if before["galaxy.db"] is not None:
            db = sqlite3.connect(staged / "galaxy.db")
            try:
                schema = db.execute("PRAGMA user_version").fetchone()[0]
                if schema != 2 or db.execute("PRAGMA quick_check").fetchall() != [("ok",)]:
                    raise ValueError("The copied database is damaged or its schema is unsupported")
                if db.execute("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('systems','stations','market')").fetchone()[0] != 3:
                    raise ValueError("The copied database lacks expected galaxy tables")
                if db.execute("PRAGMA wal_checkpoint(TRUNCATE)").fetchone()[0] != 0:
                    raise ValueError("The copied database checkpoint did not complete")
            finally:
                db.close()
        # Publish only into a newly reserved destination. Never replace a pre-existing profile.
        destination.mkdir()
        installed = []
        try:
            for path in staged.iterdir():
                output = destination / path.name
                os.link(path, output)
                installed.append(output)
        except OSError:
            for path in installed:
                if path.samefile(staged / path.name):
                    path.unlink()
            destination.rmdir()
            raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path, help="Closed Gen2 profile")
    parser.add_argument("--destination", required=True, type=Path, help="New Praetorium profile under an existing directory")
    args = parser.parse_args()
    try:
        migrate(args.source, args.destination)
    except (OSError, ValueError, sqlite3.Error) as error:
        parser.exit(1, f"Migration stopped: {error}\n")
    print("Copied allowlisted preferences and any verified galaxy database into a fresh profile.")


if __name__ == "__main__":
    main()
