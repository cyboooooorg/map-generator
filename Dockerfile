# Planet viewer: the mapgen-server web server, the built front-end and the
# command-line tools. Built and started by `mise run up` with compose.yaml.
# Worlds and presets are not copied: they are bind-mounted at runtime.

# ---------------------------------------------------------------------------------------------------- #
# Rust stage: server and command-line tools
# ---------------------------------------------------------------------------------------------------- #
FROM rust:1.99-slim-trixie AS rust
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
# Built-in presets are embedded in the binaries at compile time
COPY presets presets
RUN --mount=type=cache,target=/usr/local/cargo/registry \
	--mount=type=cache,target=/src/target \
	cargo build --release --locked -p mapgen-server -p map-generator \
	&& cp target/release/mapgen-server target/release/map-generator target/release/hexgrid /usr/local/bin/

# ---------------------------------------------------------------------------------------------------- #
# Web stage: front-end
# ---------------------------------------------------------------------------------------------------- #
FROM node:26-slim AS web
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web ./
RUN npm run build

# ---------------------------------------------------------------------------------------------------- #
# Runtime stage
# ---------------------------------------------------------------------------------------------------- #
FROM debian:trixie-slim

# curl: container health check
RUN apt-get update \
	&& apt-get install -y --no-install-recommends curl ca-certificates \
	&& rm -rf /var/lib/apt/lists/*

COPY --from=rust /usr/local/bin/mapgen-server /usr/local/bin/map-generator /usr/local/bin/hexgrid /usr/local/bin/
COPY --from=web /web/dist /app/web

# MALLOC_ARENA_MAX: fewer glibc arenas keep less freed memory after a
# generation, at no measurable cost in generation time (see
# crates/server/src/memory.rs)
ENV MAPGEN_WORLDS_DIR=/data/worlds \
	MAPGEN_SYSTEMS_DIR=/data/systems \
	MALLOC_ARENA_MAX=2 \
	MAPGEN_PRESETS_DIR=/data/presets \
	MAPGEN_STATIC_DIR=/app/web \
	HOST=0.0.0.0 \
	PORT=3000

# compose runs the container with the host UID/GID, so generated worlds keep the host owner
USER 1000:1000
EXPOSE 3000
CMD ["mapgen-server"]
