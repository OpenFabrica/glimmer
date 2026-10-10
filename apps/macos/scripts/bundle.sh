#!/usr/bin/env bash
# 把 glimmer-macos 打包成 Glimmer.app。
#
#   scripts/bundle.sh            # 只打包到 target/bundle.noindex/Glimmer.app
#   scripts/bundle.sh --install  # 打包并安装（开发用）：/Library/Input Methods/ 已有 pkg 装的正式版就 sudo 覆盖它，否则装 ~/Library/Input Methods/；杀掉旧进程
#   scripts/bundle.sh --pkg      # 打包并做成 target/pkg/Glimmer-<版本>-<arm64|x86_64>.pkg（分发给测试者）
#
# 架构：缺省编译本机架构；GLIMMER_TARGET=x86_64-apple-darwin（或 aarch64-apple-darwin）交叉编译另一种，
# 先 `rustup target add` 一次。CI 在 Apple Silicon runner 上两个都打（.github/workflows/release.yml）。
# 产品数据：data/generated/ 里有 dict.qj 就用自建词库（TSV 比 .qj 新会重打），没有就退回 assets/sample/ 样例。
#
# 签名与公证都由环境变量决定，没设就 ad-hoc 签名、pkg 不签（本机自用够了，分发给别人会被 Gatekeeper 拦，
# 对方要在「系统设置 → 隐私与安全性」里点「仍要打开」）：
#   GLIMMER_SIGN_IDENTITY       "Developer ID Application: …"   给 .app 签名（hardened runtime）
#   GLIMMER_INSTALLER_IDENTITY  "Developer ID Installer: …"     给 .pkg 签名
#   GLIMMER_NOTARY_PROFILE      notarytool store-credentials 存的 keychain profile 名，设了就公证并钉票据
#
# 首次 --install 后要在「系统设置 → 键盘 → 输入法」里添加「微明」；输入法列表不刷新就注销再登录。
# pkg 装的不用：postinstall 会以登录用户身份跑 `glimmer-macos --register` 注册并启用。
#
# 同一时刻机器上只能有一份 Glimmer.app：~/Library 与 /Library 各一份时系统按 bundle ID 拉起会挑错路径，
# 选了输入法立刻退回上一个（2026-09-18 踩到）。所以 --install 跟着现有安装位置走，pkg 的 postinstall 反过来把 ~/Library 的开发版挪走；
# 同一路径反复覆盖不用注销，换了路径（~/Library ↔ /Library）要注销再登录，系统只在登录时重扫输入法。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
APP_NAME="Glimmer"
BIN_NAME="glimmer-macos"
PROFILE="${PROFILE:-release}"
# 成品放 .noindex 目录：Spotlight / Launch Services 不扫描它，构建出来的 .app 就不会被登记成又一份同 ID 的输入法
# （登记多份时启用与切换会失灵，「添加输入法」里出现重复条目）。
APP="$ROOT/target/bundle.noindex/$APP_NAME.app"
# 开发版装哪：/Library 已有正式版就覆盖它（要 sudo），否则 ~/Library；两处不能并存，见文件头
SYSTEM_INSTALL_DIR="/Library/Input Methods"
if [[ -d "$SYSTEM_INSTALL_DIR/$APP_NAME.app" ]]; then
  INSTALL_DIR="$SYSTEM_INSTALL_DIR"
else
  INSTALL_DIR="$HOME/Library/Input Methods"
fi
# 目标三元组为空就是本机；架构名按 pkg 文件名与 distribution.xml 的 hostArchitectures 用的写法（arm64 / x86_64）
TARGET="${GLIMMER_TARGET:-}"
case "${TARGET:-$(uname -m)}" in
  aarch64-apple-darwin|arm64) ARCH="arm64" ;;
  x86_64-apple-darwin|x86_64) ARCH="x86_64" ;;
  *) echo "不认识的架构: ${TARGET:-$(uname -m)}" >&2; exit 1 ;;
esac

