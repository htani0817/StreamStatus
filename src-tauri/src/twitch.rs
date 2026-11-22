//! Twitch との通信。ログイン（Device Code Flow）、トークン更新、Helix API。
//! トークンはこのモジュールの外（画面側）には出さない

use crate::model::{now_secs, Category, ChannelInfo, UserView};
use crate::{secrets, text};
use reqwest::{Method, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::sync::Mutex;

/// dev.twitch.tv で登録した Public クライアントの ID（秘密情報ではない）。
/// ビルド時に環境変数 STREAMSTATUS_CLIENT_ID を渡すと、そちらを使う
/// 登録名は「StreamStatus Desktop」（「Twitch」を含む名前は登録できない）
const BUILTIN_CLIENT_ID: &str = "y897buzjcx9v3p1k82re8zrfriw6jd";
const SCOPES: &str = "channel:manage:broadcast";
const ID_BASE: &str = "https://id.twitch.tv/oauth2";
const HELIX: &str = "https://api.twitch.tv/helix";

pub fn client_id() -> &'static str {
    match option_env!("STREAMSTATUS_CLIENT_ID") {
        Some(id) if !id.is_empty() => id,
        _ => BUILTIN_CLIENT_ID,
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
    pub user: UserView,
}

/// 画面に返すエラー。code で画面側の扱いを分ける
#[derive(Debug, Clone, Serialize)]
pub struct CmdError {
    pub code: &'static str,
    pub message: String,
}

impl CmdError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new("validation", message)
    }
    fn not_signed_in() -> Self {
        Self::new("not_signed_in", "Twitch にログインしてください")
    }
    fn expired() -> Self {
        Self::new("auth_expired", "ログインの期限が切れました。もう一度ログインしてください")
    }
    fn network() -> Self {
        Self::new("network", "Twitch に接続できませんでした。ネットワークを確かめてください")
    }
}

pub type Res<T> = Result<T, CmdError>;

#[derive(Deserialize)]
pub struct DeviceStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

#[derive(Deserialize)]
pub struct TokenResp {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

#[derive(Deserialize, Default)]
struct TwitchErr {
    #[serde(default)]
    message: String,
}

pub enum PollOutcome {
    Pending,
    SlowDown,
    Done(TokenResp),
    Failed(String),
}

pub struct Twitch {
    http: reqwest::Client,
    session: Mutex<Option<Session>>,
}

impl Twitch {
    pub fn new(session: Option<Session>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("StreamStatus/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self { http, session: Mutex::new(session) }
    }

    pub async fn user(&self) -> Option<UserView> {
        self.session.lock().await.as_ref().map(|s| s.user.clone())
    }

    // ---- ログイン ----

    pub async fn device_start(&self) -> Res<DeviceStart> {
        if client_id().is_empty() {
            return Err(CmdError::new(
                "no_client_id",
                "Client ID が入っていません。README の「Client ID の登録」の手順で入れてからビルドしてください",
            ));
        }
        let resp = self
            .http
            .post(format!("{ID_BASE}/device"))
            .form(&[("client_id", client_id()), ("scopes", SCOPES)])
            .send()
            .await
            .map_err(|_| CmdError::network())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.json::<TwitchErr>().await.unwrap_or_default();
            return Err(CmdError::new(
                "twitch",
                format!("ログインを始められませんでした（{}: {}）", status.as_u16(), err.message),
            ));
        }
        resp.json::<DeviceStart>()
            .await
            .map_err(|_| CmdError::new("twitch", "Twitch から想定外の応答が返りました"))
    }

