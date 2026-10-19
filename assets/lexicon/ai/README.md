# AI 与软件开发词库源

面向 Claude Code、Codex 等辅助开发流程，覆盖 AI 技术、向量检索、日常开发、项目协作、Git、语言框架、包管理器、测试、部署和配置文件。
仍使用领域 ID `ai`，已有开关不需要迁移；显示名称从「AI 与人工智能」更新为「AI 与软件开发」，默认关闭。

本版有 **8,279 个词条、8,278 个不同词形**（C++ 有两个显式别名）。其中复用仓库既有 IT 词库 2,322 条，新增编写的开发与项目词 1,523 条，新增工具名称和混合词 1,506 条，保留首版 2,928 条。
词条含 AI 辅助整理和按规则筛选的内容，不表示全部经过专业术语委员会审核；不通过词语笛卡尔积凑条数。

## 数据与来源

| 文件 | 内容 |
|---|---|
| `terms.tsv` | 首版中文 AI 术语 |
| `names.tsv` | 首版模型厂商、名称、缩写及 AI 混合词 |
| `domains/development.tsv` | 开发操作、项目协作、架构、测试、排障、数据、前端、桌面、代理及向量术语 |
| `domains/it.tsv` | 从现有 `assets/lexicon/dicts/it_computing.tsv` 筛选复用的术语 |
| `domains/tools.tsv` | 工具、框架、包管理器、配置文件、英文别名及中英混合词 |
| `corpus.txt` / `corpus-sources.tsv` | 首版 2,486 段深度学习教材正文及逐行出处 |
| `development.txt` / `development-sources.tsv` | 新增 3,645 段 Vue、Rust 中文文档正文及逐行出处 |
| `sources.json` / `LICENSE.*` | 来源、固定提交、归档校验值、处理方法、署名与许可 |

正文是公开来源的真实文本；不把生成的测试句或任务清单用作词频语料。
开发正文提取脚本为 `tools/corpus/development.py`，只接受 `sources.json` 中 SHA256 匹配的本地上游归档，不联网、不扫描私人仓库。
保留中文正文，去代码、HTML 和引用标记，按段落去重。重建命令：

```bash
python3 tools/corpus/development.py --vue /path/to/vue.tar.gz --rust-book /path/to/rust-book.tar.gz
```

## 词条格式与编码

八列 TSV：`词<Tab>编码<Tab>人工权重<Tab>分类<Tab>类型<Tab>实体<Tab>来源<Tab>核对日期`。注释以 `#` 开头。

| 类型 | 编码规则 | 示例 |
|---|---|---|
| `term` / `chinese` | 每个汉字一个规范拼音音节 | `向后兼容 / xiang hou jian rong` |
| `english` | 名称去标点、空格后的 ASCII 小写字母数字码 | `Claude Code / claudecode` |
| `alias` | 显式指定的、字母开头的小写 ASCII 字母数字码，最长 128 字节 | `C++ / cpp`、`C# / csharp`、`.env / envfile` |
| `mixed` | 显式别名，显示内容同时含中文和英文字母 | `Git分支 / gitfenzhi`、`API接口 / apijiekou` |

旧式 `AI模型 / ai mo xing` 仍支持，保留其全拼、简拼、双拼输入行为；新式混合词是英文名称加全拼尾部的完整输入码，不自动转换为双拼。
同一词允许不同别名；同一别名指向不同词则构建失败，并报告两条来源位置。`go.work / gowork` 与环境变量 `GOWORK / goworkenv` 分开。
名称、源码文件不是大小写不敏感的路径匹配器；显示使用源文件名，英文模式继续遵守原有大小写输入规则。

生成后英文、别名及新式混合词都使用 `@` 键，不进入拼音词图。普通导入 TSV 也可显式使用该格式。
词表上的核对日期是本次整理快照日期，不表示逐项在线验证或模型/工具仍是最新版。

## 生成与覆盖盘点

在仓库根目录运行，无需联网或 API 密钥：

