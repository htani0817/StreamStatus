//! ログイン情報（トークン）を Windows の資格情報マネージャーに保存する。
//! ファイルには書かない

use crate::twitch::Session;

const SERVICE: &str = "StreamStatus for Twitch";
const ACCOUNT: &str = "twitch-session";

fn entry() -> Option<keyring::Entry> {
    keyring::Entry::new(SERVICE, ACCOUNT).ok()
}

pub fn load() -> Option<Session> {
    let text = entry()?.get_password().ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save(session: &Session) -> Result<(), String> {
    let text = serde_json::to_string(session).map_err(|e| e.to_string())?;
    entry()
        .ok_or_else(|| "資格情報マネージャーを開けませんでした".to_string())?
        .set_password(&text)
        .map_err(|e| format!("ログイン情報を保存できませんでした: {e}"))
}

pub fn clear() {
    if let Some(e) = entry() {
        let _ = e.delete_credential();
    }
}
