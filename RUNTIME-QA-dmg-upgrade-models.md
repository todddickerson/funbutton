# Runtime QA — DMG-upgrade model migration (2026-10-11, branch `test/dmg-upgrade-model-proof`)

**Question:** when a user drags a new `FunButton.app` over the old one, do they
re-download the ~1.1 GB cleanup model (or the 85 MB whisper model)?

**Verdict: NO — the upgrade path is already correct.** No code fix needed. This
doc is the proof, and the branch adds a unit test that pins the invariants so a
future change can't silently break them.

## Why it's safe (code audit, with line references)

1. **The model store never lives in the bundle.** `models::models_dir()`
   (`models.rs:129`) is `HOME/Library/Application Support/ai.funbutton.desktop/models`
   — derived from `HOME` + the `APP_IDENTIFIER` constant only. Replacing the
   `.app` cannot touch it.
2. **Startup loads engines from the store with a cheap check, and never
   downloads on its own.** `lib.rs` engine init goes through
   `resolve_model_path` (`lib.rs:303`) → `models::is_installed` (`models.rs:157`),
   which is *exists + exact byte length* — no SHA re-hash at launch (a full-file
   SHA of 1.1 GB would be a slow-launch bug; hashing happens once at
   download/migration time, and a finalized file only ever exists after its hash
   matched). There is **no unconditional `models::download` call at startup** —
   downloads run only from the model-manager/onboarding commands and the
   env-gated `FUNBUTTON_SELFTEST_DOWNLOAD` hook (`lib.rs:1116`).
3. **Migration is a no-op when models are installed.** `migrate_from_bundle`
   (`models.rs:619`) `continue`s on `is_installed` before scanning any bundle
   root.
4. **Onboarding doesn't re-trigger.** `first_run = !settings_path().exists()`
   (`lib.rs:857`) and `settings_path()` is `~/.funbutton/settings.json`
   (`lib.rs:766`) — also bundle-independent, so `onboarded` survives the swap.

## Empirical proof

- **v0.1.8 release QA already performed the exact scenario** (see
  `RUNTIME-QA-v0.1.8.md` §Real install QA): replaced `/Applications/FunButton.app`
  with a fresh build while the Application Support store was intact → installed
  app booted, whisper + llama-server came up from the store, **no first-run
  download was triggered**.
- **Reproduced 2026-10-11** with a binary at a *different path entirely*
  (`target/release/funbutton` in a worktree — the strongest form of
  "the .app moved"): launch log shows whisper loaded from the store (Metal,
  8.8 s), llama-server up on the store's qwen gguf, full dictation pipeline ran
  (`selftest: paste suppressed; cleaned = "Refactor the auth middleware and open
  a pull request."`), and `grep -iE "download|migrat"` over the launch log shows
  **zero model-download/migration activity** — only the 6-model manifest
  *metadata* fetch (a few KB of JSON).

## The invariants, pinned by test

`models::tests::store_is_bundle_independent_so_dmg_upgrades_keep_models`
(models.rs) asserts:

- `APP_IDENTIFIER == tauri.conf.json "identifier"` — renaming the bundle id
  would orphan every user's store (fresh path ⇒ full re-download), so drift now
  fails the suite.
- `models_dir()` is exactly the HOME-scoped App Support path and is **not**
  under the executable's directory.
- `is_installed` is a pure function of store contents (exact size → installed;
  truncated → not installed), with no bundle involvement.

Suite result on this branch: `cargo test --release --lib` → **81 passed /
0 failed / 7 ignored** (80 + the new test). `cargo fmt --check` + `cargo clippy
--release --all-targets -- -D warnings` → clean.

## The one path that DOES re-download (inherent, documented)

Upgrading from **v0.1.6 or older** (models were inside the `.app`): Finder
deletes the old bundle — and the models with it — before the new app's first
run, so `migrate_from_bundle` finds nothing and the download flow takes over.
That data loss happens outside our process and is unrecoverable by design;
migration still rescues any copy that *is* reachable (side-by-side install, dev
vendor tree, not-yet-emptied old `.app`).

## Release-discipline constraint (how to not break this later)

`is_installed` keys on **filename + exact size** from the manifest. Never change
`filename`, `size_bytes`, or `sha256` of an existing model id in
`models_manifest.json` — that would flip installed models to "missing" on every
machine (the live manifest is fetched at startup) and trigger a fleet-wide
re-download. Add a **new id** instead.
