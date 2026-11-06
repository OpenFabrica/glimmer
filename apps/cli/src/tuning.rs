//! `--tune 名=值`：把引擎里几组「拍的」常数换成别的值，回放评测时扫参数用；名字见 [`KEYS`]。
//! `en-` 开头的是英文候选打分的常数（[`glimmer_core::english::ScoreParams`]），`en-predict-` 开头的是英文下一词预测的
//! （[`glimmer_core::english::PredictParams`]），`lm-en-` 开头的是英文语言模型的平滑参数。

use glimmer_core::Engine;
use glimmer_core::correction::TypoCosts;
use glimmer_core::english::{PredictParams, ScoreParams};
use glimmer_core::sentence::Interpolation;
use glimmer_lm::{BackoffMode, Smoothing};

/// 可调的参数名。`lm-` 开头的是静态语言模型自己的（[`smoothing`] 读，模型加载时就要用上），
/// 其余的应用在引擎上（[`apply`]）。
pub const KEYS: [&str; 43] = [
    "lambda",
    "k",
    "cap",
    "d2",
    "d3",
    "initial",
    "word",
    "whole",
    "span",
    "reading",
    "reading-cap",
    "choice",
    "choice-cap",
    "transpose",
    "substitute",
    "extra",
    "missing",
    "typo-cap",
    "correction",
    "correction-transpose",
    "protect",
    "paths",
    "lm-mode",
    "lm-d3",
    "lm-d2",
    "lm-trigram",
    "en-misspelling",
    "en-near",
    "en-edit",
    "en-two",
    "en-literal",
    "en-typo-complete",
    "en-lambda-cap",
    "en-lambda-half",
    "en-k",
    "en-domain",
    "en-zipf-floor",
    "en-predict-min",
    "en-predict-count",
    "lm-en-mode",
    "lm-en-d3",
    "lm-en-d2",
    "lm-en-trigram",
];

/// 把一组 `名=值` 应用到引擎；没给的项保持缺省。
pub fn apply(engine: &mut Engine, settings: &[String]) -> Result<(), TuneError> {
    if settings.is_empty() {
        return Ok(());
    }
    let mut interpolation = Interpolation::default();
    let mut costs = TypoCosts::default();
    let mut english = ScoreParams::DEFAULT;
    let mut predict = PredictParams::DEFAULT;
    for setting in settings {
        let (key, value) = setting
            .split_once('=')
            .ok_or_else(|| TuneError::Syntax(setting.clone()))?;
        let value: f64 = value
            .trim()
            .parse()
            .map_err(|_| TuneError::Value(setting.clone()))?;
        match key.trim() {
            "lambda" => interpolation.lambda = value,
            "k" => interpolation.confidence_k = value,
            "cap" => interpolation.max_confidence = value,
            // 个人 n-gram 两层的绝对折扣，名字与静态模型那两层（lm-d2 / lm-d3）对上
            "d2" => interpolation.bigram_discount = value,
            "d3" => interpolation.trigram_discount = value,
            "initial" => interpolation.initial_weight = value,
            "word" => interpolation.word_penalty = value,
            // 整句让整词的门槛，0 为不让
            "whole" => interpolation.whole_word_margin = value,
            "span" => interpolation.span_candidates = (value as usize).max(1),
            // 多音字读音份额扣分的系数与封顶
            "reading" => interpolation.reading_weight = value,
            "reading-cap" => interpolation.reading_cap = value,
            "choice" => interpolation.choice_bonus = value,
            "choice-cap" => interpolation.choice_cap = value.max(0.0) as u32,
            "transpose" => costs.transpose = value,
            "substitute" => costs.substitute = value,
            "extra" => costs.extra = value,
            "missing" => costs.missing = value,
            "typo-cap" => costs.discount_cap = value,
            "correction" => costs.correction_penalty = value,
            "correction-transpose" => costs.correction_transpose_discount = value,
            "protect" => costs.protected_extra = value,
            "paths" => engine.set_rescore_paths(value as usize),
            // 英文候选打分（见 glimmer_core::english::score 的公式）
            "en-misspelling" => english.misspelling = value,
            "en-near" => english.near = value,
            "en-edit" => english.edit = value,
            "en-two" => english.two_edits = value,
            "en-literal" => english.literal = value,
            "en-typo-complete" => english.typo_completion = value,
            "en-lambda-cap" => english.lambda_cap = value,
            "en-lambda-half" => english.lambda_half = value,
            "en-k" => english.personal_k = value,
            "en-domain" => english.domain_bonus = value,
            "en-zipf-floor" => english.zipf_floor = value,
            // 英文下一词预测：第一名的概率下限、最多给几个
            "en-predict-min" => predict.min_probability = value,
            "en-predict-count" => predict.count = value.max(0.0) as usize,
            // 语言模型自己的参数在建模型时就用掉了，这里跳过
            name if name.starts_with("lm-") => {}
            other => {
                return Err(TuneError::Unknown {
                    name: other.to_owned(),
                });
            }
        }
    }
    tracing::info!(?interpolation, ?costs, ?english, ?predict, "参数覆盖");
    engine.set_interpolation(interpolation);
    engine.set_typo_costs(costs);
    engine.set_english_score(english);
    engine.set_english_predict_params(predict);
    Ok(())
}

/// 从同一组 `名=值` 里挑出静态语言模型的平滑参数（`lm-` 开头）；没给的保持缺省。
/// 模型是在引擎装配之前建的，所以它单独读一遍，不走 [`apply`]。
pub fn smoothing(settings: &[String]) -> Result<Smoothing, TuneError> {
    model_smoothing(settings, "lm-")
}

