# Chi

[Chi](https://github.com/go-chi/chi) is a lightweight, idiomatic and composable router for building Go HTTP services. It's built on top of Go's standard `net/http` package with zero external dependencies for routing.

## Key Features

- **100% compatible with net/http** — Chi router implements `http.Handler`, so it works with any `net/http` middleware or server
- **Lightweight** — no framework magic, just a router with middleware support
- **Context-based** — uses `context.Context` for request-scoped values
- **Middleware stack** — composable middleware with `Use()`, inline middleware, and sub-routers

## Why It's Interesting for HttpArena

Chi sits between raw `net/http` and full frameworks like Gin/Echo/Fiber. It adds routing and middleware composition but stays close to the standard library. The key comparison:

- **go-fasthttp** — raw fasthttp, custom HTTP implementation
- **Fiber** — framework on top of fasthttp
- **Gin** — framework with httprouter, custom Context
- **Echo** — framework with custom radix tree router
- **Chi** — thin router layer on stdlib `net/http`

Chi's approach means handlers are just `http.HandlerFunc` — no custom context, no framework lock-in. The performance question: how much does staying close to stdlib cost vs. custom abstractions?
