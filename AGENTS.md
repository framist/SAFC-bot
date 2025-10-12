# Repository Guidelines

## Project Structure & Module Organization
- `src/main.rs` drives the Telegram bot; `src/bin/web.rs` exposes the Actix-Web API used by the Next.js frontend.
- Shared logic lives in `src/lib.rs` with supporting modules such as `src/db.rs`, `src/msg.rs`, and `src/sec.rs`; keep additions scoped by topic.
- `assets/` holds bundled static data, `script/` contains Python crawlers and data helpers, and `web/` tracks the safc-web submodule for UI work.
- Documentation lives in `doc/`; database snapshots like `db.sqlite` should be copied locally before experiments rather than edited in place.

## Build, Test, and Development Commands
- `cargo run --bin safc_bot` starts the bot (requires `TELOXIDE_TOKEN`, `TELOXIDE_PROXY`, `SAFC_DB_PATH`).
- `cargo run --bin safc_web` exposes the HTTP API used by the static site; pair with `npm run dev` inside `web/` after `npm install`.
- `cargo build --release` produces optimized binaries for deployment; keep release artifacts out of version control.
- `cargo fmt`, `cargo +nightly fmt --all -- --check`, and `cargo clippy --all-targets --all-features` mirror CI expectations.

## Coding Style & Naming Conventions
- Use Rust’s default 4-space indentation; stick to `snake_case` for modules/functions and `CamelCase` for types and enums.
- Centralize reusable logic in `lib.rs` or dedicated submodules; avoid sprawling `main.rs`.
- Prefer structured logging via `log::info!`/`warn!` instead of `println!`; keep messages concise and actionable.

## Testing Guidelines
- Add unit tests with `#[cfg(test)]` blocks beside new code or create integration suites under `tests/`.
- Run `cargo test --all` before every PR; extend coverage when touching database or security-critical paths.
- Include lightweight fixtures or doc tests to lock down schema assumptions when altering `src/db.rs`.

## Commit & Pull Request Guidelines
- Follow the repository’s short, imperative commit style (`ci update`, `db update`); squash noisy WIP commits before review.
- Target PRs at `master`, provide a clear summary, link related issues or TODO items, and note which surfaces (`bot`, `web`, `scripts`) are affected.
- Attach screenshots or terminal transcripts when showcasing bot flows or API output changes.

## Security & Configuration Tips
- Store secrets in untracked `.env` files; never commit real tokens, proxies, or production SQLite dumps.
- When sharing repro steps, reference sanitized sample env (`TELOXIDE_TOKEN=123...XYZ`) and local database paths (`SAFC_DB_PATH=./tmp/safc.db`).

## Agent Interaction Notes
- 面向与你的用户的对话时，优先使用中文与用户沟通。
