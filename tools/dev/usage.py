"""Read local Codex JSONL usage counters. No network, AI, prompts or raw log export."""
import argparse
import json
import os
from pathlib import Path
from statistics import median


def report(path):
    fields = ("input_tokens", "cached_input_tokens", "output_tokens",
              "reasoning_output_tokens", "total_tokens")
    totals = dict.fromkeys(fields, 0)
    previous = None
    inputs = []
    snapshots = []
    skipped = 0
    with path.open(encoding="utf-8-sig", errors="replace") as stream:
        for line in stream:
            try:
                event = json.loads(line)
            except ValueError:
                skipped += 1
                continue
            if not isinstance(event, dict):
                continue
            payload = event.get("payload") or {}
            if not isinstance(payload, dict) or payload.get("type") != "token_count":
                continue
            info = payload.get("info") or {}
            usage = info.get("total_token_usage") or {}
            if isinstance(usage.get("total_tokens"), int):
                current = tuple(int(usage.get(key) or 0) for key in fields)
                if current != previous:
                    # One cumulative advance, not one token_count event:
                    # different rate-limit records often repeat the same counters.
                    delta = current if previous is None or current[-1] < previous[-1] else tuple(a-b for a,b in zip(current,previous))
                    if all(value >= 0 for value in delta):
                        for key, value in zip(fields, delta):
                            totals[key] += value
                        if delta[-1]:
                            inputs.append(delta[0])
                    else:
                        skipped += 1
                    previous = current
            limits = payload.get("rate_limits") or {}
            primary = limits.get("primary") or {}
            secondary = limits.get("secondary") or {}
            if limits.get("limit_id") == "codex" and primary.get("used_percent") is not None:
                snap = (event.get("timestamp"), primary.get("used_percent"),
                        secondary.get("used_percent"), primary.get("resets_at"),
                        secondary.get("resets_at"))
                if not snapshots or snapshots[-1][1:] != snap[1:]:
                    snapshots.append(snap)
    print(f"Session: {path.name}")
    print(f"  Counter advances: {len(inputs):,}")
    print(f"  Total processed: {totals['total_tokens']:,}")
    print(f"  Input: {totals['input_tokens']:,}; cached: {totals['cached_input_tokens']:,}")
    print(f"  Output: {totals['output_tokens']:,}; reasoning (included): {totals['reasoning_output_tokens']:,}")
    if totals["input_tokens"]:
        print(f"  Cache hit: {100*totals['cached_input_tokens']/totals['input_tokens']:.2f}%")
    if inputs:
        print(f"  Input per counter advance: median {median(inputs):,.0f}; max {max(inputs):,}")
    if snapshots:
        for label, row in (("first", snapshots[0]), ("last", snapshots[-1])):
            print(f"  {label} quota snapshot: {row[0]} / 5h used {row[1]}% / weekly used {row[2]}% / reset epochs {row[3]}, {row[4]}")
    if skipped:
        print(f"  Skipped incomplete or inconsistent records: {skipped}")
    print("  Quota snapshots are account-wide, not attributable to this session alone.")
    print("  Counts are cumulative-log advances, not guaranteed individual model calls.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("logs", nargs="*", type=Path, help="Exact rollout JSONL paths; default: most recently modified local session")
    args = parser.parse_args()
    paths = args.logs
    if not paths:
        home = Path(os.environ.get("CODEX_HOME") or Path.home() / ".codex")
        candidates = list((home / "sessions").rglob("rollout-*.jsonl"))
        if not candidates:
            parser.error("No session logs found. Supply an explicit JSONL path.")
        paths = [max(candidates, key=lambda p: p.stat().st_mtime)]
    for path in dict.fromkeys(p.resolve() for p in paths):
        if not path.is_file():
            parser.error(f"Not a file: {path.name}")
        report(path)
    print("No quota-charge estimate: cached tokens, account switches and server accounting differ.")
    print("Compare explicit fresh-session logs and account usage before/after an assignment.")


if __name__ == "__main__":
    main()
