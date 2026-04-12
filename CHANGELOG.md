# Changelog

Notable changes to test profiles, scoring, and validation.

## 2026-04-12

### Noisy test — removed

The `noisy` (resilience) test profile has been removed entirely. It previously mixed valid baseline requests with malformed noise (bad paths, bad `Content-Length`, raw binary, bare CR, obs-fold, null bytes) and scored only 2xx responses. The profile was reference-only (not scored), and the insight it provided — which frameworks gracefully reject garbage traffic — is already exercised implicitly by the `baseline` test with realistic request shapes.

**Removed:**
- `noisy` profile from `benchmark.sh`, `benchmark-lite.sh`, and the Windows variants (profile entry, `PROFILE_ORDER`, readiness-check branch, load-gen dispatch branch)
- Resilience block from `validate.sh` / `validate-windows.sh` (bad method + post-noise checks)
- `noisy` from `inner/benchmark-{h1,test,per-test}.sh` profile lists
- Test-profile documentation at `docs/test-profiles/h1/isolated/noisy/`
- Shortcode references in `leaderboard-h1-workload.html`, `leaderboard-h1-isolated.html`, `leaderboard-composite.html` (including the `lb-row-noisy` CSS class + JS branches)
- Landing page card and references in scoring/composite, running-locally/configuration, and add-framework/meta-json docs
- Result directories (`results/noisy/`), site data files (`site/data/noisy-{512,4096,16384}.json`), and `noisy-*` keys from `site/data/rounds/2.json`

The `requests/noise-*.raw` files (bad headers, binary, bare CR, etc.) remain on disk as a reference for anyone who wants to exercise resilience paths manually.

### JSON Compressed profile — added to the website

The `json-comp` profile was running in scripts but wasn't rendered anywhere in the site. Now:

- **Leaderboard shortcodes**: `leaderboard-h1-isolated.html` has a new dict entry (JSON Compressed, conns 512/4096/16384); `leaderboard-composite.html` scores it alongside the plain `json` profile (scored, not engine-scored, same weight pattern).
- **Dedicated docs page**: `docs/test-profiles/h1/isolated/json-compressed/` with `_index.md`, `implementation.md` (endpoint `/json/{count}?m={multiplier}`, counts × multiplier pairs, compression rules, parameters), and `validation.md` (the three `validate.sh` checks: `Content-Encoding` present with `Accept-Encoding`, body correctness across `(12,9) / (31,4) / (50,1)`, no `Content-Encoding` without `Accept-Encoding`).
- **Landing page card**: new "JSON Compressed" card in `content/_index.md` next to JSON Processing, pointing at `/json/{count}?m=N`.
- **Docs index**: new card in `docs/test-profiles/h1/isolated/_index.md`.
- **Scoring table**: added to `docs/scoring/composite-score.md` as a scored H/1.1 Isolated profile.
- **Running-locally config** and **add-framework/meta.json** docs updated with the new profile row.

### `json-compressed` load-generator dispatch branch added

`scripts/benchmark.sh` previously declared `[json-comp]="1|0|0-31,64-95|512,4096,16384|json-compressed"` but had **no matching `elif [ "$endpoint" = "json-compressed" ]` branch** in the load-gen dispatch. Runs fell through to the default `else` clause (three-raw baseline rotation with no `Accept-Encoding`), so all prior `results/json-comp/*` numbers were indistinguishable from `baseline` — same rps, same ~300 MB/s, same 2-byte "55" responses.

Fixed by adding the branch (and mirroring into `benchmark-lite.sh` / `benchmark-lite-windows.sh`):

```bash
elif [ "$endpoint" = "json-compressed" ]; then
    gc_args=("http://localhost:$PORT"
        --raw "$REQUESTS_DIR/json-gzip-{1,5,10,15,25,40,50}.raw"
        -c "$CONNS" -t "$THREADS" -d "$DURATION" -p "$pipeline" -r 25)
```

