use glimmer_dictionary::DictionaryError;
use glimmer_learning::LearningError;
use glimmer_lm::LmError;
use glimmer_neural::NeuralError;
use glimmer_platform::ConfigError;
use glimmer_predict::PredictError;
use glimmer_translate::GlossaryError;
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error(transparent)]
    Dictionary(#[from] DictionaryError),

    #[error(transparent)]
    Neural(#[from] NeuralError),

    #[error(transparent)]
    Glossary(#[from] GlossaryError),

    #[error(transparent)]
    Learning(#[from] LearningError),

    /// 学习语言不是 en / ja / es。
    #[error("learning language must be en, ja or es, got {0:?}")]
    Language(String),

    #[error(transparent)]
    Config(#[from] ConfigError),

    /// 五笔码表文件不存在。
    #[error(
        "wubi table not found: {0}; generate it with `dict-convert wubi` and `dict-convert pack dict` (see docs/notes/crate-notes.md)"
    )]
    WubiTable(std::path::PathBuf),

    #[error(transparent)]
    Predict(#[from] PredictError),

    #[error(transparent)]
    LanguageModel(#[from] LmError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Replay(#[from] crate::replay::ReplayError),

    #[error(transparent)]
    Eval(#[from] crate::eval::EvalError),

    #[error(transparent)]
    EvalEnglish(#[from] crate::eval::english::EnglishEvalError),

    #[error(transparent)]
    LearnText(#[from] crate::learn_text::LearnTextError),

    #[error(transparent)]
    Tune(#[from] crate::tuning::TuneError),
}
