import Config

config :httparena_phoenix, HttparenaPhoenix.Endpoint,
  http: [port: 8080, ip: {0, 0, 0, 0}, transport_options: [num_acceptors: 100]],
  server: true,
  render_errors: [formats: [json: HttparenaPhoenix.ErrorJSON]],
  pubsub_server: HttparenaPhoenix.PubSub

config :logger, level: :warning

config :phoenix, :json_library, Jason