The `json-gzip-*.raw` files (which already existed in `requests/`) contain the same 7 `(count, m)` pairs as the plain `json-*.raw` variants plus an `Accept-Encoding: gzip, br` header. Post-fix, gcannon reports `Templates: 7` and response bodies carry `Content-Encoding: gzip` or `br` depending on what the framework's compression path produces.

The stale "looks like baseline" `json-comp` result files under `results/json-comp/` and `site/data/json-comp-*.json` were deleted so the next `--save` run produces honest measurements.

### `json-comp` connection counts

`json-comp` moved from `512, 4096` to `512, 4096, 16384` — same pattern as `baseline` and `echo-ws` — to stress the compression path under extreme concurrent-connection pressure where middleware queuing shows up clearly. The 16384c run surfaces differences between frameworks that keep compression state per connection vs. those that allocate per request.

Updated in `scripts/benchmark.sh`, `site/layouts/shortcodes/leaderboard-h1-isolated.html`, `site/layouts/shortcodes/leaderboard-composite.html`, `site/content/docs/running-locally/configuration.md`, and `site/content/docs/test-profiles/h1/isolated/json-compressed/implementation.md`.

### JSON over TLS profile — added (`json-tls`)

New H/1.1 Isolated profile that runs the same `/json/{count}?m=N` workload as the plain `json` profile but transports it over **HTTP/1.1 + TLS** on a dedicated port. Measures how much of a framework's plaintext JSON throughput survives TLS record framing, symmetric cipher work, and ALPN negotiation. No compression — clients send no `Accept-Encoding` so this is pure TLS overhead on top of serialization.

**Shape**:

| Parameter | Value |
|-----------|-------|
| Endpoint | `GET /json/{count}?m={multiplier}` |
| Transport | HTTP/1.1 over TLS |
| Port | **8081** (distinct from 8080 plaintext and 8443 H2/H3) |
| ALPN | `http/1.1` only (wrk speaks HTTP/1.1 only) |
| Load generator | **wrk** + `requests/json-tls-rotate.lua` (gcannon has no TLS support) |
| Count × multiplier pairs | `(1,3) (5,7) (10,2) (15,5) (25,4) (40,8) (50,6)` (same 7 pairs as the plain `json` profile) |
| Connections | 4,096 |
| Pipeline / req-per-conn | 1 / 0 (persistent keep-alive) |
| CPU pinning | `0-31,64-95` |
| Certificates | reuses `certs/server.crt` + `certs/server.key` (same as `baseline-h2`) |

**Script plumbing**:

- `scripts/benchmark.sh`: new `H1TLS_PORT=8081`, `[json-tls]` entry in `PROFILES`, added to `PROFILE_ORDER` after `json-comp`, readiness check `https://localhost:$H1TLS_PORT/json/1?m=1`, new wrk dispatch branch
- `scripts/validate.sh`: `H1TLS_PORT=8081`, `needs_h1tls` path that mounts `/certs` + publishes `-p 8081:8081`, new validation block with three checks:
  1. ALPN negotiates HTTP/1.1 (`--http1.1` curl reports `http_version = 1.1`)
  2. Body correctness across `(7,2) / (23,11) / (50,1)` — deliberately different from the `json-comp` pairs `(12,9) / (31,4) / (50,6)` so a framework can't trivially share validation state between profiles
  3. `Content-Type: application/json`
- `requests/json-tls-rotate.lua`: new wrk Lua that round-robins the 7 `(count, m)` pairs, no `Accept-Encoding` header

**Site plumbing**:

- New profile dict in `leaderboard-h1-isolated.html` (conns `4096`) and `leaderboard-composite.html` (scored, not engine-scored)
- Dedicated docs dir `site/content/docs/test-profiles/h1/isolated/json-tls/` with `_index.md`, `implementation.md` (endpoint spec, port, ALPN, certs, parameters table), `validation.md` (the three checks)
- Landing card in `content/_index.md`, new card in `h1/isolated/_index.md`, new row in `running-locally/configuration.md`, `add-framework/meta-json.md`, and `scoring/composite-score.md`

