#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
chart="$repo_root/charts/convex"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

for values in "$chart"/ci/*-values.yaml; do
  helm lint "$chart" --strict -f "$values"
  helm template convex "$chart" --namespace convex -f "$values" > "$scratch/rendered.yaml"
done

# Required configuration and malformed values must fail before installation.
expect_failure() {
  local expected="$1"
  shift
  if helm template convex "$chart" "$@" > "$scratch/output" 2> "$scratch/error"; then
    echo "Expected Helm to reject: $*" >&2
    exit 1
  fi
  if ! grep -qE -- "$expected" "$scratch/error"; then
    cat "$scratch/error" >&2
    exit 1
  fi
}

expect_failure 'convex.publicUrl is required'
expect_failure 'Provide DATABASE_URL' --set convex.publicUrl=https://convex.example.com
expect_failure 'convex.port' -f "$chart/ci/minimal-values.yaml" --set convex.port=70000
expect_failure 'convex.publicUrl' -f "$chart/ci/minimal-values.yaml" --set convex.publicUrl=http://convex.example.com

helm package "$chart" --destination "$scratch"
helm template convex "$scratch"/convex-*.tgz -f "$chart/ci/minimal-values.yaml" > "$scratch/packaged.yaml"
echo 'Helm chart checks passed.'
