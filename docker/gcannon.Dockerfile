FROM ubuntu:24.04 AS build
RUN apt-get update && apt-get install -y gcc make liburing-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY . .
RUN make clean && make -j$(nproc)

FROM ubuntu:24.04
RUN apt-get update && apt-get install -y liburing2 && rm -rf /var/lib/apt/lists/*
COPY --from=build /build/gcannon /usr/local/bin/gcannon
ENTRYPOINT ["gcannon"]
