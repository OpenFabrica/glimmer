use std::path::PathBuf;

use clap::Parser;

/// 按优先级挑一个存在的数据文件：`data/generated/` 里打包好的 `.qj`、那里的 TSV、仓库自带的产品数据
/// （`assets/lexicon/dict.tsv`、`assets/glossary/glossary-*.tsv`、`assets/lexicon/english.tsv`），最后是 `assets/sample/` 的样例。
pub fn default_data_file(name: &str) -> PathBuf {
    let generated = PathBuf::from("data/generated").join(name);
    let packed = generated.with_extension("qj");
    let shipped = if name.starts_with("glossary-") {
        PathBuf::from("assets/glossary").join(name)
    } else {
        PathBuf::from("assets/lexicon").join(name)
    };
    for candidate in [packed, generated, shipped] {
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from("assets/sample").join(name)
}

/// 缺省配置文件位置：与输入法共用同一份。
pub fn default_config_file() -> PathBuf {
    if cfg!(target_os = "macos")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join("Library/Application Support/Glimmer/config.toml");
    }
    PathBuf::from("config.toml")
}

#[derive(Debug, Parser)]
#[command(name = "glimmer", about = "微明输入法 Core 测试工具")]
pub struct Args {
    /// 词库路径（TSV）。缺省：data/generated/dict.tsv 存在就用它，否则 assets/sample/dict.tsv
    #[arg(long)]
    pub dict: Option<PathBuf>,

    /// 释义表路径。缺省：data/generated/glossary-<language>.tsv 存在就用它，否则 assets/sample/ 下的同名文件
    #[arg(long)]
    pub glossary: Option<PathBuf>,

    /// 学习语言：en / ja / es。也可用环境变量 GLIMMER_LEARNING_LANGUAGE
    #[arg(long, env = "GLIMMER_LEARNING_LANGUAGE", default_value = "en")]
    pub language: String,

    /// 附加词库（.qj 或 TSV），可给多个，与主词库一起查
    #[arg(long)]
    pub extra_dict: Vec<PathBuf>,

    /// 英文词表路径（中英混输）。缺省：data/generated/english.tsv 存在就用它，否则不启用
    #[arg(long)]
    pub english: Option<PathBuf>,

    /// 用户词频文件；给了就在退出时写回，不给则只在本次会话内学习
    #[arg(long)]
    pub user_dict: Option<PathBuf>,

    /// 配置文件路径。缺省：~/Library/Application Support/Glimmer/config.toml（macOS）或 ./config.toml
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// 启用云联想（无视配置里的 enabled）；密钥来自配置或 GLIMMER_API_KEY（`api_key_env`）
    #[arg(long)]
    pub predict: bool,

    /// 模糊音，逗号分隔（z-zh,c-ch,s-sh,n-l,f-h,l-r,an-ang,en-eng,in-ing），`all` 全开；给了就覆盖配置里的 [fuzzy]
    #[arg(long, value_delimiter = ',')]
    pub fuzzy: Vec<String>,

    /// 英文模式（输入法里是 Caps Lock 亮着）：字母不当拼音，候选来自英文词表的补全与拼错纠正
    #[arg(long)]
    pub english_mode: bool,

    /// 打开中文优先（配置 [general] chinese_first = true）：整段是英文词时中文候选排第一、英文第二，评测两种排法用
    #[arg(long)]
    pub chinese_first: bool,

    /// 拼音方案（pinyin / xiaohe / ziranma / microsoft / sogou / xiaolang / abc / zhuyin / none），覆盖配置里的 [general] scheme；none 关掉拼音侧（配 --wubi 就是只用五笔）
    #[arg(long)]
    pub scheme: Option<String>,

    /// 双拼方案的旧写法，等同 --scheme；off 强制全拼。两个都给时以 --scheme 为准
    #[arg(long)]
    pub shuangpin: Option<String>,

    /// 五笔（86 / 98 / xsj（新世纪，也可写 06）/ off），覆盖配置里的 [general] wubi；off 关掉五笔。与拼音方案同时开着就是混输。码表读 data/generated/wubi<版本>.qj，没有就报错退出
    #[arg(long)]
    pub wubi: Option<String>,

    /// 五笔整句输入，覆盖配置里的 [wubi] sentence：连着打编码出整句、不再四码自动上屏。配 --wubi 与 --scheme none 用；--eval-text 在只用五笔时按码表把句子转成编码来评
    #[arg(long)]
    pub wubi_sentence: bool,

    /// 语言模型：`lm.qj` 文件，或含 `lm.qj` / `lm-unigram.tsv` + `lm-bigram.tsv`（+ 可选 `lm-trigram.tsv`）的目录。
    /// 缺省 data/generated；给了就不动 data/generated 里那份
    #[arg(long)]
    pub lm: Option<PathBuf>,

