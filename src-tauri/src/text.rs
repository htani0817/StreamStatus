//! タイトル・タグ・メモの検証と `{n}` の置き換え

/// 配信回数に置き換える印。全角で書いても同じ扱いにする
const COUNTER_TOKENS: [&str; 4] = ["{n}", "｛ｎ｝", "{ｎ}", "｛n｝"];

pub const TITLE_MAX: usize = 140;
pub const TAG_MAX_CHARS: usize = 25;
pub const TAGS_MAX: usize = 10;
pub const MEMO_MAX: usize = 500;

pub fn has_counter(title: &str) -> bool {
    COUNTER_TOKENS.iter().any(|t| title.contains(t))
}

pub fn fill_counter(title: &str, n: u32) -> String {
    let n = n.to_string();
    COUNTER_TOKENS
        .iter()
        .fold(title.to_string(), |acc, t| acc.replace(t, &n))
}

/// 置き換え後のタイトルを検証して返す
pub fn check_title(raw: &str, n: u32) -> Result<String, String> {
    let filled = fill_counter(raw.trim(), n);
    if filled.is_empty() {
        return Err("タイトルを入れてください".into());
    }
    let len = filled.chars().count();
    if len > TITLE_MAX {
        return Err(format!(
            "タイトルは{TITLE_MAX}文字までです（いまは{len}文字）"
        ));
    }
    Ok(filled)
}

/// タグ1つを整える。先頭の # は外す
pub fn normalize_tag(raw: &str) -> Result<String, String> {
    let tag = raw.trim().trim_start_matches(['#', '＃']).trim();
    if tag.is_empty() {
        return Err("空のタグは追加できません".into());
    }
    if tag.chars().count() > TAG_MAX_CHARS {
        return Err(format!("タグは{TAG_MAX_CHARS}文字までです: {tag}"));
    }
    // Twitch は「・」などの全角記号を受け付けるので、断るのは空白と半角記号だけにする。
    // それ以外の細かい判定は Twitch 側に任せる
    if tag.chars().any(|c| c.is_whitespace() || c.is_ascii_punctuation()) {
        return Err(format!("タグに空白や半角記号は使えません: {tag}"));
    }
    Ok(tag.to_string())
}

/// タグ一覧を整える。大文字・小文字違いの重複は1つにまとめる
pub fn normalize_tags(raw: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for t in raw {
        let tag = normalize_tag(t)?;
        if !out.iter().any(|o| o.to_lowercase() == tag.to_lowercase()) {
            out.push(tag);
        }
    }
    if out.len() > TAGS_MAX {
        return Err(format!("タグは{TAGS_MAX}個までです"));
    }
    Ok(out)
}

pub fn check_memo(memo: &str) -> Result<String, String> {
    let len = memo.chars().count();
    if len > MEMO_MAX {
        return Err(format!("メモは{MEMO_MAX}文字までです（いまは{len}文字）"));
    }
    Ok(memo.to_string())
}

/// 箱絵の URL を `{width}x{height}` のテンプレートにそろえる。
/// 検索 API は `-52x72.jpg` のようにサイズ入り、ゲーム API はテンプレートで返すため
pub fn box_art_template(url: &str) -> String {
    let Some(dot) = url.rfind('.') else {
        return url.to_string();
    };
    let (stem, ext) = url.split_at(dot);
    let Some(dash) = stem.rfind('-') else {
        return url.to_string();
    };
    let size = &stem[dash + 1..];
    let is_size = size
        .split_once('x')
        .map(|(w, h)| {
            !w.is_empty()
                && !h.is_empty()
                && w.chars().all(|c| c.is_ascii_digit())
                && h.chars().all(|c| c.is_ascii_digit())
        })
        .unwrap_or(false);
    if is_size {
        format!("{}-{{width}}x{{height}}{}", &stem[..dash], ext)
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 配信回数を置き換える() {
        assert_eq!(fill_counter("ランクマッチ #{n}", 3), "ランクマッチ #3");
        assert_eq!(fill_counter("全角 #｛ｎ｝ と #{ｎ} と #｛n｝", 12), "全角 #12 と #12 と #12");
        assert_eq!(fill_counter("回数なし", 5), "回数なし");
        assert!(has_counter("a {n}"));
        assert!(has_counter("a ｛ｎ｝"));
        assert!(!has_counter("a {m}"));
    }

    #[test]
    fn タイトルの長さは置き換え後で数える() {
        let raw = format!("{}{{n}}", "あ".repeat(139));
        assert!(check_title(&raw, 1).is_ok());
        assert!(check_title(&raw, 10).is_err());
        assert!(check_title("   ", 1).is_err());
    }

    #[test]
    fn タグを検証する() {
        assert_eq!(normalize_tag("#日本語").unwrap(), "日本語");
        assert_eq!(normalize_tag("＃FPS").unwrap(), "FPS");
        assert!(normalize_tag("two words").is_err());
        assert!(normalize_tag("a-b").is_err());
        assert!(normalize_tag("全角　空白").is_err());
        // 実際の Twitch のタグにあったもの
        assert_eq!(normalize_tag("フォロー・コメント大歓迎").unwrap(), "フォロー・コメント大歓迎");
        assert!(normalize_tag("VALORANＴ").is_ok());
        assert!(normalize_tag(&"a".repeat(26)).is_err());
        assert!(normalize_tag(&"a".repeat(25)).is_ok());
        let tags: Vec<String> = ["FPS", "fps", "雑談"].iter().map(|s| s.to_string()).collect();
        assert_eq!(normalize_tags(&tags).unwrap(), vec!["FPS", "雑談"]);
        let many: Vec<String> = (0..11).map(|i| format!("tag{i}")).collect();
        assert!(normalize_tags(&many).is_err());
    }

    #[test]
    fn 箱絵のurlをテンプレートにする() {
        assert_eq!(
            box_art_template("https://static-cdn.jtvnw.net/ttv-boxart/33214-52x72.jpg"),
            "https://static-cdn.jtvnw.net/ttv-boxart/33214-{width}x{height}.jpg"
        );
        let t = "https://static-cdn.jtvnw.net/ttv-boxart/33214-{width}x{height}.jpg";
        assert_eq!(box_art_template(t), t);
        assert_eq!(box_art_template(""), "");
    }
}
