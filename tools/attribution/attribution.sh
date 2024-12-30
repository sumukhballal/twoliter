#!/bin/bash
# Generates attributions for dependencies of Twoliter
# Meant to be run from Bottlerocket's SDK container:
# https://github.com/bottlerocket-os/bottlerocket-sdk

# See the "attribution" target in the project Makefile.

set -eo pipefail

LICENSEDIR=/tmp/twoliter-attributions

# Use the toolchain installed via `Dockerfile.attribution`
export HOME="/home/attribution-creator"
source ~/.cargo/env

# Source code is mounted to /src
# rustup will automatically use the toolchain in rust-toolchain.toml
cd /src

# =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=
echo "Clarifying crate dependency licenses..."
/usr/libexec/tools/bottlerocket-license-scan \
    --clarify /src/clarify.toml \
    --spdx-data /usr/libexec/tools/spdx-data \
    --out-dir ${LICENSEDIR}/vendor \
    cargo --locked Cargo.toml

# =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=
# krane-static
echo "Clarifying golang dependencies of krane-static"
KRANE_STATIC_VENDOR_DIR=$(mktemp -d)
cp -r /src/tools/krane/go-src/* "${KRANE_STATIC_VENDOR_DIR}"

pushd "${KRANE_STATIC_VENDOR_DIR}"
go mod vendor
popd

/usr/libexec/tools/bottlerocket-license-scan \
    --clarify /src/clarify.toml \
    --spdx-data /usr/libexec/tools/spdx-data \
    --out-dir ${LICENSEDIR}/krane-static \
    go-vendor "${KRANE_STATIC_VENDOR_DIR}/vendor"

# =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=
# cargo-make (we currently use cargo-make from the SDK, but will ship it in Twoliter in the future)
echo "Clarifying bottlerocket-sdk & dependency licenses..."
mkdir -p ${LICENSEDIR}/bottlerocket-sdk/
cp -r /usr/share/licenses/cargo-make \
    ${LICENSEDIR}/bottlerocket-sdk/

# =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=  =^.^=
# Twoliter licenses
cp /src/COPYRIGHT /src/LICENSE-MIT /src/LICENSE-APACHE \
    ${LICENSEDIR}/

pushd "$(dirname ${LICENSEDIR})"
tar czf /src/twoliter-attributions.tar.gz "$(basename ${LICENSEDIR})"
popd
