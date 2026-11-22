//! プリセットと設定を %APPDATA%\StreamStatus\data.json に保存する

use crate::model::{now_ms, Preset, Settings};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct AppData {
    pub version: u32,
    pub presets: Vec<Preset>,
    pub settings: Settings,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            version: 1,
            presets: Vec::new(),
            settings: Settings::default(),
        }
    }
}

pub struct Store {
    path: PathBuf,
    pub data: AppData,
}

pub fn data_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("StreamStatus")
}

impl Store {
    pub fn open(dir: &Path) -> Store {
        let path = dir.join("data.json");
        let data = match fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<AppData>(&text) {
                Ok(d) => d,
                Err(_) => {
                    // 壊れたファイルは消さずに残しておき、新しく始める
                    let broken = dir.join(format!("data.broken-{}.json", now_ms()));
                    let _ = fs::rename(&path, broken);
                    AppData::default()
                }
            },
            Err(_) => AppData::default(),
        };
        Store { path, data }
    }

    /// 途中で落ちても壊れないよう、一時ファイルに書いてから置き換える
    pub fn save(&self) -> Result<(), String> {
        let dir = self.path.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(dir).map_err(|e| format!("保存先を作れませんでした: {e}"))?;
        let json = serde_json::to_string_pretty(&self.data).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| format!("保存できませんでした: {e}"))?;
        fs::rename(&tmp, &self.path).map_err(|e| format!("保存できませんでした: {e}"))
    }

    pub fn find(&self, id: &str) -> Option<&Preset> {
        self.data.presets.iter().find(|p| p.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("streamstatus-test-{name}-{}", now_ms()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn 保存して読み戻せる() {
        let dir = temp_dir("roundtrip");
        let mut s = Store::open(&dir);
        assert!(s.data.presets.is_empty());
        s.data.presets.push(Preset {
            id: "a".into(),
            title: "t".into(),
            category: None,
            tags: vec![],
            memo: String::new(),
            count: 3,
            updated_at: 0,
        });
        s.data.settings.theme = "dark".into();
        s.save().unwrap();
        let s2 = Store::open(&dir);
        assert_eq!(s2.data.presets.len(), 1);
        assert_eq!(s2.data.settings.theme, "dark");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn 壊れたファイルは退避して空で始める() {
        let dir = temp_dir("broken");
        fs::write(dir.join("data.json"), "{ broken").unwrap();
        let s = Store::open(&dir);
        assert!(s.data.presets.is_empty());
        let moved = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .any(|e| e.file_name().to_string_lossy().starts_with("data.broken-"));
        assert!(moved);
        let _ = fs::remove_dir_all(dir);
    }
}
