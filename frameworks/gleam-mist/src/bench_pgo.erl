-module(bench_pgo).
-export([connect/5, query/3, coerce/1, init_config/5, ensure_pool/0]).

connect(Host, Port, Database, User, Password) ->
    PoolName = httparena_pg_pool,
    PgoConfig = #{
        host => binary_to_list(Host),
        port => Port,
        database => binary_to_list(Database),
        user => binary_to_list(User),
        password => binary_to_list(Password),
        pool_size => 64,
        ssl => false
    },
    pgo_pool:start_link(PoolName, PgoConfig),
    PoolName.

%% Store PG config for lazy connection
init_config(Host, Port, Database, User, Password) ->
    persistent_term:put(httparena_pg_config, {Host, Port, Database, User, Password}),
    nil.

%% Lazily connect: return {ok, Pool} or {error, nil}
ensure_pool() ->
    case persistent_term:get(httparena_pg_pool_ref, undefined) of
        undefined ->
            case persistent_term:get(httparena_pg_config, undefined) of
                undefined ->
                    {error, nil};
                {Host, Port, Database, User, Password} ->
                    try
                        Pool = connect(Host, Port, Database, User, Password),
                        %% Verify the pool works with a simple query
                        case pgo:query(<<"SELECT 1">>, [], #{pool => Pool}) of
                            #{command := _, num_rows := _, rows := _} ->
                                persistent_term:put(httparena_pg_pool_ref, Pool),
                                {ok, Pool};
                            _ ->
                                {error, nil}
                        end
                    catch
                        _:_ -> {error, nil}
                    end
            end;
        Pool ->
            {ok, Pool}
    end.

query(Pool, Sql, Params) ->
    case pgo:query(Sql, Params, #{pool => Pool}) of
        #{command := _, num_rows := Count, rows := Rows} ->
            {ok, {Count, Rows}};
        {error, _Reason} ->
            {error, nil}
    end.

coerce(X) -> X.
