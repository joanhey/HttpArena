using System.Collections.Concurrent;
using Microsoft.Data.Sqlite;

namespace ServiceStack.Benchmarks;

public sealed class SqlitePool
{
    private readonly ConcurrentBag<SqliteConnection> _connections = new();

    public SqlitePool(string connectionString, int size)
    {
        for (int i = 0; i < size; i++)
        {
            var conn = new SqliteConnection(connectionString);
            conn.Open();
            using var pragma = conn.CreateCommand();
            pragma.CommandText = "PRAGMA mmap_size=268435456";
            pragma.ExecuteNonQuery();
            _connections.Add(conn);
        }
    }

    public SqliteConnection Rent()
    {
        SqliteConnection? conn;
        var spin = new SpinWait();
        while (!_connections.TryTake(out conn))
            spin.SpinOnce();
        return conn;
    }

    public void Return(SqliteConnection conn) => _connections.Add(conn);
}

public static class DbConnectionFactory
{
    public static SqlitePool? Open()
    {
        const string path = "/data/benchmark.db";
        if (!File.Exists(path)) return null;
        return new SqlitePool($"Data Source={path};Mode=ReadOnly", Environment.ProcessorCount);
    }
}

public static class PgPoolFactory
{
    public static Npgsql.NpgsqlDataSource? Open()
    {
        var url = Environment.GetEnvironmentVariable("DATABASE_URL");
        if (string.IsNullOrEmpty(url)) return null;
        try
        {
            var uri  = new Uri(url);
            var info = uri.UserInfo.Split(':');
            var cs   = $"Host={uri.Host};Port={uri.Port};Username={info[0]};Password={info[1]};" +
                       $"Database={uri.AbsolutePath.TrimStart('/')};" +
                       "Maximum Pool Size=256;Minimum Pool Size=64;" +
                       "Multiplexing=true;No Reset On Close=true;" +
                       "Max Auto Prepare=4;Auto Prepare Min Usages=1";
            return new Npgsql.NpgsqlDataSourceBuilder(cs).Build();
        }
        catch { return null; }
    }
}

public static class JsonOpts
{
    public static readonly System.Text.Json.JsonSerializerOptions Default = new()
    {
        PropertyNameCaseInsensitive = true,
        PropertyNamingPolicy        = System.Text.Json.JsonNamingPolicy.CamelCase
    };
}