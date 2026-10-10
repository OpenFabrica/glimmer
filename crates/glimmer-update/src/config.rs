//! 配置文件 `[update]` 分节。

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// 缺省的发版清单地址：GitHub latest Release 上的 `releases.json`（每次发版都按全部 Release 重生成并覆盖到 latest）。
pub const DEFAULT_FEED_URL: &str =
    "https://github.com/OpenFabrica/glimmer/releases/latest/download/releases.json";

/// 自动检查的周期：距上次检查不到这么久不查；进程一直开着时每隔这么久再查一次。
pub const CHECK_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60);

/// 配置文件 `[update]` 分节：自动检查开关与清单地址。
///
/// 检查只下载一份几 KB 的版本清单，不带任何个人信息；找到新版本只提示，安装要用户自己点。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateConfig {
    /// 自动检查（每 12 小时最多一次）。
    pub check: bool,

    /// 发版清单地址，一般不用改。
    pub feed_url: String,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            check: true,
            feed_url: DEFAULT_FEED_URL.to_owned(),
        }
    }
}
