//! 微明 CLI：Phase 1 的测试工具。
//!
//! 输入拼音，打印候选（词性 + 译文）和各阶段耗时；输入序号上屏并记入用户词频。
//! 不依赖任何平台 API，是 Core 的第一个「壳」。

mod args;
mod display;
mod error;
mod eval;
mod learn_text;
mod logging;
mod next_word;
mod repl;
mod replay;
mod rescoring;
mod tuning;

use std::time::Instant;

use clap::Parser;
use glimmer_core::{EmojiTable, Engine, FuzzyRules, Language};
use glimmer_dictionary::{Dictionary, Misspellings, WordList};
use glimmer_learning::FrequencyLearner;
use glimmer_lm::NgramModel;
use glimmer_platform::{Config, Scheme};
use glimmer_predict::CloudPredictor;
use glimmer_translate::Glossary;

use crate::args::Args;
use crate::error::CliError;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), CliError> {
    dotenvy::dotenv().ok();
    let args = Args::parse();
    let _log_guard = logging::init()?;

    let started = Instant::now();
    let mut engine = build_engine(&args)?;
    tracing::info!(total_ms = started.elapsed().as_millis(), "Engine 就绪");
    engine.set_english_mode(args.english_mode);
    engine.set_chinese_first(args.chinese_first);
    tuning::apply(&mut engine, &args.tune)?;
    if !args.learn_text.is_empty() {
        learn_text::run(&mut engine, &args.learn_text)?;
    }
    if let Some(path) = &args.replay {
        let report = replay::run(&mut engine, path, args.misses)?;
        print!("{report}");
        return Ok(());
    }
    if !args.eval_text.is_empty() {
        let report = eval::run(
            &mut engine,
            &args.eval_text,
            args.eval_save.as_deref(),
            eval::Shape {
                chunk_words: args.eval_chunk,
                typos: args.eval_typos,
                context: args.eval_context,
            },
            args.misses,
        )?;
        print!("{report}");
        return Ok(());
    }
    if !args.eval_english.is_empty() {
        // 引擎里那份词表拿不出来，判断「词表里有没有这个词」另读一份
        let words = english_path(&args).map(WordList::from_path).transpose()?;
        let options = eval::english::Options {
            paths: args.eval_english.clone(),
            words: args.eval_english_words,
            seed: args.eval_english_seed,
            groups: args.eval_english_groups.clone(),
            prefix_cases: args.eval_english_cases.clone(),
            personal_cases: args.eval_english_personal.clone(),
            show_misses: args.misses,
        };
        let report = eval::english::run(&mut engine, words.as_ref(), &options)?;
        print!("{report}");
        return Ok(());
    }
    if args.inputs.is_empty() {
        repl::run(&mut engine, args.limit)?;
    } else {
        for input in &args.inputs {
            println!("> {input}");
            if args.english_next {
                next_word::show(&mut engine, input);
            } else if args.typing {
                display::show_typing(&mut engine, input);
            } else {
                display::show(&mut engine, input, args.limit);
            }
        }
    }
    engine.learner_mut().flush();
    Ok(())
}

/// 读配置文件并套上命令行的覆盖项（云联想、模糊音、双拼、五笔）。
fn load_config(args: &Args) -> Result<Config, CliError> {
    let config_path = args
        .config
        .clone()
        .unwrap_or_else(args::default_config_file);
    let mut config = Config::load(&config_path)?;
    if args.predict {
        config.predict.enabled = true;
    }
    if !args.fuzzy.is_empty() {
        let mut rules = FuzzyRules::default();
        for name in &args.fuzzy {
            if name == "all" {
                rules = FuzzyRules::ALL;
            } else if !rules.enable(name) {
                tracing::warn!(name, "不认识的模糊音规则，忽略");
            }
        }
        config.fuzzy = rules;
    }
    // `--shuangpin` 是 `--scheme` 的旧写法（同一个维度），两个都给时以 `--scheme` 为准
    if let Some(scheme) = args.scheme.as_ref().or(args.shuangpin.as_ref()) {
        config.general.scheme = if scheme == "off" {
            Scheme::Pinyin.key().to_owned()
        } else {
            scheme.clone()
        };
    }
    if let Some(wubi) = &args.wubi {
        config.general.wubi = if wubi == "off" {
            String::new()
        } else {
            wubi.clone()
        };
    }
    if args.wubi_sentence {
        config.wubi.sentence = true;
    }
    Ok(config)
}

/// 英文词表路径：`--english` 给了用它，否则 data/generated/english.tsv 存在就用它，都没有为 `None`（不启用英文）。
fn english_path(args: &Args) -> Option<std::path::PathBuf> {
    args.english.clone().or_else(|| {
        let path = args::default_data_file("english.tsv");
        path.is_file().then_some(path)
    })
}

