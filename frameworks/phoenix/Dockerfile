FROM elixir:1.17-otp-27-alpine AS build
RUN apk add --no-cache gcc musl-dev make
WORKDIR /app
ENV MIX_ENV=prod

# Install hex + rebar
RUN mix local.hex --force && mix local.rebar --force

# Get deps
COPY mix.exs ./
RUN mix deps.get --only prod
RUN mix deps.compile

# Compile app
COPY config ./config
COPY lib ./lib
RUN mix compile
RUN mix release

FROM erlang:27-alpine
RUN apk add --no-cache libstdc++
WORKDIR /app
COPY --from=build /app/_build/prod/rel/httparena_phoenix /app
EXPOSE 8080
CMD ["/app/bin/httparena_phoenix", "start"]
