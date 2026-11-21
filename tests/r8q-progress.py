#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Test device/r8q/board/tasks.json schema and tools/r8q-progress CLI."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TOOL = REPO_ROOT / "tools" / "r8q-progress"
TASKS = REPO_ROOT / "device" / "r8q" / "board" / "tasks.json"


def run(*args: str) -> str:
    res = subprocess.run([str(TOOL), *args], capture_output=True, text=True, check=True)
    return res.stdout


def main() -> None:
    data = json.loads(TASKS.read_text(encoding="utf-8"))
    assert data.get("spdx") == "MIT", "missing SPDX MIT"
    assert data.get("device") == "r8q"
    assert len(data["stages"]) >= 14

    seen_stage_ids: set[str] = set()
    seen_task_ids: set[str] = set()
    total = 0
    done = 0
    for st in data["stages"]:
        sid = st["id"]
        assert sid not in seen_stage_ids, f"duplicate stage id: {sid}"
        seen_stage_ids.add(sid)
        assert st["track"] in ("bringup", "release")
        assert st["tasks"], f"stage {sid} has no tasks"
        for t in st["tasks"]:
            tid = t["id"]
            assert tid not in seen_task_ids, f"duplicate task id: {tid}"
            seen_task_ids.add(tid)
            assert isinstance(t["done"], bool)
            total += 1
            if t["done"]:
                done += 1
                assert t.get("evidence"), f"done task {tid} missing evidence"
            else:
                assert t.get("verify"), f"open task {tid} missing verify criteria"

    out_json = json.loads(run("--json"))
    all_summary = out_json["summary"]["all"]
    assert all_summary["total"] == total
    assert all_summary["done"] == done
    expected_pct = round((done / total) * 100, 1)
    assert all_summary["percent"] == expected_pct

    help_out = run("--help")
    assert "r8q-progress" in help_out

    next_out = run("--next", "3")
    assert "Step-by-Step Remaining Atomic Tasks:" in next_out
    assert "4." not in next_out
    assert "Stage 9:" not in next_out

    stage_out = run("--stage", "14")
    assert "Remoteprocs, Sensors & Audio" in stage_out

    all_out = run("--all")
    assert "[DONE]" in all_out

    print(
        f"ok - r8q-progress: {done}/{total} atomic tasks ({expected_pct}%), "
        f"{len(data['stages'])} stages verified"
    )


if __name__ == "__main__":
    main()
