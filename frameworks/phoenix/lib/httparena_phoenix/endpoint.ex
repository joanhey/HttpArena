defmodule HttparenaPhoenix.Endpoint do
  use Phoenix.Endpoint, otp_app: :httparena_phoenix

  plug HttparenaPhoenix.Router
end
