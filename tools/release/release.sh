#!/usr/bin/env bash
# 一键发 macOS / Windows 正式版：去掉 -dev → 核对 CHANGELOG → 发版提交与标签 → 推送
# → macOS 本机签名公证发 Release（取消 CI 那份无签名的）→ 等 Windows CI 发完 → 部署官网 → 版本号推到下一个开发版。
#
#   tools/release/release.sh                   # macOS 与 Windows 一起发
#   tools/release/release.sh --macos           # 只发 macOS
#   tools/release/release.sh --windows         # 只发 Windows
#   tools/release/release.sh --yes             # 推送前不再确认
#   tools/release/release.sh --skip-website    # 不部署官网（之后可单独跑 deploy-website.sh）
#
# 发版前要做的：CHANGELOG.md 顶上写好 `## <版本> · 未发布 · <渠道>` 小节（脚本把「未发布」换成今天），
# 产品数据变了先 data-bundle.sh 发新的 data-vN 并提交 data.lock。
# 版本号从 Cargo.toml 的 -dev 版本推出来：mac `0.1.7-dev` 发 `0.1.7`，发完改成 `0.1.8-dev`；win `0.1.0-alpha.6-dev` 发 `0.1.0-alpha.6`，发完 `0.1.0-alpha.7-dev`。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
REPO="OpenFabrica/glimmer"
cd "$ROOT"
# pre-commit / pre-push 钩子跑 clippy 与测试，要用 rustup 钉的工具链（Homebrew 的新版 clippy 规则不同）
export PATH="$HOME/.cargo/bin:$PATH"

MACOS=0
WINDOWS=0
ASSUME_YES=0
WEBSITE=1
for arg in "$@"; do
  case "$arg" in
    --macos) MACOS=1 ;;
    --windows) WINDOWS=1 ;;
    --yes) ASSUME_YES=1 ;;
    --skip-website) WEBSITE=0 ;;
    *) echo "不认识的参数: $arg" >&2; exit 1 ;;
  esac
done
[[ $MACOS == 1 || $WINDOWS == 1 ]] || { MACOS=1; WINDOWS=1; }

WINDOWS_TOMLS=(apps/windows/server/Cargo.toml apps/windows/tsf/Cargo.toml apps/windows/settings/Cargo.toml)
TODAY="$(date +%Y-%m-%d)"

# 打印一步的标题
step() { printf '\n==> %s\n' "$*"; }
# 报错退出
die() { echo "错误: $*" >&2; exit 1; }
# 读 Cargo.toml 的 version
read_version() { grep -m1 '^version' "$1" | sed -E 's/.*"(.*)".*/\1/'; }
# 把 Cargo.toml 的 version 从 $2 改成 $3
set_version() { sed -i '' "s/^version = \"$2\"/version = \"$3\"/" "$1"; }
# 下一个开发版：最后一段数字加一再接 -dev（0.1.6 → 0.1.7-dev，0.1.0-alpha.5 → 0.1.0-alpha.6-dev）
next_dev() { perl -pe 's/(\d+)$/$1+1/e' <<<"$1" | sed 's/$/-dev/'; }

# 核对 CHANGELOG 有该版本的小节：「未发布」换成今天，已有日期就照用，没有就停
prepare_changelog() {
  local version="$1" escaped
  escaped="$(sed 's/\./\\./g' <<<"$version")"
  if grep -qE "^## $escaped · 未发布 · " CHANGELOG.md; then
    sed -i '' -E "s/^## $escaped · 未发布 · /## $version · $TODAY · /" CHANGELOG.md
  elif ! grep -qE "^## $escaped · [0-9]{4}-[0-9]{2}-[0-9]{2} · " CHANGELOG.md; then
    die "CHANGELOG.md 里没有 ## $version 小节，先写好更新说明（标题 ## $version · 未发布 · <渠道>）"
  fi
  local notes
  notes="$(awk -v ver="$version" '$0 ~ "^## " ver " · " {f=1; next} /^## /{f=0} f' CHANGELOG.md | grep -c '^- ' || true)"
  [[ "$notes" -gt 0 ]] || die "CHANGELOG.md 的 ## $version 小节没有条目"
}

# 等某个标签触发的 release.yml 跑起来，输出 run id
find_run() {
  local tag="$1" id=""
  for _ in $(seq 1 30); do
    id="$(gh run list -R "$REPO" --workflow release.yml --branch "$tag" --limit 1 --json databaseId --jq '.[0].databaseId // empty')"
    [[ -n "$id" ]] && { echo "$id"; return 0; }
    sleep 5
  done
  return 1
}

step "检查仓库状态"
[[ "$(git branch --show-current)" == main ]] || die "要在 main 上发版"
[[ -z "$(git status --porcelain --untracked-files=no)" ]] || die "工作区有未提交的改动"
git fetch -q origin main --tags || die "git fetch 失败（本地标签与远端冲突时先 git tag -d 掉那些标签再试）"
[[ "$(git rev-parse HEAD)" == "$(git rev-parse origin/main)" ]] || die "main 与 origin/main 不一致，先 pull / push"
gh auth status >/dev/null 2>&1 || die "gh 没登录"

TAGS=()
SUMMARY=()
NEXT=()
if [[ $MACOS == 1 ]]; then
  MAC_DEV="$(read_version apps/macos/Cargo.toml)"
  [[ "$MAC_DEV" == *-dev ]] || die "apps/macos/Cargo.toml 是 ${MAC_DEV}，不是 -dev 开发版"
  MAC_VERSION="${MAC_DEV%-dev}"
  git rev-parse -q --verify "refs/tags/macos-v$MAC_VERSION" >/dev/null && die "标签 macos-v$MAC_VERSION 已存在"
  TAGS+=("macos-v$MAC_VERSION")
  SUMMARY+=("macOS $MAC_VERSION")
  NEXT+=("macOS $(next_dev "$MAC_VERSION")")
