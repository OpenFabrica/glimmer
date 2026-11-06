#!/bin/bash
# 扫参：设置文件每行一组 `--tune` 设置（空行 = 缺省），并行跑 `glimmer-cli`，汇成 TSV。两种模式：
#
#   tools/eval/sweep.sh <冻结日志> <配置文件> <设置文件> <输出 tsv>                          # 回放（缺省）：--replay
#   GLIMMER_SWEEP_MODE=english tools/eval/sweep.sh <英文文本> <配置文件> <设置文件> <输出 tsv>  # 英文打字模拟：--eval-english
#
# 配置文件用一份 `[predict] enabled = false` 的副本（云联想开着 CLI 会因没密钥退出，也别拿输入法的密钥跑批量）。
# 二进制缺省 target/release/glimmer-cli，可用 GLIMMER_CLI 指定；GLIMMER_JOBS 是并行数（缺省 4）。
# 英文模式下 GLIMMER_EVAL_ARGS 原样追加给每次运行（如 `--eval-english-words 5000 --eval-english-groups complete,typo,prefix,space`），
# 不给就全组都跑；输出的省键率与比例都是百分数（去掉了 %），耗时是毫秒。
# 下一词预测组（predict）的列接在最后：出现率、精确率、命中占全部词位、含预测首选 / 前三省键率（实际按键）、
# 「如果给三个」前三占出现、连续命中 1 / 2 / 3 次以上的段数、取预测 p95；没跑这一组时是 `-`。
set -euo pipefail
log=$1
config=$2
settings=$3
out=$4
bin=${GLIMMER_CLI:-target/release/glimmer-cli}
jobs=${GLIMMER_JOBS:-4}
mode=${GLIMMER_SWEEP_MODE:-replay}
eval_args=${GLIMMER_EVAL_ARGS:-}
runs=$(mktemp -d)

run_one() {
  local setting="$1"
  local tag=${setting:-default}
  local file="$runs/${tag//[\/,= ]/_}.txt"
  local tune=()
  if [[ -n "$setting" ]]; then
    tune=(--tune "$setting")
  fi
  if [[ "$mode" == english ]]; then
    # shellcheck disable=SC2086 # GLIMMER_EVAL_ARGS 要按空白拆成多个参数
    "$bin" --config "$config" --eval-english "$log" --misses 0 ${tune[@]+"${tune[@]}"} $eval_args > "$file" 2>&1
    # 各组第一行的格式见 apps/cli/src/eval/english/report.rs 的文件头
    TAG=$tag perl -ne '
      s/%//g;
      if (/^首选省键率\s+文章口径 (\S+)\s+实际按键 (\S+)/) { @t1 = ($1, $2) }
      if (/^前三省键率\s+文章口径 (\S+)\s+实际按键 (\S+)/) { @t3 = ($1, $2) }
      if (/^错字组\s+\d+ 词\s+前三有正确词 (\S+)\s+首选 \S+\s+省键率 文章口径 \S+\s+实际按键 (\S+)/) { @ty = ($1, $2) }
      if (/^错前缀组\s+\d+ 词\s+省键率 文章口径 \S+\s+实际按键 (\S+).*目标词没出现 (\d+)/) { @px = ($1, $2) }
      if (/^正常词被改错\s+词表里有 (\d+) \/ \d+.*词表里没有 (\d+) \/ \d+/) { @sp = ($1, $2) }
      if (/^每键耗时\s+\d+ 次\s+p50 (\S+) ms\s+p95 (\S+) ms/) { @ms = ($1, $2) }
      if (/^个人词回归\s+过 (\d+) \/ (\d+)/) { @pe = ($1, $2) }
      if (/^预测出现\s+\d+ \/ \d+ 词位（(\S+)）/) { $po = $1 }
      if (/^第一名命中\s+\d+ \/ \d+（精确率 (\S+)）\s+占全部词位 (\S+)/) { @ph = ($1, $2) }
      if (/^含预测首选省键率\s+文章口径 \S+\s+实际按键 (\S+)/) { $p1 = $1 }
      if (/^含预测前三省键率\s+文章口径 \S+\s+实际按键 (\S+)/) { $p3 = $1 }
      if (/^如果给三个.*前三有目标词 \d+ \/ \d+（占出现 (\S+)）/) { $pt = $1 }
      if (/^连续命中.*\s1 次 (\d+)\s+2 次 (\d+)\s+3 次以上 (\d+)/) { @pk = ($1, $2, $3) }
      if (/^取预测耗时\s+\d+ 次\s+p50 \S+ ms\s+p95 (\S+) ms/) { $pm = $1 }
      END {
        my @row = map { $_ // "-" } ($t1[0], $t1[1], $t3[0], $t3[1], $ty[0], $ty[1], $px[0], $px[1], $sp[0], $sp[1], $ms[0], $ms[1], $pe[0],
          $po, $ph[0], $ph[1], $p1, $p3, $pt, $pk[0], $pk[1], $pk[2], $pm);
        print join("\t", $ENV{TAG}, @row), "\n";
      }' "$file"
    return
  fi
  "$bin" --config "$config" --replay "$log" --misses 0 ${tune[@]+"${tune[@]}"} > "$file" 2>&1
  # 词 / 整句 / 英文 三行：首选命中数 / 总数、不在候选、现在仍纠
  TAG=$tag perl -ne '
    if (/^(词|整句|英文)\s.*不在候选\s+(\d+).*命中数 (\d+) \/ (\d+)/) { $n=$1; $top{$n}=$3; $all{$n}=$4; $miss{$n}=$2 }
    if (/当时纠错生效 (\d+) 条，现在仍纠 (\d+)/) { $cor{$n}=$2 }
    END { print join("\t", $ENV{TAG}, map { ($top{$_}//0, $all{$_}//0, $miss{$_}//0, $cor{$_}//0) } qw(词 整句 英文)), "\n" }' "$file"
}
export -f run_one
export bin config log runs mode eval_args

if [[ "$mode" == english ]]; then
  printf 'tag\t首选省键文章\t首选省键实际\t前三省键文章\t前三省键实际\t错字前三\t错字省键实际\t错前缀省键实际\t错前缀没出现\t改错词表内\t改错词表外\tp50ms\tp95ms\t个人词过\t预测出现\t预测精确\t预测命中占词位\t含预测首选实际\t含预测前三实际\t给三个前三\t连按1\t连按2\t连按3+\t预测p95ms\n' > "$out"
else
  printf 'tag\t词命中\t词总\t词不在\t词仍纠\t整句命中\t整句总\t整句不在\t整句仍纠\t英文命中\t英文总\t英文不在\t英文仍纠\n' > "$out"
fi
tr '\n' '\0' < "$settings" | xargs -0 -P "$jobs" -I{} bash -c 'run_one "$1"' _ {} >> "$out"
rm -rf "$runs"
