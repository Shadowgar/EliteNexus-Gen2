"""Assemble the local Praetorium preview without upstream presentation assets."""

import argparse
from pathlib import Path
import shutil

SOURCE = Path(__file__).resolve().parents[2] / "site"
FILES = ("index.html", "style.css", "robots.txt", "llms.txt", "assets/favicon.svg")


def build(destination: Path) -> None:
    if destination != SOURCE:
        for name in FILES:
            target = destination / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(SOURCE / name, target)
    print(f"Preview available in {destination}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=SOURCE)
    build(parser.parse_args().output.resolve())
