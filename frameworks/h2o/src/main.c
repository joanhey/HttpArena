#define H2O_USE_LIBUV 0

#include <h2o.h>
#include <h2o/serverutil.h>
#include <math.h>
#include <netinet/tcp.h>
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <openssl/ssl.h>
#include <yajl/yajl_gen.h>
#include <yajl/yajl_tree.h>

static h2o_globalconf_t globalconf;
static SSL_CTX *ssl_ctx;
static char *json_response;
static size_t json_response_len;

/* Parse query string values and return their sum */
static int64_t sum_query_values(h2o_req_t *req)
{
    if (req->query_at == SIZE_MAX)
        return 0;
    int64_t sum = 0;
    const char *p = req->path.base + req->query_at + 1;
    const char *end = req->path.base + req->path.len;
    while (p < end) {
        const char *eq = memchr(p, '=', end - p);
        if (!eq) break;
        const char *v = eq + 1;
        const char *amp = memchr(v, '&', end - v);
        if (!amp) amp = end;
        char *ep;
        long long n = strtoll(v, &ep, 10);
        if (ep > v && ep <= amp) sum += n;
        p = amp < end ? amp + 1 : end;
    }
    return sum;
}

/* GET /pipeline — return "ok" */
static int on_pipeline(h2o_handler_t *h, h2o_req_t *req)
{
    (void)h;
    req->res.status = 200;
    req->res.reason = "OK";
    h2o_add_header(&req->pool, &req->res.headers, H2O_TOKEN_CONTENT_TYPE,
                   NULL, H2O_STRLIT("text/plain"));
    h2o_send_inline(req, H2O_STRLIT("ok"));
    return 0;
}

/* GET|POST /baseline11 — sum query params (+ body for POST) */
static int on_baseline11(h2o_handler_t *h, h2o_req_t *req)
{
    (void)h;
    int64_t sum = sum_query_values(req);
    if (h2o_memis(req->method.base, req->method.len, H2O_STRLIT("POST"))
        && req->entity.len > 0) {
        const char *p = req->entity.base;
        const char *end = p + req->entity.len;
        while (p < end && *p <= ' ') p++;
        char *ep;
        long long n = strtoll(p, &ep, 10);
        if (ep > p) sum += n;
    }
    char buf[32];
    int len = snprintf(buf, sizeof(buf), "%lld", (long long)sum);
    req->res.status = 200;
    req->res.reason = "OK";
    h2o_add_header(&req->pool, &req->res.headers, H2O_TOKEN_CONTENT_TYPE,
                   NULL, H2O_STRLIT("text/plain"));
    h2o_send_inline(req, buf, len);
    return 0;
}

/* GET /baseline2 — sum query params */
static int on_baseline2(h2o_handler_t *h, h2o_req_t *req)
{
    (void)h;
    int64_t sum = sum_query_values(req);
    char buf[32];
    int len = snprintf(buf, sizeof(buf), "%lld", (long long)sum);
    req->res.status = 200;
    req->res.reason = "OK";
    h2o_add_header(&req->pool, &req->res.headers, H2O_TOKEN_CONTENT_TYPE,
                   NULL, H2O_STRLIT("text/plain"));
    h2o_send_inline(req, buf, len);
    return 0;
}

/* GET /json — return pre-serialized JSON dataset */
static int on_json(h2o_handler_t *h, h2o_req_t *req)
{
    (void)h;
    if (!json_response) {
        h2o_send_error_500(req, "Error", "No dataset", 0);
        return 0;
    }
    h2o_generator_t gen;
    memset(&gen, 0, sizeof(gen));
    h2o_iovec_t body = h2o_iovec_init(json_response, json_response_len);
    req->res.status = 200;
    req->res.reason = "OK";
    req->res.content_length = json_response_len;
    h2o_add_header(&req->pool, &req->res.headers, H2O_TOKEN_CONTENT_TYPE,
                   NULL, H2O_STRLIT("application/json"));
    h2o_start_response(req, &gen);
    h2o_send(req, &body, 1, H2O_SEND_STATE_FINAL);
    return 0;
}

static void register_handler(h2o_hostconf_t *host, const char *path,
                              int (*fn)(h2o_handler_t *, h2o_req_t *))
{
    h2o_pathconf_t *pc = h2o_config_register_path(host, path, 0);
    h2o_handler_t *h = h2o_create_handler(pc, sizeof(*h));
    h->on_req = fn;
}

static void setup_host(h2o_hostconf_t *host)
{
    register_handler(host, "/pipeline", on_pipeline);
    register_handler(host, "/baseline11", on_baseline11);
    register_handler(host, "/baseline2", on_baseline2);
    register_handler(host, "/json", on_json);
}

