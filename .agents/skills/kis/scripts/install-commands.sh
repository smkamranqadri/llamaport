#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  install-commands.sh [--target <project-root>] [--skill-dir <path>] [--host project|claude|pi|codex|all]
                      [--codex-prompts-dir <path>] [--remove]

Options:
  --target <path>             Project root. Defaults to current directory.
  --skill-dir <path>          Installed skill package. Defaults to <target>/.agents/skills/kis.
  --host <name>               project (default: claude and pi), claude, pi, codex, or all.
                              Only codex writes outside the project.
  --codex-prompts-dir <path>  Codex prompts directory. Defaults to ${CODEX_HOME:-~/.codex}/prompts.
                              Codex prompts are user-level and are not shared through the repository.
  --remove                    Remove installed command adapters instead of creating them.
  -h, --help                  Show this help.
EOF
}

die() {
  echo "Error: $*" >&2
  exit 1
}

target_root="."
skill_dir=""
host="project"
codex_prompts_dir="${CODEX_HOME:-$HOME/.codex}/prompts"
remove="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      [[ $# -ge 2 ]] || die "--target requires a path"
      target_root="$2"
      shift 2
      ;;
    --skill-dir)
      [[ $# -ge 2 ]] || die "--skill-dir requires a path"
      skill_dir="$2"
      shift 2
      ;;
    --host)
      [[ $# -ge 2 ]] || die "--host requires project, claude, pi, codex, or all"
      host="$2"
      shift 2
      ;;
    --codex-prompts-dir)
      [[ $# -ge 2 ]] || die "--codex-prompts-dir requires a path"
      codex_prompts_dir="$2"
      shift 2
      ;;
    --remove)
      remove="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

case "$host" in
  project|claude|pi|codex|all) ;;
  *) die "unknown host: $host" ;;
esac

target_root="$(mkdir -p "$target_root" && cd "$target_root" && pwd -P)"
[[ -n "$skill_dir" ]] || skill_dir="$target_root/.agents/skills/kis"

claude_link="$target_root/.claude/commands/kis"
pi_prompts_dir="$target_root/.pi/prompts"

install_claude() {
  local commands_dir="$skill_dir/commands"
  [[ -d "$commands_dir" ]] || { echo "Claude commands: skipped (no commands/ in $skill_dir)"; return 0; }

  local link_target
  if [[ "$skill_dir" == "$target_root/.agents/skills/kis" ]]; then
    link_target="../../.agents/skills/kis/commands"
  else
    link_target="$commands_dir"
  fi

  mkdir -p "$(dirname "$claude_link")"
  rm -rf "$claude_link"
  ln -s "$link_target" "$claude_link"
  echo "Claude commands: $claude_link -> $link_target"
  echo "Claude usage: /kis:start /kis:plan /kis:act /kis:sync /kis:check /kis:init"
}

remove_claude() {
  if [[ -e "$claude_link" || -L "$claude_link" ]]; then
    rm -rf "$claude_link"
    echo "Claude commands: removed $claude_link"
  else
    echo "Claude commands: nothing to remove"
  fi
}

install_pi() {
  local commands_dir="$skill_dir/commands"
  [[ -d "$commands_dir" ]] || { echo "Pi prompts: skipped (no commands/ in $skill_dir)"; return 0; }

  mkdir -p "$pi_prompts_dir"
  local relative_prefix=""
  if [[ "$skill_dir" == "$target_root/.agents/skills/kis" ]]; then
    relative_prefix="../../.agents/skills/kis/commands"
  else
    relative_prefix="$commands_dir"
  fi

  local installed=0
  local file name
  while IFS= read -r file; do
    name="$(basename "$file" .md)"
    rm -f "$pi_prompts_dir/kis-$name.md"
    ln -s "$relative_prefix/$name.md" "$pi_prompts_dir/kis-$name.md"
    installed=$((installed + 1))
  done < <(find "$commands_dir" -maxdepth 1 -type f -name '*.md' | sort)

  echo "Pi prompts: $installed link(s) in ${pi_prompts_dir#"$target_root/"}"
  echo "Pi usage: /kis-start /kis-plan /kis-act /kis-sync /kis-check /kis-init"
}

remove_pi() {
  local removed=0
  local file
  while IFS= read -r file; do
    rm -f "$file"
    removed=$((removed + 1))
  done < <(find "$pi_prompts_dir" -maxdepth 1 -name 'kis-*.md' 2>/dev/null | sort)
  echo "Pi prompts: removed $removed link(s)"
}

install_codex() {
  local commands_dir="$skill_dir/commands"
  [[ -d "$commands_dir" ]] || { echo "Codex prompts: skipped (no commands/ in $skill_dir)"; return 0; }

  mkdir -p "$codex_prompts_dir"
  local installed=0
  local file name
  while IFS= read -r file; do
    name="$(basename "$file" .md)"
    cp "$file" "$codex_prompts_dir/kis-$name.md"
    installed=$((installed + 1))
  done < <(find "$commands_dir" -maxdepth 1 -type f -name '*.md' | sort)

  echo "Codex prompts: $installed file(s) in $codex_prompts_dir"
  echo "Codex usage: /prompts:kis-start /prompts:kis-plan /prompts:kis-act /prompts:kis-sync /prompts:kis-check /prompts:kis-init"
  echo "Codex prompts are user-level and copied, not linked. Re-run after a skill update."
}

remove_codex() {
  local removed=0
  local file
  while IFS= read -r file; do
    rm -f "$file"
    removed=$((removed + 1))
  done < <(find "$codex_prompts_dir" -maxdepth 1 -type f -name 'kis-*.md' 2>/dev/null | sort)
  echo "Codex prompts: removed $removed file(s) from $codex_prompts_dir"
}

want_claude="false"
want_pi="false"
want_codex="false"
if [[ "$host" == "claude" || "$host" == "project" || "$host" == "all" ]]; then
  want_claude="true"
fi
if [[ "$host" == "pi" || "$host" == "project" || "$host" == "all" ]]; then
  want_pi="true"
fi
if [[ "$host" == "codex" || "$host" == "all" ]]; then
  want_codex="true"
fi

if [[ "$remove" == "true" ]]; then
  if [[ "$want_claude" == "true" ]]; then
    remove_claude
  fi
  if [[ "$want_pi" == "true" ]]; then
    remove_pi
  fi
  if [[ "$want_codex" == "true" ]]; then
    remove_codex
  fi
  exit 0
fi

[[ -d "$skill_dir" ]] || die "skill package not found: $skill_dir"

if [[ "$want_claude" == "true" ]]; then
  install_claude
fi
if [[ "$want_pi" == "true" ]]; then
  install_pi
fi
if [[ "$want_codex" == "true" ]]; then
  install_codex
fi
