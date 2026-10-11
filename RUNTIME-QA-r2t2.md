# Runtime QA — Confucius4-R2T2 experimental STT (2026-10-11, branch `feat/r2t2-experimental-stt`)

Real execution on the Mac Studio (macOS 26 / Darwin 24.6.0, M3 Ultra, arm64).
Every PASS has a command/log trail. Nothing simulated.

## What ships

- **Manifest schema v2**: multi-file model entries (`files[]`, each with its own
  pinned sha256/size) + a `runtime` field (`builtin` | `mlx-r2t2`).
  **Forward-safety**: `fetch_manifest` only accepts a live manifest whose
  `schema_version` matches the baked one, so every v0.1.8-and-earlier install
  rejects the v2 live manifest and keeps its baked v1 copy — verified by the
  existing schema-gate code path (models.rs), no behavior change for old apps.
- **R2T2 model entry** (`confucius4-r2t2`, STT role, opt-in, never default):
  12 files from `selcukkubur/Confucius4-R2T2-mlx-4bit` + `LICENSE-R2T2.txt`
  (the NetEase Youdao Model Use License Agreement, fetched from the upstream
  repo at a pinned sha256). All 13 hashes double-verified (HF API LFS oid /
  x-linked-etag vs local `shasum -a 256`). Total 1,619,073,723 B. The required
  derivative-work notice (license §4.1(a), reproduced on the HF card) is
  written as `NOTICE-R2T2.txt` beside the weights at install time.
  **Weights are never in the repo or the .app bundle.**
- **Self-contained python runtime** under
  `~/Library/Application Support/ai.funbutton.desktop/r2t2-runtime/`:
  uv 0.13.0 (tarball sha256-pinned), a uv-managed CPython 3.12
  (`only-managed` — survives Homebrew churn), and a fully `==`-pinned wheel
  set (mlx-audio 0.5.5 / mlx 0.32.2 / transformers 5.17.0 — the exact
  versions the 2026-09-21 spike proved on this hardware). Wheel cache cleaned
  after install: ~480 MB on disk. Nothing lands in `~/.local` or `~/.cache`.
- **Sidecar lifecycle** cloned from `embedded_llm`: PID registry +
  `SPAWN_GATE` + `begin_shutdown_and_reap()` wired into the same ordered
  `shutdown()` that already tears down llama-server — a sidecar either
  registers before the reap or never spawns.
- **Pipeline**: `SttSource::R2t2` leads the order only when the R2T2 model is
  the user's active STT pick AND its sidecar is up; embedded whisper stays
  warm as the first fallback rung (then cloud/BYOK). Hotwords = user
  dictionary + focused-window AX title tokens, deduped, capped at 24,
  **never logged or persisted** (server request logging disabled; Rust logs
  presence only).

## Why sidecar python (research, 2026-10-11)

mlx-audio-swift v0.1.5 loads Qwen3-ASR but exposes **no hotword input** (both
generate paths pass an empty context) and is unproven on these weights; Rust
MLX bindings would need an architecture port. Hotword biasing is the entire
point of this engine, so the proven python path (mlx-audio 0.5.5, hotwords
upstreamed + tested) runs as a managed sidecar. Decision + evidence logged in
PROGRESS.md 2026-10-11 12:25.

## Gates

