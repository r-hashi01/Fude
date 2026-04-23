# Security Policy

## Supported versions

`fude` is pre-1.0. Security fixes are applied to the **latest published
release only** — there is no backport policy for older `0.x` releases.

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |
| older   | :x:                |

## Reporting a vulnerability

**Please do not open a public GitHub issue for security reports.**

Use one of the following private channels:

1. **Preferred: GitHub private vulnerability reports.**
   [Open a private advisory](https://github.com/r-hashi01/Fude/security/advisories/new)
   on the repository. This keeps the report, the fix discussion, and any
   coordinated disclosure entirely within GitHub.

2. **Alternative: email.** Send to <r.hashimoto@dify.ai> with `fude
   security` in the subject line. Expect an acknowledgement within 72
   hours.

Useful information in a report:

- The affected version (or commit SHA).
- A minimal reproduction — e.g. a path that bypasses the sandbox
  allow-list, or a PTY spawn that escapes the trusted-install check.
- Your view of the impact.

## Threat model

The security invariants `fude` tries to uphold are documented in
[`docs/sandbox.md`](./docs/sandbox.md). In short:

- No write path may follow a symlink to a location outside the
  allow-list.
- Paths under system / credential directories are refused regardless of
  user consent.
- PTY spawning is restricted to allow-listed tool names resolved to a
  trusted install directory, with a scrubbed `PATH` passed to the child.

A report that demonstrates any of these invariants being broken is
treated as a security bug.

## Non-vulnerabilities

Things that are **not** security bugs for this project:

- A user explicitly granting access to a path and then regretting it.
  `fude`'s job is to require explicit consent, not to overrule it.
- Denial-of-service through a trusted binary the user chose to run via
  `with_pty`. The allow-list is about preventing a compromised frontend
  from substituting binaries, not about bounding runtime behaviour.
- Anything in example apps (`examples/`) that is not also reachable from
  the crate's public API.

When in doubt, report it privately and let us triage.
