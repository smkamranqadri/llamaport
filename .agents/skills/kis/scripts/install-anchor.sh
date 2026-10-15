#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  install-anchor.sh [--target <project-root>] [--settings <path>] [--hooks-json <path>]
                    [--instructions <path>]... [--no-hook] [--no-instructions]
                    [--check] [--remove]

Installs the KIS re-anchor layer into a project:
  - a Claude SessionStart hook that injects current State at session start, resume, clear, and compact
  - an Antigravity PreInvocation hook that injects current State once per conversation
  - a marked KIS block in the project instruction files that every host loads automatically

Options:
  --target <path>        Project root. Defaults to current directory.
  --settings <path>      Claude settings file. Defaults to <target>/.claude/settings.json.
  --hooks-json <path>    Antigravity hooks file. Defaults to <target>/.agents/hooks.json.
  --instructions <path>  Instruction file to carry the KIS block. Repeatable.
                         Defaults to AGENTS.md, plus CLAUDE.md when it already exists.
  --no-hook              Skip both session hooks.
  --no-instructions      Skip the instruction file block.
  --check                Report what is installed and change nothing.
  --remove               Remove the hook entry and the instruction block.
  -h, --help             Show this help.
EOF
}

die() {
  echo "Error: $*" >&2
  exit 1
}

