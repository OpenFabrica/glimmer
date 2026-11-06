# /// script
# requires-python = ">=3.11"
# dependencies = ["pyarrow>=17", "huggingface_hub>=0.26"]
# ///
"""下载英文语料并切成训练 / 调参 / 测试 / 验收四份，供英文 n-gram 训练与 `--eval-english` 评测。

用法：
  uv run tools/corpus/english_corpus.py            # fetch + split（已下载的来源跳过）
  uv run tools/corpus/english_corpus.py fetch [--sources fineweb soda]
  uv run tools/corpus/english_corpus.py split      # 只从原始缓存重切，不联网
  uv run tools/corpus/english_corpus.py verify     # 查四份之间有没有重复文档 / 重复长句

来源（Hugging Face，revision 写死在 SOURCES；许可见 assets/lexicon/05_english/README.md）：
  fineweb   HuggingFaceFW/fineweb sample-10BT   ODC-By 1.0   写作
  soda      allenai/soda                        CC BY 4.0    聊天（合成对话，两方都取）
  wildchat  allenai/WildChat-1M                 ODC-By 1.0   聊天（只取用户说的话）
  oasst2    OpenAssistant/oasst2                Apache-2.0   聊天（只取用户说的话）
  irc       common-pile/ubuntu_irc_filtered     公共领域     技术

两步：
  fetch：按固定 revision 流式读取，只取需要的列与子集，原样存到 data/corpus/en/raw/<来源>-<分片>.jsonl.gz
         （一行一个文档：{"id", "texts", "meta"}）。文件已存在就不重下；先写 .tmp 再改名，中断不会留下半截缓存。
  split：清洗 → 去重 → 按文档 id 的 sha256 分四份 → 写文件与清单。不联网、不用随机数，同样的原始缓存产出同样的字节。
输出：
  data/corpus/en/train/<来源>.txt              一行一句 / 一条消息，文档之间空一行
  data/eval/english/{dev,test,final}/<类别>.txt  类别 chat / tech / writing，按文档截到约 2 万词
  data/eval/english/MANIFEST.tsv、SOURCES.tsv
"""

import argparse
import gzip
import hashlib
import itertools
import json
import os
import re
import sys
import time
import unicodedata
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# 四份的比例：sha256 取模 100 后，0 → dev、1 → test、2 → final，其余是训练
SPLIT_OF_BUCKET = {0: "dev", 1: "test", 2: "final"}
EVAL_SPLITS = ("dev", "test", "final")
CATEGORIES = ("chat", "tech", "writing")
# 每份每类别的评测文本截到这个词数（逐键模拟，不需要太大）
EVAL_WORDS_CAP = 20_000
# 至少这么多词的行才做跨文档去重：短句（"thank you so much"）本来就会自然重复，去掉会扭曲词频
LINE_DEDUP_MIN_WORDS = 8
# 写作类单篇文档只留开头这么多词，让同样的词数覆盖更多文档
WRITING_DOC_WORDS = 2_000
FETCH_WORKERS = 8


@dataclass(frozen=True)
class Source:
    """一个语料来源：数据集、固定 revision、许可、类别与目标词数。"""

    name: str

    dataset: str

    revision: str

    license: str

    category: str

    # 清洗后（四份合计）取到这么多词就停
    target_words: int


SOURCES = {
    s.name: s
    for s in (
        Source("fineweb", "HuggingFaceFW/fineweb", "9bb295ddab0e05d785b879661af7260fed5140fc", "ODC-By 1.0", "writing", 18_500_000),
        Source("soda", "allenai/soda", "fdc848ab0183208ea7808206c91c724414d0a071", "CC BY 4.0", "chat", 16_300_000),
        Source("wildchat", "allenai/WildChat-1M", "7d6490e462285cf85d91eabea0f9a954fbddcd1f", "ODC-By 1.0", "chat", 7_500_000),
        Source("irc", "common-pile/ubuntu_irc_filtered", "84f88c986584f11d672befab542fa4d5123f3e8f", "Public Domain", "tech", 4_400_000),
        Source("oasst2", "OpenAssistant/oasst2", "179dd21fc55192153d94adb0e0ce8f69e222bf75", "Apache-2.0", "chat", 400_000),
    )
}

