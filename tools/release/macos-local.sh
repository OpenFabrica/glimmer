#!/usr/bin/env bash
# 本机出 macOS 正式包并发 GitHub Release：签名 → 公证 → 钉票据 → SHA256SUMS / build-info.json → 建 Release → 更新 releases.json。
# CI 没配 Apple 签名 secrets，推 macos-v* 标签后 CI 只会出无签名包（release.sh 会取消它），正式包都在本机出。
#
#   tools/release/macos-local.sh               # 版本取 apps/macos/Cargo.toml，标签 macos-v<版本> 要已推送且指向 HEAD
#   tools/release/macos-local.sh --no-publish  # 只打包、公证、生成校验文件，不建 Release
#   tools/release/macos-local.sh --fetch-data  # 先按 data.lock 下载产品数据覆盖 data/（缺省只校验本机数据与锁文件一致）
#
# 环境变量：
#   GLIMMER_SIGN_IDENTITY / GLIMMER_INSTALLER_IDENTITY  缺省是 Jiangwei Lan 的 Developer ID
#   GLIMMER_NOTARY_PROFILE  notarytool keychain profile；没设就从仓库根的 .env 读 APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID
# 产物：target/pkg/macos-v<版本>/（两个 pkg、SHA256SUMS、build-info.json、notes.md、releases.json）
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
REPO="OpenFabrica/glimmer"
cd "$ROOT"
# Homebrew 的 Rust 排在 PATH 前面时没有 x86_64 std、clippy 也与 CI 不同，统一用 rustup 钉的工具链
export PATH="$HOME/.cargo/bin:$PATH"

PUBLISH=1
FETCH_DATA=0
for arg in "$@"; do
  case "$arg" in
    --no-publish) PUBLISH=0 ;;
    --fetch-data) FETCH_DATA=1 ;;
    *) echo "不认识的参数: $arg" >&2; exit 1 ;;
  esac
done

export GLIMMER_SIGN_IDENTITY="${GLIMMER_SIGN_IDENTITY:-Developer ID Application: Jiangwei Lan (HQ537XMLJY)}"
export GLIMMER_INSTALLER_IDENTITY="${GLIMMER_INSTALLER_IDENTITY:-Developer ID Installer: Jiangwei Lan (HQ537XMLJY)}"

# 打印一步的标题
step() { printf '\n==> %s\n' "$*"; }
# 报错退出
die() { echo "错误: $*" >&2; exit 1; }
# 算文件的 SHA-256
sha256() { shasum -a 256 "$1" | cut -d' ' -f1; }
# 读 data.lock 里某个键的值
lock_value() { grep -E "^$1 *= *" tools/release/data.lock | head -1 | sed -E 's/^[^=]*= *//' | tr -d '[:space:]'; }

VERSION="$(grep -m1 '^version' apps/macos/Cargo.toml | sed -E 's/.*"(.*)".*/\1/')"
TAG="macos-v$VERSION"
OUT="target/pkg/$TAG"

step "检查 $TAG"
[[ "$VERSION" != *-dev* ]] || die "apps/macos/Cargo.toml 还是开发版 ${VERSION}，先改成正式版本号再打标签"
[[ -z "$(git status --porcelain --untracked-files=no)" ]] || die "工作区有未提交的改动，正式包必须对应一个干净的提交"
git rev-parse -q --verify "refs/tags/$TAG" >/dev/null || die "本地没有标签 $TAG"
[[ "$(git rev-parse "$TAG^{commit}")" == "$(git rev-parse HEAD)" ]] || die "$TAG 没有指向 HEAD，先 git checkout $TAG"
if [[ $PUBLISH == 1 ]]; then
  git ls-remote --exit-code --tags origin "refs/tags/$TAG" >/dev/null || die "标签 $TAG 还没推到 origin"
  ! gh release view "$TAG" -R "$REPO" >/dev/null 2>&1 || die "Release $TAG 已存在；要重发先 gh release delete $TAG -R $REPO"
fi
grep -qE "^## $VERSION · [0-9]{4}-[0-9]{2}-[0-9]{2} · " CHANGELOG.md || die "CHANGELOG.md 里没有带日期的 ## $VERSION 小节"
rustup target list --installed | grep -q x86_64-apple-darwin || die "缺 x86_64 目标：rustup target add x86_64-apple-darwin"

step "核对产品数据与 data.lock（$(lock_value tag)）"
if [[ $FETCH_DATA == 1 ]]; then
  tools/release/data-fetch.sh
