//! 画面（JS）から呼ぶコマンド

use crate::model::{now_ms, Category, ChannelInfo, Preset, Settings, UserView};
use crate::store::Store;
use crate::twitch::{CmdError, PollOutcome, Res, Twitch};
use crate::{announce, backup, text};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

pub struct LoginTask {
    handle: JoinHandle<()>,
    verification_uri: String,
}

pub struct AppState {
    pub store: Mutex<Store>,
    pub twitch: Twitch,
    pub login: Mutex<Option<LoginTask>>,
    pub last_channel: Mutex<Option<ChannelInfo>>,
}

#[derive(Serialize, Clone)]
pub struct AuthEvent {
    pub state: &'static str,
    pub user: Option<UserView>,
    pub message: Option<String>,
}

pub fn emit_auth(app: &AppHandle, state: &'static str, user: Option<UserView>, message: Option<String>) {
    let _ = app.emit("auth-changed", AuthEvent { state, user, message });
}

/// ログイン切れのエラーなら、画面にログアウト状態を知らせる
fn track<T>(app: &AppHandle, r: Res<T>) -> Res<T> {
    if let Err(e) = &r {
        if e.code == "auth_expired" || e.code == "not_signed_in" {
            emit_auth(app, "signed_out", None, Some(e.message.clone()));
        }
    }
    r
}

fn io_err(m: String) -> CmdError {
    CmdError::new("io", m)
}

pub fn apply_theme(window: &WebviewWindow, theme: &str) {
    let t = match theme {
        "light" => Some(tauri::Theme::Light),
        "dark" => Some(tauri::Theme::Dark),
        _ => None,
    };
    let _ = window.set_theme(t);
}

// ---- 起動 ----