target_root="."
settings_file=""
hooks_json_file=""
instruction_files=()
want_hook="true"
want_instructions="true"
mode="apply"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      [[ $# -ge 2 ]] || die "--target requires a path"
      target_root="$2"
      shift 2
      ;;
    --settings)
      [[ $# -ge 2 ]] || die "--settings requires a path"
      settings_file="$2"
      shift 2
      ;;
    --hooks-json)
      [[ $# -ge 2 ]] || die "--hooks-json requires a path"
      hooks_json_file="$2"
      shift 2
      ;;
    --instructions)
      [[ $# -ge 2 ]] || die "--instructions requires a path"
      instruction_files+=("$2")
      shift 2
      ;;
    --no-hook)
      want_hook="false"
      shift
      ;;
    --no-instructions)
      want_instructions="false"
      shift
      ;;
    --check)
      mode="check"
      shift
      ;;
    --remove)
      mode="remove"
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

command -v ruby >/dev/null 2>&1 || die "ruby is required to edit settings and instruction files"

target_root="$(mkdir -p "$target_root" && cd "$target_root" && pwd -P)"
[[ -n "$settings_file" ]] || settings_file="$target_root/.claude/settings.json"
[[ -n "$hooks_json_file" ]] || hooks_json_file="$target_root/.agents/hooks.json"

if [[ ${#instruction_files[@]} -eq 0 ]]; then
  instruction_files=("$target_root/AGENTS.md")
  if [[ -f "$target_root/CLAUDE.md" ]]; then
    instruction_files+=("$target_root/CLAUDE.md")
  fi
fi

hook_command='"$CLAUDE_PROJECT_DIR"/.agents/skills/kis/hooks/session-anchor.sh'
hook_marker='kis/hooks/session-anchor.sh'

antigravity_hook_name="kis-anchor"
if [[ "$hooks_json_file" == "$target_root/.agents/hooks.json" ]]; then
  # Antigravity runs a hook command from the directory holding hooks.json.
  antigravity_hook_command="./skills/kis/hooks/antigravity-anchor.sh"
else
  antigravity_hook_command="$target_root/.agents/skills/kis/hooks/antigravity-anchor.sh"
fi

apply_hook() {
  ruby -rjson -e '
    settings_path, hook_command, hook_marker, mode = ARGV
    data = {}
    if File.file?(settings_path)
      raw = File.read(settings_path).strip
      unless raw.empty?
        begin
          data = JSON.parse(raw)
        rescue JSON::ParserError => e
          abort("cannot parse #{settings_path}: #{e.message}")
        end
      end
      abort("#{settings_path} is not a JSON object") unless data.is_a?(Hash)
    end

    hooks = data["hooks"] ||= {}
    events = hooks["SessionStart"] ||= []
    installed = events.any? do |entry|
      Array(entry["hooks"]).any? { |h| h["command"].to_s.include?(hook_marker) }
    end

    case mode
    when "check"
      puts(installed ? "Claude SessionStart hook: installed" : "Claude SessionStart hook: missing")
      exit 0
    when "remove"
      unless installed
        puts "Claude SessionStart hook: nothing to remove"
        exit 0
      end
      events.each do |entry|
        entry["hooks"] = Array(entry["hooks"]).reject { |h| h["command"].to_s.include?(hook_marker) }
      end
      events.reject! { |entry| Array(entry["hooks"]).empty? }
      hooks.delete("SessionStart") if events.empty?
      data.delete("hooks") if hooks.empty?
      puts "Claude SessionStart hook: removed from #{settings_path}"
    else
      if installed
        puts "Claude SessionStart hook: already installed"
        exit 0
      end
      events << {
        "matcher" => "startup|resume|clear|compact",
        "hooks" => [{ "type" => "command", "command" => hook_command }]
      }
      puts "Claude SessionStart hook: added to #{settings_path}"
    end

    require "fileutils"
    FileUtils.mkdir_p(File.dirname(settings_path))
    File.write(settings_path, JSON.pretty_generate(data) + "\n")
  ' "$settings_file" "$hook_command" "$hook_marker" "$mode"
}

apply_antigravity_hook() {
  ruby -rjson -e '
    hooks_path, hook_name, hook_command, mode = ARGV
    data = {}
    if File.file?(hooks_path)
      raw = File.read(hooks_path).strip
      unless raw.empty?
        begin
          data = JSON.parse(raw)
        rescue JSON::ParserError => e
          abort("cannot parse #{hooks_path}: #{e.message}")
        end
      end
      abort("#{hooks_path} is not a JSON object") unless data.is_a?(Hash)
    end

    installed = data.key?(hook_name)

    case mode
    when "check"
      puts(installed ? "Antigravity PreInvocation hook: installed" : "Antigravity PreInvocation hook: missing")
      exit 0
    when "remove"
      unless installed
        puts "Antigravity PreInvocation hook: nothing to remove"
        exit 0
      end
      data.delete(hook_name)
      if data.empty? && File.file?(hooks_path)
        File.delete(hooks_path)
        puts "Antigravity PreInvocation hook: removed #{hooks_path}"
        exit 0
      end
      puts "Antigravity PreInvocation hook: removed from #{hooks_path}"
    else
      data[hook_name] = {
        "PreInvocation" => [{ "type" => "command", "command" => hook_command, "timeout" => 15 }]
      }
      puts "Antigravity PreInvocation hook: #{installed ? "refreshed in" : "added to"} #{hooks_path}"
    end

    require "fileutils"
    FileUtils.mkdir_p(File.dirname(hooks_path))
    File.write(hooks_path, JSON.pretty_generate(data) + "\n")
  ' "$hooks_json_file" "$antigravity_hook_name" "$antigravity_hook_command" "$mode"
}

apply_instructions() {
  local file
  for file in "${instruction_files[@]}"; do
    ruby -e '
      path, mode = ARGV
      start_marker = "<!-- kis:anchor:start -->"
      end_marker = "<!-- kis:anchor:end -->"

      block = <<~BLOCK.strip
        #{start_marker}
        ## KIS Project Memory

        This project uses KIS memory under `kis/`. Knowledge = stable facts. Intent = goals and plans. State = current reality.

        - Read `kis/state/` before planning or implementing, then only the Intent and Knowledge the task needs.
        - Put each fact in exactly one layer, and update an existing file instead of creating a new one.
        - Prove work with real command or verification output before marking anything done.
        - Synchronize the KIS layers that changed when work finishes.
        - Full instructions: `.agents/skills/kis/SKILL.md`.
        - Commands live in `.agents/skills/kis/commands/`: start, plan, act, sync, check, init.
          Claude `/kis:start`, Pi and OpenCode `/kis-start`, Codex `/prompts:kis-start`.
          Antigravity has no custom slash commands: name the step and follow its command file.
        #{end_marker}
      BLOCK

      exists = File.file?(path)
      text = exists ? File.read(path) : ""
      pattern = /#{Regexp.escape(start_marker)}.*?#{Regexp.escape(end_marker)}\n?/m
      present = text.match?(pattern)

      case mode
      when "check"
        puts "#{path}: #{present ? "KIS block present" : "KIS block missing"}"
        exit 0
      when "remove"
        unless present
          puts "#{path}: no KIS block to remove"
          exit 0
        end
        File.write(path, text.sub(pattern, "").gsub(/\n{3,}/, "\n\n").lstrip)
        puts "#{path}: KIS block removed"
      else
        if present
          File.write(path, text.sub(pattern, block + "\n"))
          puts "#{path}: KIS block refreshed"
        else
          separator = text.empty? || text.end_with?("\n\n") ? "" : (text.end_with?("\n") ? "\n" : "\n\n")
          File.write(path, text + separator + block + "\n")
          puts "#{path}: KIS block added"
        end
      end
    ' "$file" "$mode"
  done
}

if [[ "$want_hook" == "true" ]]; then
  apply_hook
  apply_antigravity_hook
fi

if [[ "$want_instructions" == "true" ]]; then
  apply_instructions
fi
