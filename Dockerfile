# syntax=docker/dockerfile:1.27

ARG RUST_VERSION
ARG NODE_VERSION

FROM node:${NODE_VERSION?}-trixie-slim AS frontend
ARG PNPM_VERSION
WORKDIR /src/web
RUN npm install --global pnpm@${PNPM_VERSION:?}
COPY web/package.json web/pnpm-lock.yaml web/pnpm-workspace.yaml ./
RUN --mount=type=cache,target=/pnpm/store,sharing=locked \
    pnpm install --frozen-lockfile --store-dir /pnpm/store
COPY web ./
RUN pnpm build

FROM rust:${RUST_VERSION?}-slim-trixie AS builder

# TARGETARCH is supplied by buildx per platform.
ARG TARGETARCH
WORKDIR /src

COPY Cargo.lock Cargo.toml ./
COPY crates crates
COPY migrations migrations
COPY --from=frontend /src/web/dist web/dist

RUN --mount=type=cache,target=/usr/local/cargo/registry,id=cargo-registry-${TARGETARCH},sharing=locked \
    --mount=type=cache,target=/src/target,id=cargo-target-${TARGETARCH},sharing=locked \
    RUSTFLAGS="-C target-feature=+crt-static" \
    cargo build --release --locked --features embed-web --bin=convex --target="$(case ${TARGETARCH} in \
      "amd64") echo "x86_64";; \
      "arm64") echo "aarch64";; \
      *) echo "${TARGETARCH}";; \
    esac)-unknown-linux-gnu" && \
    cp "target/$(case ${TARGETARCH} in \
      "amd64") echo "x86_64";; \
      "arm64") echo "aarch64";; \
      *) echo "${TARGETARCH}";; \
    esac)-unknown-linux-gnu/release/convex" "/usr/local/bin/convex"

FROM rockylinux/rockylinux:10-ubi-micro AS runtime

# renovate: datasource=github-releases depName=krallin/tini
ENV TINI_VERSION=v0.19.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /usr/local/bin/tini

RUN echo 'convex:x:10001:10001::/:/sbin/nologin' >> /etc/passwd \
  && echo 'convex:x:10001:' >> /etc/group \
	&& chmod +x /usr/local/bin/tini

COPY --from=builder --chown=10001:10001 /usr/local/bin/convex /usr/local/bin/convex

RUN echo "#!/usr/bin/env bash" > /usr/local/bin/entrypoint && \
  echo "exec /usr/local/bin/tini -- /usr/local/bin/convex \"\${@}\"" >> /usr/local/bin/entrypoint && \
  chmod +x /usr/local/bin/entrypoint

USER 10001:10001
ENTRYPOINT ["/usr/local/bin/entrypoint"]