/* Load dataset.json and pre-serialize the /json response with yajl */
static void load_dataset(void)
{
    const char *path = getenv("DATASET_PATH");
    if (!path) path = "/data/dataset.json";
    FILE *f = fopen(path, "r");
    if (!f) return;
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *data = malloc(sz + 1);
    if (!data) { fclose(f); return; }
    fread(data, 1, sz, f);
    data[sz] = 0;
    fclose(f);

    char errbuf[1024];
    yajl_val tree = yajl_tree_parse(data, errbuf, sizeof(errbuf));
    free(data);
    if (!tree || !YAJL_IS_ARRAY(tree)) {
        if (tree) yajl_tree_free(tree);
        return;
    }

    yajl_gen gen = yajl_gen_alloc(NULL);
    yajl_gen_map_open(gen);
    yajl_gen_string(gen, (const unsigned char *)"items", 5);
    yajl_gen_array_open(gen);

    size_t count = YAJL_GET_ARRAY(tree)->len;
    for (size_t i = 0; i < count; i++) {
        yajl_val item = YAJL_GET_ARRAY(tree)->values[i];
        if (!YAJL_IS_OBJECT(item)) continue;

        const char *p_id[] = {"id", NULL};
        const char *p_name[] = {"name", NULL};
        const char *p_cat[] = {"category", NULL};
        const char *p_price[] = {"price", NULL};
        const char *p_qty[] = {"quantity", NULL};
        const char *p_active[] = {"active", NULL};
        const char *p_tags[] = {"tags", NULL};
        const char *p_rating[] = {"rating", NULL};

        yajl_val vid = yajl_tree_get(item, p_id, yajl_t_number);
        yajl_val vname = yajl_tree_get(item, p_name, yajl_t_string);
        yajl_val vcat = yajl_tree_get(item, p_cat, yajl_t_string);
        yajl_val vprice = yajl_tree_get(item, p_price, yajl_t_number);
        yajl_val vqty = yajl_tree_get(item, p_qty, yajl_t_number);
        yajl_val vactive = yajl_tree_get(item, p_active, yajl_t_any);
        yajl_val vtags = yajl_tree_get(item, p_tags, yajl_t_array);
        yajl_val vrating = yajl_tree_get(item, p_rating, yajl_t_object);

        double price = vprice ? YAJL_GET_DOUBLE(vprice) : 0;
        long long qty = vqty ? YAJL_GET_INTEGER(vqty) : 0;
        double total = round(price * (double)qty * 100.0) / 100.0;

        yajl_gen_map_open(gen);

        yajl_gen_string(gen, (const unsigned char *)"id", 2);
        yajl_gen_integer(gen, vid ? YAJL_GET_INTEGER(vid) : 0);

        yajl_gen_string(gen, (const unsigned char *)"name", 4);
        const char *name = vname ? YAJL_GET_STRING(vname) : "";
        yajl_gen_string(gen, (const unsigned char *)name, strlen(name));

        yajl_gen_string(gen, (const unsigned char *)"category", 8);
        const char *cat = vcat ? YAJL_GET_STRING(vcat) : "";
        yajl_gen_string(gen, (const unsigned char *)cat, strlen(cat));

        yajl_gen_string(gen, (const unsigned char *)"price", 5);
        yajl_gen_double(gen, price);

        yajl_gen_string(gen, (const unsigned char *)"quantity", 8);
        yajl_gen_integer(gen, qty);

        yajl_gen_string(gen, (const unsigned char *)"active", 6);
        yajl_gen_bool(gen, vactive ? YAJL_IS_TRUE(vactive) : 0);

        yajl_gen_string(gen, (const unsigned char *)"tags", 4);
        yajl_gen_array_open(gen);
        if (vtags) {
            for (size_t j = 0; j < YAJL_GET_ARRAY(vtags)->len; j++) {
                yajl_val t = YAJL_GET_ARRAY(vtags)->values[j];
                if (YAJL_IS_STRING(t)) {
                    const char *s = YAJL_GET_STRING(t);
                    yajl_gen_string(gen, (const unsigned char *)s, strlen(s));
                }
            }
        }
        yajl_gen_array_close(gen);

        yajl_gen_string(gen, (const unsigned char *)"rating", 6);
        yajl_gen_map_open(gen);
        if (vrating) {
            const char *ps[] = {"score", NULL};
            const char *pc[] = {"count", NULL};
            yajl_val vs = yajl_tree_get(vrating, ps, yajl_t_number);
            yajl_val vc = yajl_tree_get(vrating, pc, yajl_t_number);
            yajl_gen_string(gen, (const unsigned char *)"score", 5);
            yajl_gen_double(gen, vs ? YAJL_GET_DOUBLE(vs) : 0);
            yajl_gen_string(gen, (const unsigned char *)"count", 5);
            yajl_gen_integer(gen, vc ? YAJL_GET_INTEGER(vc) : 0);
        }
        yajl_gen_map_close(gen);

        yajl_gen_string(gen, (const unsigned char *)"total", 5);
        yajl_gen_double(gen, total);

        yajl_gen_map_close(gen);
    }

    yajl_gen_array_close(gen);
    yajl_gen_string(gen, (const unsigned char *)"count", 5);
    yajl_gen_integer(gen, (long long)count);
    yajl_gen_map_close(gen);

    const unsigned char *buf;
    size_t len;
    yajl_gen_get_buf(gen, &buf, &len);
    json_response = malloc(len);
    memcpy(json_response, buf, len);
    json_response_len = len;

    yajl_gen_free(gen);
    yajl_tree_free(tree);
}

