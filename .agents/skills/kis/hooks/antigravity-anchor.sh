#!/usr/bin/env bash
set -euo pipefail

hooks_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

command -v ruby >/dev/null 2>&1 || { echo '{}'; exit 0; }

ruby -rjson -rfileutils -e '
  hooks_dir = ARGV.fetch(0)

  raw = $stdin.read
  payload = {}
  begin
    parsed = JSON.parse(raw)
    payload = parsed if parsed.is_a?(Hash)
  rescue JSON::ParserError
  end

  workspace = Array(payload["workspacePaths"]).find { |p| p.to_s.strip != "" }
  workspace ||= File.expand_path("..", Dir.pwd)

  conversation_id = payload["conversationId"].to_s
  # PreInvocation fires before every model call; the marker keeps the anchor to once per conversation.
  unless conversation_id.empty?
    marker_dir = File.join(ENV["TMPDIR"] || "/tmp", "kis-antigravity-anchor")
    marker = File.join(marker_dir, conversation_id.gsub(/[^A-Za-z0-9_-]/, "_"))
    if File.exist?(marker)
      puts "{}"
      exit 0
    end
    FileUtils.mkdir_p(marker_dir)
    FileUtils.touch(marker)
  end

  anchor = File.join(hooks_dir, "session-anchor.sh")
  unless File.executable?(anchor)
    puts "{}"
    exit 0
  end

  text = IO.popen([anchor, workspace], &:read)
  if !$?.success? || text.to_s.strip.empty?
    puts "{}"
    exit 0
  end

  puts JSON.generate({ "injectSteps" => [{ "ephemeralMessage" => text }] })
' "$hooks_dir"