#[derive(Serialize)]
pub struct Bootstrap {
    presets: Vec<Preset>,
    settings: Settings,
    user: Option<UserView>,
    version: &'static str,
    client_id_ready: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_bootstrap(state: State<'_, AppState>) -> Res<Bootstrap> {
    let (presets, settings) = {
        let s = state.store.lock().unwrap();
        (s.data.presets.clone(), s.data.settings.clone())
    };
    Ok(Bootstrap {
        presets,
        settings,
        user: state.twitch.user().await,
        version: env!("CARGO_PKG_VERSION"),
        client_id_ready: !crate::twitch::client_id().is_empty(),
    })
}

/// 画面の準備ができたら表示する（白い画面がちらつかないように）
#[tauri::command(rename_all = "snake_case")]
pub fn ui_ready(window: WebviewWindow) {
    let _ = window.show();
    let _ = window.set_focus();
}

// ---- ログイン ----

#[derive(Serialize)]
pub struct LoginView {
    user_code: String,
    verification_uri: String,
    expires_in: u64,
}

fn is_twitch_url(url: &str) -> bool {
    url.starts_with("https://www.twitch.tv/") || url.starts_with("https://twitch.tv/")
}

#[tauri::command(rename_all = "snake_case")]
pub async fn login_start(app: AppHandle, state: State<'_, AppState>) -> Res<LoginView> {
    cancel_login(&state);
    let d = state.twitch.device_start().await?;
    let view = LoginView {
        user_code: d.user_code.clone(),
        verification_uri: d.verification_uri.clone(),
        expires_in: d.expires_in,
    };
    if is_twitch_url(&d.verification_uri) {
        let _ = app.opener().open_url(&d.verification_uri, None::<&str>);
    }
    let uri = d.verification_uri.clone();
    let handle = tauri::async_runtime::spawn(poll_login(app.clone(), d));
    *state.login.lock().unwrap() = Some(LoginTask { handle, verification_uri: uri });
    Ok(view)
}

async fn poll_login(app: AppHandle, d: crate::twitch::DeviceStart) {
    let deadline = crate::model::now_secs() + d.expires_in;
    let mut interval = d.interval.max(1);
    loop {
        tokio::time::sleep(Duration::from_secs(interval)).await;
        if crate::model::now_secs() > deadline {
            emit_auth(&app, "login_expired", None, Some("コードの期限が切れました。もう一度ログインしてください".into()));
            return;
        }
        let state = app.state::<AppState>();
        match state.twitch.device_poll(&d.device_code).await {
            PollOutcome::Pending => continue,
            PollOutcome::SlowDown => interval += 5,
            PollOutcome::Done(tok) => {
                match state.twitch.complete_login(tok).await {
                    Ok(user) => emit_auth(&app, "signed_in", Some(user), None),
                    Err(e) => emit_auth(&app, "login_failed", None, Some(e.message)),
                }
                return;
            }
            PollOutcome::Failed(m) => {
                emit_auth(&app, "login_failed", None, Some(m));
                return;
            }
        }
    }
}

fn cancel_login(state: &AppState) {
    if let Some(t) = state.login.lock().unwrap().take() {
        t.handle.abort();
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn login_cancel(state: State<'_, AppState>) {
    cancel_login(&state);
}

#[tauri::command(rename_all = "snake_case")]
pub fn open_activation(app: AppHandle, state: State<'_, AppState>) {
    let uri = state.login.lock().unwrap().as_ref().map(|t| t.verification_uri.clone());
    if let Some(uri) = uri.filter(|u| is_twitch_url(u)) {
        let _ = app.opener().open_url(&uri, None::<&str>);
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn logout(app: AppHandle, state: State<'_, AppState>) -> Res<()> {
    cancel_login(&state);
    state.twitch.logout().await;
    *state.last_channel.lock().unwrap() = None;
    emit_auth(&app, "signed_out", None, None);
    Ok(())
}

// ---- 配信情報 ----

#[tauri::command(rename_all = "snake_case")]
pub async fn fetch_channel(app: AppHandle, state: State<'_, AppState>) -> Res<ChannelInfo> {
    let ch = track(&app, state.twitch.channel().await)?;
    *state.last_channel.lock().unwrap() = Some(ch.clone());
    Ok(ch)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn search_categories(app: AppHandle, state: State<'_, AppState>, query: String) -> Res<Vec<Category>> {
    track(&app, state.twitch.search_categories(&query).await)
}

#[derive(Serialize)]
pub struct ApplyResult {
    preset: Preset,
    channel: ChannelInfo,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn apply_preset(app: AppHandle, state: State<'_, AppState>, id: String, increment: bool) -> Res<ApplyResult> {
    let preset = state
        .store
        .lock()
        .unwrap()
        .find(&id)
        .cloned()
        .ok_or_else(|| CmdError::validation("プリセットが見つかりません"))?;
    if increment && !text::has_counter(&preset.title) {
        return Err(CmdError::validation("このプリセットのタイトルには {n} がありません"));
    }
    let n = if increment { preset.count.saturating_add(1) } else { preset.count };
    let title = text::check_title(&preset.title, n).map_err(CmdError::validation)?;
    track(&app, state.twitch.modify(&title, preset.category.as_ref(), &preset.tags).await)?;

    // 反映できたときだけ回数を進める
    let preset = if increment {
        let mut s = state.store.lock().unwrap();
        let updated = s.data.presets.iter_mut().find(|p| p.id == id).map(|p| {
            p.count = n;
            p.updated_at = now_ms();
            p.clone()
        });
        s.save().map_err(io_err)?;
        updated.unwrap_or(preset)
    } else {
        preset
    };

    // 反映後の状態を読み直す。読めなければ送った内容で代用する
    let channel = match state.twitch.channel().await {
        Ok(ch) => ch,
        Err(_) => {
            let user = state.twitch.user().await;
            ChannelInfo {
                login: user.as_ref().map(|u| u.login.clone()).unwrap_or_default(),
                name: user.map(|u| u.display_name).unwrap_or_default(),
                title,
                category: preset.category.clone(),
                tags: preset.tags.clone(),
                fetched_at: now_ms(),
            }
        }
    };
    *state.last_channel.lock().unwrap() = Some(channel.clone());
    Ok(ApplyResult { preset, channel })
}

// ---- プリセット ----

#[derive(Deserialize)]
pub struct PresetInput {
    id: Option<String>,
    title: String,
    category: Option<Category>,
    tags: Vec<String>,
    memo: String,
    count: u32,
}

#[tauri::command(rename_all = "snake_case")]
pub fn save_preset(state: State<'_, AppState>, input: PresetInput) -> Res<Preset> {
    let title = input.title.trim().to_string();
    text::check_title(&title, input.count).map_err(CmdError::validation)?;
    let tags = text::normalize_tags(&input.tags).map_err(CmdError::validation)?;
    let memo = text::check_memo(&input.memo).map_err(CmdError::validation)?;
    let category = input.category.filter(|c| !c.id.is_empty()).map(|c| Category {
        box_art_url: text::box_art_template(&c.box_art_url),
        ..c
    });

    let mut s = state.store.lock().unwrap();
    let existing = input.id.as_ref().and_then(|id| s.data.presets.iter_mut().find(|p| &p.id == id));
    let saved = match existing {
        Some(p) => {
            p.title = title;
            p.category = category;
            p.tags = tags;
            p.memo = memo;
            p.count = input.count;
            p.updated_at = now_ms();
            p.clone()
        }
        None => {
            let p = Preset {
                id: uuid::Uuid::new_v4().to_string(),
                title,
                category,
                tags,
                memo,
                count: input.count,
                updated_at: now_ms(),
            };
            s.data.presets.push(p.clone());
            p
        }
    };
    s.save().map_err(io_err)?;
    Ok(saved)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_preset(state: State<'_, AppState>, id: String) -> Res<()> {
    let mut s = state.store.lock().unwrap();
    s.data.presets.retain(|p| p.id != id);
    s.save().map_err(io_err)
}

#[tauri::command(rename_all = "snake_case")]
pub fn duplicate_preset(state: State<'_, AppState>, id: String) -> Res<Preset> {
    let mut s = state.store.lock().unwrap();
    let idx = s
        .data
        .presets
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| CmdError::validation("プリセットが見つかりません"))?;
    let mut copy = s.data.presets[idx].clone();
    copy.id = uuid::Uuid::new_v4().to_string();
    copy.updated_at = now_ms();
    s.data.presets.insert(idx + 1, copy.clone());
    s.save().map_err(io_err)?;
    Ok(copy)
}

/// 渡された順に並べ直す。渡されなかったものは末尾に残す
#[tauri::command(rename_all = "snake_case")]
pub fn reorder_presets(state: State<'_, AppState>, ids: Vec<String>) -> Res<()> {
    let mut s = state.store.lock().unwrap();
    let mut rest = std::mem::take(&mut s.data.presets);
    let mut ordered = Vec::with_capacity(rest.len());
    for id in &ids {
        if let Some(pos) = rest.iter().position(|p| &p.id == id) {
            ordered.push(rest.remove(pos));
        }
    }
    ordered.extend(rest);
    s.data.presets = ordered;
    s.save().map_err(io_err)
}

// ---- 設定 ----

#[tauri::command(rename_all = "snake_case")]
pub fn save_settings(window: WebviewWindow, state: State<'_, AppState>, settings: Settings) -> Res<Settings> {
    if !["system", "light", "dark"].contains(&settings.theme.as_str()) {
        return Err(CmdError::validation("表示モードの値が正しくありません"));
    }
    if settings.announce_template.chars().count() > 1000 {
        return Err(CmdError::validation("告知文のひな形は1000文字までです"));
    }
    let mut s = state.store.lock().unwrap();
    s.data.settings = settings.clone();
    s.save().map_err(io_err)?;
    apply_theme(&window, &settings.theme);
    Ok(settings)
}

// ---- 告知文 ----

/// id が無ければ「いまの配信」から、あればそのプリセットから告知文を作る
#[tauri::command(rename_all = "snake_case")]
pub async fn announce_text(state: State<'_, AppState>, id: Option<String>) -> Res<String> {
    let user = state.twitch.user().await;
    let template = state.store.lock().unwrap().data.settings.announce_template.clone();
    let login = user.as_ref().map(|u| u.login.clone()).unwrap_or_default();
    let name = user.as_ref().map(|u| u.display_name.clone()).unwrap_or_default();

    let (title, category, tags, n) = match id {
        Some(id) => {
            let p = state
                .store
                .lock()
                .unwrap()
                .find(&id)
                .cloned()
                .ok_or_else(|| CmdError::validation("プリセットが見つかりません"))?;
            let n = text::has_counter(&p.title).then_some(p.count);
            (text::fill_counter(&p.title, p.count), p.category.map(|c| c.name).unwrap_or_default(), p.tags, n)
        }
        None => {
            let ch = state
                .last_channel
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| CmdError::validation("先にいまの配信情報を読み込んでください"))?;
            (ch.title, ch.category.map(|c| c.name).unwrap_or_default(), ch.tags, None)
        }
    };
    Ok(announce::render(
        &template,
        &announce::Vars { title: &title, category: &category, tags: &tags, login: &login, name: &name, n },
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub fn open_x_intent(app: AppHandle, text: String) -> Res<()> {
    app.opener()
        .open_url(announce::x_intent_url(&text), None::<&str>)
        .map_err(|_| CmdError::new("io", "ブラウザを開けませんでした"))
}

// ---- バックアップ ----

#[tauri::command(rename_all = "snake_case")]
pub async fn export_presets(app: AppHandle, state: State<'_, AppState>, ids: Option<Vec<String>>, file_name: String) -> Res<Option<String>> {
    let presets: Vec<Preset> = {
        let s = state.store.lock().unwrap();
        match &ids {
            Some(ids) => s.data.presets.iter().filter(|p| ids.contains(&p.id)).cloned().collect(),
            None => s.data.presets.clone(),
        }
    };
    if presets.is_empty() {
        return Err(CmdError::validation("書き出すプリセットがありません"));
    }
    let single = ids.as_ref().is_some_and(|i| i.len() == 1);
    let (label, ext) = if single { ("StreamStatus プリセット", "tsp") } else { ("StreamStatus バックアップ", "tsb") };

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("プリセットを書き出す")
        .set_file_name(file_name)
        .add_filter(label, &[ext])
        .save_file(move |p| {
            let _ = tx.send(p);
        });
    let Some(path) = rx.await.ok().flatten() else { return Ok(None) };
    let path = path.into_path().map_err(|_| io_err("保存先を使えませんでした".into()))?;
    std::fs::write(&path, backup::to_json(&presets)).map_err(|e| io_err(format!("書き出せませんでした: {e}")))?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

#[derive(Serialize)]
pub struct ImportResult {
    added: usize,
    skipped: usize,
    presets: Vec<Preset>,
}

/// mode は "append"（後ろに足す）か "replace"（全部入れ替える）
#[tauri::command(rename_all = "snake_case")]
pub async fn import_presets(app: AppHandle, state: State<'_, AppState>, mode: String) -> Res<Option<ImportResult>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("プリセットを読み込む")
        .add_filter("StreamStatus プリセット", &["tsp", "tsb"])
        .pick_file(move |p| {
            let _ = tx.send(p);
        });
    let Some(path) = rx.await.ok().flatten() else { return Ok(None) };
    let path = path.into_path().map_err(|_| io_err("ファイルを開けませんでした".into()))?;
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if size > backup::MAX_FILE_BYTES {
        return Err(CmdError::validation("ファイルが大きすぎます"));
    }
    let json = std::fs::read_to_string(&path).map_err(|e| io_err(format!("読み込めませんでした: {e}")))?;
    let parsed = backup::parse(&json).map_err(CmdError::validation)?;
    let added = parsed.presets.len();

    let mut s = state.store.lock().unwrap();
    if mode == "replace" {
        s.data.presets = parsed.presets;
    } else {
        s.data.presets.extend(parsed.presets);
    }
    s.save().map_err(io_err)?;
    Ok(Some(ImportResult { added, skipped: parsed.skipped, presets: s.data.presets.clone() }))
}
