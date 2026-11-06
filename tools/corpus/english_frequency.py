# /// script
# requires-python = ">=3.10"
# dependencies = ["wordfreq>=3.1"]
# ///
"""给英文词表配词频：读一列单词（第一遍 `dict-convert english` 生成的 english.tsv 的第一列），用 wordfreq 的英文 Zipf 频率（×1000 取整）
写出 `小写词\\t词频`（第一列是小写的词、撇号保留，不是去了撇号的检索编码；`dict-convert english --frequency` 按小写词查）。

输入要是一列：一行有第二列时第二列会被当键写出，三列的 english.tsv 第二列是去了撇号的编码，撇号词就查不到了。`#` 开头的行跳过。

用法（两遍生成，见 assets/lexicon/GLIMMER.md 第 5 步）：
  cargo run --release -p glimmer-dict-convert -- english assets/lexicon/05_english/00_all_words.tsv
  cut -f1 data/generated/english.tsv > data/generated/english-words.txt
  uv run tools/corpus/english_frequency.py data/generated/english-words.txt -o data/generated/english-frequency.tsv
  cargo run --release -p glimmer-dict-convert -- english-case data/corpus/en/train/fineweb.txt
  cargo run --release -p glimmer-dict-convert -- english assets/lexicon/05_english/00_all_words.tsv \
    --frequency data/generated/english-frequency.tsv --case-counts data/generated/english-case.tsv

wordfreq 的代码是 MIT，数据带 CC-BY-SA 等许可，这里只取频率数字做排序。
"""

import argparse
import sys

from wordfreq import zipf_frequency


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("word_list")
    parser.add_argument("-o", "--output", required=True)
    args = parser.parse_args()
    rows = 0
    with open(args.word_list, encoding="utf-8") as src, open(args.output, "w", encoding="utf-8") as out:
        out.write("# 编码\t词频（wordfreq 英文 Zipf × 1000）\n")
        for line in src:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            fields = line.split("\t")
            word = fields[0]
            code = fields[1] if len(fields) > 1 else word.lower()
            zipf = zipf_frequency(word, "en")
            out.write(f"{code}\t{int(zipf * 1000)}\n")
            rows += 1
    print(f"wrote {rows} rows to {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
