//! 告知文の組み立て

pub const DEFAULT_TEMPLATE: &str = "配信を始めました\n{title}\n{url}";

pub struct Vars<'a> {
    pub title: &'a str,
    pub category: &'a str,
    pub tags: &'a [String],
    pub login: &'a str,
    pub name: &'a str,
    pub n: Option<u32>,
}

/// `{title}` などを1回だけ走査して置き換える。
/// 差し込んだ文字列の中に `{url}` などがあっても二重に置き換えない
pub fn render(template: &str, v: &Vars) -> String {
    let url = if v.login.is_empty() {
        String::new()
    } else {
        format!("https://www.twitch.tv/{}", v.login)
    };
    let tags = v
        .tags
        .iter()
        .map(|t| format!("#{t}"))
        .collect::<Vec<_>>()
        .join(" ");
    let n = v.n.map(|n| n.to_string()).unwrap_or_default();

    let mut out = String::with_capacity(template.len() + 64);
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let Some(end) = after.find('}') else {
            out.push_str(after);
            rest = "";
            break;
        };
        let key = &after[1..end];
        let value = match key {
            "title" => Some(v.title),
            "category" => Some(v.category),
            "tags" => Some(tags.as_str()),
            "url" => Some(url.as_str()),
            "login" => Some(v.login),
            "name" => Some(v.name),
            "n" => Some(n.as_str()),
            _ => None,
        };
        match value {
            Some(val) => out.push_str(val),
            None => out.push_str(&after[..=end]),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

pub fn x_intent_url(text: &str) -> String {
    format!(
        "https://x.com/intent/post?text={}",
        urlencoding::encode(text)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars<'a>(tags: &'a [String]) -> Vars<'a> {
        Vars {
            title: "ランクマッチ #12 {url}",
            category: "VALORANT",
            tags,
            login: "example_streamer",
            name: "配信者",
            n: Some(12),
        }
    }

    #[test]
    fn 差し込みを置き換える() {
        let tags = vec!["日本語".to_string(), "FPS".to_string()];
        let s = render("{name}: {title}\n{category} {tags}\n{url} 第{n}回 {unknown}", &vars(&tags));
        assert_eq!(
            s,
            "配信者: ランクマッチ #12 {url}\nVALORANT #日本語 #FPS\nhttps://www.twitch.tv/example_streamer 第12回 {unknown}"
        );
    }

    #[test]
    fn 閉じ括弧がなくても壊れない() {
        let tags = vec![];
        assert_eq!(render("abc {title", &vars(&tags)), "abc {title");
    }

    #[test]
    fn xの投稿画面urlを作る() {
        assert_eq!(
            x_intent_url("配信 #1"),
            "https://x.com/intent/post?text=%E9%85%8D%E4%BF%A1%20%231"
        );
    }
}
