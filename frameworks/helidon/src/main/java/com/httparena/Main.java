package com.httparena;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.zaxxer.hikari.HikariConfig;
import com.zaxxer.hikari.HikariDataSource;
import io.helidon.http.HeaderNames;
import io.helidon.http.HeaderName;
import io.helidon.http.Status;
import io.helidon.webserver.WebServer;
import io.helidon.webserver.http.HttpRouting;
import io.helidon.webserver.http.ServerRequest;
import io.helidon.webserver.http.ServerResponse;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.InputStream;
import java.net.URI;
import java.nio.file.Files;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import java.util.zip.Deflater;
import java.util.zip.GZIPOutputStream;

public class Main {

    private static final ObjectMapper MAPPER = new ObjectMapper();
    private static final HeaderName SERVER_HEADER = HeaderNames.create("Server");
    private static final HeaderName CONTENT_TYPE = HeaderNames.CONTENT_TYPE;
    private static final HeaderName CONTENT_ENCODING = HeaderNames.CONTENT_ENCODING;
    private static final HeaderName ACCEPT_ENCODING = HeaderNames.ACCEPT_ENCODING;

    private static final String DB_QUERY =
            "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN ? AND ? LIMIT 50";

    private static List<Map<String, Object>> dataset;
    private static byte[] largeJsonResponse;
    private static boolean dbAvailable = false;
    private static final Map<String, byte[]> staticFiles = new ConcurrentHashMap<>();
    private static final Map<String, String> staticContentTypes = new ConcurrentHashMap<>();
    private static HikariDataSource pgPool;
    private static final ThreadLocal<Connection> tlConn = new ThreadLocal<>();

    private static final Map<String, String> MIME_TYPES = Map.of(
            ".css", "text/css", ".js", "application/javascript", ".html", "text/html",
            ".woff2", "font/woff2", ".svg", "image/svg+xml", ".webp", "image/webp", ".json", "application/json"
    );

    public static void main(String[] args) throws Exception {
        loadData();

        WebServer server = WebServer.builder()
                .port(8080)
                .routing(Main::routing)
                .build()
                .start();

        System.out.println("Helidon HttpArena server started on port " + server.port());
    }

    private static void routing(HttpRouting.Builder routing) {
        routing.get("/pipeline", Main::pipeline)
                .get("/baseline11", Main::baselineGet)
                .post("/baseline11", Main::baselinePost)
                .get("/baseline2", Main::baseline2)
                .get("/json", Main::json)
                .get("/compression", Main::compression)
                .post("/upload", Main::upload)
                .get("/db", Main::db)
                .get("/async-db", Main::asyncDb)
                .get("/static/{filename}", Main::staticFile);
    }

    private static void pipeline(ServerRequest req, ServerResponse res) {
        res.header(SERVER_HEADER, "helidon");
        res.header(CONTENT_TYPE, "text/plain");
        res.send("ok");
    }

    private static void baselineGet(ServerRequest req, ServerResponse res) {
        long sum = sumQueryParams(req);
        res.header(SERVER_HEADER, "helidon");
        res.header(CONTENT_TYPE, "text/plain");
        res.send(String.valueOf(sum));
    }

    private static void baselinePost(ServerRequest req, ServerResponse res) {
        long sum = sumQueryParams(req);
        String body = req.content().as(String.class).trim();
        try {
            sum += Long.parseLong(body);
        } catch (NumberFormatException ignored) {
        }
        res.header(SERVER_HEADER, "helidon");
        res.header(CONTENT_TYPE, "text/plain");
        res.send(String.valueOf(sum));
    }

    private static void baseline2(ServerRequest req, ServerResponse res) {
        long sum = sumQueryParams(req);
        res.header(SERVER_HEADER, "helidon");
        res.header(CONTENT_TYPE, "text/plain");
        res.send(String.valueOf(sum));
    }

    private static void json(ServerRequest req, ServerResponse res) {
        if (dataset == null || dataset.isEmpty()) {
            res.status(Status.INTERNAL_SERVER_ERROR_500);
            res.header(CONTENT_TYPE, "text/plain");
            res.send("Dataset not loaded");
            return;
        }
        try {
            List<Map<String, Object>> items = new ArrayList<>(dataset.size());
            for (Map<String, Object> item : dataset) {
                Map<String, Object> processed = new LinkedHashMap<>(item);
                double price = ((Number) item.get("price")).doubleValue();
                int quantity = ((Number) item.get("quantity")).intValue();
                processed.put("total", Math.round(price * quantity * 100.0) / 100.0);
                items.add(processed);
            }
            byte[] body = MAPPER.writeValueAsBytes(Map.of("items", items, "count", items.size()));
            res.header(SERVER_HEADER, "helidon");
            res.header(CONTENT_TYPE, "application/json");
            res.send(body);
        } catch (Exception e) {
            res.status(Status.INTERNAL_SERVER_ERROR_500);
            res.send("Error");
        }
    }

