# nginx

Nginx with a custom C handler module (`ngx_http_httparena_module`) compiled with `-O3 -march=native`. Supports HTTP/2 and HTTP/3 via quictls.

## Stack

- **Language:** C
- **Engine:** nginx 1.26.2
- **TLS:** quictls (OpenSSL fork for QUIC)
- **Build:** Debian bookworm, compiles nginx + quictls from source

## Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/pipeline` | GET | Returns `ok` (plain text) |
| `/baseline11` | GET | Sums query parameter values |
| `/baseline11` | POST | Sums query parameters + request body |
| `/baseline2` | GET | Sums query parameter values (HTTP/2 variant) |
| `/static/{filename}` | GET | Serves static files with MIME types |

## Notes

- Custom C module using cJSON for JSON serialization
- Worker processes auto-configured to CPU count
- 65536 worker connections per process
- HTTP/1.1, HTTP/2, and HTTP/3 support
- Gzip compression at server level