    pub async fn device_poll(&self, device_code: &str) -> PollOutcome {
        let resp = self
            .http
            .post(format!("{ID_BASE}/token"))
            .form(&[
                ("client_id", client_id()),
                ("scopes", SCOPES),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await;
        // 通信が一時的に切れても、期限までは待ち続ける
        let Ok(resp) = resp else { return PollOutcome::Pending };
        if resp.status().is_success() {
            return match resp.json::<TokenResp>().await {
                Ok(t) => PollOutcome::Done(t),
                Err(_) => PollOutcome::Failed("Twitch から想定外の応答が返りました".into()),
            };
        }
        let err = resp.json::<TwitchErr>().await.unwrap_or_default();
        match err.message.as_str() {
            "authorization_pending" => PollOutcome::Pending,
            "slow_down" => PollOutcome::SlowDown,
            m if m.contains("denied") => PollOutcome::Failed("ログインが許可されませんでした".into()),
            m => PollOutcome::Failed(format!("ログインできませんでした（{m}）")),
        }
    }

    pub async fn complete_login(&self, tok: TokenResp) -> Res<UserView> {
        let resp = self
            .http
            .get(format!("{HELIX}/users"))
            .header("Client-Id", client_id())
            .bearer_auth(&tok.access_token)
            .send()
            .await
            .map_err(|_| CmdError::network())?;
        let body: Value = check(resp).await?.json().await.map_err(|_| unexpected())?;
        let u = &body["data"][0];
        let user = UserView {
            id: str_of(u, "id"),
            login: str_of(u, "login"),
            display_name: str_of(u, "display_name"),
            profile_image_url: str_of(u, "profile_image_url"),
        };
        if user.id.is_empty() {
            return Err(unexpected());
        }
        let session = Session {
            access_token: tok.access_token,
            refresh_token: tok.refresh_token,
            expires_at: now_secs() + tok.expires_in,
            user: user.clone(),
        };
        secrets::save(&session).map_err(|m| CmdError::new("io", m))?;
        *self.session.lock().await = Some(session);
        Ok(user)
    }

    pub async fn logout(&self) {
        let taken = self.session.lock().await.take();
        secrets::clear();
        if let Some(s) = taken {
            let _ = self
                .http
                .post(format!("{ID_BASE}/revoke"))
                .form(&[("client_id", client_id()), ("token", s.access_token.as_str())])
                .send()
                .await;
        }
    }

    // ---- トークン ----

    /// refresh token は使い捨てなので、ロックを持ったまま更新する
    async fn refresh_locked(&self, s: &mut Session) -> Res<()> {
        let resp = self
            .http
            .post(format!("{ID_BASE}/token"))
            .form(&[
                ("client_id", client_id()),
                ("grant_type", "refresh_token"),
                ("refresh_token", s.refresh_token.as_str()),
            ])
            .send()
            .await
            .map_err(|_| CmdError::network())?;
        let status = resp.status();
        if status == StatusCode::BAD_REQUEST || status == StatusCode::UNAUTHORIZED {
            return Err(CmdError::expired());
        }
        if !status.is_success() {
            return Err(CmdError::new("twitch", format!("ログインを更新できませんでした（{}）", status.as_u16())));
        }
        let tok: TokenResp = resp.json().await.map_err(|_| unexpected())?;
        s.access_token = tok.access_token;
        s.refresh_token = tok.refresh_token;
        s.expires_at = now_secs() + tok.expires_in;
        secrets::save(s).map_err(|m| CmdError::new("io", m))
    }

    /// 使えるアクセストークンと配信者 ID を返す。
    /// `stale` に断られたトークンを渡すと、それがまだ使われていれば更新する
    async fn token(&self, stale: Option<&str>) -> Res<(String, String)> {
        let mut guard = self.session.lock().await;
        let s = guard.as_mut().ok_or_else(CmdError::not_signed_in)?;
        let must = stale.is_some_and(|t| t == s.access_token) || s.expires_at <= now_secs() + 120;
        if must {
            if let Err(e) = self.refresh_locked(s).await {
                if e.code == "auth_expired" {
                    *guard = None;
                    secrets::clear();
                }
                return Err(e);
            }
        }
        Ok((s.access_token.clone(), s.user.id.clone()))
    }

    /// 起動時と1時間ごとに呼ぶ（Twitch の決まり）
    pub async fn validate(&self) -> Res<()> {
        let (tok, _) = self.token(None).await?;
        let resp = self
            .http
            .get(format!("{ID_BASE}/validate"))
            .header("Authorization", format!("OAuth {tok}"))
            .send()
            .await
            .map_err(|_| CmdError::network())?;
        if resp.status() == StatusCode::UNAUTHORIZED {
            self.token(Some(&tok)).await?;
        }
        Ok(())
    }

    // ---- Helix ----

    async fn helix(&self, method: Method, path: &str, query: &[(&str, String)], body: Option<&Value>) -> Res<Response> {
        let (tok, _) = self.token(None).await?;
        let resp = self.send(method.clone(), path, query, body, &tok).await?;
        if resp.status() == StatusCode::UNAUTHORIZED {
            let (tok2, _) = self.token(Some(&tok)).await?;
            let retry = self.send(method, path, query, body, &tok2).await?;
            return check(retry).await;
        }
        check(resp).await
    }

    async fn send(&self, method: Method, path: &str, query: &[(&str, String)], body: Option<&Value>, tok: &str) -> Res<Response> {
        let mut req = self
            .http
            .request(method, format!("{HELIX}{path}"))
            .query(query)
            .header("Client-Id", client_id())
            .bearer_auth(tok);
        if let Some(b) = body {
            req = req.json(b);
        }
        req.send().await.map_err(|_| CmdError::network())
    }

    async fn broadcaster_id(&self) -> Res<String> {
        Ok(self.token(None).await?.1)
    }

    pub async fn channel(&self) -> Res<ChannelInfo> {
        let id = self.broadcaster_id().await?;
        let resp = self.helix(Method::GET, "/channels", &[("broadcaster_id", id)], None).await?;
        let body: Value = resp.json().await.map_err(|_| unexpected())?;
        let c = &body["data"][0];
        let game_id = str_of(c, "game_id");
        let category = if game_id.is_empty() {
            None
        } else {
            Some(Category {
                box_art_url: self.box_art(&game_id).await.unwrap_or_default(),
                id: game_id,
                name: str_of(c, "game_name"),
            })
        };
        let tags = c["tags"]
            .as_array()
            .map(|a| a.iter().filter_map(|t| t.as_str().map(String::from)).collect())
            .unwrap_or_default();
        Ok(ChannelInfo {
            login: str_of(c, "broadcaster_login"),
            name: str_of(c, "broadcaster_name"),
            title: str_of(c, "title"),
            category,
            tags,
            fetched_at: crate::model::now_ms(),
        })
    }

    async fn box_art(&self, game_id: &str) -> Res<String> {
        let resp = self.helix(Method::GET, "/games", &[("id", game_id.to_string())], None).await?;
        let body: Value = resp.json().await.map_err(|_| unexpected())?;
        Ok(text::box_art_template(&str_of(&body["data"][0], "box_art_url")))
    }

    pub async fn search_categories(&self, query: &str) -> Res<Vec<Category>> {
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let resp = self
            .helix(Method::GET, "/search/categories", &[("query", q.to_string()), ("first", "10".into())], None)
            .await?;
        let body: Value = resp.json().await.map_err(|_| unexpected())?;
        Ok(body["data"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|c| Category {
                        id: str_of(c, "id"),
                        name: str_of(c, "name"),
                        box_art_url: text::box_art_template(&str_of(c, "box_art_url")),
                    })
                    .filter(|c| !c.id.is_empty())
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn modify(&self, title: &str, category: Option<&Category>, tags: &[String]) -> Res<()> {
        let id = self.broadcaster_id().await?;
        let body = json!({
            "title": title,
            // 空文字でカテゴリを外す
            "game_id": category.map(|c| c.id.as_str()).unwrap_or(""),
            "tags": tags,
        });
        self.helix(Method::PATCH, "/channels", &[("broadcaster_id", id)], Some(&body)).await?;
        Ok(())
    }
}

fn str_of(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_default().to_string()
}

fn unexpected() -> CmdError {
    CmdError::new("twitch", "Twitch から想定外の応答が返りました")
}

/// 失敗した応答を、画面に出せる言葉にする
async fn check(resp: Response) -> Res<Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let msg = resp.json::<TwitchErr>().await.unwrap_or_default().message;
    Err(match status.as_u16() {
        401 => CmdError::expired(),
        403 => CmdError::new("forbidden", "このチャンネルを変更する権限がありません"),
        429 => CmdError::new("rate_limited", "短い間に送りすぎました。少し待ってからもう一度試してください"),
        400 => CmdError::new("twitch", format!("Twitch に受け付けられませんでした（{msg}）")),
        s if s >= 500 => CmdError::new("twitch", format!("Twitch 側でエラーが起きました（{s}）")),
        s => CmdError::new("twitch", format!("Twitch でエラーが起きました（{s}: {msg}）")),
    })
}
