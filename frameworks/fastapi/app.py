import os
import sys
import multiprocessing
import zlib
import json
from contextlib import asynccontextmanager

import asyncpg
import orjson
from fastapi import FastAPI, Request, Response, Path, Query, HTTPException

# -- Dataset and constants --------------------------------------------------------

CPU_COUNT = int(multiprocessing.cpu_count())
WRK_COUNT = min(len(os.sched_getaffinity(0)), 128)
WRK_COUNT = max(WRK_COUNT, 4)

DATASET_LARGE_PATH = "/data/dataset-large.json"
DATASET_PATH = os.environ.get("DATASET_PATH", "/data/dataset.json")
DATASET_ITEMS = None
try:
    with open(DATASET_PATH) as file:
        DATASET_ITEMS = json.load(file)
except Exception:
    pass

# -- Postgres DB ------------------------------------------------------------

pg_pool: asyncpg.Pool | None = None

PG_QUERY = (
    "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count "
    "FROM items WHERE price BETWEEN $1 AND $2 LIMIT $3"
)

class NoResetConnection(asyncpg.Connection):
    __slots__ = ()
    def get_reset_query(self):
        return ""

@asynccontextmanager
async def lifespan(application: FastAPI):
    global pg_pool, NoResetConnection
    DATABASE_URL = os.environ.get("DATABASE_URL")
    if DATABASE_URL:
        try:
            if DATABASE_URL.startswith("postgres://"):
                DATABASE_URL = "postgresql://" + DATABASE_URL[len("postgres://"):]
            PG_POOL_MAX_SIZE = 2
            DATABASE_MAX_CONN = os.environ.get("DATABASE_MAX_CONN", None)
            if DATABASE_MAX_CONN:
                pool_size = int(DATABASE_MAX_CONN) * 0.92 / WRK_COUNT
                PG_POOL_MAX_SIZE = int(pool_size + 0.95)
            pg_pool = await asyncpg.create_pool(
                dsn = DATABASE_URL,
                min_size = 1,
                max_size = max(PG_POOL_MAX_SIZE, 2),
                connection_class = NoResetConnection
            )
        except Exception:
            pg_pool = None
    yield
    if pg_pool:
        await pg_pool.close()
    pg_pool = None


app = FastAPI(lifespan=lifespan)

# -- Helpers ----------------------------------------------------------------

def make_resp(status: int, media_type: str, body: str | bytes, contenc: str | None = None) -> Response:
    headers = { "Server": "fastapi" }
    if isinstance(body, str):
        body = body.encode('utf-8')
    if contenc and contenc != 'BR':
        if contenc == 'GZIP':
            body = zlib.compress(body, level = 1, wbits = 31)
        headers['Content-Encoding'] = contenc.lower()
    return Response(content = body, status_code = status, media_type = media_type, headers = headers)


def text_resp(body: str | bytes, status: int = 200, contenc: str | None = None) -> Response:
    return make_resp(status, body, "text/plain", contenc)


def json_resp(body: bytes, status: int = 200, contenc: str | None = None) -> Response:
    if isinstance(body, dict):
        body = orjson.dumps(body)
    return make_resp(status, body, "application/json", contenc)


# -- Routes ------------------------------------------------------------------
@app.get("/pipeline")
async def pipeline():
    return text_resp(b"ok")


@app.api_route("/baseline11", methods=["GET", "POST"])
async def baseline11(request: Request):
    total = 0
    for v in request.query_params.values():
        try:
            total += int(v)
        except ValueError:
            pass
    if request.method == "POST":
        body = await request.body()
        if body:
            try:
                total += int(body.strip())
            except ValueError:
                pass
    return _text(str(total))


@app.get("/baseline2")
async def baseline2(request: Request):
    total = 0
    for v in request.query_params.values():
        try:
            total += int(v)
        except ValueError:
            pass
    return _text(str(total))


async def json_common(request: Request, count: int, m_val: float):
    global DATASET_ITEMS
    if DATASET_ITEMS is None:
        return text_resp("No dataset", 500)
    accept_encoding = request.headers.get("accept-encoding", "")
    contenc = 'GZIP' if 'gzip' in accept_encoding else ''
    try:
        items = [ ]
        for idx, dsitem in enumerate(DATASET_ITEMS):
            if idx >= count:
                break
            item = dict(dsitem)
            item["total"] = dsitem["price"] * dsitem["quantity"] * m_val
            items.append(item)
        return json_resp( { "items": items, "count": len(items) }, contenc = contenc)
    except Exception:
        return json_resp( { "items": [ ], "count": 0 }, contenc = contenc)


@app.get("/json/{count}")
async def json_endpoint(request: Request, count: int = Path(...), m: float = Query(...)):
    return await json_common(request, count, m)


@app.get("/json-comp/{count}")
async def json_comp_endpoint(request: Request, count: int = Path(...), m: float = Query(...)):
    return await json_common(request, count, m)


@app.get("/async-db")
async def async_db_endpoint(request: Request, min_val: float = Query(..., alias="min"), max_val: float = Query(..., alias="max"), limit: int = Query(...)):
    if pg_pool is None:
        return json_resp( { "items": [ ], "count": 0 } )
    try:
        db_conn = await pg_pool.acquire()
        try:
            rows = await db_conn.fetch(PG_QUERY, min_val, max_val, limit)
        finally:
            await pg_pool.release(db_conn)
        items = [
            {
                'id'      : row['id'],
                'name'    : row['name'],
                'category': row['category'],
                'price'   : row['price'],
                'quantity': row['quantity'],
                'active'  : row['active'],
                'tags'    : json.loads(row['tags']) if isinstance(row['tags'], str) else row['tags'],
                'rating': {
                    'score': row['rating_score'],
                    'count': row['rating_count'],
                }
            }
            for row in rows
        ]
        return json_resp( { "items": items, "count": len(items) } )
    except Exception:
        return json_resp( { "items": [ ], "count": 0 } )


@app.post("/upload")
async def upload_endpoint(request: Request):
    size = 0
    async for chunk in request.stream():
        size += len(chunk)
    return text_resp(str(size))
