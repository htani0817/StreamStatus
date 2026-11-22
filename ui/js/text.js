// 画面側で使う文字の処理。正しさの判定は Rust 側（text.rs）が持ち、ここは表示と入力補助だけ

export const TITLE_MAX = 140;
export const TAG_MAX_CHARS = 25;
export const TAGS_MAX = 10;
export const MEMO_MAX = 500;

const COUNTER_RE = /\{n\}|｛ｎ｝|\{ｎ\}|｛n｝/g;

export const hasCounter = (title) => new RegExp(COUNTER_RE.source).test(title ?? '');

export const fillCounter = (title, n) => (title ?? '').replace(COUNTER_RE, String(n));

/** タイトルを「文字列」と「回数」の並びに分ける（回数だけ見た目を変えるため） */
export function splitCounter(title, n) {
  const parts = [];
  let last = 0;
  for (const m of (title ?? '').matchAll(COUNTER_RE)) {
    if (m.index > last) parts.push({ text: title.slice(last, m.index) });
    parts.push({ count: n });
    last = m.index + m[0].length;
  }
  if (last < (title ?? '').length) parts.push({ text: title.slice(last) });
  return parts;
}

export const charCount = (s) => [...(s ?? '')].length;

/** タグ1つの入力チェック。問題なければ null */
export function tagProblem(tag) {
  if (!tag) return '空のタグは追加できません';
  if (charCount(tag) > TAG_MAX_CHARS) return `タグは${TAG_MAX_CHARS}文字までです`;
  // 全角の「・」などは Twitch が受け付けるので、断るのは空白と半角記号だけ
  if (/[\s!-\/:-@\[-`{-~]/.test(tag)) return 'タグに空白や半角記号は使えません';
  return null;
}

export const cleanTag = (raw) => raw.trim().replace(/^[#＃]+/, '').trim();

export function boxArt(url, w, h) {
  return url ? url.replace('{width}', String(w)).replace('{height}', String(h)) : '';
}

/** プリセットが、いま Twitch に出ている内容と同じか */
export function matchesChannel(preset, ch) {
  if (!ch) return false;
  if (fillCounter(preset.title.trim(), preset.count) !== ch.title) return false;
  if ((preset.category?.id ?? '') !== (ch.category?.id ?? '')) return false;
  const a = preset.tags.map((t) => t.toLowerCase()).sort();
  const b = ch.tags.map((t) => t.toLowerCase()).sort();
  return a.length === b.length && a.every((t, i) => t === b[i]);
}

/** 設定画面の見本用。実際の告知文は Rust 側の announce.rs で作る */
export function renderAnnounce(template, v) {
  const url = v.login ? `https://www.twitch.tv/${v.login}` : '';
  const map = {
    title: v.title,
    category: v.category,
    tags: v.tags.map((t) => `#${t}`).join(' '),
    url,
    login: v.login,
    name: v.name,
    n: v.n == null ? '' : String(v.n),
  };
  return template.replace(/\{([a-z]+)\}/g, (all, key) => (key in map ? map[key] : all));
}

export function timeLabel(ms) {
  return new Date(ms).toLocaleTimeString('ja-JP', { hour: '2-digit', minute: '2-digit' });
}

/** 書き出すファイル名に使えない文字を外す */
export function safeFileName(s) {
  const name = (s ?? '').replace(/[\\/:*?"<>|\u0000-\u001f]/g, '').replace(/\s+/g, ' ').trim();
  return [...name].slice(0, 40).join('') || 'preset';
}

export function today() {
  const d = new Date();
  const p = (x) => String(x).padStart(2, '0');
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}`;
}
