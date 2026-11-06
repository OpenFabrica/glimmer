#!/usr/bin/env bash
# 把 glimmer-linux（IBus 引擎进程 glimmer-ibus）与 Fcitx5 插件（glimmer-fcitx5 静态库 + C++ 插件 glimmer.so）打成同一个 deb。只在 Linux 上跑（CI 的 ubuntu runner 或 Docker 里）；
# macOS 本机用同目录的 package-docker.sh。
#
#   apps/linux/scripts/package.sh     # 出 target/deb/Glimmer-<版本>-<amd64|arm64>.deb
#
# 需要 cargo、dpkg-deb、dpkg-shlibdeps（dpkg-dev）；Fcitx5 插件另要 cmake、g++、extra-cmake-modules、
# libfcitx5core-dev、libfcitx5config-dev、libfcitx5utils-dev、fcitx5-modules-dev（Fcitx5 5.1 起，ubuntu 24.04 有）。
# 发版包必须两个框架都带：缺 cmake 或 Fcitx5 开发包时直接失败，本地只打 IBus 包用 GLIMMER_SKIP_FCITX5=1。
# 架构：缺省编译本机架构；GLIMMER_TARGET=x86_64-unknown-linux-gnu / aarch64-unknown-linux-gnu 交叉编译（要自备链接器）。
# 产品数据：data/generated/ 里有 dict.qj 就按产品数据装（缺哪个随包文件打警告），没有就只带 assets/sample/ 样例词库并打警告；
# 与 bundle.sh 不同，这里不从 TSV 重打 .qj，CI 从 data Release 解出的就是 .qj，本机先跑过 bundle.sh 或 data-bundle.sh 流程。
# 本地整句模型 data/model/model.qjm、五笔码表 data/generated/wubi{86,98,xsj}.qj 有就带，没有就跳过。
#
# 版本号（apps/linux/Cargo.toml 的 version，各平台壳独立）：
#   - 文件名用 Cargo 原样的版本；带 -dev 时接 git 短哈希（工作区有改动再加 +）：Glimmer-0.1.0-linux.1-dev-1a2b3c4-arm64.deb
#   - deb 的 Version 字段把 - 换成 ~（~ 排在一切之前，预发布版低于正式版）：0.1.0-linux.1 → 0.1.0~linux.1；
#     dev 版再接 +g<短哈希>（有改动加 .dirty）：0.1.0~linux.1~dev+g1a2b3c4，排在 0.1.0~linux.1 之前。
#
# 预编产物（发版 CI 用：两半在不同系统上编，这里只装包）：
#   GLIMMER_IBUS_BIN      现成的 glimmer-ibus，设了就不 cargo build glimmer-linux。CI 在 Ubuntu 22.04 上编它，
#                         Depends 只由它经 dpkg-shlibdeps 算出，所以 22.04 / Debian 12 起都能装
#   GLIMMER_FCITX5_STAGE  现成的 Fcitx5 插件安装目录（apps/linux/fcitx5/addon 的 `DESTDIR=<目录> cmake --install`，前缀 /usr），
#                         设了就整个拷进 staging，不编静态库也不跑 cmake。CI 在 Ubuntu 24.04 上编（要 Fcitx5 5.1 开发包）
#   都不设时在本机一把编完（本地 Docker），Depends 跟着构建机的 glibc / OpenSSL 走，会打警告；发版包由 CI 分开编。
#   打包本身（dpkg-shlibdeps）也要在 22.04 上跑，24.04 的 shlibs 会把 libssl3 写成 libssl3t64。
# 调试用环境变量：
#   GLIMMER_SKIP_FCITX5=1  不带 Fcitx5 插件，只出 IBus 包（本地调试用，发版不能用）
#   GLIMMER_GIT_REV  git 短哈希（Docker 里的 worktree 读不到 git 时由 package-docker.sh 从宿主传进来，脏的带 +）
#
# 布局：/usr/lib/glimmer/glimmer-ibus，资源与它同级（data/ 与 assets/，glimmer_platform::resources::bundled_root 认这个）；
# IBus 组件 /usr/share/ibus/component/glimmer.xml；Fcitx5 插件 /usr/lib/<multiarch>/fcitx5/glimmer.so 与
# /usr/share/fcitx5/{addon,inputmethod}/glimmer.conf（CMake 安装规则决定，插件从 /usr/lib/glimmer 读同一份资源）；
# 图标 /usr/share/icons/hicolor/256x256/apps/glimmer.png；许可 /usr/share/doc/glimmer/copyright。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
BIN_NAME="glimmer-ibus"
PACKAGING="$ROOT/apps/linux/packaging"
TARGET="${GLIMMER_TARGET:-}"
case "${TARGET:-$(uname -m)}" in
  x86_64-unknown-linux-gnu|x86_64) ARCH="amd64" ;;
  aarch64-unknown-linux-gnu|aarch64|arm64) ARCH="arm64" ;;
  *) echo "不认识的架构: ${TARGET:-$(uname -m)}" >&2; exit 1 ;;
