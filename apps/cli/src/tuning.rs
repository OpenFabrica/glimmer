//! `--tune 名=值`：把引擎里几组「拍的」常数换成别的值，回放评测时扫参数用；名字见 [`KEYS`]。

use glimmer_core::Engine;
use glimmer_core::correction::TypoCosts;
use glimmer_core::sentence::Interpolation;
use glimmer_lm::{BackoffMode, Smoothing};

/// 可调的参数名。`lm-` 开头的是静态语言模型自己的（[`smoothing`] 读，模型加载时就要用上），
/// 其余的应用在引擎上（[`apply`]）。
pub const KEYS: [&str; 26] = [
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
];

/// 把一组 `名=值` 应用到引擎；没给的项保持缺省。
pub fn apply(engine: &mut Engine, settings: &[String]) -> Result<(), TuneError> {
    if settings.is_empty() {
        return Ok(());
    }
    let mut interpolation = Interpolation::default();
    let mut costs = TypoCosts::default();
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
            // 语言模型自己的参数在建模型时就用掉了，这里跳过
            name if name.starts_with("lm-") => {}
            other => {
                return Err(TuneError::Unknown {
                    name: other.to_owned(),
                });
            }
        }
    }
    tracing::info!(?interpolation, ?costs, "参数覆盖");
    engine.set_interpolation(interpolation);
    engine.set_typo_costs(costs);
    Ok(())
}

/// 从同一组 `名=值` 里挑出静态语言模型的平滑参数（`lm-` 开头）；没给的保持缺省。
/// 模型是在引擎装配之前建的，所以它单独读一遍，不走 [`apply`]。
pub fn smoothing(settings: &[String]) -> Result<Smoothing, TuneError> {
    let mut smoothing = Smoothing::DEFAULT;
    for setting in settings {
        let (key, value) = setting
            .split_once('=')
            .ok_or_else(|| TuneError::Syntax(setting.clone()))?;
        let key = key.trim();
        if !key.starts_with("lm-") {
            continue;
        }
        let value: f64 = value
            .trim()
            .parse()
            .map_err(|_| TuneError::Value(setting.clone()))?;
        match key {
            "lm-mode" => smoothing.mode = BackoffMode::from_code(value as u8),
            "lm-d3" => smoothing.trigram_discount = value,
            "lm-d2" => smoothing.bigram_discount = value,
            "lm-trigram" => smoothing.use_trigram = value != 0.0,
            other => {
                return Err(TuneError::Unknown {
                    name: other.to_owned(),
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