/// 英文语言模型与误拼表：`--lm-en` / `--english-misspellings` 给了用它，否则 data/generated/ 下的同名文件；
/// 文件不在就跳过（英文候选按词频排、没有误拼表那一档），与输入法里一样正常降级。
fn with_english_extras(args: &Args, mut engine: Engine) -> Result<Engine, CliError> {
    let generated = std::path::Path::new("data/generated");
    let model_path = args
        .lm_en
        .clone()
        .unwrap_or_else(|| generated.join("lm-en.qj"));
    if model_path.is_file() {
        let started = Instant::now();
        let mut model = NgramModel::from_path(&model_path)?;
        model.set_smoothing(tuning::english_smoothing(&args.tune)?);
        tracing::info!(
            path = %model_path.display(),
            words = model.word_count(),
            bigrams = model.bigram_count(),
            trigrams = model.trigram_count(),
            smoothing = ?model.smoothing(),
            load_ms = started.elapsed().as_millis(),
            "英文语言模型已加载"
        );
        engine = engine.with_english_lm(Box::new(model));
    } else {
        tracing::info!(path = %model_path.display(), "没有英文语言模型，英文候选按词频排");
    }
    let table_path = args
        .english_misspellings
        .clone()
        .unwrap_or_else(|| generated.join("english-misspellings.tsv"));
    if table_path.is_file() {
        let table = Misspellings::from_path(&table_path)?;
        tracing::info!(path = %table_path.display(), pairs = table.len(), "英文误拼表已加载");
        engine = engine.with_english_misspellings(table);
    } else {
        tracing::info!(path = %table_path.display(), "没有英文误拼表");
    }
    Ok(engine)
}

