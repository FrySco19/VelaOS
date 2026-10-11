FROM ubuntu:22.04

ARG DEBIAN_FRONTEND=noninteractive
ARG MICROKIT_VERSION=2.3.0
ARG MICROKIT_SDK_SHA256=e12e507f72c87cbf5c514df9e9b0c66103b42298852f44c045d5729ea3de4f89
ARG RUST_TOOLCHAIN=nightly-2025-10-20

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    cmake \
    curl \
    gcc-aarch64-linux-gnu \
    git \
    libclang-dev \
    ninja-build \
    python3 \
    python3-venv \
    qemu-system-arm \
    qemu-utils \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.cargo/bin:${PATH}"
ENV MICROKIT_SDK=/microkit
ENV RUSTUP_TOOLCHAIN=${RUST_TOOLCHAIN}

RUN set -eux; \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain "${RUST_TOOLCHAIN}" \
            --component rust-src --component llvm-tools-preview; \
    rustup target add --toolchain "${RUST_TOOLCHAIN}" aarch64-unknown-none; \
    rustc --version

RUN set -eux; \
    curl --proto '=https' --tlsv1.2 -fsSL \
        "https://github.com/seL4/microkit/releases/download/${MICROKIT_VERSION}/microkit-sdk-${MICROKIT_VERSION}-linux-x86-64.tar.gz" \
        -o /tmp/microkit-sdk.tar.gz; \
    echo "${MICROKIT_SDK_SHA256}  /tmp/microkit-sdk.tar.gz" | sha256sum -c -; \
    mkdir -p "${MICROKIT_SDK}"; \
    tar -xzf /tmp/microkit-sdk.tar.gz -C "${MICROKIT_SDK}" --strip-components=1; \
    rm /tmp/microkit-sdk.tar.gz; \
    test -x "${MICROKIT_SDK}/bin/microkit"; \
    test -x "$(command -v qemu-system-aarch64)"

WORKDIR /work

COPY Makefile gate_f_timing.py velaos.system ./
COPY smoke_boot.py ./
COPY app/ ./app/
COPY pcb_root/ ./pcb_root/
COPY patch-payload-helper.sh ./

RUN chmod +x app/rustc-wrapper.sh pcb_root/rustc-wrapper.sh patch-payload-helper.sh

RUN make build

RUN make gate-c

CMD ["make", "run"]
