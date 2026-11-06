//! 个人词回归用例的通过条件。

/// 查前缀时个人词该排在哪。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rule {
    /// 个人词排第一（`first`）。
    First,

    /// 个人词不能压过这个词：这个词在候选里，而且排在个人词前面（`below:hello`）。
    Below(String),
}

impl Rule {
    /// 解析用例表里的写法；认不出为 `None`。
    pub fn parse(text: &str) -> Option<Self> {
        if text == "first" {
            return Some(Self::First);
        }
        let word = text.strip_prefix("below:")?;
        (!word.is_empty()).then(|| Self::Below(word.to_owned()))
    }

    /// 个人词排 `own`、对手排 `rival`（都是下标，不在候选里为 `None`）时过不过。
    pub fn passes(&self, own: Option<usize>, rival: Option<usize>) -> bool {
        match self {
            Self::First => own == Some(0),
            Self::Below(_) => match (rival, own) {
                (Some(rival), Some(own)) => rival < own,
                (Some(_), None) => true,
                (None, _) => false,
            },
        }
    }

    /// 要比的那个词；`first` 没有。
    pub fn rival(&self) -> Option<&str> {
        match self {
            Self::First => None,
            Self::Below(word) => Some(word),
        }
    }
}
