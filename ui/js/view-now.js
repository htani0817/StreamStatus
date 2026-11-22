// 「いまの配信」カード

import { copyAnnounce, postToX, refreshChannel } from './actions.js';
import { $, h, icon, img } from './dom.js';
import { openEditor } from './editor.js';
import { openLogin } from './login.js';
import { state } from './state.js';
import { boxArt, timeLabel } from './text.js';

export function renderNow() {
  const body = $('#now-body');
  const fetched = $('#now-fetched');
  const refresh = $('#now-refresh');
  refresh.hidden = !state.user;
  refresh.disabled = state.channelStatus === 'loading';
  fetched.textContent = state.channelStatus === 'loading'
    ? '読み込み中'
    : state.channel ? `${timeLabel(state.channel.fetched_at)} に取得` : '';

  if (!state.user) {
    body.replaceChildren(signedOut());
    return;
  }
  if (!state.channel) {
    body.replaceChildren(state.channelStatus === 'error' ? failed() : skeleton());
    return;
  }
  body.replaceChildren(channelView(state.channel));
}

function signedOut() {
  if (!state.clientIdReady) {
    return h('div', { class: 'now-empty' },
      h('p', { text: 'Client ID が入っていないため、ログインできません。README の「Client ID の登録」を見てください' }));
  }
  return h('div', { class: 'now-empty' },
    h('p', { text: 'Twitch にログインすると、いまの配信情報を読み込み、プリセットをそのまま反映できます' }),
    h('button', { class: 'btn btn-primary', type: 'button', onclick: openLogin }, 'Twitch でログイン'));
}

function skeleton() {
  return h('div', { 'aria-busy': 'true' },
    h('div', { class: 'now-top' },
      h('div', { class: 'art skeleton' }),
      h('div', { class: 'now-category sk-grow' },
        h('div', { class: 'skeleton sk-name' }),
        h('div', { class: 'skeleton sk-sub' }))),
    h('div', { class: 'skeleton sk-title' }),
    h('div', { class: 'skeleton sk-title sk-short' }));
}

function failed() {
  return h('div', { class: 'now-empty' },
    h('p', { class: 'now-error', text: state.channelError || '読み込めませんでした' }),
    h('button', { class: 'btn btn-secondary', type: 'button', onclick: () => refreshChannel() }, icon('refresh-cw'), 'もう一度読み込む'));
}

function channelView(ch) {
  const cat = ch.category;
  const art = cat?.box_art_url
    ? img(boxArt(cat.box_art_url, 144, 192), 'art')
    : h('div', { class: 'art art-empty', text: 'カテゴリなし' });

  return h('div', {},
    h('div', { class: 'now-top' },
      art,
      h('div', { class: 'now-category' },
        h('p', { class: 'now-category-name', text: cat?.name ?? 'カテゴリなし' }),
        h('p', { class: 'now-channel', text: ch.name || ch.login }))),
    h('p', { class: 'now-title', text: ch.title || '（タイトルなし）' }),
    ch.tags.length ? h('div', { class: 'tags' }, ch.tags.map((t) => h('span', { class: 'tag', text: t }))) : null,
    h('p', { class: 'tally-line' }, h('span', { class: 'lamp', 'aria-hidden': 'true' }), 'Twitch に出ている内容'),
    h('div', { class: 'now-actions' },
      h('button', {
        class: 'btn btn-secondary span-2',
        type: 'button',
        onclick: () => openEditor(null, { fromChannel: ch }),
      }, icon('bookmark-plus'), 'プリセットに保存'),
      h('button', { class: 'btn btn-secondary', type: 'button', onclick: () => copyAnnounce(null) }, '告知文をコピー'),
      h('button', { class: 'btn btn-secondary', type: 'button', onclick: () => postToX(null) }, 'X で投稿')));
}
