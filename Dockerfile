# syntax=docker/dockerfile:1.7

FROM debian:trixie-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tzdata ffmpeg libvips-tools \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 --shell /usr/sbin/nologin lumenstream

WORKDIR /app

COPY ./target/release/ls-app /usr/local/bin/ls-app
COPY config.example.yaml /app/config.example.yaml

RUN chmod +x /usr/local/bin/ls-app \
    && chown -R lumenstream:lumenstream /app

USER lumenstream

EXPOSE 8096

ENTRYPOINT ["/usr/local/bin/ls-app"]
