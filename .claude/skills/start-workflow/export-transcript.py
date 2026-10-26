#!/usr/bin/env python3
"""Renders every Claude Code session of one project (main + subagents) to Markdown for an AI-use disclosure.

Usage: export-transcript.py <project-dir> <out-dir>
  project-dir: the repo the sessions ran in (e.g. <parent-dir>/my-takehome)
Writes one <date>-<session>.md per session, subagents inline after the main thread.
"""
import glob
import json
import os
import re
import sys

# Harness-injected context (global CLAUDE.md, memory, skill lists) is not a prompt; the disclosure says it was left out.
REMINDER = re.compile(r"<system-reminder>.*?</system-reminder>", re.S)


def render(path):
    out = []
    for line in open(path, errors="ignore"):
        try:
            d = json.loads(line)
        except json.JSONDecodeError:
            continue
        msg = d.get("message") or {}
        role, content = msg.get("role"), msg.get("content")
        if role not in ("user", "assistant") or d.get("isMeta"):
            continue
        ts = (d.get("timestamp") or "")[:19].replace("T", " ")
        blocks = [{"type": "text", "text": content}] if isinstance(content, str) else content or []
        for b in blocks:
            t = b.get("type")
            text = REMINDER.sub("", b.get("text", "")) if t == "text" else ""
            if t == "text" and text.strip():
                who = "👤 Prompt" if role == "user" else "🤖 Claude"
                out.append(f"### {who} · {ts}\n\n{text.strip()}\n")
            elif t == "tool_use":
                args = json.dumps(b.get("input", {}), indent=2, ensure_ascii=False)
                out.append(f"#### 🔧 Tool: {b.get('name')}\n\n```json\n{args}\n```\n")
            elif t == "tool_result":
                c = b.get("content")
                text = c if isinstance(c, str) else "\n".join(x.get("text", "") for x in c or [] if isinstance(x, dict))
                out.append(f"#### 📤 Result\n\n```\n{REMINDER.sub('', text).strip()}\n```\n")
    return "\n".join(out)


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    project = os.path.realpath(os.path.expanduser(sys.argv[1]))
    out_dir = os.path.expanduser(sys.argv[2])
    key = "".join(c if c.isalnum() else "-" for c in project)
    base = os.path.join(os.path.expanduser(os.environ.get("CLAUDE_CONFIG_DIR", "~/.claude")), "projects", key)
    sessions = sorted(glob.glob(f"{base}/*.jsonl"), key=os.path.getmtime)
    if not sessions:
        sys.exit(f"no sessions in {base}")
    os.makedirs(out_dir, exist_ok=True)
    for s in sessions:
        sid = os.path.basename(s)[:-6]
        day = ""
        for line in open(s, errors="ignore"):
            if '"timestamp"' in line:
                day = json.loads(line).get("timestamp", "")[:10]
                if day:
                    break
        parts = [f"# Session {sid}\n", render(s)]
        for sub in sorted(glob.glob(f"{base}/{sid}/subagents/*.jsonl")):
            parts += [f"\n---\n\n## Subagent {os.path.basename(sub)[:-6]}\n", render(sub)]
        path = os.path.join(out_dir, f"{day}-{sid[:8]}.md")
        open(path, "w").write("\n".join(parts))
        print(path)


if __name__ == "__main__":
    main()