URL = re.compile(r"(?:https?://|ftp://|www\.)\S+|\b\S+\.(?:com|org|net|io|html?|php)\b", re.I)
EMAIL = re.compile(r"\S+@\S+\.\w+")
PHONE = re.compile(r"\+?\d[\d ().-]{7,}\d")
# 出现就判为代码 / 标记的字符；聊天比写作更严（markdown 的 * # 也算）
CODE_CHARS = {
    "writing": re.compile(r"[{}<>\\|`=^~_\[\]]"),
    "chat": re.compile(r"[{}<>\\|`=^~_\[\]#*$]"),
    # 技术聊天允许路径的 / 与包名里的 - .，管道与括号类仍算代码
    "tech": re.compile(r"[{}<>\\|`=^~\[\]$]"),
}
# 常见英文功能词：文档级判断是不是英文（IRC 有法语、德语等本地化频道）
STOPWORDS = frozenset(
    "the a an and or but of to in on at for with is are was were be been it this that i you he she we they "
    "my your me not do does did have has had can will would so if what how there just".split()
)
QUOTES = str.maketrans({"\u2018": "'", "\u2019": "'", "\u201b": "'", "\u2032": "'", "\u201c": '"', "\u201d": '"',
                        "\u201e": '"', "\u2033": '"', "\u00a0": " ", "\u200b": "", "\u200c": "", "\u200d": "",
                        "\ufeff": ""})
# 句末标点后跟空白、下一句以大写 / 数字 / 引号开头，就在这里断句
SENTENCE_END = re.compile(r"(?<=[.!?])[\"')\]]*\s+(?=[\"'(\[]?[A-Z0-9])")
IRC_LINE = re.compile(r"^\[\d\d:\d\d\] <([^>]+)> (.*)$")
IRC_ADDRESS = re.compile(r"^(\S+?)[:,]\s+")
IRC_SKIP_CHANNELS = ("meeting", "bugs")
BULLET = re.compile(r"^(?:[-*\u2022]\s+)+")


def hf_fs():
    """惰性创建 HfFileSystem（不登录也能读公开数据集；有本机凭据时自动带上）。"""
    from huggingface_hub import HfFileSystem

    return HfFileSystem()


def hf_path(source: Source, file: str) -> str:
    """带 revision 的 hf:// 路径，保证重跑读到同一份。"""
    return f"datasets/{source.dataset}@{source.revision}/{file}"


def retry(action, what: str):
    """网络读取失败时退避重试 5 次，仍失败就抛出。"""
    for attempt in range(5):
        try:
            return action()
        except Exception as error:  # 网络抖动、限流
            if attempt == 4:
                raise
            print(f"  重试 {what}：{error}", file=sys.stderr)
            time.sleep(5 * (attempt + 1))


def read_row_group(fs, path: str, index: int, columns: list[str]):
    """读 parquet 的一个 row group，只取指定列（HTTP 范围请求，不下整个文件）。"""
    import pyarrow.parquet as pq

    def action():
        with fs.open(path, block_size=4 << 20) as f:
            return pq.ParquetFile(f).read_row_group(index, columns=columns)

    return retry(action, f"{path} row group {index}")


def row_group_count(fs, path: str) -> int:
    """parquet 文件的 row group 数。"""
    import pyarrow.parquet as pq

    def action():
        with fs.open(path) as f:
            return pq.ParquetFile(f).metadata.num_row_groups

    return retry(action, f"{path} metadata")


def write_raw(path: Path, records) -> int:
    """把文档记录写成 gzip JSONL：先写 .tmp，写完才改名；gzip 头不带时间戳。"""
    tmp = path.with_name(path.name + ".tmp")
    count = 0
    with open(tmp, "wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", mtime=0) as gz:
        for record in records:
            gz.write((json.dumps(record, ensure_ascii=False) + "\n").encode("utf-8"))
            count += 1
    tmp.replace(path)
    return count