```bash
cargo run --release --locked -p glimmer-dict-convert -- ai \
  --exclude assets/lexicon/dict.tsv \
  --corpus assets/lexicon/ai/corpus.txt --corpus assets/lexicon/ai/development.txt
```

`terms.tsv`、`names.tsv` 和 `domains/*.tsv` 一起校验后打包。输出：

- `data/generated/dicts/ai.tsv` / `ai.qj`：可加载词库；相对仓库主词库去重后 7,726 条（2026-10-19）。
- `data/generated/ai-audit.tsv`：人工权重、真实正文次数、最终权重、加入或已存在状态。
- `data/generated/ai-coverage.tsv`：主词库、可选 IT 词库、主英文表的覆盖情况，以及英文同码其他词；参考表缺失标为 `unknown`。

只自动排除 `--exclude` 指定主词库的同词同码，编码类型也参与区分。IT 词库默认关闭，不能用它去除 AI/开发词，否则单独开启后会缺词。
英文主表同词不强制去掉，保留领域标准大小写与优先级，查询时去重；不同显示词的同码关系列入覆盖报告。
构建内部的别名冲突直接失败；跨用户导入词库的冲突在加载时警告并保留先加载者，不静默改写别人的词条。

人工权重：既有 AI 术语 20、开发术语 30、复用 IT 词 15、工具名称 40。无真实出现次数时仍明确保留人工权重。
最终词级权重是人工值与封顶 200 的正文出现次数的较大值。中文按非重叠子串计数；英文名称要求 ASCII 标识符边界，不把 C 从 CSS、C++ 中拆出计数。
语料按段落跨文件去重，文件/段落之间不拼接。正文计数不是分词后的通用语言模型一元概率。

## 独立验收与语言模型实验

`tools/eval/development/acceptance.tsv` 是按开发任务独立编写的 164 项验收，不从词表批量反向生成；`general.txt` 为 80 条普通中文回归输入。
运行：

```bash
cargo build -p glimmer-cli --locked
python3 tools/eval/development/run.py \
  --cli target/debug/glimmer-cli --extra data/generated/dicts/ai.qj
```

中文/混合词前 5 命中目标 ≥95%，必备英文与文件名按各自最大位置全部通过，普通中文首选不得回退；JSON 报告保留实际候选及预热后的完整查询耗时。
`--baseline-extra /path/to/old-ai.qj` 可比较上一版。另用 CLI `--typing` 测逐键耗时。验收集不作为词频语料。

真实语料的一元和相邻词统计复用 `bigram` 工具，写到隔离实验目录，避免覆盖产品模型：

```bash
mkdir -p data/generated/ai-corpus
cat assets/lexicon/dict.tsv data/generated/dicts/ai.tsv > data/generated/ai-corpus/dict.tsv
cargo run --release --locked -p glimmer-dict-convert -- --out-dir data/generated/ai-corpus \
  bigram assets/lexicon/ai/corpus.txt assets/lexicon/ai/development.txt --dict data/generated/ai-corpus/dict.tsv
```

当前不自动替换全局 bigram 模型。真实语料仍不能覆盖所有新工具和项目表达，未出现的词采用透明的人工权重。

## 许可与维护

自编数据沿用 GPL-3.0-or-later；深度学习教材 Apache-2.0；AI Glossary in Mandarin CC-BY-SA-4.0；THUOCL MIT 与规范拼音 Unicode-3.0；Vue 正文 CC-BY-4.0；Rust 中文正文采用其 MIT 许可。
作者、固定提交和处理方式见 `sources.json`，完整许可见 `LICENSE.*`，三平台安装包沿用通配规则附带这些文件。
第三方词表有筛选、去重、拼音修订及译名修订；文档正文删去代码和标记。名称只记录公开事实，不复制产品说明，不引入厂商模型权重。
未引入雾凇词库或非商业限制词库。

具体项目的内部模块、服务及业务专名使用个人 `dicts/` 导入和开关，不能自动汇入随包词库，见用户文档「项目词库」。
更新词表后运行生成器、Core 测试与独立验收，再生成产品数据包；修改源码不等于已安装或发布。
