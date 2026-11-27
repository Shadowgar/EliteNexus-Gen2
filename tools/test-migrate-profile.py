"""Synthetic migration smoke test. Run from the repository root with Python 3.10+."""

from contextlib import closing
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile


with tempfile.TemporaryDirectory(prefix="praetorium-migration-fixture-") as temp:
    root = Path(temp)
    source = root / "nexus-gen2-data"
    destination = root / "praetorium-data"
    source.mkdir()
    (source / "settings.json").write_text(json.dumps({
        "ui_scale": 1.25, "tts_enabled": False, "accounts": {"key": "fixture"},
        "journal_dir": "fixture", "game_dir": "fixture", "ai": {"agents": True},
        "overlay": {"path": "fixture"}, "updates": {"pending": True},
    }), encoding="utf-8")
    (source / "upload.queue").write_text("fixture", encoding="utf-8")
    with closing(sqlite3.connect(source / "galaxy.db")) as db:
        db.executescript("""
            PRAGMA user_version=2;
            CREATE TABLE systems (name TEXT);
            CREATE TABLE stations (name TEXT);
            CREATE TABLE market (name TEXT);
            INSERT INTO systems VALUES ('Fixture');
        """)
    before = {file.name: file.read_bytes() for file in source.iterdir()}
    command = [sys.executable, "tools/migrate-profile.py", "--source", str(source), "--destination"]
    subprocess.run(command + [str(destination)], check=True)
    assert json.loads((destination / "settings.json").read_bytes()) == {"ui_scale": 1.25, "tts_enabled": False}
    assert {file.name for file in destination.iterdir()} == {"settings.json", "galaxy.db"}
    with closing(sqlite3.connect(destination / "galaxy.db")) as db:
        assert db.execute("SELECT name FROM systems").fetchall() == [("Fixture",)]
    assert subprocess.run(command + [str(destination)], capture_output=True).returncode == 1
    assert subprocess.run(command + [str(source / "nested")], capture_output=True).returncode == 1
    assert {file.name: file.read_bytes() for file in source.iterdir()} == before
    with closing(sqlite3.connect(source / "galaxy.db")) as db:
        db.execute("PRAGMA user_version=99")
    newer = root / "newer-db-destination"
    assert subprocess.run(command + [str(newer)], capture_output=True).returncode == 1
    assert not newer.exists()
    print("Synthetic migration, rollback source preservation, and refusal checks passed.")