/* Create listener socket with SO_REUSEPORT */
static int create_listener(int port)
{
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);

    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return -1;

    int on = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &on, sizeof(on));
    setsockopt(fd, SOL_SOCKET, SO_REUSEPORT, &on, sizeof(on));
    setsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &on, sizeof(on));

    int qlen = 4096;
    setsockopt(fd, IPPROTO_TCP, TCP_FASTOPEN, &qlen, sizeof(qlen));

    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) { close(fd); return -1; }
    if (listen(fd, 4096) < 0) { close(fd); return -1; }
    return fd;
}

/* Accept callback */
static void on_accept(h2o_socket_t *listener, const char *err)
{
    if (err) return;
    h2o_accept_ctx_t *ctx = listener->data;
    h2o_socket_t *sock;
    while ((sock = h2o_evloop_socket_accept(listener)) != NULL)
        h2o_accept(ctx, sock);
}

/* Worker thread: own event loop + listeners */
static void *worker_run(void *arg)
{
    (void)arg;
    h2o_evloop_t *loop = h2o_evloop_create();
    h2o_context_t ctx;
    h2o_context_init(&ctx, loop, &globalconf);

    /* HTTP/1.1 on port 8080 */
    h2o_accept_ctx_t accept_http;
    memset(&accept_http, 0, sizeof(accept_http));
    accept_http.ctx = &ctx;
    accept_http.hosts = globalconf.hosts;

    int fd = create_listener(8080);
    if (fd >= 0) {
        h2o_socket_t *sock = h2o_evloop_socket_create(loop, fd,
                                                       H2O_SOCKET_FLAG_DONT_READ);
        sock->data = &accept_http;
        h2o_socket_read_start(sock, on_accept);
    }

    /* HTTPS/H2 on port 8443 */
    h2o_accept_ctx_t accept_ssl;
    if (ssl_ctx) {
        memset(&accept_ssl, 0, sizeof(accept_ssl));
        accept_ssl.ctx = &ctx;
        accept_ssl.hosts = globalconf.hosts;
        accept_ssl.ssl_ctx = ssl_ctx;

        int fd_ssl = create_listener(8443);
        if (fd_ssl >= 0) {
            h2o_socket_t *sock = h2o_evloop_socket_create(loop, fd_ssl,
                                                           H2O_SOCKET_FLAG_DONT_READ);
            sock->data = &accept_ssl;
            h2o_socket_read_start(sock, on_accept);
        }
    }

    while (h2o_evloop_run(loop, INT32_MAX) == 0)
        ;
    return NULL;
}

/* Initialize TLS for HTTP/2 */
static void init_tls(void)
{
    const char *cert = getenv("TLS_CERT");
    const char *key = getenv("TLS_KEY");
    if (!cert) cert = "/certs/server.crt";
    if (!key) key = "/certs/server.key";
    if (access(cert, R_OK) != 0 || access(key, R_OK) != 0) return;

    ssl_ctx = SSL_CTX_new(TLS_server_method());
    SSL_CTX_set_min_proto_version(ssl_ctx, TLS1_2_VERSION);
    h2o_ssl_register_alpn_protocols(ssl_ctx, h2o_http2_alpn_protocols);

    if (SSL_CTX_use_certificate_file(ssl_ctx, cert, SSL_FILETYPE_PEM) != 1 ||
        SSL_CTX_use_PrivateKey_file(ssl_ctx, key, SSL_FILETYPE_PEM) != 1) {
        SSL_CTX_free(ssl_ctx);
        ssl_ctx = NULL;
    }
}

int main(void)
{
    signal(SIGPIPE, SIG_IGN);
    load_dataset();
    init_tls();

    h2o_config_init(&globalconf);
    globalconf.server_name = h2o_iovec_init(H2O_STRLIT("h2o"));

    /* Register host for HTTP (8080) */
    h2o_hostconf_t *host_http = h2o_config_register_host(
        &globalconf, h2o_iovec_init(H2O_STRLIT("default")), 8080);
    setup_host(host_http);

    /* Register host for HTTPS (8443) */
    if (ssl_ctx) {
        h2o_hostconf_t *host_ssl = h2o_config_register_host(
            &globalconf, h2o_iovec_init(H2O_STRLIT("default")), 8443);
        setup_host(host_ssl);
    }

    int nthreads = sysconf(_SC_NPROCESSORS_ONLN);
    if (nthreads < 1) nthreads = 1;

    for (int i = 1; i < nthreads; i++) {
        pthread_t t;
        pthread_create(&t, NULL, worker_run, NULL);
    }

    worker_run(NULL);
    return 0;
}
