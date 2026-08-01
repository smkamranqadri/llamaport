#!/usr/bin/env bash
set -euo pipefail

project_root="${1:-${CLAUDE_PROJECT_DIR:-$PWD}}"
kis_dir="$project_root/kis"
max_state_lines="${KIS_ANCHOR_MAX_STATE_LINES:-60}"

echo "# KIS Anchor"
echo
echo "This project uses KIS project memory under \`kis/\`. Knowledge = stable facts. Intent = goals and plans. State = current reality."
echo "Load State first, put each fact in one layer, prove work before marking it done, and synchronize only what changed."
echo "Skill: .agents/skills/kis/SKILL.md"
echo "Commands: /kis:start /kis:plan /kis:act /kis:sync /kis:check /kis:init"
echo

if [[ ! -d "$kis_dir" ]]; then
  echo "KIS is not initialized in this project. Run /kis:init before non-trivial work."
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
  echo "No State file found under kis/state/. Recover context with /kis:start."
  exit 0
fi

echo "## Current State (${state_file#"$project_root/"})"
echo
total_lines="$(wc -l < "$state_file" | tr -d ' ')"
head -n "$max_state_lines" "$state_file"
if (( total_lines > max_state_lines )); then
  echo
  echo "[truncated at $max_state_lines of $total_lines lines - read ${state_file#"$project_root/"} for the rest]"
fi

other_state="$(find "$kis_dir/state" -maxdepth 1 -type f -name '*.md' ! -path "$state_file" 2>/dev/null | sort | sed "s|^$project_root/||")"
if [[ -n "$other_state" ]]; then
  echo
  echo "Other State files:"
  echo "$other_state" | sed 's/^/- /'
fi
