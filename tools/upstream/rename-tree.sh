#!/usr/bin/env bash
# 在当前目录（一棵已检出的上游 qingjian 树）里做青简 → 微明的整树改名：路径、包名、标识符、文案，
# Windows Server 的公共部分（dispatch / assembly / error.rs / tests）映射到 crates/glimmer-server，最后 rustfmt 让 use 顺序回到规范。
# 例外：.qj 魔数 QINGJIAN、cosmic-text 补丁仓库 qingjian-team/cosmic-text、释义表词条、CHANGELOG.md（手工并入）、Cargo.lock（cargo 重生成）。
# 用法（对一段上游历史逐提交改名，生成本地分支）：
#   git branch -f upstream-renamed upstream/main
#   FILTER_BRANCH_SQUELCH_WARNING=1 git filter-branch -f --tree-filter "$PWD/tools/upstream/rename-tree.sh" -- <上次同步的上游基点>^..upstream-renamed
# 基点用 ^ 包进去：这样第一个改名提交的父也是改名过的树，cherry-pick 时不会把整树改名当成差异。
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
# 1. 路径：Server 公共部分映射，再改名字里带 qingjian 的路径（深的先动）
if [ -d apps/windows/server/src/dispatch ] || [ -d apps/windows/server/src/assembly ]; then
  mkdir -p crates/glimmer-server/src
  for d in dispatch assembly; do [ -e apps/windows/server/src/$d ] && mv apps/windows/server/src/$d crates/glimmer-server/src/$d; done
  [ -e apps/windows/server/src/error.rs ] && mv apps/windows/server/src/error.rs crates/glimmer-server/src/error.rs
fi
src=apps/windows/server/tests
if [ -e $src/engine_loop.rs ] || [ -d $src/engine_loop ]; then
  mkdir -p crates/glimmer-server/tests
  [ -e $src/engine_loop.rs ] && mv $src/engine_loop.rs crates/glimmer-server/tests/engine_loop.rs
  [ -d $src/engine_loop ] && mv $src/engine_loop crates/glimmer-server/tests/engine_loop
  find crates/glimmer-server/tests -name '*.rs' -exec sed -i '' 's/qingjian_windows_server/glimmer_server/g; s#join("../../..")#join("../..")#g' {} +
fi
find . -depth -path ./.git -prune -o \( -iname '*qingjian*' \) -print | while read -r p; do
  mv "$p" "$(dirname "$p")/$(basename "$p" | sed 's/qingjian/glimmer/g; s/QINGJIAN/GLIMMER/g; s/Qingjian/Glimmer/g')"
done
# 2. 内容。释义表只改 # 开头的注释行（词条里的 清涧 / Qingjian 是地名，不能动）
for f in assets/glossary/glossary-*.tsv; do [ -f "$f" ] && sed -i '' '/^#/ s/qingjian/glimmer/g' "$f"; done
grep -rlI --exclude-dir=.git --exclude=CHANGELOG.md --exclude=Cargo.lock --exclude='glossary-*.tsv' -e qingjian -e Qingjian -e QINGJIAN -e QingJian -e 青简 . | while read -r f; do
  sed -i '' \
    -e 's#qingjian-team/cosmic-text#@@COSMIC@@#g' \
    -e 's#qingjian-team/qingjian#OpenFabrica/glimmer#g' \
    -e 's#qingjian-team#rambocode#g' \
    -e '/青简/ s/qing jian/wei ming/g' \
    -e 's/qingjian/glimmer/g' -e 's/Qingjian/Glimmer/g' -e 's/QingJian/Glimmer/g' -e 's/QINGJIAN/GLIMMER/g' -e 's/青简/微明/g' \
    -e 's#@@COSMIC@@#qingjian-team/cosmic-text#g' "$f"
done
# 3. 例外还原：.qj 魔数
f=crates/glimmer-format/src/layout/header.rs
[ -f "$f" ] && sed -i '' 's/\*b"GLIMMER"/*b"QINGJIAN"/' "$f"
# 4. 改名后 use 的字母序变了，rustfmt 一遍
cargo fmt --all >/dev/null 2>&1 || true
true
