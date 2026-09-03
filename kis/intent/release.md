# Releases

Eleven releases have shipped on GitHub, all unsigned by Apple. Every asset was
downloaded back from GitHub and compared byte for byte against what was built;
checksums for each are on its GitHub release page.

| Version | Date | Shipped | Release |
|---|---|---|---|
| v0.7.0 | 2026-09-04 | Redesign, Appearance, `tauri-plugin-dialog`, Activity Monitor, Discover (the app's first read-only network client), a private API | [tag](https://github.com/smkamranqadri/llamaport/releases/tag/v0.7.0) |
| v0.6.1 | 2026-09-02 | Packaging fix: bundles now sign ad-hoc properly | [tag](https://github.com/smkamranqadri/llamaport/releases/tag/v0.6.1) |
| v0.6.0 | 2026-09-02 | The pi button, the app's first write to a file it does not own ([pi.md](pi.md)) | [tag](https://github.com/smkamranqadri/llamaport/releases/tag/v0.6.0) |
| v0.5.0 | 2026-09-01 | Tune: run history and an on-demand speed ladder ([tune.md](tune.md)) | [tag](https://github.com/smkamranqadri/llamaport/releases/tag/v0.5.0) |
| v0.4.0 | 2026-08-31 | Context and layer offload can be left unset for llama.cpp to fit; model screen reorganised ([fitting.md](fitting.md)) | [tag](https://github.com/smkamranqadri/llamaport/releases/tag/v0.4.0) |
| v0.3.2 | 2026-08-31 | Three corrected figures and one slider step | [tag](https://github.com/smkamranqadri/llamaport/releases/tag/v0.3.2) |
| v0.3.1 | 2026-08-08 | One fix; first release published as Latest rather than pre-release | [tag](https://github.com/smkamranqadri/llamaport/releases/tag/v0.3.1) |
| v0.3.0 | 2026-08-08 | Last used, plus a universal (Apple Silicon and Intel) build | [tag](https://github.com/smkamranqadri/llamaport/releases/tag/v0.3.0) |
| v0.2.1 | 2026-08-04 | Download queue, plus a path traversal fix | [tag](https://github.com/smkamranqadri/llamaport/releases/tag/v0.2.1) |
| v0.2.0 | 2026-08-03 | The Persistence phase, plus three security fixes | [tag](https://github.com/smkamranqadri/llamaport/releases/tag/v0.2.0) |
| v0.1.0 | 2026-08-03 | First public beta: unsigned, MIT-licensed, macOS | [tag](https://github.com/smkamranqadri/llamaport/releases/tag/v0.1.0) |

## Notes on releases whose lessons still bind

**v0.7.0.** A general-purpose security review ran over `v0.6.1..HEAD` before the
tag, not the `/security-review` skill. It found two Medium, three Low and four
Informational issues and fixed all but one: a browse cursor and an avatar URL
could reach an internal host, the transfer engine could follow a redirect to
another scheme, a planted file could be served into an `<img src>` unread, and
three Discover commands blocked the async runtime by doing blocking work on
`async fn`. The release also ships `macOSPrivateApi`, required for the
translucent sidebar, which bars the App Store permanently
([appearance.md](appearance.md)).

**v0.6.1.** Packaging only, no functional change from v0.6.0. The shipped
bundles carried only the linker's ad-hoc signature, so macOS could not verify
them and refused with "Llamaport.app is damaged and can't be opened", not the
unidentified-developer prompt the README had described. `"signingIdentity": "-"`
makes the build sign ad-hoc properly. Every release from v0.1.0 to v0.6.0 now
carries a warning block naming the dialog and the fix, since each had told
readers to use Open Anyway, a button that dialog does not have.

**v0.3.0.** The first universal build: Apple Silicon and Intel in one bundle.
The x86_64 slice was run under Rosetta and registered with LaunchServices as
`Arch=x86_64`, so it is not a stub, but Rosetta is not Intel hardware and no
Intel Mac has run it.

**v0.2.0.** A security review became part of shipping, because this one
changed the release: it found a symlinked `.part` being written through and a
resume that never re-validated its URL. Both were fixed and the `.dmg` was
rebuilt before publishing.

**v0.1.0.** `CI=true` is mandatory for `tauri build`; without it,
`bundle_dmg.sh` fails to drive Finder through Apple events. The version is read
from the built bundle, not `CARGO_PKG_VERSION`, so what a tester quotes always
matches the `.dmg` filename.

## Decisions

- **Public GitHub release, not a private handout**, accepting the licence, the
  README for strangers, and the issue tracker that come with it.
- **Unsigned, and said out loud.** No Apple Developer Program membership. This
  loses testers at the Gatekeeper wall, accepted because the audience already
  compiles llama.cpp, and notarization can be added later without invalidating
  anything a tester did.
- **MIT**, matching llama.cpp itself.
- **No auto-updater, no CI.** Both need infrastructure a single-target beta does
  not justify yet; both are right after the beta shows whether anyone cares.
- **`rawArgs` may not set what the app owns.** The app's `--host` and `--port`
  are appended after `rawArgs` and win by being last, so where the server binds
  no longer depends on a blocklist staying complete.
- **Do not write "Latest" into a release entry.** GitHub owns and moves that
  state; record what is durable (full release or pre-release) and let
  `gh release list` answer which is current.

## Closing conditions

### Before the tag

- `rawArgs` containing `--host` or `--port` is refused, naming the field that
  owns it, covered by a test.
- No Tauri logo anywhere in the bundle.
- LICENSE present; the README answers what, who and how for a reader who has
  not seen the code.
- The change is demonstrable in the artefact, not inferred from the tree: a
  frontend change by its new bundle digest, a Rust change by a string only it
  introduces.
- `codesign --verify --deep --strict` on the built `.app` reports valid on
  disk.
- The published asset is downloaded back and compared byte for byte.

### After the tag

- Launching the installed `.app` five times from Finder shows a usable window
  every time, with nothing else fullscreen.
- Closing the window and clicking the Dock icon brings it back, checked with a
  real click.
- No `llama-server` found, and no models directory, both say what to do.
- The `.dmg` is downloaded through a browser and opened following only what
  the README says.

One item stays open: no Intel Mac has run the universal build, since v0.3.0.

## Distribution

A Show and tell post is live in llama.cpp's own Discussions:
https://github.com/ggml-org/llama.cpp/discussions/26772. It claims what the
app does, that it has no chat of its own, that it is an unsigned beta, and
that no Intel Mac has run the universal build. Edit it whenever a release
changes what it claims.

Four list submissions are open, all disclosing authorship:

- `jaywcjlove/awesome-mac`: https://github.com/jaywcjlove/awesome-mac/pull/2526
- `serhii-londar/open-source-mac-os-apps`: https://github.com/serhii-londar/open-source-mac-os-apps/pull/1252
- `rafska/awesome-local-llm`: https://github.com/rafska/awesome-local-llm/pull/171
- `vince-lam/awesome-local-llms`: https://github.com/vince-lam/awesome-local-llms/issues/66

Two large lists were skipped as inactive rather than missed:
`Hannibal046/Awesome-LLM` and `underlines/awesome-ml`.

**r/LocalLLaMA** forbids primarily LLM-generated copy, requires disclosed
affiliation under a 1-in-10 self-promotion guideline, and needs karma this
account does not have. Any post has to be the author's own words, checked in a
browser since Reddit blocks automated routes to its rules.

**Homebrew is deferred.** `homebrew/cask` requires notability this repository
does not have, and an own tap would quarantine every download with no way to
opt out, so an unsigned build would install and then refuse to open.

The repository's description, topics and social preview are set;
`assets/social-preview.png` is the committed source, and the upload is
browser-only, so `usesCustomOpenGraphImage` is the only way to verify it.
