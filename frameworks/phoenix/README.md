# Phoenix — Elixir Web Framework

[Phoenix](https://github.com/phoenixframework/phoenix) is the most popular Elixir web framework, built on the BEAM VM (Erlang's runtime). It leverages OTP's lightweight processes for massive concurrency and fault tolerance.

## Stack

- **Phoenix 1.7** on Cowboy HTTP server
- **BEAM VM** (Erlang/OTP 27) — preemptive scheduling, millions of lightweight processes
- **Jason** for JSON encoding/decoding
- **Exqlite** for SQLite access
- Pre-computed JSON + gzip caches via `:persistent_term` (global read-optimized storage)
- Mix release for production deployment

## Why Phoenix?

Phoenix is THE Elixir web framework — ~23k GitHub stars, used in production by companies like Discord (originally), Bleacher Report, PepsiCo. The BEAM VM's concurrency model (lightweight processes with preemptive scheduling) is fundamentally different from thread-based or async/await approaches. Interesting to see how it compares with gleam-mist (also BEAM) and traditional frameworks.