def fetch_row_groups(fs, path: str, indexes: list[int], columns: list[str]):
    """并发读多个 row group，按给定顺序逐个产出表。"""
    with ThreadPoolExecutor(FETCH_WORKERS) as pool:
        yield from pool.map(lambda i: read_row_group(fs, path, i, columns), indexes)


def fetch_fineweb(source: Source, fs):
    """FineWeb sample-10BT 第一个文件里均匀取 48 个 row group（每个约 1000 篇），足够覆盖目标词数。"""
    path = hf_path(source, "sample/10BT/000_00000.parquet")

    def records():
        total = row_group_count(fs, path)
        indexes = [i * total // 48 for i in range(48)]
        for table in fetch_row_groups(fs, path, indexes, ["id", "text"]):
            for doc_id, text in zip(table.column("id").to_pylist(), table.column("text").to_pylist()):
                yield {"id": doc_id, "texts": [text]}

    yield "0000", records()


def fetch_soda(source: Source, fs):
    """SODA 训练文件里每 8 段对话取 1 段（官方切分不用，四份由本脚本统一切）。"""
    path = hf_path(source, "train.parquet")

    def records():
        table = read_row_group(fs, path, 0, ["original_index", "dialogue"])
        ids = table.column("original_index").to_pylist()
        dialogues = table.column("dialogue").to_pylist()
        for i in range(0, len(ids), 8):
            yield {"id": str(ids[i]), "texts": dialogues[i]}

    yield "0000", records()


WILDCHAT_COLUMNS = ["language", "toxic", "redacted"] + [
    f"conversation.list.element.{c}" for c in ("role", "content", "language", "toxic", "redacted")
]


def fetch_wildchat(source: Source, fs):
    """WildChat 14 个文件各成一个分片；只存英文、未标有害、未脱敏的对话里的英文用户轮次。"""
    for shard in range(14):
        path = hf_path(source, f"data/train-{shard:05d}-of-00014.parquet")

        def records(path=path, shard=shard):
            indexes = list(range(row_group_count(fs, path)))
            for group, table in zip(indexes, fetch_row_groups(fs, path, indexes, WILDCHAT_COLUMNS)):
                columns = (table.column(c).to_pylist() for c in ("conversation", "language", "toxic", "redacted"))
                for row, (conversation, language, toxic, redacted) in enumerate(zip(*columns)):
                    if language != "English" or toxic or redacted:
                        continue
                    turns = [t["content"] for t in conversation if t["role"] == "user" and t["language"] == "English"
                             and not t["toxic"] and not t["redacted"]]
                    if turns:
                        yield {"id": f"{shard}:{group}:{row}", "texts": turns}

        yield f"{shard:04d}", records()


def fetch_oasst2(source: Source, fs):
    """oasst2 训练 + 验证文件：按对话树分组，只留英文、未删除、非合成的用户（prompter）消息。"""
    columns = ["message_id", "message_tree_id", "created_date", "text", "role", "lang", "deleted", "synthetic"]

    def records():
        trees: dict[str, list[tuple[str, str, str]]] = {}
        for name in ("train-00000-of-00001-88ba0162028a73fc", "validation-00000-of-00001-1deeef95c3248fe0"):
            path = hf_path(source, f"data/{name}.parquet")
            indexes = list(range(row_group_count(fs, path)))
            for table in fetch_row_groups(fs, path, indexes, columns):
                for mid, tree, created, text, role, lang, deleted, synthetic in zip(
                        *(table.column(c).to_pylist() for c in columns)):
                    if role == "prompter" and lang == "en" and not deleted and not synthetic:
                        trees.setdefault(tree, []).append((created, mid, text))
        # 树内按创建时间排，树按 id 排，产出顺序与读取顺序无关
        for tree in sorted(trees):
            yield {"id": tree, "texts": [text for _, _, text in sorted(trees[tree])]}

    yield "0000", records()


IRC_RAW_WORDS_PER_FILE = 8_000_000


def fetch_irc(source: Source, fs):
    """Ubuntu IRC 三个文件各成一个分片，每个从头顺序读到原始消息约 800 万词为止（一个文档 = 一个频道的一天）。"""
    for shard in range(3):
        path = hf_path(source, f"ubuntu-chat-dolma-{shard:04d}.json.gz")

        def records(path=path):
            words = 0
            with fs.open(path, block_size=8 << 20) as f, gzip.open(f, "rt", encoding="utf-8") as g:
                for line in g:
                    doc = json.loads(line)
                    meta = doc["metadata"]
                    yield {"id": doc["id"], "texts": doc["text"].split("\n"),
                           "meta": {"channel": meta["channel"], "authors": meta["authors"]}}
                    words += len(doc["text"].split())
                    if words >= IRC_RAW_WORDS_PER_FILE:
                        return

        yield f"{shard:04d}", records()


FETCHERS = {"fineweb": fetch_fineweb, "soda": fetch_soda, "wildchat": fetch_wildchat, "oasst2": fetch_oasst2,
            "irc": fetch_irc}


def raw_shards(raw_dir: Path, name: str) -> list[Path]:
    """某来源已下好的原始分片，按文件名排序。"""
    return sorted(raw_dir.glob(f"{name}-*.jsonl.gz"))


def fetch(raw_dir: Path, names: list[str]) -> None:
    """逐个来源下载原始缓存；分片文件已存在就跳过，失败的来源记下原因、其余继续。"""
    raw_dir.mkdir(parents=True, exist_ok=True)
    fs = hf_fs()
    failed = {}
    for name in names:
        source = SOURCES[name]
        started = time.time()
        print(f"{name}：{source.dataset}@{source.revision[:10]}", file=sys.stderr)
        try:
            for shard, records in FETCHERS[name](source, fs):
                path = raw_dir / f"{name}-{shard}.jsonl.gz"
                if path.exists():
                    print(f"  {path.name} 已存在，跳过", file=sys.stderr)
                    records.close()
                    continue
                count = retry_free_write(path, records)
                print(f"  {path.name}：{count} 篇，累计 {time.time() - started:.0f} s", file=sys.stderr)
        except Exception as error:
            failed[name] = repr(error)
            print(f"  {name} 失败：{error!r}", file=sys.stderr)
    if failed:
        print(f"未下到：{failed}", file=sys.stderr)


def retry_free_write(path: Path, records) -> int:
    """写一个分片；生成器内部已对每次网络读取做了重试，这里失败就删掉 .tmp 再抛出。"""
    try:
        return write_raw(path, records)
    except BaseException:
        path.with_name(path.name + ".tmp").unlink(missing_ok=True)
        raise


def normalize(text: str) -> str:
    """NFKC、弯引号转直引号、去零宽字符；不改大小写、不去标点。"""
    return unicodedata.normalize("NFKC", text).translate(QUOTES)


def squash(text: str) -> str:
    """一行内的连续空白压成一个空格。"""
    return " ".join(text.split())


def natural(line: str, category: str, min_words: int, max_words: int) -> bool:
    """判断一行是不是自然语言：词数在范围内、没有 URL / 邮箱 / 电话、没有代码字符、几乎全是 ASCII、字母占大头。"""
    words = len(line.split())
    if words < min_words or words > max_words:
        return False
    if URL.search(line) or EMAIL.search(line) or PHONE.search(line) or CODE_CHARS[category].search(line):
        return False
    visible = [c for c in line if not c.isspace()]
    if sum(ord(c) > 127 and c not in "\u2013\u2014\u2026\u00e9" for c in visible) > 0.02 * len(visible):
        return False
    letters = sum(c.isalpha() for c in visible)
    return letters >= (0.6 if category == "tech" else 0.75) * len(visible)


def english(lines: list[str]) -> bool:
    """文档级英文判断：词足够多时，功能词要占一成五以上；太短的文档不判（交给逐行过滤）。"""
    tokens = [t.strip(".,!?;:\"'()").lower() for line in lines for t in line.split()]
    if len(tokens) < 30:
        return True
    return sum(t in STOPWORDS for t in tokens) >= 0.15 * len(tokens)


def clean_writing(record: dict) -> list[str]:
    """写作：按段落再按句末标点断句，丢掉不像正文的句子，整篇只留前 WRITING_DOC_WORDS 词。"""
    lines, words = [], 0
    for paragraph in normalize(record["texts"][0]).split("\n"):
        # 列表项开头的「- 」「* 」「• 」不是正文，去掉
        for sentence in SENTENCE_END.split(BULLET.sub("", squash(paragraph))):
            if natural(sentence, "writing", 3, 200):
                lines.append(sentence)
                words += len(sentence.split())
        if words >= WRITING_DOC_WORDS:
            break
    return lines


def clean_messages(record: dict, category: str, min_words: int, max_words: int) -> list[str]:
    """聊天：一条消息超过 3 个换行（贴的文档、列表）整条丢；其余每个非空行单独判断。"""
    lines = []
    for message in record["texts"]:
        message = normalize(message).strip()
        if "```" in message or message.count("\n") > 3:
            continue
        parts = [squash(p) for p in message.split("\n") if p.strip()]
        words = sum(len(p.split()) for p in parts)
        if parts and min_words <= words <= max_words and all(natural(p, category, 1, max_words) for p in parts):
            lines.extend(parts)
    return lines


def clean_irc(record: dict) -> list[str]:
    """IRC：跳过会议与 bug 机器人频道；去掉时间戳、昵称和开头的「某人: 」称呼，动作行（* nick ...）丢掉。"""
    meta = record["meta"]
    if any(k in meta["channel"] for k in IRC_SKIP_CHANNELS):
        return []
    nicks = set(meta["authors"])
    lines = []
    for raw in record["texts"]:
        match = IRC_LINE.match(raw)
        if not match:
            continue
        message = squash(normalize(match.group(2)))
        address = IRC_ADDRESS.match(message)
        if address and address.group(1) in nicks:
            message = message[address.end():]
        # 「!」开头的是给频道机器人的命令（!pkg、!info），不是人话
        if not message.startswith("!") and natural(message, "tech", 2, 60):
            lines.append(message)
    return lines


def clean(name: str, record: dict) -> list[str]:
    """按来源分派清洗规则，返回这个文档清洗后的行。"""
    if name == "fineweb":
        return clean_writing(record)
    if name == "soda":
        return clean_messages(record, "writing", 1, 200)
    if name in ("wildchat", "oasst2"):
        return clean_messages(record, "chat", 2, 100)
    return clean_irc(record)


def digest(text: str) -> bytes:
    """规范化（casefold + 压空白）后的 sha256 前 16 字节，用作去重键。"""
    return hashlib.sha256(squash(text).casefold().encode("utf-8")).digest()[:16]


def split_of(name: str, doc_id: str) -> str:
    """按「来源 + 文档 id」的 sha256 定落在哪份；与清洗规则无关，规则改了文档也不换份。"""
    bucket = int.from_bytes(hashlib.sha256(f"{name}\0{doc_id}".encode("utf-8")).digest()[:8], "big") % 100
    return SPLIT_OF_BUCKET.get(bucket, "train")


def read_round_robin(paths: list[Path]):
    """多个分片轮流各取一篇，让「取够就停」时各分片（不同时间段）都有份。"""
    files = [gzip.open(p, "rt", encoding="utf-8") for p in paths]
    try:
        for group in itertools.zip_longest(*files):
            for line in group:
                if line is not None:
                    yield json.loads(line)
    finally:
        for f in files:
            f.close()


@dataclass
class Doc:
    """切分后进评测候选的一个文档。"""

    key: bytes

    lines: list[str]


def write_doc(out, lines: list[str]) -> None:
    """写一个文档：一行一句，文档后空一行。"""
    out.write("\n".join(lines))
    out.write("\n\n")


def split(data: Path) -> None:
    """从原始缓存清洗、去重、切分，写训练文件、评测文件和两份清单。"""
    raw_dir = data / "corpus/en/raw"
    train_dir = data / "corpus/en/train"
    eval_dir = data / "eval/english"
    train_dir.mkdir(parents=True, exist_ok=True)
    for s in EVAL_SPLITS:
        (eval_dir / s).mkdir(parents=True, exist_ok=True)

    seen_lines: set[bytes] = set()
    seen_docs: set[bytes] = set()
    candidates: dict[tuple[str, str], list[Doc]] = {(s, c): [] for s in EVAL_SPLITS for c in CATEGORIES}
    stats = []
    for name, source in SOURCES.items():
        shards = raw_shards(raw_dir, name)
        if not shards:
            print(f"{name}：没有原始缓存，跳过", file=sys.stderr)
            stats.append((source, 0, 0, 0, 0, 0))
            continue
        raw_docs = used_docs = used_words = train_docs = train_words = 0
        target = train_dir / f"{name}.txt"
        with open(target.with_name(target.name + ".tmp"), "w", encoding="utf-8", newline="\n") as out:
            for record in read_round_robin(shards):
                if used_words >= source.target_words:
                    break
                raw_docs += 1
                lines, keys = [], set()
                # 长句跨文档、跨来源只留第一次出现的；短句自然重复，不去。
                # 文档被整篇丢掉时它的长句不登记，免得挡住后面正常文档里的同一句
                for line in clean(name, record):
                    if len(line.split()) >= LINE_DEDUP_MIN_WORDS:
                        key = digest(line)
                        if key in seen_lines or key in keys:
                            continue
                        keys.add(key)
                    lines.append(line)
                if not lines or not english(lines):
                    continue
                doc_key = digest("\n".join(lines))
                if doc_key in seen_docs:
                    continue
                seen_docs.add(doc_key)
                seen_lines.update(keys)
                words = sum(len(line.split()) for line in lines)
                used_docs += 1
                used_words += words
                part = split_of(name, record["id"])
                if part == "train":
                    write_doc(out, lines)
                    train_docs += 1
                    train_words += words
                else:
                    order = hashlib.sha256(f"{name}\0{record['id']}".encode("utf-8")).digest()
                    candidates[(part, source.category)].append(Doc(order, lines))
        target.with_name(target.name + ".tmp").replace(target)
        stats.append((source, raw_docs, used_docs, used_words, train_docs, train_words))
        print(f"{name}：读 {raw_docs} 篇，用 {used_docs} 篇 {used_words} 词，训练 {train_docs} 篇 {train_words} 词",
              file=sys.stderr)

    write_eval(eval_dir, candidates)
    write_manifest(data, eval_dir)
    write_sources(eval_dir, stats)


def write_eval(eval_dir: Path, candidates: dict[tuple[str, str], list[Doc]]) -> None:
    """每份每类别按文档哈希排序后取到约 EVAL_WORDS_CAP 词；越线的那篇在行边界截断，没取到的文档丢弃（不回流训练）。"""
    for (part, category), docs in candidates.items():
        path = eval_dir / part / f"{category}.txt"
        words = 0
        with open(path.with_name(path.name + ".tmp"), "w", encoding="utf-8", newline="\n") as out:
            for doc in sorted(docs, key=lambda d: d.key):
                if words >= EVAL_WORDS_CAP:
                    break
                lines = []
                for line in doc.lines:
                    lines.append(line)
                    words += len(line.split())
                    if words >= EVAL_WORDS_CAP:
                        break
                write_doc(out, lines)
        path.with_name(path.name + ".tmp").replace(path)
        if words < 2000:
            print(f"警告：{part}/{category} 只有 {words} 词，不到 2000", file=sys.stderr)


def file_stats(path: Path) -> tuple[int, int, int, str]:
    """统计一个输出文件的文档数（空行分隔）、非空行数、词数与 sha256。"""
    docs = lines = words = 0
    in_doc = False
    sha = hashlib.sha256()
    with open(path, "rb") as f:
        for raw in f:
            sha.update(raw)
            text = raw.decode("utf-8").strip()
            if text:
                lines += 1
                words += len(text.split())
                if not in_doc:
                    docs += 1
                in_doc = True
            else:
                in_doc = False
    return docs, lines, words, sha.hexdigest()


def output_files(data: Path) -> list[Path]:
    """全部输出文件：训练按来源、评测按份与类别，顺序固定。"""
    train = [data / "corpus/en/train" / f"{name}.txt" for name in SOURCES]
    evals = [data / "eval/english" / s / f"{c}.txt" for s in EVAL_SPLITS for c in CATEGORIES]
    return [p for p in train + evals if p.exists()]


def write_manifest(data: Path, eval_dir: Path) -> None:
    """MANIFEST.tsv：每个输出文件一行（相对 data/ 的路径、文档数、行数、词数、sha256）。"""
    rows = ["path\tdocuments\tlines\twords\tsha256"]
    for path in output_files(data):
        docs, lines, words, sha = file_stats(path)
        rows.append(f"{path.relative_to(data).as_posix()}\t{docs}\t{lines}\t{words}\t{sha}")
    (eval_dir / "MANIFEST.tsv").write_text("\n".join(rows) + "\n", encoding="utf-8")


def write_sources(eval_dir: Path, stats) -> None:
    """SOURCES.tsv：每个来源的数据集、revision、许可、类别，以及读了 / 用了 / 进训练多少。"""
    columns = ("source", "dataset", "revision", "license", "category", "raw_documents", "used_documents",
               "used_words", "train_documents", "train_words")
    rows = ["\t".join(columns)]
    for source, raw_docs, used_docs, used_words, train_docs, train_words in stats:
        rows.append(f"{source.name}\t{source.dataset}\t{source.revision}\t{source.license}\t{source.category}"
                    f"\t{raw_docs}\t{used_docs}\t{used_words}\t{train_docs}\t{train_words}")
    (eval_dir / "SOURCES.tsv").write_text("\n".join(rows) + "\n", encoding="utf-8")


def read_docs(path: Path):
    """按空行把输出文件拆回文档，逐个产出行列表。"""
    lines = []
    with open(path, encoding="utf-8") as f:
        for raw in f:
            text = raw.rstrip("\n")
            if text:
                lines.append(text)
            elif lines:
                yield lines
                lines = []
    if lines:
        yield lines


def verify(data: Path) -> bool:
    """自检：四份两两之间文档哈希的交集、长句（≥ LINE_DEDUP_MIN_WORDS 词）哈希的交集都应为空。"""
    docs: dict[str, set[bytes]] = {}
    lines: dict[str, set[bytes]] = {}
    for path in output_files(data):
        part = "train" if path.parent.name == "train" else path.parent.name
        doc_set = docs.setdefault(part, set())
        line_set = lines.setdefault(part, set())
        for doc in read_docs(path):
            doc_set.add(digest("\n".join(doc)))
            line_set.update(digest(line) for line in doc if len(line.split()) >= LINE_DEDUP_MIN_WORDS)
    ok = True
    for a, b in itertools.combinations(sorted(docs), 2):
        shared_docs, shared_lines = len(docs[a] & docs[b]), len(lines[a] & lines[b])
        print(f"{a} ∩ {b}：文档 {shared_docs}，长句 {shared_lines}（{a} {len(docs[a])} 篇，{b} {len(docs[b])} 篇）")
        ok = ok and shared_docs == 0 and shared_lines == 0
    print("通过" if ok else "有重复")
    return ok


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("command", nargs="?", default="all", choices=["all", "fetch", "split", "verify"])
    parser.add_argument("--sources", nargs="+", choices=list(SOURCES), default=list(SOURCES),
                        help="fetch 只下这些来源（默认全部）")
    parser.add_argument("--data", type=Path, default=ROOT / "data", help="数据根目录（默认仓库的 data/）")
    args = parser.parse_args()
    data = Path(os.path.realpath(args.data))
    if args.command in ("all", "fetch"):
        fetch(data / "corpus/en/raw", args.sources)
    if args.command in ("all", "split"):
        split(data)
    if args.command == "verify":
        sys.exit(0 if verify(data) else 1)


if __name__ == "__main__":
    main()
