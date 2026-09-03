# Current

```text
Branch:   main, working tree clean. v0.7.0 is tagged at 12de686 and HEAD has
          moved past it with memory-only commits. Run
          `git rev-list --count v0.7.0..HEAD` before building a release
          artefact. The reason is in knowledge/technical.md, under Releases.

Task:     Nothing in progress. v0.7.0 shipped 2026-09-04 (intent/release.md).

Mode:     Standard.

Command:  export PATH="$HOME/.cargo/bin:$PATH"
          bun run build
          cargo test --manifest-path src-tauri/Cargo.toml
          cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
          cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
          Capture each exit status on its own line. Run the app with
          `bun run tauri dev`. Working rules are in AGENTS.md.

Blocker:  None. Open and not blocking:
          - No Intel Mac has run the universal build (open since v0.3.0).
          - pi answering a prompt through the entry the app wrote, the pi
            button's last acceptance check, is the author's to run
            (intent/pi.md).

Next:     Nothing chosen. The author decides.
```

Where the project stands: [intent/roadmap.md](../intent/roadmap.md).
What the app is for: [intent/direction.md](../intent/direction.md).

## Proof

Verified 2026-09-04: all four checks passed, with 319 tests passing and 19
ignored. The ignored tests are the `real_launch`, `real_tune` and `real_hub`
suites, which need the binary, a real model, or the network.
`tests/stylesheet.rs` is the only test of the frontend.

Each phase file under `intent/` records its own verification.
[persistence.md](../intent/persistence.md) has none, because it was not
recorded at the time.
