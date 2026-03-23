# Vapor

[Vapor](https://github.com/vapor/vapor) is the most popular server-side Swift web framework, built on top of [SwiftNIO](https://github.com/apple/swift-nio). It features async/await support, Codable routing, built-in response compression, and a rich middleware system.

## Implementation Details

- **Framework:** Vapor 4.x
- **Runtime:** Swift 6.2 on SwiftNIO
- **Compression:** Built-in `responseCompression` (gzip)
- **JSON:** Pre-computed and cached at startup
- **SQLite:** Direct C bindings via `sqlite3_*` API
- **Static files:** Pre-loaded into memory at startup
