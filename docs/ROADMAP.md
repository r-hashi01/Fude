# Roadmap

A rough sketch of where `fude` is headed. Nothing here is a promise — scope
and ordering will shift as real usage reveals what's actually load-bearing.

If something on this page matters to you (or is *missing* and should be
here), please open a [Discussion](https://github.com/r-hashi01/Fude/discussions)
before we commit engineering time to it.

## Philosophy

`fude` is narrow on purpose. It is a *brush*: a minimal shell (window,
webview, IPC, a few guarded capabilities) that an AI-native document
editor can sit on top of. Scope decisions follow one rule:

> **Does this belong in every AI-native editor's shell, or does it belong
> in the app on top?** If it belongs to one app, it doesn't belong in `fude`.

Everything optional ships as an opt-in `with_*` layer, so the minimum-viable
app stays truly minimal.

## Now (shipping today)

Stable on macOS and Linux:

- Core shell: window + webview + `asset://localhost/` protocol + JSON IPC
- `with_fs_sandbox` — allow-listed filesystem access with canonicalized writes
- `with_settings` — scoped key/value store
- `with_shell_open` — OS-native "open URL / file" with an allow policy
- `with_dialogs` — native open/save/message dialogs
- `with_pty` — spawn + stream PTY-backed subprocesses with a tool allow-list
- `with_acp` — experimental Agent Client Protocol integration

## Next (~0.2.x)

The stuff that sharpens the "AI-native editor shell" pitch.

- **IPC streams.** Current IPC is request/reply. Add a first-class
  streaming channel (`window.__shell_stream(id)`) so LLM token streams,
  long tool runs, and progress events don't have to be shoe-horned into
  the event emitter. This is arguably *the* feature that justifies the
  "AI-native" framing.
- **`with_inspector`.** Toggleable devtools, defaulted on in debug builds.
- **`with_menu`.** Native application menu with IPC-driven handlers.
  Required for anything an editor user would recognize as "a real app".
- **ACP stabilization.** Pin a protocol version, document the wire format,
  and bring it out of the experimental tag. Semver guarantees for the
  stable subset.
- **Windows as a first-class target.** Add to CI, fix any platform gaps,
  document quirks. Today it probably *works*, but we don't verify.

## Later (toward 1.0)

Desktop-app table stakes + the guardrails a 1.0 deserves.

- **`with_single_instance`** — suppress a second launch and forward its
  args to the existing process.
- **`with_deep_links`** — register a `fude://`-style URL scheme.
- **Distribution docs.** Step-by-step for code-signing and notarization on
  macOS, signing on Windows, and the minimum `Cargo.toml` profile for
  shippable binaries.
- **API-surface lock.** Gate the CI on a tool like `cargo-public-api` so
  new `pub` items can't land without an intentional review.
- **MSRV policy.** Write down the support window (e.g. "latest two stable
  Rust releases") so downstreams know what to expect.
- **1.0 criteria.** A concrete checklist: every `with_*` layer documented,
  IPC streams stable, Windows verified, surface locked, ACP out of
  experimental. Ship 1.0 when all boxes are ticked — not before.

## Non-goals

Things `fude` will deliberately *not* grow into:

- **A plugin marketplace.** That's Tauri's territory; `fude` stays a
  library you vendor, not a platform you extend through third-party
  plugins.
- **A UI widget set.** The frontend is the app's problem. `fude` provides
  the bridge, not the buttons.
- **Multi-window / tab abstractions.** If your editor needs tabs, that's a
  frontend concern. Multiple OS windows are out of scope for the core.
- **A custom error type.** Handlers return `Result<_, String>` and will
  keep doing so. No `thiserror`, no enum explosion.
- **Non-web frontends.** Webview only. No native-widget bridge.

## How to propose changes

- **Small things** (typos, missing docs, obvious bugs): open a PR directly.
- **New `with_*` layers or scope changes**: open a
  [Discussion](https://github.com/r-hashi01/Fude/discussions) first so we
  can agree on whether it belongs *in* `fude` or *on top of* it.
- **Experience reports** ("I tried to build X and hit Y"): also
  Discussions — they're the highest-signal input for what to work on next.
