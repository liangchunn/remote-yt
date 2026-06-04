# AGENTS.md

## Project Shape
- Single Rust binary crate (`remote-yt`, edition 2024, toolchain pinned to Rust `1.96.0`) with entrypoint `src/main.rs`.
- The backend binds `0.0.0.0:8080`, serves `/api/*`, and falls back to static files from `ui/dist`.
- The UI is a separate Vite/React app under `ui/`; run all npm commands from `ui/` and use `npm`/`package-lock.json`, not pnpm/yarn.
- Vite dev proxies `/api` to `http://localhost:8080`, so local development usually needs `cargo run` at repo root plus `npm run dev` in `ui/`.

## Commands
- Rust check: `cargo check`
- Rust tests: `cargo test` (currently compiles and runs 0 tests)
- Rust format check: `cargo fmt -- --check`
- Rust clippy: `cargo clippy` currently exits successfully with warnings; `cargo clippy -- -D warnings` is not clean in the current tree.
- UI install/build/lint: `cd ui && npm install`, `cd ui && npm run build`, `cd ui && npm run lint`
- `npm run build` runs `tsc -b && vite build`; use it for UI typecheck + production bundle.
- `npm run lint` currently exits 0 with Fast Refresh warnings in `ui/src/components/ui/button.tsx` and `ui/src/components/ui/toggle.tsx`.
- `just build` is a Raspberry Pi deploy, not a local build: it uses `cross` for `aarch64-unknown-linux-gnu`, builds `ui/dist`, stops/starts `remote-yt.service` on `pi@pi.local`, and copies the binary and UI assets over SSH.

## Runtime Files And Services
- `config.toml` is required at runtime from the process cwd and is gitignored; `config.toml.example` shows the intended keys.
- `history.json` is created/read in the process cwd and is gitignored by the root `/*.json` rule.
- VLC is spawned per queued job with HTTP RPC on `127.0.0.1:8081` and password `abc`; player command APIs depend on that RPC being reachable.
- `src/yt_dlp.rs::download_file` uses the hard-coded path `/Users/liangchun/dev/ex/yt-dlp/yt-dlp.sh`, unlike most yt-dlp calls that use `config.yt_dlp_path`.

## Backend Notes
- Queue endpoints are in `src/main.rs`: `/api/queue_config`, `/api/queue_merged`, `/api/queue_split`, `/api/queue_file`, cancel/clear/reorder/history/player command routes.
- `QueueManager` in `src/queue.rs` owns the in-memory queue, current child process, cancellation tokens, and `History` writes.
- Config providers are flattened top-level TOML tables; `Video::get_track` picks the first provider whose key is contained in the URL host.
- `QueueMerged` and `QueueSplit` re-run yt-dlp when playback starts because queued media URLs can expire.

## UI Notes
- UI API calls live mostly in `ui/src/App.tsx` and `ui/src/lib/commands.ts`; queue polling is in `ui/src/components/queue.tsx` via React Query every 1s.
- TypeScript path alias `@/*` points to `ui/src/*` in both Vite and tsconfig.
- React Compiler is enabled through `babel-plugin-react-compiler` in `ui/vite.config.ts`; avoid adding memoization purely out of habit.
- `ui/src/components/ui/*` follows shadcn-style component conventions from `ui/components.json` (`new-york`, neutral base, lucide icons).
