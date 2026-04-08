FROM mariasocute/swoole:6.2.0


COPY swoole.php .

EXPOSE 8080
CMD ["php", "-dmemory_limit=2048M","/swoole.php", "start"]
