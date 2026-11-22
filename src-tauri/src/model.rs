//! アプリ内で受け渡しするデータの形

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Twitch のカテゴリ。box_art_url は `{width}x{height}` を含むテンプレートにそろえて持つ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Category {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub box_art_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,
    /// `{n}` を含むことがある、置き換え前のタイトル
    pub title: String,
    #[serde(default)]
    pub category: Option<Category>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 自分用のメモ。Twitch には送らない
    #[serde(default)]
    pub memo: String,
    /// 最後に使った配信回数
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// "system" / "light" / "dark"
    pub theme: String,
    pub announce_template: String,
    /// × でタスクトレイにしまう
    pub close_to_tray: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            announce_template: crate::announce::DEFAULT_TEMPLATE.into(),
            close_to_tray: true,
        }
    }
}

/// いま Twitch に反映されている配信情報
#[derive(Debug, Clone, Serialize)]
pub struct ChannelInfo {
    pub login: String,
    pub name: String,
    pub title: String,
    pub category: Option<Category>,
    pub tags: Vec<String>,
    pub fetched_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserView {
    pub id: String,
    pub login: String,
    pub display_name: String,
    #[serde(default)]
    pub profile_image_url: String,
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