cd "$ROOT"
# 构建标识进「关于」页与诊断信息：git 短哈希（工作区有改动加 +）与日期；编译期 option_env! 读
GIT_REV="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
if [[ -n "$(git status --porcelain 2>/dev/null)" ]]; then GIT_REV="${GIT_REV}+"; fi
export GLIMMER_BUILD="${GIT_REV} · $(date +%Y-%m-%d)"
BUILD_ARGS=(-p "$BIN_NAME" --locked)
[[ "$PROFILE" == "release" ]] && BUILD_ARGS+=(--release)
BIN_DIR="target/$PROFILE"
if [[ -n "$TARGET" ]]; then
  BUILD_ARGS+=(--target "$TARGET")
  BIN_DIR="target/$TARGET/$PROFILE"
fi
cargo build "${BUILD_ARGS[@]}"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN_DIR/$BIN_NAME" "$APP/Contents/MacOS/$BIN_NAME"
cp apps/macos/Info.plist "$APP/Contents/Info.plist"
# 版本号来自 apps/macos/Cargo.toml（各平台壳版本号独立，不跟 workspace 走），构建号用提交数（单调递增，pkg 升级判断靠它）。
# 发版之间版本号带 -dev（0.1.2-dev）：本地与 CI 中间构建一眼能与线上包区分；发版提交去掉 -dev 再打标签（docs/notes/release.md）。
# 开发版再接上 git 短哈希（0.1.3-dev-1a2b3c4，工作区有改动加 +），测试时一眼知道装的是哪个提交；Cargo.toml 里仍只写 -dev。
# pkgbuild / distribution 的 version 只认数字点号，去掉预发布后缀；Info.plist 与 pkg 文件名保留完整版本
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' apps/macos/Cargo.toml | head -1)"
if [[ "$VERSION" == *-dev ]]; then VERSION="${VERSION}-${GIT_REV}"; fi
PKG_VERSION="${VERSION%%-*}"
BUILD_NUMBER="$(git rev-list --count HEAD 2>/dev/null || echo 1)"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" \
  -c "Set :CFBundleVersion $BUILD_NUMBER" "$APP/Contents/Info.plist"
