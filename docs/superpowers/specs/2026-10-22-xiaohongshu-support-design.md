# Xiaohongshu live-platform support design

Date: 2026-10-22
Branch: `feature/xiaohongshu-support`
Baseline: `9cd17311fa32575fa68f7e79feafdb165c13edd3`

## Goal

Add Xiaohongshu as a maintainable fifth DTV platform. A saved creator is identified by a stable user identity, while every playback resolves the creator's current dynamic live room. The implementation must reuse DTV's navigation, follows, folders, sorting, player, import/export and LAN-sync flows without ever placing Xiaohongshu credentials in browser storage, exports, sync payloads, URLs, logs, or Git.

## Evidence and constraints

### DTV capability matrix at baseline

| Capability | Douyu | Huya | Douyin | Bilibili | Xiaohongshu design |
|---|---|---|---|---|---|
| Platform page/navigation | categories + rooms | categories + rooms | categories + rooms | categories + rooms | real saved-creators page; no fabricated recommendations |
| Creator lookup | name/room | name/room | numeric room | name/room | UID, profile URL, share URL, exact redId; keyword results require user selection |
| Stable follows | global follow store | global | global | global | same store, keyed by stable user ID |
| Folders/drag/filter/sort | global | global | global | global | extend exhaustive platform lists and reuse global behavior |
| Status refresh | startup once + manual | same | same | same | same shared refresh plus XHS-only optional scheduler with backoff |
| Playback | xgplayer/FLV | xgplayer/FLV | xgplayer/FLV | xgplayer/FLV/HLS | resolve current room then real FLV/HLS; reuse session sequencing |
| Quality | mapped 3 labels | mapped 3 labels | mapped 3 labels | mapped 3 labels | expose only variants returned by the API |
| Lines | CDN selector | CDN selector | none | none | expose only distinct returned endpoints |
| Danmaku | WebSocket | WebSocket | WebSocket | WebSocket | P2 only after a real-device protocol validation |
| Player UI | global | global | global | global | reuse all compatible controls |
| Export/import/sync | follows/settings | follows/settings | follows/settings | follows/settings, including legacy Bilibili cookie export | XHS follow metadata only; never secret material |

The current follow and player code contains exhaustive four-platform branches. Every such branch must be updated deliberately; unknown values must not silently fall back to Bilibili or Douyu.

### Xiaohongshu research

- biliLive-tools PR #379 separates a creator identity (`redId`/user identity) from each live session's `roomId`; automatic monitoring requires a user Cookie.
- Its current StreamGet code signs creator search requests with `a1`, while authenticated search requires both `a1` and `web_session`.
- Its public-profile HTML parsing and direct `live-source-play.xhscdn.com/live/{roomId}` construction are not reliable enough today: an anonymous profile probe redirected to login, the unsigned room endpoint rejected the request, and a syntactically valid stale stream URL did not play.
- Current open-source implementations use signed `user/otherinfo`, `search/usersearch`, `user/me`, and live-room endpoints. Responses must be parsed defensively because fields and endpoint variants change.
- Multiple same-name results make “pick index 0” unsafe. Exact redId or stable-ID matches may be selected automatically; keyword searches must present choices.
- No reliable, permissively licensed, real-device-verified Xiaohongshu chat implementation was found. Catbus documents a celestial-token WebSocket flow but explicitly marks live listening as not real-device verified. P2 danmaku therefore remains an evidence-gated experiment.

### Licensing

- DTV: MIT.
- `renmu123/biliLive-tools`: GPL-3.0 at the repository root; `packages/StreamGet`: MIT; `packages/XHSRecorder`: LGPL. Only behavior and interfaces are studied; LGPL/GPL implementation is not copied.
- `renmu123/xhshow-js`: MIT. Its signing behavior may be adapted with attribution if used.
- Catbus: MIT, used as secondary protocol evidence. No unverified live-chat implementation is copied.

## Architecture

### Trust boundary

All credential handling and signed requests live in the Rust/Tauri backend. The React application receives only credential status, public creator metadata, typed error codes and playback variants. The raw Cookie is accepted only by a `set/test` command and is never returned to the frontend.

Use the operating-system credential store through Rust's `keyring` crate (`apple-native` on macOS, the corresponding native providers on Windows/Linux). If secure persistence is unavailable, the save command returns a typed storage error; a separate explicit session-only command may keep the Cookie in process memory. There is no silent plaintext fallback.

The credential service exposes:

- status: absent, session-only, securely stored, or invalid/expired;
- set and test without echoing the secret;
- replace;
- clear;
- connection test through an authenticated “current user” endpoint.