**Framework implementation requirement**:

Each framework that subscribes to `json-tls` must bind a second HTTPS listener on port **8081** with ALPN `http/1.1` (separate from any existing HTTP/2 listener on 8443). The `/json/{count}?m=N` handler itself is shared with the plain `json` profile — no new route needed, just the listener.

- **Pilot**: `aspnet-minimal` (added a second `Kestrel.ListenAnyIP(8081)` with `Protocols = HttpProtocols.Http1` + `UseHttps(…)` alongside the existing `:8443` H1+H2+H3 listener, and `json-tls` in `meta.json`)
- Other frameworks need a similar ~5-10 line addition and `"json-tls"` in their `meta.json` tests array to opt in

### JSON Processing docs — fixed `?m=` inconsistency

The `json-processing/implementation.md` page had a self-contradiction: the "How it works" section said `GET /json/{count}` and `total = price × quantity`, while the example URL and rule text referenced `?m=3` and `total = price * quantity * m`. The load generator has always sent `?m=N` with the 7 fixed multipliers. Docs now consistently describe `GET /json/{count}?m={multiplier}` and enumerate the `(count, m)` pairs `(1,3) (5,7) (10,2) (15,5) (25,4) (40,8) (50,6)` in the parameters table.

## 2026-04-10

### JSON test — variable item count and multiplier

The JSON endpoint changed from `GET /json` to `GET /json/{count}?m=N`, where `count` (1–50) controls how many items the server returns and `m` (integer, default 1) is a multiplier applied to the total field: `total = price * quantity * m`. Each benchmark template uses a different `m` value, making every response unique and preventing response caching. All dataset fields are now integers (no floats) to avoid culture-specific decimal formatting and floating-point rounding issues.

### Compression test — merged into JSON

The standalone `/compression` endpoint has been removed. Compression is now tested through the JSON endpoint by sending `Accept-Encoding: gzip, br` in the request headers. The compression middleware handles on-the-fly compression when the header is present. Two separate benchmark profiles use the same endpoint:
- **json** — no `Accept-Encoding`, measures pure serialization
- **json-compressed** — with `Accept-Encoding: gzip, br`, measures serialization + compression

This eliminates the need for pre-loaded dataset files (`dataset-large.json`, `dataset-{100,1000,1500,6000}.json`) and the separate `/compression/{count}` route.

### Upload test — variable payload size

The upload benchmark now rotates across four payload sizes: 500 KB, 2 MB, 10 MB, and 20 MB (using gcannon `-r 5`). Previously only a fixed 20 MB payload was sent. Validation tests all four sizes. No endpoint change — `POST /upload` still returns the byte count.

### Async DB test — variable limit

The async-db endpoint now accepts a `limit` query parameter: `GET /async-db?min=10&max=50&limit=N`. The benchmark rotates across limits 5, 10, 20, 35, and 50 (using gcannon `-r 25` to balance requests evenly). Validation uses different limits (7, 18, 33, 50) **and** different price ranges (`min`/`max`) per request to prevent hardcoded responses. The SQL `LIMIT` clause is now parameterized instead of hardcoded to 50.

### All data fields changed to integers

All numeric fields in the datasets and database are now integers — no floats or doubles anywhere. This eliminates floating-point rounding inconsistencies, locale-specific decimal formatting issues, and type mismatch errors with parameterized database queries.

- **dataset.json**: `price` (was float → int 1–500), `rating.score` (was float → int 1–50)
- **dataset-large.json**: same changes across 6,000 items
- **pgdb-seed.sql**: `price` and `rating_score` columns changed from `DOUBLE PRECISION` to `INTEGER`
- **JSON `total` field**: now `price * quantity * m` — pure integer multiplication, no rounding needed
- All frameworks updated: query parameters, DB readers, and model types changed from float/double to int/long

### TCP Fragmentation test — removed

The `tcp-frag` test profile has been removed. With loopback MTU now set to 1500 (realistic Ethernet) for all tests, every benchmark already exercises TCP segmentation under production-like conditions. The extreme MTU 69 stress test no longer adds meaningful signal.