else
  # 数据包里的每个文件都要与本机 data/generated 一致，模型与锁文件一致；不一致就停，免得打出与锁文件不符的包
  # 只下载到 target/release-data 并按锁文件校验；不带参数的 data-fetch.sh 会解包覆盖 data/，这里不用它
  mkdir -p target/release-data
  rm -f target/release-data/glimmer-data.tar.gz target/release-data/model.qjm
  gh release download "$(lock_value tag)" -R "$REPO" --pattern glimmer-data.tar.gz --pattern model.qjm --dir target/release-data
  tools/release/data-fetch.sh --verify >/dev/null
  check_dir="$(mktemp -d)"
  trap 'rm -rf "$check_dir"' EXIT
  tar -xzf target/release-data/glimmer-data.tar.gz -C "$check_dir"
  mismatch=0
  while IFS= read -r file; do
    rel="${file#"$check_dir"/}"
    if [[ ! -f "data/generated/$rel" || "$(sha256 "$file")" != "$(sha256 "data/generated/$rel")" ]]; then
      echo "  不一致: data/generated/$rel" >&2
      mismatch=1
    fi
  done < <(find "$check_dir" -type f)
  [[ "$(sha256 data/model/model.qjm)" == "$(lock_value model.qjm)" ]] || { echo "  不一致: data/model/model.qjm" >&2; mismatch=1; }
  [[ $mismatch == 0 ]] || die "本机数据与 data.lock 不符：加 --fetch-data 用锁定的数据覆盖，或先 data-bundle.sh 发新数据"
  echo "本机数据与 $(lock_value tag) 一致"
fi

step "打包并签名（arm64、x86_64）"
apps/macos/scripts/bundle.sh --pkg
GLIMMER_TARGET=x86_64-apple-darwin apps/macos/scripts/bundle.sh --pkg
BUILT_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
rm -rf "$OUT"
mkdir -p "$OUT"
PKGS=("Glimmer-$VERSION-arm64.pkg" "Glimmer-$VERSION-x86_64.pkg")
for pkg in "${PKGS[@]}"; do
  cp "target/pkg/$pkg" "$OUT/"
  pkgutil --check-signature "$OUT/$pkg" | grep -q "Developer ID Installer" || die "$pkg 没有用 Developer ID Installer 签名"
done

step "公证并钉票据"
if [[ -z "${GLIMMER_NOTARY_PROFILE:-}" ]]; then
  [[ -f .env ]] || die "没有 GLIMMER_NOTARY_PROFILE，也没有 .env"
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
  [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]] || die ".env 里缺 APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID"
fi
for pkg in "${PKGS[@]}"; do
  if [[ -n "${GLIMMER_NOTARY_PROFILE:-}" ]]; then
    xcrun notarytool submit "$OUT/$pkg" --keychain-profile "$GLIMMER_NOTARY_PROFILE" --wait
  else
    xcrun notarytool submit "$OUT/$pkg" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" --wait
  fi
  xcrun stapler staple "$OUT/$pkg"
  # notarytool 被拒时退出码也可能是 0，最后以 Gatekeeper 的判定为准
  spctl -a -vv -t install "$OUT/$pkg" 2>&1 | grep -q "Notarized Developer ID" || die "$pkg 没通过 Gatekeeper 公证检查"
done

step "生成 SHA256SUMS、build-info.json、更新说明"
(cd "$OUT" && shasum -a 256 "${PKGS[@]/#/./}" > SHA256SUMS)
jq -n --arg version "$VERSION" --arg tag "$TAG" --arg commit "$(git rev-parse HEAD)" --arg built_at "$BUILT_AT" \
  --arg toolchain "$(rustc --version)" --arg runner "macOS $(sw_vers -productVersion) (本机)" \
  --arg data_tag "$(lock_value tag)" --arg data_sha256 "$(lock_value glimmer-data.tar.gz)" --arg model_sha256 "$(lock_value model.qjm)" \
  '{version: $version, tag: $tag, commit: $commit, built_at: $built_at, toolchain: $toolchain, runner: $runner, data_tag: $data_tag, data_sha256: $data_sha256, model_sha256: $model_sha256}' \
  > "$OUT/build-info.json"
awk -v ver="$VERSION" '$0 ~ "^## " ver " · " {f=1; next} /^## /{f=0} f' CHANGELOG.md | sed '/^$/d' > "$OUT/notes.md"
cat "$OUT/SHA256SUMS"

if [[ $PUBLISH == 0 ]]; then
  echo
  echo "已打好：${OUT}（--no-publish，没有建 Release）"
  exit 0
fi

step "建 Release $TAG"
gh release create "$TAG" -R "$REPO" --title "微明 $VERSION" --notes-file "$OUT/notes.md" --verify-tag --latest \
  "$OUT/${PKGS[0]}" "$OUT/${PKGS[1]}" "$OUT/SHA256SUMS" "$OUT/build-info.json"

step "更新 releases.json"
GH_REPO="$REPO" GITHUB_REPOSITORY="$REPO" tools/release/publish-releases-json.sh "$TAG" "$OUT" >/dev/null
echo "完成：https://github.com/$REPO/releases/tag/$TAG"
