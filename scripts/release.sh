#!/usr/bin/env bash

set -Eeuo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${PROJECT_DIR}"

fail() {
    echo "error: $*" >&2
    exit 1
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || fail "command '$1' belum terpasang"
}

require_command git
require_command node
require_command gh

git rev-parse --is-inside-work-tree >/dev/null 2>&1 || fail "folder ini bukan repository Git"

if [[ -n "$(git status --porcelain)" ]]; then
    fail "working tree belum bersih; commit atau stash perubahan sebelum release"
fi

version="${1:-}"
if [[ -z "${version}" ]]; then
    read -r -p "Version release (contoh: 1.2.3): " version
fi

changelog="${*:2}"
if [[ -z "${changelog}" ]]; then
    echo "Changelog release (akhiri dengan baris kosong):"
    changelog_lines=()
    while IFS= read -r line && [[ -n "${line}" ]]; do
        changelog_lines+=("${line}")
    done
    changelog="$(printf '%s\n' "${changelog_lines[@]}")"
fi

[[ -n "${changelog}" ]] || fail "changelog tidak boleh kosong"

version="${version#v}"
if [[ ! "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
    fail "version '${version}' bukan semantic version yang valid"
fi

readonly VERSION="${version}"
readonly TAG="v${VERSION}"
readonly BRANCH="$(git branch --show-current)"
readonly CHANGELOG="${changelog}"

[[ -n "${BRANCH}" ]] || fail "release tidak dapat dibuat dari detached HEAD"

current_cargo_version="$(
    node -e "const fs=require('fs'); const text=fs.readFileSync('Cargo.toml','utf8'); const match=text.match(/^version\\s*=\\s*\"([^\"]+)\"/m); if (!match) process.exit(1); process.stdout.write(match[1]);"
)" || fail "version package tidak ditemukan di Cargo.toml"
current_npm_version="$(node -p "require('./package.json').version")"

if [[ "${current_cargo_version}" != "${current_npm_version}" ]]; then
    fail "version Cargo (${current_cargo_version}) dan npm (${current_npm_version}) tidak sama"
fi

if [[ "${VERSION}" == "${current_cargo_version}" ]]; then
    fail "version ${VERSION} sama dengan version package saat ini"
fi

echo "Release ${TAG} dari branch ${BRANCH}"
echo "  Cargo: ${current_cargo_version} -> ${VERSION}"
echo "  npm:   ${current_npm_version} -> ${VERSION}"

git fetch --tags origin

if git rev-parse --verify --quiet "refs/tags/${TAG}" >/dev/null; then
    fail "tag lokal ${TAG} sudah ada"
fi

if git ls-remote --exit-code --tags origin "refs/tags/${TAG}" >/dev/null 2>&1; then
    fail "tag ${TAG} sudah ada di origin"
fi

gh auth status >/dev/null

RELEASE_VERSION="${VERSION}" node <<'NODE'
const fs = require('fs')

const version = process.env.RELEASE_VERSION

const cargoPath = 'Cargo.toml'
const cargo = fs.readFileSync(cargoPath, 'utf8')
const packageSection = /(\[package\][\s\S]*?\nversion\s*=\s*")[^"]+("\s*\n)/

if (!packageSection.test(cargo)) {
    throw new Error('version package tidak ditemukan di Cargo.toml')
}

fs.writeFileSync(cargoPath, cargo.replace(packageSection, `$1${version}$2`))

const packagePath = 'package.json'
const packageJson = JSON.parse(fs.readFileSync(packagePath, 'utf8'))
packageJson.version = version
fs.writeFileSync(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`)
NODE

git add Cargo.toml package.json

git diff --cached --quiet && fail "tidak ada perubahan version untuk di-commit"

git commit -m "chore(release): ${TAG}"
printf 'Release %s\n\n%s\n' "${TAG}" "${CHANGELOG}" | git tag -a "${TAG}" -F -

echo "Mendorong commit dan tag ke origin..."
git push origin "${BRANCH}"
git push origin "${TAG}"

release_args=(
    "${TAG}"
    --verify-tag
    --title "${TAG}"
    --notes-file -
)

if [[ "${VERSION}" == *-* ]]; then
    release_args+=(--prerelease)
fi

gh release create "${release_args[@]}" <<< "${CHANGELOG}"

echo "Release ${TAG} berhasil dibuat."