| # | Check | Result |
|---|-------|--------|
| 1 | `cargo check` | PASS |
| 2 | `cargo fmt --check` | PASS |
| 3 | `cargo test --release --lib` | PASS — 85 passed / 0 failed / 9 ignored |
| 4 | `cargo clippy --release --all-targets -- -D warnings` | PASS (rc 0) |
| 5 | macOS-26 crash grep guard | PASS (comments only — hotkey/injection untouched) |
| 6 | Baked manifest v2 validity (per-file hashes, size sums, defaults stay builtin) | PASS (unit tests) |
| 7 | Multi-file install/delete semantics | PASS (unit test) |
| 8 | Hotword builder (dict-first, dedupe, cap, comma-safety, no fabrication) | PASS (unit test) |
| 9 | **LIVE multi-file download → real store** (production path, per-file SHA, license + notice) | PASS |
| 10 | **Provisioning from scratch on this machine** (uv → python → venv → transcribe) | PASS (×3: twice pre-build in /tmp, once in-app) |
| 11 | **In-app honest degradation: R2T2 picked but unavailable → whisper fallback, dictation still lands** | PASS |
| 12 | **In-app: pipeline transcribes via r2t2** (selftest through the hotkey code path, real .app bundle) | PASS |
| 13 | **Quit teardown: AppleEvent quit reaps sidecar + llama, 0 new .ips** | PASS |
| 14 | **Hotword effect live** (`r2t2_sidecar_live`: "cube control"→"kubectl") | PASS |
| 15 | Bench R2T2 leg via the app-provisioned runtime | PASS |

## Evidence

### 9 — live multi-file download through the production path (29.6 s)

`cargo test --release --lib multifile_download_live -- --ignored --nocapture`
(no FUNBUTTON_MODELS_DIR override → the real store):

```
INSTALLED at ~/Library/Application Support/ai.funbutton.desktop/models/confucius4-r2t2-mlx-4bit (1619073723 bytes)
ok. 1 passed … finished in 29.58s
```

All 13 files on disk at pinned sizes; `LICENSE-R2T2.txt` (11,071 B, the NetEase
Youdao Model Use License Agreement text) and the generated `NOTICE-R2T2.txt`
(§4.1(a) derivative notice) sit beside the weights.

### 10–12 — in-app full stack (release binary, then the real .app bundle)

Fresh machine state (no `r2t2-runtime` dir), `stt_model_id=confucius4-r2t2`
(exactly what Settings → Models "use" persists), `RUST_LOG=info
FUNBUTTON_SELFTEST=1 FUNBUTTON_SELFTEST_WAV=…`:

```
16:47:58 r2t2 runtime: downloading uv
16:47:58 embedded STT model loaded ("MTL0" backend, 76ms)      ← whisper warm as fallback
16:48:00 r2t2 runtime: installing python 3.12 (uv-managed)
16:48:03 r2t2 runtime: installing pinned wheels (mlx-audio 0.5.5)
16:48:03 r2t2 runtime: runtime ready                            ← full provisioning: ~7 s on this pipe
16:48:08 r2t2 sidecar ready at http://127.0.0.1:64263 (4647ms)
16:48:15 selftest: synthetic Down/Up sent
16:48:17 transcribed via r2t2
16:48:17 selftest: paste suppressed; cleaned = "Refactor the auth middleware and open a pull request."
```

Repeated from the **bundled `FunButton.app`** (built `cargo tauri build
--bundles app`, 40 MB, 0 gguf / 0 safetensors inside): sidecar ready in
**1,665 ms** warm, `transcribed via r2t2`, same exact cleaned phrase.

**Honest degradation (check 11):** with the model dir renamed away (same as
"not installed"), the identical launch logged no sidecar spawn, the pipeline
ran `transcribed via embedded-whisper`, and the dictation still produced the
exact phrase. Model dir restored afterwards.

### 13 — quit teardown

`osascript -e 'quit app "FunButton"'` on the running bundle
(app 67078, r2t2 sidecar 67090, llama-server 67089):

```
shutdown: starting ordered teardown
shutdown: teardown complete
→ app GONE · sidecar GONE · llama GONE · crash reports 0 → 0
```

