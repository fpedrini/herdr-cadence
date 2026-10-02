#!/bin/sh
set -eu

cadence_version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' herdr-plugin.toml | head -n 1)
if [ -z "$cadence_version" ]; then
  printf 'Unable to determine Cadence version from herdr-plugin.toml\n' >&2
  exit 1
fi
cadence_os=$(uname -s)
cadence_arch=$(uname -m)

case "${cadence_os}-${cadence_arch}" in
  Darwin-arm64) cadence_target="aarch64-apple-darwin" ;;
  Darwin-x86_64) cadence_target="x86_64-apple-darwin" ;;
  Linux-aarch64|Linux-arm64) cadence_target="aarch64-unknown-linux-gnu" ;;
  Linux-x86_64) cadence_target="x86_64-unknown-linux-gnu" ;;
  *)
    printf 'Unsupported platform: %s-%s\n' "$cadence_os" "$cadence_arch" >&2
    exit 1
    ;;
esac

cadence_asset="herdr-cadence-${cadence_target}.tar.gz"
cadence_base="https://github.com/zhenyufu/herdr-cadence/releases/download/v${cadence_version}"
cadence_tmp=$(mktemp -d)
cadence_stage=""
trap 'rm -rf "$cadence_tmp"; if [ -n "$cadence_stage" ]; then rm -rf "$cadence_stage"; fi' EXIT INT TERM

curl --fail --silent --show-error --location \
  "$cadence_base/$cadence_asset" --output "$cadence_tmp/$cadence_asset"
curl --fail --silent --show-error --location \
  "$cadence_base/$cadence_asset.sha256" --output "$cadence_tmp/$cadence_asset.sha256"

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$cadence_tmp" && sha256sum --check "$cadence_asset.sha256")
else
  (cd "$cadence_tmp" && shasum -a 256 --check "$cadence_asset.sha256")
fi

mkdir -p bin
cadence_stage=$(mktemp -d "bin/.herdr-cadence.XXXXXX")
tar -xzf "$cadence_tmp/$cadence_asset" -C "$cadence_stage"
test -f "$cadence_stage/herdr-cadence"
chmod 0755 "$cadence_stage/herdr-cadence"
mv "$cadence_stage/herdr-cadence" bin/herdr-cadence
