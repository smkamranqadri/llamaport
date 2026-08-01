#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  update-installed-skill.sh --source <skill-dir-or-repo-root> [--install-dir <skill-dir>] [--check|--apply]

Options:
  --source <path>       Source skill package or repo root containing .agents/skills/kis.
                        Can also be set with KIS_SKILL_SOURCE.
  --install-dir <path>  Installed skill package to check/update.
                        Defaults to the parent directory of this script.
  --check               Report whether an update is available. Default.
  --apply               Replace the installed package with the source package after validation.
  -h, --help            Show this help.
EOF
}

die() {
  echo "Error: $*" >&2
  exit 1
}

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
install_dir="$(cd "$script_dir/.." && pwd -P)"
source_arg="${KIS_SKILL_SOURCE:-}"
mode="check"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source)
      [[ $# -ge 2 ]] || die "--source requires a path"
      source_arg="$2"
      shift 2
      ;;
    --install-dir)
      [[ $# -ge 2 ]] || die "--install-dir requires a path"
      install_dir="$2"
      shift 2
      ;;
    --check)
      mode="check"
      shift
      ;;
    --apply)
      mode="apply"
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

[[ -n "$source_arg" ]] || die "missing --source <skill-dir-or-repo-root> or KIS_SKILL_SOURCE"

normalize_skill_dir() {
  local candidate="$1"

  if [[ -f "$candidate/SKILL.md" ]]; then
    (cd "$candidate" && pwd -P)
    return 0
  fi

  # source repo layout
  if [[ -f "$candidate/skills/kis/SKILL.md" ]]; then
    (cd "$candidate/skills/kis" && pwd -P)
    return 0
  fi

  # installed project layout
  if [[ -f "$candidate/.agents/skills/kis/SKILL.md" ]]; then
    (cd "$candidate/.agents/skills/kis" && pwd -P)
    return 0
  fi

  return 1
}

read_skill_version() {
  local skill_dir="$1"

  ruby -ryaml -e '
    path = File.join(ARGV.fetch(0), "SKILL.md")
    text = File.read(path)
    frontmatter = text[/\A---\n(.*?)\n---\n/m, 1]
    abort("missing YAML frontmatter in #{path}") unless frontmatter
    data = YAML.safe_load(frontmatter)
    version = data.dig("metadata", "version")
    abort("missing metadata.version in #{path}") if version.to_s.strip.empty?
    puts version
  ' "$skill_dir"
}

validate_skill_dir() {
  local skill_dir="$1"

  if [[ -x "$skill_dir/scripts/validate-skill.sh" ]]; then
    "$skill_dir/scripts/validate-skill.sh" "$skill_dir" >/dev/null
    return 0
  fi

  [[ -f "$skill_dir/SKILL.md" ]] || die "source package missing SKILL.md: $skill_dir"
}

source_dir="$(normalize_skill_dir "$source_arg")" || die "source is not a KIS skill package or repo root: $source_arg"
install_dir="$(normalize_skill_dir "$install_dir")" || die "install dir is not a KIS skill package: $install_dir"

source_version="$(read_skill_version "$source_dir")"
installed_version="$(read_skill_version "$install_dir")"

if [[ "$source_dir" == "$install_dir" ]]; then
  echo "Already current: $installed_version"
  echo "Source and install directory are the same: $install_dir"
  exit 0
fi

if [[ "$source_version" == "$installed_version" ]]; then
  echo "Already current: $installed_version"
  exit 0
fi

if [[ "$mode" == "check" ]]; then
  echo "Update available: $installed_version -> $source_version"
  echo "Source: $source_dir"
  echo "Install: $install_dir"
  echo "Run with --apply to update."
  exit 0
fi

validate_skill_dir "$source_dir"

tmp_dir="$(mktemp -d)"
backup_dir="${install_dir}.backup.$(date +%Y%m%d%H%M%S)"
trap 'rm -rf "$tmp_dir"' EXIT

mkdir -p "$tmp_dir/kis"
cp -R "$source_dir/." "$tmp_dir/kis/"
validate_skill_dir "$tmp_dir/kis"

mv "$install_dir" "$backup_dir"
mkdir -p "$(dirname "$install_dir")"
cp -R "$tmp_dir/kis" "$install_dir"

if validate_skill_dir "$install_dir"; then
  rm -rf "$backup_dir"
  echo "Updated KIS skill: $installed_version -> $source_version"
  echo "Install: $install_dir"
else
  rm -rf "$install_dir"
  mv "$backup_dir" "$install_dir"
  die "updated package failed validation; restored previous install"
fi
