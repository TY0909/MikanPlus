# MikanPlus Agent Instructions

These instructions apply to the entire repository.

## Start Here

Before changing code, read:

- `docs/development.md` for the development workflow and quality gates
- `docs/architecture.md` for crate responsibilities and dependency rules

Keep the root `README.md` user-facing. Put implementation, architecture, build, and contributor details under `docs/`.

## Project Structure

MikanPlus is a Rust workspace:

- `crates/app`: executable, windows, menus, shortcuts, and service composition
- `crates/domain`: framework-independent domain models and navigation state
- `crates/downloader`: librqbit session and download lifecycle
- `crates/source`: HTTP access and HTML parsing
- `crates/storage`: persistence, cache, migrations, and platform paths
- `crates/ui`: GPUI Kit pages, components, actions, and themes

Crate names must not use a `mikan-` prefix. Keep the executable name `MikanPlus`.

Dependencies must point toward stable capability layers. In particular:

- `domain` must not depend on GPUI, networking, storage, or the download engine.
- Lower-level crates must not depend on `app`.
- `app` is the composition root; avoid putting reusable business logic there.
- Keep UI state ownership explicit and local to the view that owns its lifecycle.

## Rust Guidelines

- Prefer the shortest clear, idiomatic implementation.
- Prefer immutability, expressions, iterators, pattern matching, and early returns.
- Keep control flow flat and avoid unnecessary abstractions or helper layers.
- Use the standard library unless an existing dependency clearly simplifies the solution.
- Handle recoverable errors with `Result` or `Option`; do not silently discard meaningful failures.
- Do not add `unsafe` code. Workspace lints forbid it.
- Do not fix unrelated issues while completing a focused task.

## GPUI Kit Guidelines

- Use GPUI APIs only through `gpui-kit`.
- Do not add direct dependencies on `gpui`, `gpui-component`, or `gpui-base`.
- Call `gpui_kit::init(cx)` before creating component views.
- Use `gpui_kit::component::Root` as the first view layer of every window.
- Use stable domain-derived IDs for repeated UI elements.
- Use semantic theme values in rendering code.
- Background tasks must stop when their owning weak entity can no longer be upgraded.

## Behavior Compatibility

Do not change user-visible storage behavior unless the task explicitly requires it. This includes:

- application data and cache locations
- the default download directory
- download folder naming
- `state.json`, `dht.dat`, `torrents/session`, and `torrents/meta` layouts
- persisted JSON fields and their meaning

The subtitle-group download folder format is:

```text
<番剧名称> - <字幕组名称>
```

If a persistent format or location must change, add an idempotent migration and regression tests. Never overwrite a damaged state file with empty state; preserve or back it up first.

## Assets

The application AssetSource loads project assets first and falls back to `gpui_kit::assets::Assets`.

- Keep application-specific assets in `assets/`.
- Do not copy resources already provided by GPUI Kit into the repository.
- Add a local icon only when GPUI Kit does not provide it or the design intentionally overrides it.
- Preserve `assets/mikan-pic.png` and `assets/mikan_icon.ico`; they are application branding and packaging resources.

## Documentation

- Write root `README.md` for end users, not developers.
- Put build commands, crate boundaries, design decisions, and maintenance instructions in `docs/`.
- Update documentation when a change affects architecture, setup, resource ownership, or user-visible behavior.

## Validation

Run the narrowest relevant tests first, then complete the workspace quality gate:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- --deny warnings
cargo test --workspace --all-features --locked
```

For a release-affecting change, also run:

```bash
cargo build --release --locked -p app
```

If the system temporary directory is full, use the project-local directory:

```bash
TMPDIR=/home/ty/Projects/MikanPlus/target/tmp cargo test --workspace --all-features --locked
```

Do not claim a command passed unless it was run successfully. Network smoke tests may require access to `mikanani.me`; report environmental failures separately from source failures.

## Git Safety

- Preserve user changes and do not revert unrelated work.
- Do not create branches or commits unless explicitly requested.
- Keep commits focused and use concise imperative commit messages.