    /// 神经重打分：字级 Transformer 的 .qjm 文件或导出目录（model.safetensors / config.json / vocab.json），整句前几条路径用它重排
    #[arg(long)]
    pub neural: Option<PathBuf>,

    /// 神经重打分的权重 λ（0 到 1，缺省 0.5）：最终分 = 路径分 + λ·(神经分 − 静态二元分)，个人学习与代价不受影响
    #[arg(long)]
    pub neural_weight: Option<f64>,

    /// 神经重打分的门槛（nat，缺省不设）：路径分落后最优路径超过这么多的不参与重排
    #[arg(long)]
    pub neural_margin: Option<f64>,

    /// 神经重打分给模型看的前文字符数（缺省 64，0 为不给前文）
    #[arg(long)]
    pub neural_context: Option<usize>,

    /// 神经重打分走后台线程（输入法壳里的接法）：查询先按词级模型出候选，再请求 / 等待重打分后重查一次；结果应与同步一致
    #[arg(long)]
    pub neural_async: bool,

    /// 逐键模式：把每个输入当作一键一键敲进去，每个前缀都查一次，打印每键各阶段耗时（性能测试用）
    #[arg(long)]
    pub typing: bool,

    /// 只显示前 N 个候选
    #[arg(long, default_value_t = 9)]
    pub limit: usize,

    /// 回放评测：读输入日志（input-log.jsonl），把每次上屏时的键重新喂给引擎，算首选命中率等指标。只在内存里学习，不写任何文件
    #[arg(long)]
    pub replay: Option<PathBuf>,

    /// 回放 / 整句评测时打印前 N 条没命中首选的例子
    #[arg(long, default_value_t = 20)]
    pub misses: usize,

    /// 覆盖引擎里的调参常数，`名=值`，逗号分隔或多次给。名字：lambda / k / cap / d2 / d3（个人 n-gram 插值 λ / K / 封顶 / 二元与三元的绝对折扣），
    /// transpose / substitute / extra / missing / typo-cap / correction（敲错四类代价 / 个人折扣上限 / 整段纠错代价），
    /// initial（整句第一个词按上文算的成分）/ protect（原样成词保护多扣的代价）/ word（每词代价）/ whole（整句让整词的门槛，0 为不让）/ paths（重排路径数）/
    /// span（词图每格候选数）/ reading / reading-cap（多音字读音份额扣分的系数与封顶）/ choice / choice-cap（词级排序里同输入串选过的加分系数与次数封顶）/ lm-mode（静态模型回退：0 老的固定 λ、1 绝对折扣、2 绝对折扣 + 接续概率）/ lm-d3 / lm-d2（两层的折扣 D）/
    /// lm-trigram（0 关掉三元那层，量三元自己的净效果）
    #[arg(long, value_delimiter = ',')]
    pub tune: Vec<String>,

    /// 整句评测：读中文文本（一行一段，按标点切句、按词库转成全拼）或 `--eval-save` 冻结下来的三列文件，
    /// 冷启动喂给引擎看整句能不能还原原句；可给多个文件
    #[arg(long, num_args = 1..)]
    pub eval_text: Vec<PathBuf>,

    /// 把整句评测用到的句子集写成 `句子\t拼音\t上文` 三列文件，下次直接 `--eval-text` 它，保证比的是同一份句子
    #[arg(long)]
    pub eval_save: Option<PathBuf>,

    /// 整句评测改成分段输入：每句按语言模型切词、每 N 个词一段逐段喂，前面各段的原文当上文（量「整句接上文」用）
    #[arg(long, value_name = "N")]
    pub eval_chunk: Option<usize>,

    /// 整句评测里上文怎么给引擎：chain 当成自己刚上屏的词（缺省），app 只经应用光标前文给、上屏链保持空
    /// （模拟接着对方的话 / 粘贴之后接着打）
    #[arg(long, value_name = "来源", default_value = "chain")]
    pub eval_context: crate::eval::ContextSource,

    /// 整句评测时给每条拼音注一处敲错（相邻键换位 / 敲到旁边的键，按句子哈希定，可复现）：量敲错纠正的召回
    #[arg(long)]
    pub eval_typos: bool,

    /// 先从这些中文文本里学个人 n-gram（按语言模型切词、逐词记转移），再做后面的事；给了 `--user-dict` 就随退出落盘
    #[arg(long, num_args = 1..)]
    pub learn_text: Vec<PathBuf>,

    /// 直接查询这些拼音后退出；不给则进入交互模式
    pub inputs: Vec<String>,
}