    private static void compression(ServerRequest req, ServerResponse res) {
        if (largeJsonResponse == null || largeJsonResponse.length == 0) {
            res.status(Status.INTERNAL_SERVER_ERROR_500);
            res.header(CONTENT_TYPE, "text/plain");
            res.send("Dataset not loaded");
            return;
        }
        try {
            String acceptEncoding = req.headers().first(ACCEPT_ENCODING).orElse("");
            res.header(SERVER_HEADER, "helidon");
            res.header(CONTENT_TYPE, "application/json");
            if (acceptEncoding.contains("gzip")) {
                res.header(CONTENT_ENCODING, "gzip");
                res.send(gzipCompress(largeJsonResponse));
            } else {
                res.send(largeJsonResponse);
            }
        } catch (Exception e) {
            res.status(Status.INTERNAL_SERVER_ERROR_500);
            res.send("Error");
        }
    }

    private static void upload(ServerRequest req, ServerResponse res) {
        try {
            InputStream is = req.content().inputStream();
            byte[] buf = new byte[65536];
            long total = 0;
            int n;
            while ((n = is.read(buf)) != -1) {
                total += n;
            }
            res.header(SERVER_HEADER, "helidon");
            res.header(CONTENT_TYPE, "text/plain");
            res.send(String.valueOf(total));
        } catch (Exception e) {
            res.status(Status.INTERNAL_SERVER_ERROR_500);
            res.send("Error");
        }
    }

    private static void db(ServerRequest req, ServerResponse res) {
        if (!dbAvailable) {
            res.header(CONTENT_TYPE, "application/json");
            res.send("{\"items\":[],\"count\":0}");
            return;
        }
        try {
            double min = parseDouble(req.query().first("min").orElse("10"));
            double max = parseDouble(req.query().first("max").orElse("50"));
            Connection conn = getDbConnection();
            List<Map<String, Object>> items = new ArrayList<>();
            PreparedStatement stmt = conn.prepareStatement(DB_QUERY);
            stmt.setDouble(1, min);
            stmt.setDouble(2, max);
            ResultSet rs = stmt.executeQuery();
            while (rs.next()) {
                Map<String, Object> item = new LinkedHashMap<>();
                item.put("id", rs.getLong("id"));
                item.put("name", rs.getString("name"));
                item.put("category", rs.getString("category"));
                item.put("price", rs.getDouble("price"));
                item.put("quantity", rs.getInt("quantity"));
                item.put("active", rs.getInt("active") == 1);
                item.put("tags", MAPPER.readValue(rs.getString("tags"), new TypeReference<List<String>>() {}));
                item.put("rating", Map.of("score", rs.getDouble("rating_score"), "count", rs.getInt("rating_count")));
                items.add(item);
            }
            rs.close();
            stmt.close();
            byte[] body = MAPPER.writeValueAsBytes(Map.of("items", items, "count", items.size()));
            res.header(SERVER_HEADER, "helidon");
            res.header(CONTENT_TYPE, "application/json");
            res.send(body);
        } catch (Exception e) {
            res.header(CONTENT_TYPE, "application/json");
            res.send("{\"items\":[],\"count\":0}");
        }
    }

    private static void asyncDb(ServerRequest req, ServerResponse res) {
        if (pgPool == null) {
            res.header(CONTENT_TYPE, "application/json");
            res.send("{\"items\":[],\"count\":0}");
            return;
        }
        try {
            double min = parseDouble(req.query().first("min").orElse("10"));
            double max = parseDouble(req.query().first("max").orElse("50"));
            try (Connection conn = pgPool.getConnection()) {
                PreparedStatement stmt = conn.prepareStatement(DB_QUERY);
                stmt.setDouble(1, min);
                stmt.setDouble(2, max);
                ResultSet rs = stmt.executeQuery();
                List<Map<String, Object>> items = new ArrayList<>();
                while (rs.next()) {
                    Map<String, Object> item = new LinkedHashMap<>();
                    item.put("id", rs.getLong("id"));
                    item.put("name", rs.getString("name"));
                    item.put("category", rs.getString("category"));
                    item.put("price", rs.getDouble("price"));
                    item.put("quantity", rs.getInt("quantity"));
                    item.put("active", rs.getBoolean("active"));
                    item.put("tags", MAPPER.readValue(rs.getString("tags"), new TypeReference<List<String>>() {}));
                    item.put("rating", Map.of("score", rs.getDouble("rating_score"), "count", rs.getInt("rating_count")));
                    items.add(item);
                }
                rs.close();
                stmt.close();
                byte[] body = MAPPER.writeValueAsBytes(Map.of("items", items, "count", items.size()));
                res.header(SERVER_HEADER, "helidon");
                res.header(CONTENT_TYPE, "application/json");
                res.send(body);
            }
        } catch (Exception e) {
            res.header(CONTENT_TYPE, "application/json");
            res.send("{\"items\":[],\"count\":0}");
        }
    }

