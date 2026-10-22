# Xiaohongshu live-platform implementation plan

**Spec:** `docs/superpowers/specs/2026-10-22-xiaohongshu-support-design.md`

**Goal:** Deliver a secure, first-class Xiaohongshu platform on `feature/xiaohongshu-support`, preserving all original platforms and separating verified behavior from credential/live-condition gaps.

**Global constraints:** Work test-first; never persist, export, sync, print or commit a Cookie; never treat a network/auth/schema error as offline; never use a dynamic room ID as a stable follow ID; never invent quality, lines, recommendations or danmaku; keep original-platform refresh behavior unchanged.

## Task 1: Test harness and platform contract

**Files:** root/web manifests, `web/src/platforms/common/types.ts`, follow/search/player platform mappings, new focused tests.

1. Add minimal Rust and frontend test commands without changing runtime behavior.
2. Write failing tests for explicit Xiaohongshu platform mapping, non-Bilibili fallback behavior, and secret-free public follow serialization.
3. Add `XIAOHONGSHU`/`xiaohongshu` to exhaustive types and mappings.
4. Run focused tests, full frontend tests/typecheck/build, and Rust tests.
5. Commit the platform contract.

## Task 2: Secure credential service

**Files:** `src-tauri/Cargo.toml`, `src-tauri/src/platforms/xiaohongshu/{mod,credential,error}.rs`, `src-tauri/src/main.rs`, frontend command types, settings UI tests.

1. Write failing Rust tests for Cookie parsing, required fields, secret redaction, session storage and secure-store error behavior.
2. Implement a backend-only credential abstraction using native keyring plus explicit session-memory fallback.
3. Add status, set/test, replace and clear Tauri commands with typed errors and no secret-return path.
4. Write failing UI tests for password display, reveal, state badges and cleared component state; implement the settings card.
5. Verify source and generated artifacts contain no fixture secret; commit.

## Task 3: Stable creator resolution and search

**Files:** `src-tauri/src/platforms/xiaohongshu/{input,client,signing,types}.rs`, Tauri commands, `web/src/platforms/xiaohongshu/*`, search UI.

1. Write failing tests for UID/profile/share/redId parsing, host allowlisting, short-link redirect parsing, exact matching and ambiguous-name results.
2. Implement signed request primitives from permissively licensed protocol evidence with attribution.
3. Implement creator lookup/search and return stable public metadata only.
4. Add direct-add and selectable search-result UI.
5. Run fixtures and regression builds; commit.

## Task 4: Dynamic status refresh

**Files:** XHS backend room/status modules, `FollowProvider.tsx`, `FollowsList.tsx`, follow types/tests.

1. Write failing tests proving room IDs can change while stable IDs do not, and failures remain UNKNOWN.
2. Implement current-room resolution, typed LIVE/OFFLINE/UNKNOWN results and metadata updates.
3. Integrate manual/startup refresh with concurrency two.
4. Add opt-in XHS-only polling with interval validation, backoff and jitter if the endpoint fixtures support it.
5. Verify follow persistence, deduplication, folders and sorting; commit.

## Task 5: Playback and relay

**Files:** XHS stream parsing, `src-tauri/src/flv_relay.rs`, `src-tauri/src/proxy.rs`, `MainPlayer.tsx`, `PlayerPage.tsx`, player tests.

1. Write failing tests for response-derived variants, duplicate-line removal, missing streams, header scoping and safe error mapping.
2. Implement playback resolution from stable ID through current room to current stream.
3. Add Xiaohongshu-scoped relay headers and internal credential lookup only when required.
4. Extend player reload/cleanup/follow mappings while preserving session sequencing.
5. Verify FLV/HLS build paths and original-platform branches; commit.

## Task 6: Full navigation, follows, import/export and LAN sync

**Files:** Navbar, Sidebar, AppShell, platform page, configTransfer, lanSync, follow/player overlays and tests.

1. Write failing tests/fixtures for navigation, filtering, folder round-trip, import merge and secret exclusion.
2. Add the fifth-platform route, name/icon, follows-first page and explicit mappings.
3. Extend every exhaustive platform validation and merge path.
4. Convert XHS serialization to a public-field allowlist and verify Cookie-like fields cannot enter export/sync.
5. Run repository-wide hardcoded-platform audit and regression tests; commit.

## Task 7: P2 evidence-gated extensions and documentation

**Files:** XHS platform modules/UI, `docs/xiaohongshu.md`, attribution/notice as needed.

1. Probe current endpoints only with no credential or a credential entered in the local app; never request it in chat.
2. Implement only verified distinct quality/line variants and stable public metadata.
3. Keep danmaku and recommendations disabled unless real-device evidence satisfies the spec.
4. Document setup, supported inputs, security model, errors, limitations, licenses and maintenance notes.
5. Commit documentation and any verified P2 work.

## Task 8: Whole-branch verification, packaging and delivery

1. Run formatting, frontend tests/typecheck/build, Rust tests/check, secret scans and changed-file audit.
2. Run Tauri/macOS build; if full Xcode remains absent, record the exact blocker and provide reproducible commands without claiming an app bundle exists.
3. Perform a fresh whole-branch code review, fix Critical/Important findings with RED→GREEN tests, and re-run the full suite.
4. Push `feature/xiaohongshu-support` to the user's Fork. Do not open an upstream PR.
5. Produce the final capability matrix, commit list, usage instructions, evidence-separated test report, known gaps and license references.

## Review focus

- Credential exposure through logs, frontend state persistence, export, sync, proxy query strings or panic text.
- Any branch that defaults an unknown/XHS platform to another platform.
- Stable creator identity accidentally replaced by a dynamic room ID.
- Authentication/network/risk-control failures reported as offline.
- Unverified stream URL construction or duplicate quality/line entries.
- Regressions in the four original platforms and non-macOS build paths.