### Assets-4 / Assets-16 tests — removed

The `assets-4` and `assets-16` workload profiles have been removed. These were mixed static/JSON/compression tests constrained to 4 and 16 CPUs respectively. The `static` and `json` isolated profiles already cover file serving and serialization independently, and the `api-4`/`api-16` profiles cover resource-constrained workloads.

### Static files — realistic file sizes

Regenerated all 20 static files with varied sizes typical of a modern web application. Files now have realistic size distribution — large bundles (vendor.js 300 KB, app.js 200 KB, components.css 200 KB) alongside small utilities (reset.css 8 KB, analytics.js 12 KB, logo.svg 15 KB). Content uses realistic repetition patterns for compression ratios matching real-world code.

| Category | Files | Size range |
|----------|-------|------------|
| CSS | 5 | 8–200 KB |
| JavaScript | 5 | 12–300 KB |
| HTML | 2 | 55–120 KB |
| Fonts | 2 | 18–22 KB |
| SVG | 2 | 15–70 KB |
| Images | 3 | 6–45 KB |
| JSON | 1 | 3 KB |

Total: ~842 KB original, ~219 KB brotli-compressed, ~99 KB binary.

### Static files — pre-compressed files on disk

All 15 text-based static files now ship with pre-compressed variants alongside the originals:

- `.gz` — gzip at maximum level (level 9)
- `.br` — brotli at maximum level (quality 11)

Compression ratios: gzip 64–93%, brotli 68–94%. These files allow frameworks that support pre-compressed file serving (e.g., Nginx `gzip_static`/`brotli_static`, ASP.NET `MapStaticAssets`) to serve compressed responses with **zero CPU overhead** — no on-the-fly compression needed.

Binary files (woff2, webp) do not have pre-compressed variants since they are already compressed formats.

### Static test — load generator changed to wrk

The H/1.1 static file test now uses **wrk** with a Lua rotation script instead of gcannon. wrk achieves higher throughput on large-response workloads (~20% more bandwidth than gcannon's io_uring buffer ring path), ensuring the load generator is not the bottleneck. The Lua script rotates across all 20 static file paths with `Accept-Encoding: br;q=1, gzip;q=0.8`.

### Loopback MTU set to 1500

All benchmark scripts now set the loopback interface MTU to 1500 (realistic Ethernet) before benchmarking and restore to 65536 on exit. This ensures TCP segmentation behavior matches real-world production networks.

### Static files — compression support

All static file requests now include `Accept-Encoding: br;q=1, gzip;q=0.8`. Compression is **optional** — frameworks that compress will benefit from reduced I/O, but there is no penalty for serving uncompressed.

- **Production**: must use framework's standard middleware or built-in handler. No handmade compression.
- **Tuned**: free to use any compression approach.
- **Engine**: pre-compressed files on disk allowed, must respect Accept-Encoding header presence/absence.

Validation updated: new compression verification step tests all 20 files with Accept-Encoding, verifies decompressed size matches original. PASS if correct, SKIP if server doesn't compress, FAIL if decompressed size is wrong.

### Sync DB test — removed

The `sync-db` test profile (SQLite range query over 100K rows) has been removed. The test was redundant with `json` (pure serialization) and `async-db` (real database with network I/O, connection pooling). At 8 MB, the entire database was cached in RAM regardless of mmap settings, making it essentially a JSON serialization test with constant SQLite overhead.

**Removed:**
- `sync-db` profile from benchmark scripts and validation
- `sync-db` from all 54 framework `meta.json` test arrays
- Database documentation (`test-profiles/h1/isolated/database/`)
- Sync DB tab from H/1.1 Isolated and Composite leaderboards
- `sync-db` from composite scoring formula
- `benchmark.db` volume mount from Docker containers
- Result data (`sync-db-1024.json`)

The `/db` endpoint code remains in framework source files but is no longer tested or scored.
