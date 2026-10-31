# syntax=docker/dockerfile:1
#
# The advent engine as an image, for the company edition's `advent serve`.
# It holds no calendar: each event repo builds its own image FROM this one,
# copying its sealed files into /event (see templates/event/Dockerfile).

FROM node:24-bookworm-slim AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
# The type check covers the tests, which read the shared vectors.
COPY test-vectors.json /src/test-vectors.json
RUN npm run build

FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
# The binary embeds web/dist when it exists at compile time.
COPY --from=web /src/web/dist web/dist
RUN cargo build --release --locked -p advent-cli \
    && cp target/release/advent /usr/local/bin/advent

FROM debian:bookworm-slim
# CA certificates for SMTP over TLS and the Resend API.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home-dir /data --shell /usr/sbin/nologin advent \
    && mkdir -p /data /event \
    && chown advent /data
COPY --from=build /usr/local/bin/advent /usr/local/bin/advent
USER advent
WORKDIR /data
VOLUME ["/data"]
ENV ADVENT_LISTEN=0.0.0.0:8080 \
    ADVENT_DATA=/data
EXPOSE 8080
ENTRYPOINT ["advent"]
CMD ["serve", "--repo", "/event"]