Cookie validation parses names without logging values. `a1` and `web_session` are required for the complete feature set. Errors distinguish malformed input, missing fields, authentication expiry, risk control, timeout/network, upstream-schema failure and secure-storage failure.

### Backend platform module

`src-tauri/src/platforms/xiaohongshu/` owns:

- input parsing for stable UID, profile URL, short/share URL and redId;
- redirect resolution restricted to known Xiaohongshu hosts;
- request signing and authenticated HTTP calls;
- exact creator resolution and keyword search results;
- current-room/status resolution;
- playback-variant extraction;
- a typed public error contract with safe user messages.

The client applies conservative rate limits, bounded timeouts, low concurrency and jittered exponential backoff for risk-control/transient failures. HTTP error bodies, signed headers and URLs containing tokens are never logged.

### Stable identity and persistence

The generic follow ID for an XHS creator is the stable Xiaohongshu user ID. Persisted public data is:

```text
platform = XIAOHONGSHU
id / stableUserId
redId?                human-facing Xiaohongshu ID
nickname
avatarUrl
currentRoomId?        mutable session data
liveStatus            LIVE | OFFLINE | UNKNOWN
roomTitle?
```

Deduplication uses `platform + stableUserId`, never `roomId`, display name, or the first search result. A share link can seed a dynamic `roomId`, but the creator must still resolve to a stable ID before being saved.

### Resolution and playback flow

```text
user input / saved stable ID
  -> resolve creator (signed lookup or explicit share metadata)
  -> query current live-room info
  -> LIVE: obtain current roomId and response-provided streams
  -> proxy FLV/HLS with Xiaohongshu Referer/Origin headers
  -> existing xgplayer session
```

Only use stream URLs returned by a successful current-room response. A constructed CDN URL may exist only as an explicitly marked fallback after a successful room response and a reachability probe; a URL shape alone is never proof of playback.

The backend returns distinct `Offline`, `AuthRequired`, `RiskControl`, `Network`, `Upstream`, and `NoPlayableStream` outcomes. The frontend must preserve UNKNOWN on query failures and must not report those failures as offline. Player reload performs the whole stable-ID-to-current-stream resolution again. Existing player request sequencing prevents stale requests from replacing the active session.

`flv_relay` adds host-scoped Xiaohongshu headers without accepting credential material in the browser-facing proxy URL. If an upstream stream requires a Cookie, the relay obtains it from the credential service internally.

### Frontend integration

- Add `xiaohongshu` / `XIAOHONGSHU` to every exhaustive platform type and explicit branch.
- Add navigation, icon, route, platform page and unified search/add experience.
- The XHS page shows saved creators and a settings card when no reliable recommendation feed is available.
- Cookie controls use a password input, momentary reveal, test, save/replace, clear, secure/session storage choice, state badge and browser-copy instructions. The value is held only in component state until submission and then cleared.
- Search accepts direct identifiers and links. Keyword results render selectable creator cards with stable ID/redId details.
- FollowProvider, FollowsList, Navbar and PlayerOverlay reuse all shared follow/folder/sort/filter behavior.
- Player UI shows only real quality/line variants. Global controls remain unchanged.

### Polling

The shared startup/manual refresh behavior remains unchanged for the original four platforms. XHS may have an opt-in scheduler, disabled by default, with a five-minute default interval, one-minute minimum, concurrency two, per-creator backoff and jitter. The setting is nonsecret and can participate in ordinary config export.

### Serialization and migration

Existing follow records remain compatible. New optional XHS fields are tolerated by old-record parsing. Import and LAN merge validate `XIAOHONGSHU` explicitly and deduplicate with the stable follow ID. Export and sync use allowlisted public fields; credential keys and raw request state are absent by construction.

### P2 decisions

- Danmaku: do not ship until the WebSocket handshake and reconnect behavior are validated against a real live room with a user-provided Cookie.
- Quality/lines: ship only response-derived distinct streams.
- Recommendations/categories: ship only after a stable, low-risk endpoint is verified. Otherwise the platform page is follows-first.

## Verification strategy

Rust unit tests cover Cookie parsing/redaction, input parsing, safe redirects, exact-result selection, error mapping, dynamic-room replacement, stream extraction and serialization exclusion. Frontend tests cover platform mappings, follow deduplication, refresh failure semantics and secret-free export/sync. Fixture-based integration tests cover API schemas without real credentials.

Evidence is reported separately as code/type checks, unit tests, fixture tests, real authenticated API tests, and real macOS audio/video playback. A missing personal Cookie or active live creator is reported as unverified, never passed. Full `.app`/`.dmg` packaging additionally requires a complete Xcode installation; this machine currently exposes Command Line Tools only.