esac
WITH_FCITX5=1
if [[ "${GLIMMER_SKIP_FCITX5:-}" == 1 ]]; then
  WITH_FCITX5=0
  echo "注意: GLIMMER_SKIP_FCITX5=1，包里不带 Fcitx5 插件（不能用于发版）" >&2
fi

# 预编产物的相对路径按调用时的当前目录解析（下面会 cd 到仓库根）
IBUS_BIN="${GLIMMER_IBUS_BIN:-}"
[[ -n "$IBUS_BIN" ]] && IBUS_BIN="$(cd "$(dirname "$IBUS_BIN")" && pwd)/$(basename "$IBUS_BIN")"
FCITX5_STAGE_IN=""
if [[ $WITH_FCITX5 == 1 && -n "${GLIMMER_FCITX5_STAGE:-}" ]]; then
  FCITX5_STAGE_IN="$(cd "$GLIMMER_FCITX5_STAGE" && pwd)"
fi
# 本机编 Fcitx5 插件时工具链先查，免得 cargo 编完几分钟才发现缺东西
if [[ $WITH_FCITX5 == 1 && -z "$FCITX5_STAGE_IN" ]]; then
  # 插件是本机 C++ 编译，交叉编译要另配 CMake 工具链，这里不支持
  if [[ -n "$TARGET" ]]; then
    echo "交叉编译（GLIMMER_TARGET）不支持 Fcitx5 插件；在目标架构上打包，或设 GLIMMER_SKIP_FCITX5=1" >&2
    exit 1
  fi
  for tool in cmake g++; do
    command -v "$tool" >/dev/null 2>&1 || {
      echo "缺 $tool，编不了 Fcitx5 插件：apt install cmake g++ extra-cmake-modules libfcitx5core-dev libfcitx5config-dev libfcitx5utils-dev fcitx5-modules-dev（或设 GLIMMER_SKIP_FCITX5=1 只打 IBus 包）" >&2
      exit 1
    }
  done
fi

cd "$ROOT"
GIT_REV="${GLIMMER_GIT_REV:-}"
if [[ -z "$GIT_REV" ]]; then
  GIT_REV="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
  if [[ -n "$(git status --porcelain 2>/dev/null)" ]]; then GIT_REV="${GIT_REV}+"; fi
fi
# 构建标识进诊断信息（与 bundle.sh 同一个编译期变量）
export GLIMMER_BUILD="${GIT_REV} · $(date +%Y-%m-%d)"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' apps/linux/Cargo.toml | head -1)"
[[ -n "$VERSION" ]] || { echo "apps/linux/Cargo.toml 里没找到 version" >&2; exit 1; }
if [[ "$VERSION" == *-dev ]]; then
  FILE_VERSION="$VERSION-$GIT_REV"
  REV_CLEAN="${GIT_REV%+}"
  DEB_VERSION="${VERSION//-/\~}+g$REV_CLEAN"
  [[ "$GIT_REV" == *+ ]] && DEB_VERSION="$DEB_VERSION.dirty"
else
  FILE_VERSION="$VERSION"
  DEB_VERSION="${VERSION//-/\~}"
fi