(The AppleEvent quit path only works against the bundled app — a bare
`target/release/funbutton` binary isn't LaunchServices-registered and
`osascript` no-ops with rc 0. QA'd on the bundle for exactly that reason.)

### 14 — hotword effect, live sidecar (the feature's point)

`cargo test --release --lib r2t2_sidecar_live -- --ignored --nocapture`:

```
PLAIN: Open auth underscore middleware dot rs and fix the ClickFunnels webhook. Then run cube control get pods in the production namespace.
HOT:   Open auth middleware.rs and fix the ClickFunnels webhook. Then run kubectl get pods in the production namespace.
ok. 1 passed
```

Decoding is greedy (temperature 0) but the hotword list folds into the system
prompt, so **list order changes output** — the test pins the spike's proven
order (ClickFunnels, auth_middleware.rs, kubectl). An order permutation
produced "kube control" during QA; worth remembering when tuning hotword
assembly.

### 15 — bench through the app-provisioned runtime

`bench-stt.py` with the runtime venv
(`…/r2t2-runtime/venv/bin/python`) on the repo fixtures — R2T2 170–260 ms per
utterance (vs whisper-base 25–90 ms, both ≪ real-time); hotwords fix
`kubectl` and `auth middleware.rs` on code.wav. Full table in PR #16's bench
output; identical results from the spike venv and the app runtime.

## Disk/teardown footprint

- Model: 1.62 GB under `models/confucius4-r2t2-mlx-4bit/` (deletable in
  Settings → Models — `delete` removes the whole subdirectory).
- Runtime: **466 MB** under `r2t2-runtime/` (uv 43 MB + managed CPython
  70 MB + venv 367 MB; wheel cache cleaned post-install). Nothing in
  `~/.local` / `~/.cache`.

## Review-pass fixes (same branch, post-initial-QA)

- **Pick flip-flop race closed.** `sync_r2t2` previously only checked the
  *stored* handle, so R2T2→whisper→R2T2 during the minutes-long first
  provisioning could race two `uv` runs over the runtime dir, and an
  abandoned pick could leave a 1.5 GB model resident until quit. Now: a
  `r2t2_starting` flag admits one provision-and-spawn task at a time, the
  task re-checks the pick after provisioning, and `store_or_reap_r2t2`
  decides under the `r2t2` lock (`should_store_r2t2`: quit or pick-change →
  reap, never store). Pinned by `r2t2_store_decision_follows_the_live_pick`
  (sparse files at the manifest's exact pinned sizes, so `is_installed` is
  real without 1.6 GB).
- **Window-title hotwords actually arrive now.** `context_now()` at STT time
  nearly always missed the AX read (app name lands in µs, AX in ms) — title
  tokens were silently absent from the hotword list. New
  `DetectHandle::wait_context_up_to(250 ms)`, paid **only** on the
  R2T2-active + capture-on path; capture-off returns Disconnected-instantly
  (pinned by `wait_context_returns_fast_when_capture_is_off`), and the slow-AX
  pickup + cleanup-time caching is pinned by
  `wait_context_picks_up_a_slow_ax_read`. Debug log carries counts only.
- **Privacy hardening:** sidecar error bodies truncated to 160 chars before
  they reach the app log (a python exception string could embed hotword
  text); scratch WAVs moved from system tmp into `r2t2-runtime/tmp/`
  (stale ones swept at sidecar start — verified 0 leftovers after the live
  test).
- **UX:** Settings → Models now toasts each runtime-setup stage
  (`funbutton:r2t2-runtime`), so the one-time provisioning isn't a silent
  "starting". Desktop `tsc` + `vite build` clean.

Post-fix gates: `cargo test --release --lib` → **88 passed / 0 failed /
9 ignored**; `r2t2_sidecar_live` re-run green with the new scratch-dir
handling; fmt + clippy `-D warnings` rc 0.

## Not verified (honest gaps)

- Real-mic dictation with R2T2 active (same human TCC gate as every prior
  release QA; the selftest path substitutes the WAV *after* a real recorder
  run, so capture itself is exercised).
- Settings → Models UI click-through for the new entry (headless shell has no
  Accessibility; the `models_set_active` → `reload_stt` → `sync_r2t2` chain it
  invokes is the exact code path QA'd above via the persisted setting).
- Provisioning on a slow/flaky network (this pipe did ~500 MB of wheels in
  ~3 s; uv retries internally, and a provisioning failure surfaces as the
  engine-board "failed" state via `r2t2_error` — not silently).
- Streaming partials (stretch goal, intentionally not started).