/// 英文语言模型的平滑参数（`lm-en-` 开头，名字后半与中文模型的一样）；没给的保持缺省。
pub fn english_smoothing(settings: &[String]) -> Result<Smoothing, TuneError> {
    model_smoothing(settings, "lm-en-")
}

/// 挑出以 `prefix` 开头的平滑参数。中文模型（`lm-`）不收英文模型（`lm-en-`）的项。
fn model_smoothing(settings: &[String], prefix: &str) -> Result<Smoothing, TuneError> {
    let mut smoothing = Smoothing::DEFAULT;
    for setting in settings {
        let (key, value) = setting
            .split_once('=')
            .ok_or_else(|| TuneError::Syntax(setting.clone()))?;
        let key = key.trim();
        let Some(name) = key.strip_prefix(prefix) else {
            continue;
        };
        if prefix == "lm-" && name.starts_with("en-") {
            continue;
        }
        let value: f64 = value
            .trim()
            .parse()
            .map_err(|_| TuneError::Value(setting.clone()))?;
        match name {
            "mode" => smoothing.mode = BackoffMode::from_code(value as u8),
            "d3" => smoothing.trigram_discount = value,
            "d2" => smoothing.bigram_discount = value,
            "trigram" => smoothing.use_trigram = value != 0.0,
            _ => {
                return Err(TuneError::Unknown {
                    name: key.to_owned(),
                });
            }
        }
    }
    Ok(smoothing)
}

#[derive(Debug, thiserror::Error)]
pub enum TuneError {
    #[error("--tune expects name=value, got {0:?}")]
    Syntax(String),

    #[error("--tune value is not a number: {0:?}")]
    Value(String),

    #[error("unknown --tune name {name:?}; known: {}", KEYS.join(", "))]
    Unknown { name: String },
}

#[cfg(test)]
mod tests {
    use glimmer_dictionary::Dictionary;

    use super::*;

    fn engine() -> Engine {
        Engine::new(Dictionary::parse("我\two\t100\n").unwrap())
    }

    #[test]
    fn empty_settings_keep_defaults() {
        let mut engine = engine();
        apply(&mut engine, &[]).unwrap();
        assert_eq!(engine.interpolation(), Interpolation::DEFAULT);
        assert_eq!(engine.typo_costs(), TypoCosts::DEFAULT);
    }

    #[test]
    fn settings_override_only_the_named_values() {
        let mut engine = engine();
        apply(
            &mut engine,
            &["lambda=0.7".to_owned(), " substitute = 5.5 ".to_owned()],
        )
        .unwrap();
        let interpolation = engine.interpolation();
        assert_eq!(interpolation.lambda, 0.7);
        assert_eq!(
            interpolation.confidence_k,
            Interpolation::DEFAULT.confidence_k
        );
        let costs = engine.typo_costs();
        assert_eq!(costs.substitute, 5.5);
        assert_eq!(costs.transpose, TypoCosts::DEFAULT.transpose);
    }

    #[test]
    fn language_model_settings_are_read_separately() {
        let settings = ["lm-mode=1".to_owned(), "lambda=0.7".to_owned()];
        let smoothing = smoothing(&settings).unwrap();
        assert_eq!(smoothing.mode, glimmer_lm::BackoffMode::Absolute);
        assert_eq!(
            smoothing.trigram_discount,
            Smoothing::DEFAULT.trigram_discount
        );
        // apply 不会因为 lm- 的名字报错
        let mut engine = engine();
        apply(&mut engine, &settings).unwrap();
        assert_eq!(engine.interpolation().lambda, 0.7);
    }

    #[test]
    fn english_settings_reach_the_engine_and_the_english_model() {
        let settings = [
            "en-edit=0.001".to_owned(),
            "lm-en-d2=1.5".to_owned(),
            "lm-d2=2".to_owned(),
        ];
        let mut engine = engine();
        apply(&mut engine, &settings).unwrap();
        assert_eq!(engine.english_score().edit, 0.001);
        assert_eq!(engine.english_score().near, ScoreParams::DEFAULT.near);
        assert_eq!(english_smoothing(&settings).unwrap().bigram_discount, 1.5);
        // 两个模型的同名参数互不干扰
        assert_eq!(smoothing(&settings).unwrap().bigram_discount, 2.0);
        assert_eq!(
            english_smoothing(&["lm-d2=2".to_owned()])
                .unwrap()
                .bigram_discount,
            Smoothing::DEFAULT.bigram_discount
        );
    }

    #[test]
    fn prediction_settings_reach_the_engine() {
        let mut engine = engine();
        apply(
            &mut engine,
            &["en-predict-count=3".to_owned(), "en-edit=0.001".to_owned()],
        )
        .unwrap();
        let predict = engine.english_predict_params();
        assert_eq!(predict.count, 3);
        assert_eq!(
            predict.min_probability,
            PredictParams::DEFAULT.min_probability
        );
        apply(&mut engine, &["en-predict-min=0.25".to_owned()]).unwrap();
        assert_eq!(engine.english_predict_params().min_probability, 0.25);
        // 没给的回到缺省（每次 apply 整组换）
        assert_eq!(engine.english_predict_params().count, 1);
    }

    #[test]
    fn rejects_bad_syntax_values_and_names() {
        let mut engine = engine();
        assert!(matches!(
            apply(&mut engine, &["lambda".to_owned()]),
            Err(TuneError::Syntax(_))
        ));
        assert!(matches!(
            apply(&mut engine, &["lambda=abc".to_owned()]),
            Err(TuneError::Value(_))
        ));
        assert!(matches!(
            apply(&mut engine, &["gamma=1".to_owned()]),
            Err(TuneError::Unknown { .. })
        ));
    }
}
