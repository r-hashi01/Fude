<!--
Thanks for the PR!

For scope-changing PRs (new `with_*` layers, new public API, behaviour
changes), please link the Discussion where the scope was agreed. PRs
without a prior Discussion may be asked to start one before review.
-->

## Summary

<!-- What does this change, and why? One short paragraph is enough. -->

## Related

<!-- Link the issue or Discussion this resolves, if any. -->

- Closes #
- Discussion:

## Type of change

<!-- Conventional Commits prefix — this is what release-plz parses. -->

- [ ] `fix:` (bug fix, patch bump)
- [ ] `feat:` (new user-visible functionality, minor bump)
- [ ] `docs:` / `refactor:` / `test:` / `ci:` / `chore:` (no version bump)

## Checklist

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test --all-targets && cargo test --doc` passes
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` passes
- [ ] Commit messages use Conventional Commits
- [ ] No manual edits to `CHANGELOG.md` (release-plz owns it)
- [ ] If this adds a new `pub` item, the PR description says why it
      needs to be public
- [ ] If this changes sandbox / FS / PTY / ACP behaviour, the change is
      covered by a test and `docs/sandbox.md` still matches reality
