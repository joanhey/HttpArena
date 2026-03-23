defmodule HttparenaPhoenix.Application do
  use Application

  @impl true
  def start(_type, _args) do
    # Pre-load data
    HttparenaPhoenix.DataLoader.load()

    children = [
      HttparenaPhoenix.Endpoint
    ]

    opts = [strategy: :one_for_one, name: HttparenaPhoenix.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
