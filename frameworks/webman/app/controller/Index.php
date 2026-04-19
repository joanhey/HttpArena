<?php
namespace app\controller;

use support\Request;
use app\bootstrap\Pgsql;
use support\Response;
use support\annotation\route\DisableDefaultRoute;
use support\annotation\route\Get;
use support\annotation\route\Post;
use support\annotation\route\Route;

#[DisableDefaultRoute]
class Index
{

    #[Get('/pipeline')]
    public function pipeline()
    {
        return new Response(
            200,
            ['Content-Type' => 'text/plain'],
            'ok'
        );
    }

    #[Route('/baseline', ['GET', 'POST'])]
    public function baseline(Request $request)
    {
        $sum = \array_sum($request->get());
        if($request->method() === 'POST') {
            $sum += $request->rawBody();
        }
        
        return new Response (
            200,
            ['Content-Type' => 'text/plain'],
            $sum
        );
    }

    #[Get('/json/{count:\d+}')]
    public function json(Request $request, $count)
    {
        $m = $request->get('m', 1);
        $total = [];
        $i = 0;
        while ($i < $count) {
            $item = \JSON_DATA[$i++];
            $item['total'] = $item['price'] * $item['quantity'] * $m;
            $total[] = $item;
        }

        return json(['items' => $total, 'count' => $count]);
    }

    #[Get('/async-db')]
    public function asyncDb(Request $request)
    {
        return new Response (
            200,
            ['Content-Type' => 'application/json'],
            Pgsql::query(
                $request->get('min', 10),
                $request->get('max', 50),
                $request->get('limit', 50)
            )
        );
    }

    #[Post('/upload')]
    public function upload($request)
    {
        return new Response (
            200,
            ['Content-Type' => 'text/plain'],
            \strlen($request->rawBody())
        );
    }

    #[Get('/static/{path}')]
    public function files($request, $path)
    {
        return new Response()->withFile('/data/static/' . $path);
    }
}
