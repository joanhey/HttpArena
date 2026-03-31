<?php

require 'Db.php';
require 'Pgsql.php';
require 'data.php';

// Init
Db::init();
Pgsql::init();

function guard()
{
    if (!in_array(ngx_request_method(), ['POST', 'GET'])) {
        ngx_header_set('Content-Type', 'text/plain');
        echo 'Method Not Allowed';
        ngx::_exit(405);
    }
}

function baseline()
{
    $sum = array_sum(ngx::query_args());
    if(ngx_request_method() === 'POST') {
        $sum += ngx_request_body();
    }

    ngx_header_set('Content-Type', 'text/plain');
    echo $sum;
}

function json()
{
    $total = [];
    foreach (JSON_DATA as $item) {
        $item['total'] = $item['price'] * $item['quantity'];
        $total[] = $item;
    }

    ngx_header_set('Content-Type', 'application/json');
    echo json_encode(['items' => $total, 'count' => count($total)],
                    JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES);
}

function upload()
{
    ngx_header_set('Content-Type', 'text/plain');
    echo strlen(ngx_request_body());
}

function compression()
{
    ngx_header_set('Content-Type', 'application/json');
    echo LARGE_JSON;
}

function pipeline()
{
    ngx_header_set('Content-Type', 'text/plain');
    echo 'ok';
}

function db()
{
    ngx_header_set('Content-Type', 'application/json');
    echo Db::query(
        ngx::query_args()['min'] ?? 10,
        ngx::query_args()['max'] ?? 50
    );
}

function asyncDb()
{
    ngx_header_set('Content-Type', 'application/json');
    echo Pgsql::query(
        ngx::query_args()['min'] ?? 10,
        ngx::query_args()['max'] ?? 50
    );
}

function files()
{
    $path = ngx_request_uri();
    if (!isset(STATIC_FILES[$path])) {
        return notFound();
    }

    ngx_header_set('Content-Type', STATIC_FILES[$path][1]);
    echo STATIC_FILES[$path][0];
}

function notFound()
{
    ngx_header_set('Content-Type', 'text/plain');
    echo 'Not Found';
    ngx_status(404);
}
