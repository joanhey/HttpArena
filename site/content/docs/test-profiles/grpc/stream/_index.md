---
title: Server Streaming (gRPC)
---

Measures gRPC **server-streaming** throughput in messages per second. Each call sends one request and the server emits N replies on the same HTTP/2 stream. Runs over both h2c (port 8080) and h2 TLS (port 8443).

{{< cards >}}
  {{< card link="implementation" title="Implementation Guidelines" subtitle="Endpoint specification, expected request/response format, and type-specific rules." icon="code" >}}
  {{< card link="validation" title="Validation" subtitle="All checks executed by the validation script for this test profile." icon="check-circle" >}}
{{< /cards >}}
