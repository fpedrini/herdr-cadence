#!/bin/sh
set -eu

cadence_script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
cadence_project_root=$(CDPATH= cd "$cadence_script_dir/.." && pwd)
cd "$cadence_project_root"

cargo build --release --locked
mkdir -p bin
cadence_stage=$(mktemp -d "bin/.herdr-cadence.XXXXXX")
trap 'rm -rf "$cadence_stage"' EXIT INT TERM
install -m 0755 target/release/herdr-cadence "$cadence_stage/herdr-cadence"
mv "$cadence_stage/herdr-cadence" bin/herdr-cadence
herdr plugin link "$cadence_project_root"
printf 'Built and linked %s\n' "$(pwd)/bin/herdr-cadence"