# 卸载脚本随包，装了 pkg 的用户从 Resources 里运行
cp apps/macos/scripts/uninstall.sh "$APP/Contents/Resources/uninstall.sh"
# 首装微明后部分沙盒应用仍复用旧输入源缓存；提供定向、可恢复的修复工具。
cp apps/macos/scripts/repair-input-cache.sh "$APP/Contents/Resources/repair-input-cache.sh"
# 输入源名字按系统语言本地化（中文系统显示「微明」，其他显示 Glimmer）
cp -R apps/macos/resources/*.lproj "$APP/Contents/Resources/"
# 词库与释义表打进 Resources。data/generated/ 里有生成好的产品数据（自建词库 + 语言模型 + LLM 释义表）就用它，
# 否则用 assets/sample/ 的样例。没有数据管道的机器跑 tools/release/data-fetch.sh 按 tools/release/data.lock 下载。
cp assets/sample/*.tsv "$APP/Contents/Resources/"
# AI 领域数据的来源、改动说明与许可证随包，不把训练正文带进应用。
mkdir -p "$APP/Contents/Resources/licenses/ai"
cp assets/lexicon/ai/LICENSE.* assets/lexicon/ai/sources.json assets/lexicon/ai/README.md "$APP/Contents/Resources/licenses/ai/"
# emoji 表（Unicode CLDR，可发布）
cp assets/emoji/*.tsv "$APP/Contents/Resources/"
# 词汇等级表（CEFR-J / Octanove / JLPT，见 assets/levels/README.md），「统计」页按级数词汇
cp assets/levels/levels-*.tsv "$APP/Contents/Resources/"
if [[ -f data/generated/dict.tsv || -f data/generated/dict.qj ]]; then
  # 词库与语言模型打成 .qj（mmap 直接用），TSV 比 .qj 新时重新打包；只有 .qj（CI 从数据包解出来的）就直接用
  if [[ -f data/generated/dict.tsv && ( ! -f data/generated/dict.qj || data/generated/dict.tsv -nt data/generated/dict.qj ) ]]; then
    cargo run --release -q -p glimmer-dict-convert -- pack dict --name "微明基础词库" \
      --license "MIT AND Unicode-3.0 AND CC-BY-SA-4.0" --attribution "通用规范汉字表；现代汉语常用词表（liuxilu 校对版）；THUOCL（清华大学自然语言处理实验室，MIT）；补充常用词白名单 CC-CEDICT（MDBG，CC BY-SA 4.0）与 jieba 词表（MIT）；读音 Unihan（Unicode）" \
      --source https://github.com/OpenFabrica/glimmer/tree/main/assets/lexicon
  fi
  if [[ -f data/generated/lm-bigram.tsv && ( ! -f data/generated/lm.qj || data/generated/lm-bigram.tsv -nt data/generated/lm.qj ) ]]; then
    cargo run --release -q -p glimmer-dict-convert -- pack lm --name "微明语言模型（中文维基 + LCCC，微明词库分词）" \
      --license "CC-BY-SA-4.0 AND MIT" --attribution "中文维基百科（CC BY-SA 4.0）；LCCC（清华大学 CoAI，MIT）"
  fi
  cp data/generated/dict.qj "$APP/Contents/Resources/"
  # 领域词库（lexicon 拆出的 dicts/*.qj）随包放 Resources/dicts/，缺省只开成语，偏好设置「词库」页可勾选
  if ls data/generated/dicts/*.qj >/dev/null 2>&1; then
    mkdir -p "$APP/Contents/Resources/dicts"
    cp data/generated/dicts/*.qj "$APP/Contents/Resources/dicts/"
  fi
  [[ -f data/generated/lm.qj ]] && cp data/generated/lm.qj "$APP/Contents/Resources/"
  # 本地整句模型（字级 Transformer）：训练仓库 ../train 导出三件套到 data/model/，tools/release/pack-model.sh 打成 model.qjm，
  # 随包只带这一个文件放 Resources/model/（三件套比 .qjm 新就重打）；什么都没有就不重排
  model_dir="${GLIMMER_MODEL_DIR:-data/model}"
  if [[ -f "$model_dir/model.safetensors" ]]; then
    GLIMMER_MODEL_DIR="$model_dir" tools/release/pack-model.sh
  fi
  if [[ -f "$model_dir/model.qjm" ]]; then
    mkdir -p "$APP/Contents/Resources/model"
    cp "$model_dir/model.qjm" "$APP/Contents/Resources/model/"
    chmod 644 "$APP/Contents/Resources/model/model.qjm"
    echo "打包本地整句模型：$model_dir/model.qjm"
  fi
  # 释义表打成 .qj（TSV 比 .qj 新时重打），英文词表仍是 TSV。各表来源不同，元数据按表写（见 assets/glossary/README.md）
  for lang in en ja zh es; do
    src="assets/glossary/glossary-$lang.tsv"
    out="data/generated/glossary-$lang.qj"
    [[ -f "$src" ]] || continue
    if [[ "$lang" == es ]]; then
      license="GPL-3.0-or-later"
      attribution="Azure Translator 机器翻译（Tofuzhu，tools/corpus/glossary_es.py）"
    elif [[ "$lang" == en ]]; then
      license="MIT"
      attribution="LLM 生成（DeepSeek），glimmer-gloss-gen；音标来自 ipa-dict（MIT）"
    else
      license="MIT"
      attribution="LLM 生成（DeepSeek），glimmer-gloss-gen"
    fi
    if [[ ! -f "$out" || "$src" -nt "$out" ]]; then
      cargo run --release -q -p glimmer-dict-convert -- pack glossary --language "$lang" --input "$src" \
        --name "微明释义表（${lang}）" --license "$license" --attribution "$attribution"
    fi
    cp "$out" "$APP/Contents/Resources/"
  done
  for f in assets/lexicon/english.tsv data/generated/english.tsv; do
    [[ -f "$f" ]] && cp "$f" "$APP/Contents/Resources/"
  done
  echo "使用 data/generated/ 的产品数据（自建词库）"
fi
# 五笔码表（assets/wubi/<方案>/ 的码表转出，各方案的许可证与署名全文随包，具体许可见各自的 README.md）：
# 哪一版生成了就带哪一版，缺的只提示不中断，包照样能打，只是开不了那一版五笔
for wubi in wubi86 wubi98 wubixsj; do
  if [[ ! -f "data/generated/$wubi.qj" ]]; then
    echo "注意: 没有 data/generated/$wubi.qj，包里不带这一版五笔码表（生成命令见 assets/wubi/$wubi/README.md）" >&2
    continue
  fi
  cp "data/generated/$wubi.qj" "$APP/Contents/Resources/"
  # 许可证与署名按方案加前缀放进 Resources（LICENSE.wubi86.LGPL-3.0 这样）。各方案带的文件不一样
  # （98 只有 LICENSE、新世纪只有 AUTHORS，它的 LGPL 全文引用 86 那份），所以逐个判断存在才拷；
  # set -u 下 glob 无匹配时 $f 会是字面量，靠 -f 挡掉
  for f in "assets/wubi/$wubi"/LICENSE*; do
    [[ -f "$f" ]] || continue
    license_name="$(basename "$f")"
    cp "$f" "$APP/Contents/Resources/LICENSE.$wubi.${license_name#LICENSE.}"
  done
  [[ -f "assets/wubi/$wubi/AUTHORS" ]] && cp "assets/wubi/$wubi/AUTHORS" "$APP/Contents/Resources/AUTHORS.$wubi"
  echo "打包五笔码表：data/generated/$wubi.qj"
done
printf 'APPL????' > "$APP/Contents/PkgInfo"

# 图标：从 assets/icon/logo.png 生成 .icns（应用图标）；输入法菜单图标用 assets/icon/menu-icon.pdf
# （纯黑矢量、斜线镂空，系统当模板图渲染：菜单高亮时反白、深色模式自动变色；改 logo 后用 gen-menu-icon.py 重生成）
ICONSET="$ROOT/target/Glimmer.iconset"
rm -rf "$ICONSET" && mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z $size $size assets/icon/logo.png --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z $double $double assets/icon/logo.png --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Glimmer.icns"
cp assets/icon/menu-icon.pdf "$APP/Contents/Resources/glimmer-menu.pdf"
# 仓库放在 iCloud 同步的目录（Documents）时新建的 .app 会带上 Finder 扩展属性，codesign 会拒（detritus not allowed）：签名前清掉
xattr -cr "$APP"
# Apple Silicon 上未签名的二进制不会被系统加载。有 Developer ID 证书就正式签（开 hardened runtime，公证要求），
# 没有就 ad-hoc 签名，本机自用够了
if [[ -n "${GLIMMER_SIGN_IDENTITY:-}" ]]; then
  codesign --force --deep --options runtime --timestamp --sign "$GLIMMER_SIGN_IDENTITY" "$APP"
  echo "已用 Developer ID 签名: $GLIMMER_SIGN_IDENTITY"
else
  codesign --force --deep --sign - "$APP"
fi
echo "打包完成: ${APP}（版本 ${VERSION}，构建 ${BUILD_NUMBER}，${ARCH}）"

if [[ "${1:-}" == "--pkg" ]]; then
  # 每个架构一个工作目录，成品都放 target/pkg/，两个架构接着打互不覆盖
  PKG="$ROOT/target/pkg/$APP_NAME-$VERSION-$ARCH.pkg"
  PKG_DIR="$ROOT/target/pkg.noindex/$ARCH"
  mkdir -p "$(dirname "$PKG")"
  rm -rf "$PKG_DIR"
  mkdir -p "$PKG_DIR/root" "$PKG_DIR/resources"
  # 不带扩展属性复制，否则载荷里全是 ._ 元数据文件
  ditto --noextattr --norsrc --noacl "$APP" "$PKG_DIR/root/$APP_NAME.app"
  # 组件描述里关掉 bundle 重定位：否则机器上别处已有同 bundle id 的 .app（比如 ~/Library 下的开发副本）时，
  # 安装器会把新版装到那里而不是 /Library/Input Methods
  pkgbuild --analyze --root "$PKG_DIR/root" "$PKG_DIR/component.plist" >/dev/null
  # 新系统（macOS 26+）的 --analyze 不再输出这一项，Set 会报 Does Not Exist，此时改用 Add
  /usr/libexec/PlistBuddy -c "Set :0:BundleIsRelocatable false" "$PKG_DIR/component.plist" 2>/dev/null \
    || /usr/libexec/PlistBuddy -c "Add :0:BundleIsRelocatable bool false" "$PKG_DIR/component.plist"
  pkgbuild --root "$PKG_DIR/root" --component-plist "$PKG_DIR/component.plist" \
    --install-location "/Library/Input Methods" --scripts apps/macos/pkg/scripts \
    --identifier app.glimmer.inputmethod --version "$PKG_VERSION" "$PKG_DIR/$APP_NAME-component.pkg" >/dev/null
  cp apps/macos/pkg/resources/*.html "$PKG_DIR/resources/"
  cp LICENSE "$PKG_DIR/resources/license.txt"
  # 二进制只有一种架构，hostArchitectures 限定只在对应机器上装；另一种架构用 GLIMMER_TARGET 再打一份
  sed -e "s/@VERSION@/$VERSION/g" -e "s/@PKG_VERSION@/$PKG_VERSION/g" -e "s/@ARCH@/$ARCH/g" apps/macos/pkg/distribution.xml > "$PKG_DIR/distribution.xml"
  SIGN_ARGS=()
  if [[ -n "${GLIMMER_INSTALLER_IDENTITY:-}" ]]; then
    SIGN_ARGS=(--sign "$GLIMMER_INSTALLER_IDENTITY" --timestamp)
  fi
  # bash 3.2 下空数组展开会撞 set -u，用 ${arr[@]+"${arr[@]}"} 写法
  productbuild --distribution "$PKG_DIR/distribution.xml" --package-path "$PKG_DIR" \
    --resources "$PKG_DIR/resources" ${SIGN_ARGS[@]+"${SIGN_ARGS[@]}"} "$PKG" >/dev/null
  if [[ -n "${GLIMMER_NOTARY_PROFILE:-}" ]]; then
    xcrun notarytool submit "$PKG" --keychain-profile "$GLIMMER_NOTARY_PROFILE" --wait
    xcrun stapler staple "$PKG"
    echo "已公证并钉上票据"
  elif [[ -z "${GLIMMER_INSTALLER_IDENTITY:-}" ]]; then
    echo "注意: pkg 未签名未公证，测试者首次打开要在「系统设置 → 隐私与安全性」里点「仍要打开」"
  fi
  echo "pkg: $PKG"
  shasum -a 256 "$PKG"
fi

if [[ "${1:-}" == "--install" ]]; then
  if [[ "$INSTALL_DIR" == "$SYSTEM_INSTALL_DIR" ]]; then
    # 覆盖 pkg 装的那份：目录归 root，文件所有权也照 pkg 的样子给 root:wheel
    echo "覆盖 $INSTALL_DIR/$APP_NAME.app（pkg 装的正式版，需要管理员密码）"
    sudo rm -rf "$INSTALL_DIR/$APP_NAME.app"
    sudo cp -R "$APP" "$INSTALL_DIR/$APP_NAME.app"
    sudo chown -R root:wheel "$INSTALL_DIR/$APP_NAME.app"
  else
    mkdir -p "$INSTALL_DIR"
    rm -rf "$INSTALL_DIR/$APP_NAME.app"
    cp -R "$APP" "$INSTALL_DIR/$APP_NAME.app"
  fi
  # 系统会在下次切换到该输入法时重新拉起进程
  pkill -x "$BIN_NAME" 2>/dev/null || true
  bash "$APP/Contents/Resources/repair-input-cache.sh"
  echo "已安装到: $INSTALL_DIR/$APP_NAME.app"
  echo "日志: ~/Library/Logs/Glimmer/"
fi

# 构建目录里的 .app（target/ 与 pkg 载荷）会被 Launch Services 顺手登记成输入法，
# 「添加输入法」对话框里就会出现多条同名甚至空白的条目，同一个输入源 ID 对应多个包时启用也会失灵。
# 打完包就把它们从登记里注销，只留真正装到 Input Methods 下的那份。
LSREGISTER=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
# 顺带把旧版本脚本留在 target/ 里的那些登记也注销掉（路径可能已不存在，lsregister -u 对不存在的路径无害）
# PKG_DIR 只在 --pkg 时定义，--install 路径下 set -u 会掐掉整段，所以给缺省值
for stray in "$APP" "${PKG_DIR:-$ROOT/target/pkg.noindex/$ARCH}/root/$APP_NAME.app" "$ROOT/target/$APP_NAME.app" "$ROOT/target/Qingjian.app" \
  "$ROOT"/target/pkg/*/root/*.app "$ROOT"/target/install-*/root/*.app; do
  "$LSREGISTER" -u "$stray" >/dev/null 2>&1 || true
done
# 装到 Input Methods 的那份重新登记一次，系统里只认它
if [[ "${1:-}" == "--install" ]]; then
  "$LSREGISTER" -f "$INSTALL_DIR/$APP_NAME.app" >/dev/null 2>&1 || true
fi
