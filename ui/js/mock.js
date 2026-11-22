// ブラウザで ui/index.html を開いたときだけ使う、見た目確認用の偽の Rust 側。
// URL に ?signed-out を付けるとログイン前、?empty でプリセットなしの画面になる

import { fillCounter, hasCounter, renderAnnounce } from './text.js';

const params = new URLSearchParams(location.search);
const art = (id) => `https://static-cdn.jtvnw.net/ttv-boxart/${id}-{width}x{height}.jpg`;
const CATS = {
  valorant: { id: '516575', name: 'VALORANT', box_art_url: art('516575') },
  chat: { id: '509658', name: 'Just Chatting', box_art_url: art('509658') },
  mc: { id: '27471', name: 'Minecraft', box_art_url: art('27471') },
  elden: { id: '512953', name: 'ELDEN RING', box_art_url: art('512953') },
  apex: { id: '511224', name: 'Apex Legends', box_art_url: art('511224') },
};

let seq = 10;
const presets = params.has('empty') ? [] : [
  { id: 'p1', title: '【VALORANT】ランクマッチ #{n}｜ダイヤ帯から抜けたい', category: CATS.valorant, tags: ['日本語', 'FPS', 'ランク'], memo: '21時から。始める前にクロスヘアを確認', count: 41, updated_at: 0 },
  { id: 'p2', title: '雑談しながらマシュマロを読む #{n}', category: CATS.chat, tags: ['日本語', '雑談', 'マシュマロ'], memo: '', count: 12, updated_at: 0 },
  { id: 'p3', title: '【Minecraft】村のまわりを整地する夜', category: CATS.mc, tags: ['日本語', 'マイクラ', '作業配信'], memo: 'BGM はなし。コメント読みは少なめ', count: 0, updated_at: 0 },
  { id: 'p4', title: '【ELDEN RING】DLC を最後まで #{n}', category: CATS.elden, tags: ['日本語', '初見', 'ネタバレ禁止'], memo: '', count: 7, updated_at: 0 },
  { id: 'p5', title: '【Apex】カジュアルで肩ならし', category: CATS.apex, tags: ['日本語', 'FPS'], memo: '', count: 0, updated_at: 0 },
];
let settings = { theme: params.get('theme') ?? 'system', announce_template: '配信を始めました\n{title}\n{url}', close_to_tray: true };
let user = params.has('signed-out') ? null : { id: '1', login: 'yoru_no_tsuki', display_name: '夜ノ月', profile_image_url: '' };
let channel = {
  login: 'yoru_no_tsuki',
  name: '夜ノ月',
  title: '【VALORANT】ランクマッチ #41｜ダイヤ帯から抜けたい',
  category: CATS.valorant,
  tags: ['日本語', 'FPS', 'ランク'],
  fetched_at: Date.now() - 4 * 60 * 1000,
};
const listeners = {};
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
const fail = (code, message) => { throw { code, message }; };
const clone = (x) => structuredClone(x);
const find = (id) => presets.find((p) => p.id === id) ?? fail('validation', 'プリセットが見つかりません');

function emit(event, payload) {
  for (const cb of listeners[event] ?? []) cb(payload);
}

export async function listen(event, cb) {
  (listeners[event] ??= []).push(cb);
  return () => {};
}

const handlers = {
  get_bootstrap: () => ({ presets, settings, user, version: '1.0.0', client_id_ready: true }),
  ui_ready: () => null,
  async login_start() {
    setTimeout(() => {
      user = { id: '1', login: 'yoru_no_tsuki', display_name: '夜ノ月', profile_image_url: '' };
      if (!params.has('login-wait')) emit('auth-changed', { state: 'signed_in', user });
    }, 4000);
    return { user_code: 'WDJBMJHT', verification_uri: 'https://www.twitch.tv/activate', expires_in: 1800 };
  },
  login_cancel: () => null,
  open_activation: () => null,
  logout() { user = null; emit('auth-changed', { state: 'signed_out' }); },
  async fetch_channel() {
    if (!user) fail('not_signed_in', 'Twitch にログインしてください');
    await wait(300);
    channel = { ...channel, fetched_at: Date.now() };
    return channel;
  },
  async search_categories({ query }) {
    await wait(200);
    const q = query.toLowerCase();
    return Object.values(CATS).filter((c) => c.name.toLowerCase().includes(q));
  },
  async apply_preset({ id, increment }) {
    if (!user) fail('not_signed_in', 'Twitch にログインしてください');
    const p = find(id);
    await wait(400);
    if (increment) p.count += 1;
    channel = { ...channel, title: fillCounter(p.title, p.count), category: p.category, tags: [...p.tags], fetched_at: Date.now() };
    return { preset: p, channel };
  },
  save_preset({ input }) {
    if (!input.title.trim()) fail('validation', 'タイトルを入れてください');
    let p = input.id && presets.find((x) => x.id === input.id);
    if (p) Object.assign(p, input, { id: p.id });
    else { p = { ...input, id: `p${++seq}` }; presets.push(p); }
    return p;
  },
  delete_preset({ id }) { presets.splice(presets.indexOf(find(id)), 1); },
  duplicate_preset({ id }) {
    const p = find(id);
    const copy = { ...clone(p), id: `p${++seq}` };
    presets.splice(presets.indexOf(p) + 1, 0, copy);
    return copy;
  },
  reorder_presets({ ids }) { presets.sort((a, b) => ids.indexOf(a.id) - ids.indexOf(b.id)); },
  save_settings({ settings: s }) {
    settings = s;
    return s;
  },
  announce_text({ id }) {
    const src = id ? find(id) : null;
    const title = src ? fillCounter(src.title, src.count) : channel.title;
    return renderAnnounce(settings.announce_template, {
      title,
      category: (src ? src.category : channel.category)?.name ?? '',
      tags: src ? src.tags : channel.tags,
      login: user?.login ?? '',
      name: user?.display_name ?? '',
      n: src && hasCounter(src.title) ? src.count : null,
    });
  },
  open_x_intent: () => null,
  export_presets: () => 'C:\\Users\\example\\Documents\\streamstatus-backup.tsb',
  import_presets: () => ({ added: 0, skipped: 0, presets }),
};

export async function invoke(cmd, args) {
  const fn = handlers[cmd];
  if (!fn) fail('mock', `mock に ${cmd} がありません`);
  return clone(await fn(args ?? {}));
}