# 认 CARGO_TARGET_DIR（package-docker.sh 把容器里的 target 放 target/linux-docker*）；CMake 要绝对路径
TARGET_DIR="$(mkdir -p "${CARGO_TARGET_DIR:-target}" && cd "${CARGO_TARGET_DIR:-target}" && pwd)"
# 没给的那一半才 cargo build；两半都要编时一次 cargo 调用编完
BUILD_ARGS=(build --release --locked)
if [[ -n "$IBUS_BIN" ]]; then
  BIN="$IBUS_BIN"
  echo "用预编的 glimmer-ibus：$BIN" >&2
else
  BUILD_ARGS+=(-p glimmer-linux)
  echo "警告: 没给 GLIMMER_IBUS_BIN，在本机编 glimmer-ibus：Depends 由构建机决定，发版包由 CI 分开编" >&2
fi
if [[ -n "$FCITX5_STAGE_IN" ]]; then
  echo "用预编的 Fcitx5 插件：$FCITX5_STAGE_IN" >&2
elif [[ $WITH_FCITX5 == 1 ]]; then
  BUILD_ARGS+=(-p glimmer-fcitx5)
fi
RUST_OUT="$TARGET_DIR/release"
if [[ -n "$TARGET" ]]; then
  BUILD_ARGS+=(--target "$TARGET")
  RUST_OUT="$TARGET_DIR/$TARGET/release"
fi
[[ -n "$IBUS_BIN" ]] || BIN="$RUST_OUT/$BIN_NAME"
RUST_LIB="$RUST_OUT/libglimmer_fcitx5.a"
# 除了 build --release --locked 还有 -p 才需要编
if [[ " ${BUILD_ARGS[*]} " == *" -p "* ]]; then
  cargo "${BUILD_ARGS[@]}"
fi
[[ -x "$BIN" ]] || { echo "缺二进制 $BIN" >&2; exit 1; }

# 每个架构一个工作目录，成品都放 target/deb/，两个架构接着打互不覆盖
WORK="$ROOT/target/deb/$ARCH"
STAGE="$WORK/root"
LIB="$STAGE/usr/lib/glimmer"
DEB="$ROOT/target/deb/Glimmer-$FILE_VERSION-$ARCH.deb"
rm -rf "$WORK"
mkdir -p "$STAGE/DEBIAN" "$LIB" "$STAGE/usr/share/ibus/component" \
  "$STAGE/usr/share/icons/hicolor/256x256/apps" "$STAGE/usr/share/doc/glimmer"

install -m 755 "$BIN" "$LIB/$BIN_NAME"
# strip_native <文件>：本机架构才 strip（交叉编译的二进制本机 strip 认不了）；调试符号不随包，预编产物也在这里 strip
strip_native() {
  if [[ -z "$TARGET" ]] && command -v strip >/dev/null 2>&1; then
    strip --strip-unneeded "$1" || echo "注意: strip $1 失败，按原样打包" >&2
  fi
}
strip_native "$LIB/$BIN_NAME"

# Fcitx5 插件：C++ 半边链进 Rust 静态库，按 CMake 的安装规则装进 staging（路径由 Fcitx5 的 CMake 配置给出，带 multiarch）。
# 给了 GLIMMER_FCITX5_STAGE 就拷那份安装结果；否则本机编，找不到 Fcitx5 开发包时 cmake 配置那一步失败，整个打包跟着失败
if [[ -n "$FCITX5_STAGE_IN" ]]; then
  [[ -d "$FCITX5_STAGE_IN/usr" ]] || { echo "GLIMMER_FCITX5_STAGE=$FCITX5_STAGE_IN 下没有 usr/（应是 cmake --install 的 DESTDIR）" >&2; exit 1; }
  cp -R "$FCITX5_STAGE_IN/usr/." "$STAGE/usr/"
elif [[ $WITH_FCITX5 == 1 ]]; then
  [[ -f "$RUST_LIB" ]] || { echo "缺静态库 $RUST_LIB" >&2; exit 1; }
  CMAKE_BUILD="$TARGET_DIR/fcitx5-build/$ARCH"
  cmake -S apps/linux/fcitx5/addon -B "$CMAKE_BUILD" -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr \
    -DGLIMMER_RUST_LIB="$RUST_LIB" -DGLIMMER_DATA_ROOT=/usr/lib/glimmer || {
    echo "Fcitx5 插件 CMake 配置失败：确认装了 extra-cmake-modules libfcitx5core-dev libfcitx5config-dev libfcitx5utils-dev fcitx5-modules-dev（或设 GLIMMER_SKIP_FCITX5=1 只打 IBus 包）" >&2
    exit 1
  }
  cmake --build "$CMAKE_BUILD" -j "$(nproc)"
  DESTDIR="$STAGE" cmake --install "$CMAKE_BUILD"