    private static void staticFile(ServerRequest req, ServerResponse res) {
        String filename = req.path().pathParameters().get("filename");
        byte[] data = staticFiles.get(filename);
        if (data == null) {
            res.status(Status.NOT_FOUND_404);
            res.send("");
            return;
        }
        String ct = staticContentTypes.getOrDefault(filename, "application/octet-stream");
        res.header(SERVER_HEADER, "helidon");
        res.header(CONTENT_TYPE, ct);
        res.send(data);
    }

    // --- Helpers ---

    private static long sumQueryParams(ServerRequest req) {
        long sum = 0;
        for (String name : req.query().names()) {
            try {
                sum += Long.parseLong(req.query().first(name).orElse(""));
            } catch (NumberFormatException ignored) {
            }
        }
        return sum;
    }

    private static double parseDouble(String s) {
        try {
            return Double.parseDouble(s);
        } catch (NumberFormatException e) {
            return 10.0;
        }
    }

    private static Connection getDbConnection() {
        Connection conn = tlConn.get();
        if (conn == null) {
            try {
                Properties props = new Properties();
                props.setProperty("open_mode", "1"); // SQLITE_OPEN_READONLY
                conn = DriverManager.getConnection("jdbc:sqlite:/data/benchmark.db", props);
                conn.createStatement().execute("PRAGMA mmap_size=268435456");
                tlConn.set(conn);
            } catch (SQLException e) {
                throw new RuntimeException(e);
            }
        }
        return conn;
    }

    private static byte[] gzipCompress(byte[] data) throws Exception {
        ByteArrayOutputStream baos = new ByteArrayOutputStream(data.length / 4);
        GZIPOutputStream gzip = new GZIPOutputStream(baos) {{
            def.setLevel(Deflater.BEST_SPEED);
        }};
        gzip.write(data);
        gzip.close();
        return baos.toByteArray();
    }

    private static void loadData() throws Exception {
        // Dataset
        String path = System.getenv("DATASET_PATH");
        if (path == null) path = "/data/dataset.json";
        File f = new File(path);
        if (f.exists()) {
            dataset = MAPPER.readValue(f, new TypeReference<>() {});
        }

        // Large dataset for compression
        File largef = new File("/data/dataset-large.json");
        if (largef.exists()) {
            List<Map<String, Object>> largeDataset = MAPPER.readValue(largef, new TypeReference<>() {});
            List<Map<String, Object>> largeItems = new ArrayList<>(largeDataset.size());
            for (Map<String, Object> item : largeDataset) {
                Map<String, Object> processed = new LinkedHashMap<>(item);
                double price = ((Number) item.get("price")).doubleValue();
                int quantity = ((Number) item.get("quantity")).intValue();
                processed.put("total", Math.round(price * quantity * 100.0) / 100.0);
                largeItems.add(processed);
            }
            largeJsonResponse = MAPPER.writeValueAsBytes(Map.of("items", largeItems, "count", largeItems.size()));
        }

        // SQLite database
        dbAvailable = new File("/data/benchmark.db").exists();

        // PostgreSQL connection pool
        String dbUrl = System.getenv("DATABASE_URL");
        if (dbUrl != null && !dbUrl.isEmpty()) {
            try {
                URI uri = new URI(dbUrl.replace("postgres://", "postgresql://"));
                String host = uri.getHost();
                int port = uri.getPort() > 0 ? uri.getPort() : 5432;
                String database = uri.getPath().substring(1);
                String[] userInfo = uri.getUserInfo().split(":");
                HikariConfig config = new HikariConfig();
                config.setDriverClassName("org.postgresql.Driver");
                config.setJdbcUrl("jdbc:postgresql://" + host + ":" + port + "/" + database);
                config.setUsername(userInfo[0]);
                config.setPassword(userInfo.length > 1 ? userInfo[1] : "");
                config.setMaximumPoolSize(64);
                config.setMinimumIdle(16);
                pgPool = new HikariDataSource(config);
            } catch (Exception e) {
                System.err.println("PG pool init failed: " + e);
            }
        }

        // Static files
        File staticDir = new File("/data/static");
        if (staticDir.isDirectory()) {
            File[] files = staticDir.listFiles();
            if (files != null) {
                for (File sf : files) {
                    if (sf.isFile()) {
                        try {
                            staticFiles.put(sf.getName(), Files.readAllBytes(sf.toPath()));
                            int dot = sf.getName().lastIndexOf('.');
                            String ext = dot >= 0 ? sf.getName().substring(dot) : "";
                            staticContentTypes.put(sf.getName(), MIME_TYPES.getOrDefault(ext, "application/octet-stream"));
                        } catch (Exception ignored) {
                        }
                    }
                }
            }
        }
    }
}
