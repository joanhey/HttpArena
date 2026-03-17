FROM ruby:3.4-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends build-essential libsqlite3-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Gemfile .
RUN bundle install --jobs=$(nproc)

COPY . .

EXPOSE 8080

CMD ["bundle", "exec", "puma", "-C", "puma.rb"]