fi
if [[ $WITH_FCITX5 == 1 ]]; then
  FCITX5_SO="$(find "$STAGE/usr/lib" -path '*/fcitx5/glimmer.so' | head -1)"
  [[ -n "$FCITX5_SO" ]] || { echo "CMake 安装后没找到 fcitx5/glimmer.so" >&2; exit 1; }
  for f in addon/glimmer.conf inputmethod/glimmer.conf; do
    [[ -f "$STAGE/usr/share/fcitx5/$f" ]] || { echo "CMake 安装后缺 /usr/share/fcitx5/$f" >&2; exit 1; }
  done
  strip_native "$FCITX5_SO"
  chmod 644 "$FCITX5_SO"
  echo "Fcitx5 插件：${FCITX5_SO#"$STAGE"}"
fi

# 随 git 的资源：与 glimmer.iss 的清单一致
install -D -m 644 assets/emoji/emoji-zh.tsv "$LIB/assets/emoji/emoji-zh.tsv"
install -D -m 644 assets/emoji/emoji-en.tsv "$LIB/assets/emoji/emoji-en.tsv"
install -D -m 644 assets/levels/levels-en.tsv "$LIB/assets/levels/levels-en.tsv"
install -D -m 644 assets/levels/levels-ja.tsv "$LIB/assets/levels/levels-ja.tsv"
install -D -m 644 assets/sample/dict.tsv "$LIB/assets/sample/dict.tsv"

