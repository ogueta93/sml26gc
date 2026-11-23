# The image the Linux packages are built in (`cargo xtask package linux-x64 | linux-arm64`):
# Ubuntu 18.04, whose glibc 2.27 is the oldest the executables then need, so they run on Ubuntu
# 18.04, Debian 10, RHEL 8 and every later distribution. Docker takes the image of the computer's
# own architecture (amd64 or arm64). Rust comes from rustup at the workspace's `rust-version`.
FROM ubuntu:18.04

# Ubuntu 18.04's packages are on archive.ubuntu.com (amd64) and ports.ubuntu.com (arm64) while
# its extended support lasts (2028); after that, point both at old-releases.ubuntu.com here.
RUN apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends build-essential pkg-config libasound2-dev libudev-dev curl ca-certificates \
 && rm -rf /var/lib/apt/lists/*

ARG RUST_VERSION
ENV RUSTUP_HOME=/usr/local/rustup PATH=/usr/local/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | CARGO_HOME=/usr/local/cargo sh -s -- -y --no-modify-path --profile minimal --default-toolchain "$RUST_VERSION" \
 && chmod -R a+rX /usr/local/rustup /usr/local/cargo

# The crates' cache, a volume shared by every build; open to the developer's user id, which the
# builds run as.
RUN mkdir /cargo && chmod 1777 /cargo
ENV CARGO_HOME=/cargo HOME=/tmp
WORKDIR /work
