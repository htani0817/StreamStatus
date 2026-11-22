//! プリセットの書き出しと読み込み（.tsp は1件、.tsb は全件。中身は同じ形の JSON）

use crate::model::{now_ms, Category, Preset};
use crate::text;
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "streamstatus-presets";
pub const VERSION: u32 = 1;
/// 読み込むファイルの上限（誤って大きなファイルを選んだとき用）
pub const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct BackupFile {
    format: String,
    version: u32,
    #[serde(default)]
    exported_at: u64,
    #[serde(default)]
    presets: Vec<BackupPreset>,
}

#[derive(Serialize, Deserialize)]
struct BackupPreset {
    title: String,
    #[serde(default)]
    category: Option<Category>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    memo: String,
    #[serde(default)]
    count: u32,
}

pub fn to_json(presets: &[Preset]) -> String {
    let file = BackupFile {
        format: FORMAT.into(),
        version: VERSION,
        exported_at: now_ms(),
        presets: presets
            .iter()
            .map(|p| BackupPreset {
                title: p.title.clone(),
                category: p.category.clone(),
                tags: p.tags.clone(),
                memo: p.memo.clone(),
                count: p.count,
            })
            .collect(),
    };
    serde_json::to_string_pretty(&file).unwrap_or_default()
}

pub struct Parsed {
    pub presets: Vec<Preset>,
    pub skipped: usize,
}

/// 読み込んだプリセットには新しい id を振る。壊れている項目は飛ばして数える
pub fn parse(json: &str) -> Result<Parsed, String> {
    let file: BackupFile = serde_json::from_str(json.trim_start_matches('\u{feff}'))
        .map_err(|_| "StreamStatus のプリセットファイルではありません".to_string())?;
    if file.format != FORMAT {
        return Err("StreamStatus のプリセットファイルではありません".into());
    }
    if file.version > VERSION {
        return Err("新しい版の StreamStatus で書き出されたファイルです。アプリを更新してから読み込んでください".into());
    }
    let mut presets = Vec::new();
    let mut skipped = 0;
    for p in file.presets {
        match sanitize(p) {
            Some(preset) => presets.push(preset),
            None => skipped += 1,
        }
    }
    Ok(Parsed { presets, skipped })
}

fn sanitize(p: BackupPreset) -> Option<Preset> {
    let title = p.title.trim().to_string();
    text::check_title(&title, p.count).ok()?;
    // 使えないタグだけ落とし、残りは生かす
    let mut tags: Vec<String> = Vec::new();
    for tag in p.tags.iter().filter_map(|t| text::normalize_tag(t).ok()) {
        if !tags.iter().any(|o| o.to_lowercase() == tag.to_lowercase()) {
            tags.push(tag);
        }
    }
    tags.truncate(text::TAGS_MAX);
    let memo: String = p.memo.chars().take(text::MEMO_MAX).collect();
    let category = p.category.filter(|c| !c.id.is_empty()).map(|c| Category {
        box_art_url: text::box_art_template(&c.box_art_url),
        ..c
    });
    Some(Preset {
        id: uuid::Uuid::new_v4().to_string(),
        title,
        category,
        tags,
        memo,
        count: p.count,
        updated_at: now_ms(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(title: &str) -> Preset {
        Preset {
            id: "old".into(),
            title: title.into(),
            category: Some(Category {
                id: "516575".into(),
                name: "VALORANT".into(),
                box_art_url: "https://static-cdn.jtvnw.net/ttv-boxart/516575-52x72.jpg".into(),
            }),
            tags: vec!["日本語".into(), "FPS".into()],
            memo: "メモ".into(),
            count: 41,
            updated_at: 0,
        }
    }

    #[test]
    fn 書き出して読み戻せる() {
        let json = to_json(&[preset("ランクマッチ #{n}"), preset("雑談")]);
        let parsed = parse(&json).unwrap();
        assert_eq!(parsed.skipped, 0);
        assert_eq!(parsed.presets.len(), 2);
        let p = &parsed.presets[0];
        assert_ne!(p.id, "old");
        assert_eq!(p.title, "ランクマッチ #{n}");
        assert_eq!(p.count, 41);
        assert_eq!(p.tags, vec!["日本語", "FPS"]);
        assert!(p.category.as_ref().unwrap().box_art_url.contains("{width}x{height}"));
    }

    #[test]
    fn 別の形式や新しい版は断る() {
        assert!(parse("{}").is_err());
        assert!(parse("not json").is_err());
        assert!(parse(r#"{"format":"other","version":1,"presets":[]}"#).is_err());
        assert!(parse(r#"{"format":"streamstatus-presets","version":99,"presets":[]}"#).is_err());
    }

    #[test]
    fn 足りない項目や知らない項目があっても読める() {
        let json = r#"{"format":"streamstatus-presets","version":1,"extra":true,
            "presets":[{"title":"だけ","future_field":1},{"title":""},{"title":"タグ","tags":["ok","bad tag"],"memo":""}]}"#;
        let parsed = parse(json).unwrap();
        assert_eq!(parsed.presets.len(), 2);
        assert_eq!(parsed.skipped, 1);
        assert_eq!(parsed.presets[1].tags, vec!["ok"]);
    }

    #[test]
    fn bom付きでも読める() {
        let json = format!("\u{feff}{}", to_json(&[preset("a")]));
        assert_eq!(parse(&json).unwrap().presets.len(), 1);
    }
}