# AI 数据许可与署名随包，正文语料只用于离线生成，不安装。
install -d -m 755 "$LIB/assets/lexicon/ai"
install -m 644 assets/lexicon/ai/LICENSE.* assets/lexicon/ai/sources.json assets/lexicon/ai/README.md "$LIB/assets/lexicon/ai/"
# 产品数据：只装运行时要的 .qj / .tsv，不装 dev 中间产物
GEN="data/generated"
if [[ -f "$GEN/dict.qj" ]]; then
  for f in dict.qj lm.qj glossary-en.qj glossary-ja.qj glossary-zh.qj english.tsv; do
    if [[ -f "$GEN/$f" ]]; then
      install -D -m 644 "$GEN/$f" "$LIB/data/generated/$f"
    else
      echo "警告: 缺 $GEN/$f，包里不带它" >&2
    fi
  done
  # 英文语言模型与误拼表可选：没有就不带，英文候选按词频排、少一档纠错
  for f in lm-en.qj english-misspellings.tsv; do
    if [[ -f "$GEN/$f" ]]; then
      install -D -m 644 "$GEN/$f" "$LIB/data/generated/$f"
    else
      echo "注意: 没有 $GEN/$f，包里不带它" >&2
    fi
  done
  if ls "$GEN"/dicts/*.qj >/dev/null 2>&1; then
    install -d -m 755 "$LIB/data/generated/dicts"
    install -m 644 "$GEN"/dicts/*.qj "$LIB/data/generated/dicts/"
  else
    echo "警告: 没有 $GEN/dicts/*.qj，包里不带领域词库" >&2
  fi
  echo "使用 $GEN/ 的产品数据（自建词库）"
else
  echo "警告: 没有 $GEN/dict.qj，只带 assets/sample/ 样例词库（不是产品词库，分发前先下载或生成产品数据）" >&2
fi
# 五笔码表三版（86 / 98 / 新世纪，各自的许可与署名随包）：缺哪版就不装哪版，引擎选到缺的版本时当没开五笔
for v in wubi86 wubi98 wubixsj; do
  if [[ -f "$GEN/$v.qj" ]]; then
    install -D -m 644 "$GEN/$v.qj" "$LIB/data/generated/$v.qj"
    # 各版带的许可 / 署名文件不一样（98 只有 LICENSE.LGPL-3.0，新世纪只有 AUTHORS）：
    # LICENSE* 是通配，set -euo pipefail 下无匹配时 shell 原样返回带 * 的串，所以逐个判存在；
    # 这里用 if 而不是 [[ … ]] && …，后者判假会让循环以非零状态结束，set -e 直接掐掉整个打包
    for f in "assets/wubi/$v/"LICENSE* "assets/wubi/$v/AUTHORS"; do
      if [[ -f "$f" ]]; then
        install -D -m 644 "$f" "$LIB/assets/wubi/$v/$(basename "$f")"
      fi
    done
    echo "打包五笔码表：$GEN/$v.qj"
  else
    echo "注意: 没有 $GEN/$v.qj，包里不带这版五笔码表（生成命令见 assets/wubi/$v/README.md）" >&2
  fi
done
# 本地整句模型单文件（tools/release/pack-model.sh 打成）：没有就不装，引擎不重排
MODEL="${GLIMMER_MODEL_DIR:-data/model}/model.qjm"
if [[ -f "$MODEL" ]]; then
  install -D -m 644 "$MODEL" "$LIB/data/model/model.qjm"
  echo "打包本地整句模型：$MODEL"
fi

# IBus 组件、图标、许可
sed -e "s/@VERSION@/$FILE_VERSION/g" "$PACKAGING/glimmer.xml.in" > "$STAGE/usr/share/ibus/component/glimmer.xml"
chmod 644 "$STAGE/usr/share/ibus/component/glimmer.xml"
install -m 644 "$PACKAGING/glimmer.png" "$STAGE/usr/share/icons/hicolor/256x256/apps/glimmer.png"
install -m 644 "$PACKAGING/copyright" "$STAGE/usr/share/doc/glimmer/copyright"

# 动态库依赖由 dpkg-shlibdeps 只从 IBus 引擎二进制算（它要求当前目录有 debian/control，给它一个最小的）；算不出来退回只依赖 libc6。
# 插件 .so 不进来：它在 24.04 上编，会把 libc6 (>= 2.39) 与 libfcitx5core 带进 Depends，逼 22.04 / Debian 12 的 IBus 用户装不上；
# 它要的 libstdc++ / libfcitx5* 由 fcitx5 (>= 5.1) 包带来，OpenSSL 与引擎共用
SHLIBS_DEPENDS="libc6"
mkdir -p "$WORK/shlibs/debian"
printf 'Source: glimmer\n\nPackage: glimmer\nArchitecture: any\n' > "$WORK/shlibs/debian/control"
if deps="$(cd "$WORK/shlibs" && dpkg-shlibdeps -O -e "$LIB/$BIN_NAME" 2>/dev/null)"; then
  deps="$(printf '%s\n' "$deps" | sed -n 's/^shlibs:Depends=//p')"
  [[ -n "$deps" ]] && SHLIBS_DEPENDS="$deps"
else
  echo "注意: dpkg-shlibdeps 失败，Depends 只写 libc6" >&2
fi
rm -rf "$WORK/shlibs"

INSTALLED_SIZE="$(du -sk --exclude=DEBIAN "$STAGE" | cut -f1)"
sed -e "s/@DEB_VERSION@/$DEB_VERSION/g" -e "s/@ARCH@/$ARCH/g" \
  -e "s/@INSTALLED_SIZE@/$INSTALLED_SIZE/g" -e "s/@SHLIBS_DEPENDS@/$SHLIBS_DEPENDS/g" \
  "$PACKAGING/control.in" > "$STAGE/DEBIAN/control"
install -m 755 "$PACKAGING/postinst" "$STAGE/DEBIAN/postinst"
install -m 755 "$PACKAGING/postrm" "$STAGE/DEBIAN/postrm"

# 目录一律 755（宿主 umask 或 Docker 挂载可能给出别的权限）
find "$STAGE" -type d -exec chmod 755 {} +
dpkg-deb --build --root-owner-group -Zxz "$STAGE" "$DEB"
echo "deb: $DEB（Version $DEB_VERSION，$ARCH）"
sha256sum "$DEB"