/// 组装 Engine：这是 Core 之外唯一知道具体 Translator / Learner 类型的地方。
fn build_engine(args: &Args) -> Result<Engine, CliError> {
    let language: Language = args
        .language
        .parse()
        .map_err(|_| CliError::Language(args.language.clone()))?;
    if language == Language::Chinese {
        return Err(CliError::Language(args.language.clone()));
    }
    let config = load_config(args)?;
    let dict_path = args
        .dict
        .clone()
        .unwrap_or_else(|| args::default_data_file("dict.tsv"));
    let glossary_path = args
        .glossary
        .clone()
        .unwrap_or_else(|| args::default_data_file(&format!("glossary-{}.tsv", language.code())));

    let started = Instant::now();
    let dictionary = Dictionary::from_path(&dict_path)?;
    let dict_load = started.elapsed();
    let started = Instant::now();
    let glossary = Glossary::from_path(language, &glossary_path)?;
    let glossary_load = started.elapsed();
    let english_path = english_path(args);
    let started = Instant::now();
    let english = english_path.as_ref().map(WordList::from_path).transpose()?;
    let english_load = started.elapsed();
    // 五笔下按输入串记的表落在方案子目录（编码与拼音音节撞键）
    let scheme_key = config.general.wubi().map(|variant| variant.key());
    let learner = match &args.user_dict {
        Some(path) => FrequencyLearner::from_path_with_scheme(path, scheme_key)?,
        None => FrequencyLearner::default(),
    };
    tracing::info!(
        dict = %dict_path.display(),
        entries = dictionary.len(),
        glossary = %glossary_path.display(),
        glosses = glossary.len(),
        english = english.as_ref().map_or(0, WordList::len),
        learned = learner.len(),
        dict_ms = dict_load.as_millis(),
        glossary_ms = glossary_load.as_millis(),
        english_ms = english_load.as_millis(),
        "加载完成"
    );
    let mut engine = Engine::new(dictionary)
        .with_translator(Box::new(glossary))
        .with_learner(Box::new(learner));
    if !args.extra_dict.is_empty() {
        let mut extras = Vec::new();
        for path in &args.extra_dict {
            let dictionary = Dictionary::from_path(path)?;
            tracing::info!(path = %path.display(), entries = dictionary.len(), "附加词库已加载");
            extras.push(dictionary);
        }
        engine.set_extra_dictionaries(extras);
    }
    // 英文候选的中文释义可选
    let zh_glossary = args::default_data_file("glossary-zh.tsv");
    if zh_glossary.is_file() {
        let glossary = Glossary::from_path(Language::Chinese, &zh_glossary)?;
        tracing::info!(glosses = glossary.len(), "英→中释义表已加载");
        engine = engine.with_english_translator(Box::new(glossary));
    }
    if let Some(words) = english {
        engine = engine.with_english(words);
    }
    // emoji 表随仓库提供（Unicode License）：中文表 + 英文表合成一张，一张都没有就不出 emoji 候选
    let mut emoji: Option<EmojiTable> = None;
    let started = Instant::now();
    for name in ["emoji-zh.tsv", "emoji-en.tsv"] {
        let path = std::path::PathBuf::from("assets/emoji").join(name);
        if !path.is_file() {
            continue;
        }
        let table = EmojiTable::from_path(&path)?;
        match &mut emoji {
            Some(all) => all.merge(table),
            None => emoji = Some(table),
        }
    }
    if let Some(table) = emoji {
        tracing::info!(
            words = table.len(),
            load_ms = started.elapsed().as_millis(),
            "emoji 表已加载"
        );
        engine = engine.with_emoji(table);
    }
    // 语言模型可选：没有就退化成一元词频整句；打包过的 lm.qj 优先，`--lm` 换一份别的
    let dir = args
        .lm
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from("data/generated"));
    let (packed, unigram, bigram, trigram) = if dir.is_dir() {
        (
            dir.join("lm.qj"),
            dir.join("lm-unigram.tsv"),
            dir.join("lm-bigram.tsv"),
            dir.join("lm-trigram.tsv"),
        )
    } else {
        // 直接给了一个 .qj 文件
        (dir.clone(), dir.clone(), dir.clone(), dir.clone())
    };
    if packed.is_file() || (unigram.is_file() && bigram.is_file()) {
        let started = Instant::now();
        let mut model = if packed.is_file() {
            NgramModel::from_path(&packed)?
        } else {
            NgramModel::from_paths(&unigram, &bigram, trigram.is_file().then_some(&*trigram))?
        };
        model.set_smoothing(tuning::smoothing(&args.tune)?);
        tracing::info!(
            words = model.word_count(),
            bigrams = model.bigram_count(),
            trigrams = model.trigram_count(),
            smoothing = ?model.smoothing(),
            load_ms = started.elapsed().as_millis(),
            "语言模型已加载"
        );
        engine = engine.with_language_model(Box::new(model));
    }
    engine = with_english_extras(args, engine)?;
    if let Some(dir) = &args.neural {
        let started = Instant::now();
        let scorer = glimmer_neural::CharScorer::load(dir)?;
        tracing::info!(
            load_ms = started.elapsed().as_millis(),
            weight = args.neural_weight.unwrap_or(glimmer_core::NEURAL_WEIGHT),
            "神经重打分已启用"
        );
        engine = if args.neural_async {
            engine.with_async_sentence_scorer(
                Box::new(scorer),
                args.neural_weight,
                args.neural_margin,
                args.neural_context,
            )
        } else {
            engine.with_sentence_scorer(
                Box::new(scorer),
                args.neural_weight,
                args.neural_margin,
                args.neural_context,
            )
        };
    }
    if config.fuzzy.any() {
        tracing::info!(rules = ?config.fuzzy, "模糊音已启用");
    }
    engine.set_traditional_mode(config.general.traditional);
    engine.set_fuzzy(config.fuzzy);
    engine.set_mode_keys(config.shortcut.mode);
    // 两条轴：拼音侧看 `[general] scheme`，形码侧看 `[general] wubi`，两边都开就是混输
    let scheme = config.general.scheme();
    tracing::info!(
        pinyin = scheme.key(),
        wubi = config.general.wubi().map_or("关", |v| v.key()),
        mixed = config.general.mixed(),
        "输入方案已启用"
    );
    engine.set_shuangpin(scheme.shuangpin());
    engine.set_zhuyin_mode(scheme == Scheme::Zhuyin);
    // 拼音侧关掉且五笔开着才是「只用形码」；两边都关着时留拼音兜底
    engine.set_phonetic(scheme.is_on() || config.general.wubi().is_none());
    // 五笔：码表只认 data/generated/ 下打包好的 .qj，没有就报错（用 dict-convert 生成）
    if let Some(variant) = config.general.wubi() {
        let path = std::path::PathBuf::from("data/generated").join(variant.data_file());
        if !path.is_file() {
            return Err(CliError::WubiTable(path));
        }
        let started = Instant::now();
        let table = Dictionary::from_path(&path)?;
        let scheme = glimmer_core::wubi::Scheme::new(variant, table, config.wubi.options());
        tracing::info!(
            scheme = variant.key(),
            entries = scheme.dictionary().len(),
            chars = scheme.reverse().len(),
            load_ms = started.elapsed().as_millis(),
            "五笔已启用"
        );
        engine.set_wubi(Some(scheme));
    }
    if config.predict.enabled {
        let predictor = CloudPredictor::new(&config.predict)?;
        engine = engine.with_predictor(Box::new(predictor));
    }
    Ok(engine)
}
