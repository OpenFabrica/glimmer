//! 启动装配：按配置与随包资源找数据文件、装 Engine、建好 [`Router`]（本地整句模型、配置热加载都接上）。
//! Windows Server 与 Linux IBus 引擎共用，平台差异只在 [`StartupPaths`] 里。

mod paths;

use std::path::{Path, PathBuf};

use glimmer_core::{Engine, Language};
use glimmer_platform::Config;

use crate::assembly::{
    self, AssemblySpec, LanguageModelFiles, WubiSpec, glossary_file, learning_language,
};
use crate::dispatch::{self, Router, RouterConfig};
use crate::error::ServerError;

pub use self::paths::StartupPaths;

/// 装配 Engine 并建好 Router。正式词库装不起来回落样例词库，连样例都不行才报错。
/// 环境变量 `GLIMMER_DICT` / `GLIMMER_GLOSSARY` 可覆盖词库与释义表（开发用）。
pub fn build_router(paths: &StartupPaths, config: &Config) -> Result<Router, ServerError> {
    let root = paths.root.as_path();
    let language = learning_language(config);
    let dict = std::env::var_os("GLIMMER_DICT")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_dict(root));
    // 学习语言关着（`off`）就不装释义表
    let glossary_path = language.and_then(|language| {
        std::env::var_os("GLIMMER_GLOSSARY")
            .map(PathBuf::from)
            .or_else(|| glossary_file(root, language))
            .filter(|path| path.is_file())
    });
    let glossary = language.zip(glossary_path);
    let spec = AssemblySpec {
        glossary: glossary.clone(),
        english_glossary: glossary_file(root, Language::Chinese),
        english: generated(root, "english.tsv"),
        english_lm: generated(root, "lm-en.qj"),
        english_misspellings: generated(root, "english-misspellings.tsv"),
        emoji: ["emoji-zh.tsv", "emoji-en.tsv"]
            .into_iter()
            .filter_map(|name| asset(root, &format!("emoji/{name}")))
            .collect(),
        language_model: LanguageModelFiles::find(&root.join("data/generated")),
        bundled_dicts_dir: Some(root.join("data/generated/dicts")).filter(|dir| dir.is_dir()),
        dictionaries: config.dictionaries.clone(),
        levels_dir: Some(root.join("assets/levels")),
        user_dir: paths.user_dir.clone(),
        input_log: config.general.input_log,
        // 码表与 dict.qj 同目录；文件不在就记 warn 当没开
        wubi: config.general.wubi().map(|variant| WubiSpec {
            variant,
            options: config.wubi.options(),
        }),
        ..AssemblySpec::new(&dict)
    };
    let mut engine = assemble_with_fallback(spec, root)?;
    engine.set_fuzzy(config.fuzzy);
    engine.set_shuangpin(config.general.shuangpin());
    engine.set_zhuyin_mode(config.general.is_zhuyin());
    // 拼音侧：配置说关（`scheme = "none"`）且五笔真装上了才关；码表缺了就留着拼音兜底，不然一个候选都没有
    engine.set_phonetic(config.general.scheme().is_on() || !engine.wubi_mode());
    engine.set_traditional_mode(config.general.traditional);
    engine.set_mode_keys(config.shortcut.mode);
    engine.set_chinese_first(config.general.chinese_first);
    engine.set_mixed_english(config.general.mixed_english_candidates);
    engine.set_emoji_candidates(config.general.emoji_candidates);
    engine.set_translation_reading(config.general.translation_reading);
    engine.set_english_prediction(config.general.english_prediction);
    engine.log_session(paths.version, paths.platform);
    dispatch::attach_cloud(&mut engine, &config.predict);
    let router_config = RouterConfig::from(config);
    let mut router = Router::new(engine, router_config.clone());
    router.set_log_identity(paths.version, paths.platform);
    let model_path = dispatch::find_model(paths.user_dir.as_deref(), root);
    router.configure_local_model(model_path.clone(), &config.model);
    if let Some(path) = &paths.config_path {
        router.watch_config(
            config,
            path.clone(),
            root.to_path_buf(),
            paths.user_dir.clone(),
            Some(assembly::wubi_dir(&dict).to_path_buf()),
        );
    }
    tracing::info!(
        platform = paths.platform,
        version = paths.version,
        dict = %dict.display(),
        glossary = glossary.as_ref().map(|(_, p)| p.display().to_string()).unwrap_or_default(),
        language = language.map_or("off", |l| l.code()),
        page_size = router_config.page_size,
        page_keys = %format!("{}{}", router_config.page_keys.0, router_config.page_keys.1),
        layout = router_config.layout.key(),
        theme = router_config.theme.key(),
        scheme = config.general.scheme().key(),
        wubi = router.wubi_key().unwrap_or("关"),
        fuzzy = config.fuzzy.any(),
        cloud = config.predict.enabled,
        model = model_path.as_deref().map(|p| p.display().to_string()).unwrap_or_default(),
        model_enabled = config.model.enabled,
        "微明 Router 就绪"
    );
    Ok(router)
}

/// `<root>/data/generated/<name>`，不存在为 `None`。
fn generated(root: &Path, name: &str) -> Option<PathBuf> {
    existing(root.join("data/generated").join(name))
}

/// `<root>/assets/<rel>`，不存在为 `None`。
fn asset(root: &Path, rel: &str) -> Option<PathBuf> {
    existing(root.join("assets").join(rel))
}

/// 是文件就原样返回。
fn existing(path: PathBuf) -> Option<PathBuf> {
    path.is_file().then_some(path)
}

/// 正式词库，没有就回落手写样例。
fn default_dict(root: &Path) -> PathBuf {
    generated(root, "dict.qj").unwrap_or_else(|| sample_dict(root))
}

/// 随 git 的样例词库。
fn sample_dict(root: &Path) -> PathBuf {
    root.join("assets/sample/dict.tsv")
}

/// 正式词库装配失败回落样例词库，连样例都装不起来才报错。
fn assemble_with_fallback(mut spec: AssemblySpec, root: &Path) -> Result<Engine, ServerError> {
    assembly::assemble(&spec).or_else(|error| {
        tracing::error!(%error, dict = %spec.dict.display(), "正式词库装配失败，回落样例词库");
        spec.dict = sample_dict(root);
        assembly::assemble(&spec)
    })
}