fi
if [[ $WINDOWS == 1 ]]; then
  WIN_DEV="$(read_version apps/windows/server/Cargo.toml)"
  [[ "$WIN_DEV" == *-dev ]] || die "apps/windows/server/Cargo.toml 是 ${WIN_DEV}，不是 -dev 开发版"
  for toml in "${WINDOWS_TOMLS[@]}"; do
    [[ "$(read_version "$toml")" == "$WIN_DEV" ]] || die "$toml 的版本号与 server 不一致"
  done
  WIN_VERSION="${WIN_DEV%-dev}"
  git rev-parse -q --verify "refs/tags/windows-v$WIN_VERSION" >/dev/null && die "标签 windows-v$WIN_VERSION 已存在"
  TAGS+=("windows-v$WIN_VERSION")
  SUMMARY+=("Windows $WIN_VERSION")
  NEXT+=("Windows $(next_dev "$WIN_VERSION")")
fi
TITLE="$(IFS='、'; echo "${SUMMARY[*]}")"

step "发版提交：$TITLE"
[[ $MACOS == 1 ]] && { prepare_changelog "$MAC_VERSION"; set_version apps/macos/Cargo.toml "$MAC_DEV" "$MAC_VERSION"; }
if [[ $WINDOWS == 1 ]]; then
  prepare_changelog "$WIN_VERSION"
  for toml in "${WINDOWS_TOMLS[@]}"; do set_version "$toml" "$WIN_DEV" "$WIN_VERSION"; done
fi
# 刷新 Cargo.lock 里壳的版本号，CI 全 --locked
cargo metadata --format-version 1 >/dev/null
git add CHANGELOG.md Cargo.lock apps/macos/Cargo.toml "${WINDOWS_TOMLS[@]}"
git commit -q -m "chore(release): $TITLE"
for tag in "${TAGS[@]}"; do
  case "$tag" in
    macos-v*) git tag -a "$tag" -m "微明 macOS $MAC_VERSION" ;;
    windows-v*) git tag -a "$tag" -m "微明 Windows $WIN_VERSION" ;;
  esac
done
git --no-pager show --stat HEAD

if [[ $ASSUME_YES == 0 ]]; then
  read -r -p "推送 main 与标签 ${TAGS[*]}（推了就会触发 CI 发版）？[y/N] " answer
  if [[ "$answer" != y && "$answer" != Y ]]; then
    git tag -d "${TAGS[@]}" >/dev/null
    git reset -q --hard HEAD~1
    die "已取消，发版提交与标签都撤回了"
  fi
fi

step "推送"
git push origin main "${TAGS[@]}"

if [[ $MACOS == 1 ]]; then
  step "取消 CI 的 macOS 无签名构建"
  if run="$(find_run "macos-v$MAC_VERSION")"; then
    gh run cancel "$run" -R "$REPO" || true
  else
    echo "没找到 macos-v$MAC_VERSION 的 CI 运行，跳过"
  fi
  step "本机打包、签名、公证、发 Release"
  tools/release/macos-local.sh
fi

if [[ $WINDOWS == 1 ]]; then
  step "等 Windows CI 发版"
  run="$(find_run "windows-v$WIN_VERSION")" || die "没找到 windows-v$WIN_VERSION 的 CI 运行"
  gh run watch "$run" -R "$REPO" --exit-status >/dev/null || die "Windows CI 失败：gh run view $run -R $REPO --log-failed"
  gh release view "windows-v$WIN_VERSION" -R "$REPO" --json assets --jq '.assets[].name' | grep -q -- '-Setup.exe$' \
    || die "Release windows-v$WIN_VERSION 上没有安装包"
  echo "Windows $WIN_VERSION 已发布"
fi

if [[ $MACOS == 1 && $WINDOWS == 1 ]]; then
  # 两边都会把 releases.json 覆盖到 latest 上，后发完的那次可能没见到另一版，最后按全部 Release 再生成一次
  step "重新生成 releases.json"
  GH_REPO="$REPO" GITHUB_REPOSITORY="$REPO" tools/release/publish-releases-json.sh "macos-v$MAC_VERSION" "target/pkg/macos-v$MAC_VERSION" >/dev/null
fi

if [[ $WEBSITE == 1 ]]; then
  step "部署官网"
  tools/release/deploy-website.sh "${TAGS[@]}"
fi

NEXT_TITLE="$(IFS='、'; echo "${NEXT[*]}")"
step "版本号推到下一个开发版：$NEXT_TITLE"
[[ $MACOS == 1 ]] && set_version apps/macos/Cargo.toml "$MAC_VERSION" "$(next_dev "$MAC_VERSION")"
if [[ $WINDOWS == 1 ]]; then
  for toml in "${WINDOWS_TOMLS[@]}"; do set_version "$toml" "$WIN_VERSION" "$(next_dev "$WIN_VERSION")"; done
fi
cargo metadata --format-version 1 >/dev/null
git add Cargo.lock apps/macos/Cargo.toml "${WINDOWS_TOMLS[@]}"
git commit -q -m "chore(release): 版本号推到下一个开发版（${NEXT_TITLE}）"
git push origin main

echo
echo "发版完成：$TITLE"
for tag in "${TAGS[@]}"; do echo "  https://github.com/$REPO/releases/tag/$tag"; done
