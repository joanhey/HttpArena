# Webman

Webman is a high-performance service framework built on Workerman, integrating HTTP, WebSocket, TCP, UDP, and other modules. Through advanced technologies such as resident memory, coroutines, and connection pools, Webman not only breaks through the performance bottlenecks of traditional PHP but also greatly expands its application scenarios.

In addition, Webman provides a powerful plugin mechanism that enables developers to quickly integrate and reuse functional modules developed by other developers. Whether building websites, developing HTTP APIs, implementing instant messaging, building IoT systems, or developing games, TCP/UDP services, Unix Socket services, and more, Webman handles all of these effortlessly with outstanding performance and flexibility.

https://github.com/walkor/webman
https://webman.workerman.net/doc/en/

https://github.com/walkor/workerman
https://manual.workerman.net/doc/en/
https://www.workerman.net/


## Stack

- **Language:** PHP
- **Engine:** Workerman (event)


## Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/pipeline` | GET | Returns `ok` (plain text) |
| `/baseline11` | GET | Sums query parameter values |
| `/baseline11` | POST | Sums query parameters + request body |
| `/json` | GET | Processes 50-item dataset, serializes JSON |
| `/db` | GET | SQLite range query with JSON response |
| `/upload` | POST | Receives 1 MB body, returns byte count |
| `/static/{filename}` | GET | Serves preloaded static files |

## Notes

