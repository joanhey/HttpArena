<?php

namespace app\bootstrap;

use Webman\Bootstrap;

class Json implements Bootstrap {

    /**
     * @param \Workerman\Worker $worker
     * @return void
     */
    public static function start($worker)
    {
        // benchmark data
        define('JSON_DATA', json_decode(file_get_contents('/data/dataset.json'), true));
    }

}
