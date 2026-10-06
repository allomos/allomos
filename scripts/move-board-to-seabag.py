#!/usr/bin/env python3
"""Move FlipHopper's own cards from docs/TASK_BOARD.md into this repository's seabag (Task-A01-07).

Every change goes through `fh`; the only file written directly is project.json, for the
project ID and the label ladder, which `fh` has no option for yet.

    python3 scripts/move-board-to-seabag.py --fh <path to fh> [--board docs/TASK_BOARD.md]   # move
    python3 scripts/move-board-to-seabag.py --fh <path to fh> --check                         # compare only

The move refuses to run when a seabag already exists; the check compares every board card
with the seabag and exits 1 on any difference.
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SEABAG = ROOT / ".fliphopper" / "seabag"
MOVER = "move-board"

# Owner, 2026-10-06 (A01-07): one ladder for code and document cards.
LADDER = ["drafted", "implemented", "locally verified", "deployed and verified", "owner verified"]

AREAS = [
    ("A00", "Planning"),
    ("A01", "Tracking"),
    ("A02", "Progress & forecast"),
    ("A03", "Agents"),
    ("A04", "Companions"),
    ("A05", "Project setup & import"),
    ("A06", "Cloud & team"),
    ("A07", "Platform & release"),
]

# Card file names that no longer say what the card is.
TITLES = {
    "A00-17": "Rename to FlipHopper",
    "A01-03": "The fh command",
}

# Words a card file name writes in lower case.
WORDS = {"ai": "AI", "ui": "UI", "ssr": "SSR", "github": "GitHub"}

ROW = re.compile(r"^\| \[`Task-(A\d\d-\d\d)-([a-z0-9-]+)`\]\(\./tasks/([^)]+)\) \|(.*)\|\s*$")


def cells(rest):
    return [c.strip() for c in rest.split("|")]


def finished_label(text):
    """A finished card's label from the board's free text; None when it names no owner check."""
    t = text.lower().lstrip("*")
    if t.startswith(("owner verified", "owner accepted", "owner confirmed", "owner decided")):
        return "owner verified"
    return None


def read_board(board):
    """Cards as dicts: id, slug, file, state, label, holder, next."""
    section, cards = None, []
    for line in board.read_text(encoding="utf-8").splitlines():
        if line.startswith("### "):
            section = line[4:].strip()
            continue
        m = ROW.match(line)
        if not m or section not in ("Open", "Recently finished"):
            continue
        cid, slug, file, rest = m.groups()
        c = cells(rest)
        card = {"id": cid, "slug": slug, "file": file}
        if section == "Open":
            state, label, session, nxt = c[0], c[1], c[2], c[3]
            card["state"] = state.replace(" ", "_")
            card["label"] = None if label in ("—", "") else label
            card["holder"] = None if session in ("—", "") else session.strip("`")
            card["next"] = nxt
        else:
            card["state"] = "done"
            card["label"] = finished_label(c[0])
            card["holder"] = None
            card["next"] = None
        cards.append(card)
    return cards


def title_of(card):
    if card["id"] in TITLES:
        return TITLES[card["id"]]
    words = [WORDS.get(w, w) for w in card["slug"].split("-")]
    title = " ".join(words)
    return title[0].upper() + title[1:]


def description_of(card):
    text = f"Card: [docs/tasks/{card['file']}](../../../../docs/tasks/{card['file']})\n"
    if card["next"] and card["state"] in ("backlog", "ready"):
        text += f"\nNext dependency when the card moved in (2026-10-06): {card['next']}\n"
    return text


def fh(fh_path, *args, holder=None, ok_codes=(0,)):
    cmd = [fh_path, "--seabag", str(ROOT), "--json", *args]
    if holder:
        cmd += ["--as", holder]
    r = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT)
    if r.returncode not in ok_codes:
        sys.exit(f"fh {' '.join(args)} failed ({r.returncode}): {r.stdout}{r.stderr}")
    return json.loads(r.stdout)


def move(fh_path, cards):
    if SEABAG.exists():
        sys.exit(f"{SEABAG} already exists; the move runs once. Use --check.")
    subprocess.run([fh_path, "init", "--name", "FlipHopper"], cwd=ROOT, check=True, capture_output=True)
    project = SEABAG / "project.json"
    data = json.loads(project.read_text(encoding="utf-8"))
    data["id"] = "fliphopper"  # fh init names it after the folder, which may be a worktree
    data["labels"] = LADDER
    project.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    for aid, name in AREAS:
        fh(fh_path, "add", "area", aid, name)
    for card in cards:
        cid = card["id"]
        fh(fh_path, "add", "task", cid, "--area", cid[:3], "--title", title_of(card),
           "--description", description_of(card))
        state = card["state"]
        if state == "backlog":
            continue
        who = card["holder"] or MOVER
        fh(fh_path, "claim", cid, holder=who)
        if card["label"]:
            fh(fh_path, "label", cid, card["label"], holder=who)
        if state == "waiting":
            fh(fh_path, "wait", cid, "--on", card["next"], holder=who)
        elif state == "done":
            fh(fh_path, "done", cid, holder=who)
        if state != "in_progress":
            fh(fh_path, "release", cid, holder=who)


def check(fh_path, cards):
    listed = fh(fh_path, "list")
    tasks = {t["id"]: t for a in listed["areas"] for t in a["tasks"]}
    problems = []
    for card in cards:
        t = tasks.pop(card["id"], None)
        if t is None:
            problems.append(f"{card['id']}: missing from the seabag")
            continue
        want = {"state": card["state"], "label": card["label"], "holder": card["holder"]}
        got = {"state": t.get("state"), "label": t.get("label"), "holder": t.get("holder")}
        if want != got:
            problems.append(f"{card['id']}: board {want} but seabag {got}")
        if card["state"] == "waiting" and t.get("waiting_on") != card["next"]:
            problems.append(f"{card['id']}: waiting on {t.get('waiting_on')!r}, board says {card['next']!r}")
    problems += [f"{tid}: in the seabag but not on the board" for tid in tasks]
    for p in problems:
        print(p)
    print(f"{len(cards)} board cards, {len(problems)} differences")
    return 1 if problems else 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--fh", default="fh", help="the fh program (default: fh on PATH)")
    ap.add_argument("--board", default=str(ROOT / "docs" / "TASK_BOARD.md"))
    ap.add_argument("--check", action="store_true", help="only compare the board with the seabag")
    a = ap.parse_args()
    cards = read_board(Path(a.board))
    if not a.check:
        move(a.fh, cards)
    sys.exit(check(a.fh, cards))


if __name__ == "__main__":
    main()
