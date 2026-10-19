# 输入法字词库

面向中英混合输入的分类文本字词库，包含规范汉字、现代汉语常用词、11类中文领域词和英文词库，提供TXT与TSV两种格式，可用于词条检索、候选生成和词库索引构建。

## 内容

| 目录 | 内容 |
| --- | --- |
| `01_characters` | 8,105个规范汉字；另分一级3,500字、二级3,000字、三级1,605字 |
| `02_common` | liuxilu版现代汉语常用词表；56,008个原始词号，拆分异写后56,064条记录、55,735个不同词形 |
| `03_domains` | IT与计算机、财经、地名、历史人物、医学、饮食、法律、汽车、动物、成语、诗词名句 |
| `04_internet_slang` | 网络用语与聊天短语[候选表](04_internet_slang/candidates.tsv)1,294条，其中词库没有的752条**已于2026-09-22并入**（汉字词拆成[slang_words.tsv](04_internet_slang/slang_words.tsv)，中英混杂词进[mixed_words.tsv](mixed_words.tsv)）；收词标准、次数档位与接入命令见[目录说明](04_internet_slang/README.md) |
| `05_english` | 英文补全总表114,876词形，含常用词、扩展词、专名与技术词；附词形变化及误拼对照 |
| `00_meta` | 分类统计、异写关系、异常DF、文件校验值与许可证 |

领域词跨类去重后共156,289个不同词形。各领域保留独立词表，允许同一词条属于多个领域。完整数量见[分类统计](00_meta/category_stats.tsv)。英文内容和字段见[英文词库说明](05_english/README.md)。

## 文件格式

所有文本采用UTF-8编码。以下统一字段适用于中文字词库；英文词表、词形变化和误拼对照使用各自的字段，详见英文目录。

- **TXT**：一行一个词形，同一文件内去重，适合词表查询或导入不需要权重的系统。
- **TSV**：Tab分隔、首行为字段名，适合保留读音、排序与来源的导入流程。

| 字段 | 含义 |
| --- | --- |
| `word` | 词条：汉字、词语或固定短语 |
| `pinyin` | 源词表提供的读音；仅通用词库有值 |
| `rank` | 排序号：标准字库中的字表编号，或通用词库中的原词号；不是出现次数 |
| `doc_freq` | THUOCL的DF，即包含该词的文档数 |
| `level` | 规范汉字的1、2、3级 |
| `source` | 数据来源标识 |

字段名、JSON 键与领域标识（`places`、`law`……）统一用英文，中文含义见本表与各文件说明。

空字段表示数据缺失，不表示零。通用词的排序号越小通常越常用。不同领域DF所依据的语料不同，不宜直接比较为统一权重。

## 使用

1. 选择规范字全表或分级字表，两者无需重复加载。
2. 导入通用词TSV；同一词形可能有不同读音或原词号，应按引擎规则合并。
3. 按需加载领域词库，保留多领域归属，并处理与通用词库的重叠。
4. 补充缺失读音、建立候选权重，再编译为输入法支持的查询索引。

通用词表的分号异写已拆为独立记录，主词形与异写的对应关系见[异写关系](00_meta/variant_forms.tsv)。领域词按词形去除首尾空白并去重，重复词形的有效DF取最大值；无效DF留空，原值见[异常DF](00_meta/anomalous_df.tsv)。

## sources

- **规范字**：[iDvel文字转录](https://github.com/iDvel/The-Table-of-General-Standard-Chinese-Characters)，以[shengdoushi分级字表](https://github.com/shengdoushi/common-standard-chinese-characters-table)交叉核对。两份字序一致。此数据仅提供字形与级别，不含完整读音。
- **通用词**：[liuxilu校对词表](https://github.com/liuxilu/Proofread-Modern-Chinese-Common-Lexicon)。读音仍需按输入法需求校验。
- **领域词**：[THUOCL](https://github.com/thunlp/THUOCL)，采用[MIT许可证](00_meta/THUOCL_LICENSE.txt)，分发时保留版权及许可声明。
- **网络用语与聊天短语**（微明加的，`04_internet_slang/slang_words.tsv` 与 `mixed_words.tsv`）：起草者（Claude）自身的中文互联网知识，没有抄任何外部词表，**不引入新的许可约束**；次数按三档固定值给，部分取微明语料的成分二元合成次数；做法与数字见[internet-slang.md](../../docs/notes/internet-slang.md)。
- **补充常用词**（微明加的，`common_words.tsv`）：词库缺的常用词，候选来自[CC-CEDICT](https://www.mdbg.net/chinese/dictionary?page=cc-cedict)（MDBG，[CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/)）与[jieba](https://github.com/fxsjy/jieba)词表（MIT），次数由微明语料统计合成；第一轮（2026-09-17）逐条人工审定，第二轮（2026-10-19）放宽门槛，另收起草者（Claude）凭自身知识按界面 / 购物 / 聊天 / 办公 / 生活服务列的常用说法（没有抄任何外部词表），由 LLM 分批初审、两位审核员细查，未经人工逐条复核；读音都逐条核对；做法见[GLIMMER.md](GLIMMER.md)第4f步与[common-words.md](../../docs/notes/common-words.md)。

英文来源为ESDB、CSpell和typos，许可声明见[英文sources](05_english/README.md#sources)。

本数据包不以单一许可证覆盖所有来源。上游文件、版本及质量参考见相邻的[资源归档](../输入法词库资源_2026-09-05/README.md)。
