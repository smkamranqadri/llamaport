#!/usr/bin/env bash
set -euo pipefail

skill_dir="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"

if command -v uv >/dev/null 2>&1; then
  uv tool run \
    --from 'git+https://github.com/agentskills/agentskills.git#subdirectory=skills-ref' \
    skills-ref validate "$skill_dir"
  exit 0
fi

if ! command -v ruby >/dev/null 2>&1; then
  echo "Error: uv or ruby is required to validate a skill package." >&2
  exit 127
fi

ruby -ryaml -e '
  skill_dir = ARGV.fetch(0)
  skill_md = File.join(skill_dir, "SKILL.md")
  abort("SKILL.md not found: #{skill_md}") unless File.file?(skill_md)

  text = File.read(skill_md)
  frontmatter = text[/\A---\n(.*?)\n---\n/m, 1]
  abort("Invalid or missing YAML frontmatter: #{skill_md}") unless frontmatter

  data = YAML.safe_load(frontmatter)
  abort("Frontmatter must be a YAML map") unless data.is_a?(Hash)
  abort("Missing name") if data["name"].to_s.strip.empty?
  abort("Invalid name") unless data["name"].match?(/\A[a-z0-9-]+\z/)
  abort("Missing description") if data["description"].to_s.strip.empty?
  abort("Description exceeds 1024 characters") if data["description"].length > 1024

  puts "Valid skill: #{skill_dir} (ruby fallback)"
' "$skill_dir"
