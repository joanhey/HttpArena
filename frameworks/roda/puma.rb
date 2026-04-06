threads ENV.fetch('MAX_THREADS', 4).to_i

bind 'tcp://0.0.0.0:8080'

# Allow all HTTP methods so unknown ones reach Rack middleware (returned as 405)
supported_http_methods :any

preload_app!

before_fork do
  # Close any inherited DB connections
end
