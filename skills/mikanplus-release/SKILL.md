---
name: mikanplus-release
description: Prepare, validate, and commit MikanPlus versioned releases without implicitly tagging or pushing.
---

# MikanPlus Release

Use this skill for explicit requests to bump the MikanPlus version, prepare a release, review release packaging, or publish a release. It does not authorize creating tags, pushing branches, or publishing GitHub Releases unless the user explicitly asks for those actions.

## Repository release model

- The workspace version is the source of truth in `Cargo.toml` under `[workspace.package]`.
- `Cargo.lock` stores versions for the six local workspace packages and must stay synchronized.
- Local packaging defaults are duplicated in:
  - `packaging/linux/build-deb.sh`
  - `packaging/macos/bundle.sh`
  - `packaging/windows/installer.nsi`
- Each release has a user-facing announcement at `docs/releases/<tag>.md` (for example `docs/releases/v0.2.3.md`). CI passes this file to `gh release create --notes-file`, so its content becomes the GitHub Release body verbatim. Write it while preparing the version bump; never fix a published announcement by hand.
- `.github/workflows/release.yml` runs when a `v*` tag is pushed and currently publishes:
  - Windows x86_64 portable ZIP and NSIS installer
  - macOS arm64 DMG
  - Linux x86_64 tarball, `.deb`, and AppImage
- CI derives the artifact version from the tag and uses `docs/releases/<tag>.md` as the Release body. The packaging defaults still need to match the workspace version for local builds.

## Prepare a version bump

1. Read `docs/development.md` and `docs/architecture.md` before changing files.
2. Inspect `git status`, the current branch, upstream tracking, recent history, and the current workspace version. Preserve unrelated user changes.
3. Confirm the target version if the user did not provide one. Use SemVer and do not invent a major/minor/patch change from context alone.
4. Update the release version locations listed above. Do not globally replace dependency versions in `Cargo.lock`.
5. Write the release announcement at `docs/releases/<tag>.md`. Start from the previous release's file as a template when useful. Do not repeat a top-level `# MikanPlus <version>` heading; the Release page already shows the tag. This content is published verbatim.
6. Run `cargo check --workspace --all-targets` once to synchronize local workspace package entries in `Cargo.lock`, then verify the diff.
7. Run the release validation below. Keep the version bump uncommitted when the user says “prepare” or otherwise has not asked to commit it.

For a normal patch release, the release commit touches `Cargo.toml`, `Cargo.lock`, the three packaging defaults, and one new file under `docs/releases/`. A release commit should use the imperative message `chore: bump version to X.Y.Z`.

## Validation

For a release-affecting change, run the narrowest relevant checks and then:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- --deny warnings
cargo test --workspace --all-features --locked
cargo build --release --locked -p app
git diff --check
```

Do not claim a command passed unless it completed successfully. Download-engine integration tests may need permission to create local listener sockets; report sandbox failures separately and rerun only with the required approval. Network smoke-test failures are environmental unless the source assertions fail.

## Git and publishing boundaries

- Do not create a release tag or push `main` merely because a version was prepared.
- If the user explicitly asks to commit the version bump, review the exact release-commit diff first and commit it with `chore: bump version to X.Y.Z`.
- Do not hand-edit a published GitHub Release body. If an announcement needs a fix, update `docs/releases/<tag>.md` and refresh the body with `gh release edit <tag> --notes-file docs/releases/<tag>.md`, so the repository stays the source of truth.
- If the user asks to rewrite an unpushed release-related commit message, prefer `git commit --amend -m "..."` and preserve the commit content. Do not rewrite pushed history without explicit direction.
- If work must temporarily move between branches, stash only the named release files and restore them after switching. Verify the stash contents before applying it.
- To cancel an already-created but unpushed release commit, prefer a normal revert so branch history remains recoverable; use history rewriting only when the user explicitly requests it.
- Before any tag or push explicitly requested by the user, verify the working tree, branch, commit, target remote, tag name, and release artifacts.

## Release handoff

Before handing off, report:

- target version and files changed, including the release notes file;
- validation commands and their actual results;
- commit hash if committed;
- whether a tag was created;
- whether the GitHub Release body was taken from `docs/releases/<tag>.md`;
- whether `main` or another branch was pushed, and its remote status.
