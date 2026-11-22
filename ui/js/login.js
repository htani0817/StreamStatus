// Twitch ログイン（Device Code Flow）のダイアログ

import { errorText, invoke } from './api.js';
import { $, h, icon } from './dom.js';
import { toast } from './ui.js';

let timer = 0;
let waiting = false;

function stopTimer() {
  clearInterval(timer);
  timer = 0;
}

export async function openLogin() {
  const dlg = $('#login');
  if (!dlg.open) dlg.showModal();
  $('#login-body').replaceChildren(h('p', { class: 'login-lead', text: 'コードを用意しています' }));
  try {
    const v = await invoke('login_start');
    waiting = true;
    renderCode(v);
  } catch (e) {
    renderError(errorText(e));
  }
}

function renderCode(v) {
  const chars = [...v.user_code];
  const half = Math.ceil(chars.length / 2);
  const code = h('div', { class: 'code', 'aria-label': `コード ${v.user_code}` },
    chars.flatMap((c, i) => [
      i === half && chars.length > 6 ? h('span', { class: 'code-gap' }) : null,
      h('span', { class: 'keycap', 'aria-hidden': 'true', text: c }),
    ]));

  const remain = h('span', { class: 'meta' });
  const deadline = Date.now() + v.expires_in * 1000;
  const tick = () => {
    const s = Math.max(0, Math.round((deadline - Date.now()) / 1000));
    remain.textContent = `残り ${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
  };
  stopTimer();
  tick();
  timer = setInterval(tick, 1000);

  $('#login-body').replaceChildren(h('div', { class: 'login-steps' },
    h('p', { class: 'login-lead', text: 'ブラウザで Twitch の認証ページを開きました。次のコードを入れて、許可してください' }),
    h('div', { class: 'code-row' },
      code,
      h('button', {
        class: 'icon-btn',
        type: 'button',
        'aria-label': 'コードをコピー',
        title: 'コードをコピー',
        onclick: async () => {
          await navigator.clipboard.writeText(v.user_code);
          toast('コードをコピーしました');
        },
      }, icon('copy'))),
    h('p', { class: 'wait-line' }, h('span', { class: 'lamp', 'aria-hidden': 'true' }), '許可を待っています', remain),
    h('div', { class: 'button-row' },
      h('button', { class: 'btn btn-secondary', type: 'button', onclick: () => invoke('open_activation') }, icon('external-link'), 'ブラウザでもう一度開く'),
      h('button', { class: 'btn btn-quiet', type: 'button', 'data-close': true }, 'キャンセル')),
    h('p', { class: 'fine', text: 'パスワードはこのアプリに入力しません。このアプリが使うのは、配信タイトル・カテゴリ・タグの変更だけです' })));
}

function renderError(message) {
  stopTimer();
  $('#login-body').replaceChildren(h('div', { class: 'login-steps' },
    h('p', { class: 'now-error', text: message || 'ログインできませんでした' }),
    h('div', { class: 'button-row' },
      h('button', { class: 'btn btn-primary', type: 'button', onclick: openLogin }, 'もう一度試す'),
      h('button', { class: 'btn btn-quiet', type: 'button', 'data-close': true }, '閉じる'))));
}

/** Rust 側からのログイン状態の知らせ */
export function loginEvent(p) {
  if (p.state === 'signed_in') {
    waiting = false;
    stopTimer();
    $('#login').close();
  } else if (p.state === 'login_failed' || p.state === 'login_expired') {
    waiting = false;
    if ($('#login').open) renderError(p.message);
  }
}

export function wireLogin() {
  $('#login').addEventListener('close', () => {
    stopTimer();
    if (waiting) {
      waiting = false;
      invoke('login_cancel');
    }
  });
}
