#!/usr/bin/env bash
set -euo pipefail

project_root="${1:-${CLAUDE_PROJECT_DIR:-$PWD}}"
kis_dir="$project_root/kis"
# The anchor is loaded at every session start, resume, clear and compact, so it must stay small.
# KIS_ANCHOR_MAX_LINES is the whole-anchor line budget, stated here and in the tool's env docs;
# the output below is built to fit it (minimum 16, the header plus the four summary fields and
# the pointer), and a State with no summary fields gets a capped head instead of the whole file.
max_lines="${KIS_ANCHOR_MAX_LINES:-20}"
# A summary field (its label plus its indented continuation lines) is joined and then cut at
# this many characters, "..." marking the cut. The full value is in the State file it points at.
max_field_chars="${KIS_ANCHOR_MAX_FIELD_CHARS:-200}"
[[ "$max_lines" =~ ^[0-9]+$ ]] || max_lines=20
[[ "$max_lines" -lt 16 ]] && max_lines=16
[[ "$max_field_chars" =~ ^[0-9]+$ ]] || max_field_chars=200

printed=0
p() { echo "$@"; printed=$((printed + 1)); }

p "# KIS Anchor"
p
p "This project uses KIS project memory under \`kis/\`. Knowledge = stable facts. Intent = goals and plans. State = current reality."
p "Load State first, put each fact in one layer, prove work before marking it done, and synchronize only what changed."
p "Skill: .agents/skills/kis/SKILL.md"
p "Commands in .agents/skills/kis/commands/: start, plan, act, sync, check, init."
p "Claude /kis:start, Pi and OpenCode /kis-start, Codex /prompts:kis-start. Antigravity has no custom slash commands: name the step instead."
p

if [[ ! -d "$kis_dir" ]]; then
  p "KIS is not initialized in this project. Run the init command before non-trivial work."
  exit 0
fi

state_file=""
if [[ -f "$kis_dir/state/current.md" ]]; then
  state_file="$kis_dir/state/current.md"
else
  while IFS= read -r candidate; do
    state_file="$candidate"
    break
  done < <(find "$kis_dir/state" -maxdepth 1 -type f -name '*.md' 2>/dev/null | sort)
fi

if [[ -z "$state_file" ]]; then
  p "No State file found under kis/state/. Recover context with the start command."
  exit 0
fi

rel="${state_file#"$project_root/"}"
total_lines="$(wc -l < "$state_file" | tr -d ' ')"
other_state="$(find "$kis_dir/state" -maxdepth 1 -type f -name '*.md' ! -path "$state_file" 2>/dev/null | sort | sed "s|^$project_root/||")"

p "## Current State: $rel"
p

# A short summary of the four recovery fields, never the whole file. Branch, Task, Blocker and
# Next come from a `Label:` line (any bullet marker, indented continuation lines joined, capped
# at max_field_chars). When a label is absent, Next falls back to the first item under a
# `## Next` heading and Blocker to the first item under a `## Open` heading: the shape real
# State files use.
summary="$(awk -v max="$max_field_chars" '
function trim(s) { sub(/^[[:space:]]+/, "", s); sub(/[[:space:]]+$/, "", s); return s }
function add(name, s) {
  s = trim(s)
  gsub(/[[:space:]]+/, " ", s)
  if (s == "" || v[name] != "") return
  v[name] = s
}
function app(name, s) {
  s = trim(s)
  if (s == "") return
  gsub(/[[:space:]]+/, " ", s)
  v[name] = (v[name] == "" ? s : v[name] " " s)
}
{
  line = $0
  if (line ~ /^##[[:space:]]/) { heading = trim(line); cur = ""; next }
  if (line ~ /^[[:space:]]*[-*]?[[:space:]]*(Branch|Task|Blocker|Next)[[:space:]]*:/) {
    n = line
    sub(/^[[:space:]]*[-*]?[[:space:]]*/, "", n)
    name = n; sub(/[[:space:]]*:.*/, "", name)
    val = n; sub(/^[^:]*:[[:space:]]*/, "", val)
    cur = name
    add(name, val)
    next
  }
  if (cur != "" && line ~ /^[[:space:]]*$/) { cur = ""; next }
  if (cur != "" && line ~ /^[[:space:]]/) { app(cur, line); next }
  if (cur != "") cur = ""
  slot = (heading == "## Next" ? "Next" : heading == "## Open" ? "Blocker" : "")
  if (slot != "" && v[slot] == "" && line ~ /^[[:space:]]*([-*]|[0-9]+[.)])[[:space:]]/) {
    item = line
    sub(/^[[:space:]]*([-*]|[0-9]+[.)])[[:space:]]*/, "", item)
    cur = slot
    add(slot, item)
  }
}
END {
  order[1] = "Branch"; order[2] = "Task"; order[3] = "Blocker"; order[4] = "Next"
  for (i = 1; i <= 4; i++) {
    n = order[i]
    if (v[n] == "") continue
    s = v[n]
    if (length(s) > max) s = substr(s, 1, max - 3) "..."
    print n ": " s
  }
}
' "$state_file")"

other_lines=0
[[ -n "$other_state" ]] && other_lines=1

if [[ -n "$summary" ]]; then
  printf '%s\n' "$summary"
  printed=$((printed + $(printf '%s\n' "$summary" | wc -l | tr -d ' ')))
  p
  p "Full State: $rel ($total_lines lines; summary within $max_lines lines) - read it before planning."
else
  # Nothing to summarise: show a head of State that fits the budget, and point at the file.
  room=$((max_lines - printed - 2 - other_lines))
  [[ "$room" -lt 1 ]] && room=1
  head -n "$room" "$state_file"
  printed=$((printed + room))
  p
  p "No Branch/Task/Blocker/Next summary fields found; read $rel ($total_lines lines) before planning."
fi

if [[ -n "$other_state" ]] && (( printed + 1 <= max_lines )); then
  p "Other State files: $(printf '%s\n' "$other_state" | paste -sd ', ' -)"
fi
