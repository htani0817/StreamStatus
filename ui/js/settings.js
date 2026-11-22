// 設定ダイアログ。変えたらその場で保存する

import { exportPresets, importPresets, logout, saveSettings } from './actions.js';
import { $, h, icon, img } from './dom.js';
import { openLogin } from './login.js';
import { state, subscribe } from './state.js';
import { fillCounter, hasCounter, renderAnnounce } from './text.js';
import { confirmDialog } from './ui.js';

const TOKENS = [
  ['title', 'タイトル'],
  ['category', 'カテゴリ'],
  ['tags', 'タグ'],
  ['url', 'チャンネル URL'],
  ['name', '表示名'],
  ['n', '配信回数'],
];

let saveTimer = 0;

export function openSettings() {
  renderAccount();
  for (const r of document.querySelectorAll('input[name="theme"]')) r.checked = r.value === state.settings.theme;
  $('#s-close-to-tray').checked = state.settings.close_to_tray;
  $('#s-template').value = state.settings.announce_template;
  $('#s-version').textContent = state.version;
  renderPreview();
  $('#settings').showModal();
}

function renderAccount() {
  const row = $('#s-account');
  const u = state.user;
  if (u) {
    row.replaceChildren(
      img(u.profile_image_url, 'avatar'),
      h('div', { class: 'who' }, h('span', { text: u.display_name }), h('span', { class: 'meta', text: u.login })),
      h('button', { class: 'btn btn-secondary', type: 'button', onclick: logout }, icon('log-out'), 'ログアウト'));
  } else {
    row.replaceChildren(
      h('div', { class: 'who' }, h('span', { text: 'ログインしていません' })),
      h('button', {
        class: 'btn btn-primary',
        type: 'button',
        disabled: !state.clientIdReady,
        onclick: () => {
          $('#settings').close();
          openLogin();
        },
      }, icon('log-in'), 'Twitch でログイン'));
  }
}

/** いまの配信、なければ先頭のプリセットで見本を作る */
function renderPreview() {
  const tpl = $('#s-template').value;
  const ch = state.channel;
  const p = state.presets[0];
  const v = ch
    ? { title: ch.title, category: ch.category?.name ?? '', tags: ch.tags, n: null }
    : p
      ? { title: fillCounter(p.title, p.count), category: p.category?.name ?? '', tags: p.tags, n: hasCounter(p.title) ? p.count : null }
      : { title: '雑談しながらマシュマロを読む #12', category: 'Just Chatting', tags: ['日本語', '雑談'], n: 12 };
  $('#s-preview').textContent = renderAnnounce(tpl, {
    ...v,
    login: state.user?.login ?? 'your_channel',
    name: state.user?.display_name ?? 'あなたの表示名',
  }) || '（空です）';
}

function flushTemplate() {
  clearTimeout(saveTimer);
  const tpl = $('#s-template').value;
  if (tpl !== state.settings.announce_template) saveSettings({ announce_template: tpl });
}

export function wireSettings() {
  for (const r of document.querySelectorAll('input[name="theme"]')) {
    r.addEventListener('change', () => r.checked && saveSettings({ theme: r.value }));
  }
  $('#s-close-to-tray').addEventListener('change', (e) => saveSettings({ close_to_tray: e.target.checked }));

  const ta = $('#s-template');
  ta.addEventListener('input', () => {
    renderPreview();
    clearTimeout(saveTimer);
    saveTimer = setTimeout(flushTemplate, 600);
  });
  $('#settings').addEventListener('close', flushTemplate);

  $('#s-tokens').replaceChildren(...TOKENS.map(([key, label]) => h('button', {
    class: 'token',
    type: 'button',
    title: `${label}を差し込む`,
    onclick: () => {
      ta.focus();
      ta.setRangeText(`{${key}}`, ta.selectionStart, ta.selectionEnd, 'end');
      ta.dispatchEvent(new Event('input'));
    },
  }, `{${key}}`, h('small', { text: label }))));

  $('#s-export').addEventListener('click', () => exportPresets(null));
  $('#s-import-append').addEventListener('click', () => importPresets('append'));
  $('#s-import-replace').addEventListener('click', async () => {
    const ok = await confirmDialog({
      title: 'プリセットを置き換えますか',
      text: `いまのプリセット ${state.presets.length} 件を消して、選んだファイルの内容に入れ替えます。先に「すべて書き出す」で控えを取っておくと安心です`,
      ok: 'ファイルを選んで置き換える',
    });
    if (ok) importPresets('replace');
  });

  // ログイン状態が変わったら、開いている設定画面にも反映する
  subscribe(() => {
    if ($('#settings').open) renderAccount();
  });
}
