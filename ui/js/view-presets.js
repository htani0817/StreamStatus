// プリセット一覧（カード・並び替え・メニュー）

import { applyPreset, copyAnnounce, deletePreset, duplicatePreset, exportPresets, move, postToX, reorder } from './actions.js';
import { errorText } from './api.js';
import { $, h, icon, img } from './dom.js';
import { openEditor } from './editor.js';
import { state } from './state.js';
import { boxArt, hasCounter, matchesChannel, splitCounter } from './text.js';
import { confirmDialog, openMenu, toast } from './ui.js';

export function renderPresets() {
  const list = $('#preset-list');
  const empty = $('#preset-empty');
  const n = state.presets.length;
  $('#preset-count').textContent = n ? `${n} 件` : '';
  empty.hidden = n > 0;
  list.hidden = n === 0;

  // 描き直してもフォーカスの位置を保つ
  const active = document.activeElement;
  const keep = list.contains(active)
    ? { id: active.closest('.card')?.dataset.id, act: active.dataset.act }
    : null;

  list.replaceChildren(...state.presets.map(card));

  if (keep?.id) {
    const sel = keep.act ? `[data-act="${keep.act}"]` : '.grip';
    list.querySelector(`.card[data-id="${CSS.escape(keep.id)}"] ${sel}`)?.focus();
  }
}

function titleView(p, n) {
  return h('p', { class: 'card-title', title: p.title },
    splitCounter(p.title, n).map((part) =>
      'count' in part ? h('span', { class: 'counter', text: String(part.count) }) : part.text));
}

function card(p) {
  const live = matchesChannel(p, state.channel);
  const counter = hasCounter(p.title);
  const signedIn = Boolean(state.user);
  const busySet = state.busy.has(`${p.id}:set`);
  const busyInc = state.busy.has(`${p.id}:inc`);
  const anyBusy = state.busy.size > 0;
  const needLogin = signedIn ? null : 'ログインすると使えます';
  const cat = p.category;

  const applyBtn = h('button', {
    class: `btn btn-primary btn-sm${busySet ? ' is-busy' : ''}`,
    type: 'button',
    dataset: { act: 'apply' },
    disabled: !signedIn || anyBusy,
    title: needLogin,
    onclick: () => applyPreset(p.id, false),
  }, '適用');

  const incBtn = counter
    ? h('button', {
      class: `btn btn-secondary btn-sm${busyInc ? ' is-busy' : ''}`,
      type: 'button',
      dataset: { act: 'apply-inc' },
      disabled: !signedIn || anyBusy,
      title: needLogin ?? `配信回数を ${p.count + 1} にして反映します`,
      onclick: () => applyPreset(p.id, true),
    }, '加算して適用', h('span', { class: 'counter', text: String(p.count + 1) }))
    : null;

  const menuBtn = h('button', {
    class: 'icon-btn',
    type: 'button',
    dataset: { act: 'menu' },
    'aria-label': 'ほかの操作',
    'aria-haspopup': 'menu',
    'aria-expanded': 'false',
    title: 'ほかの操作',
    onclick: (e) => openMenu(e.currentTarget, menuItems(p)),
  }, icon('ellipsis'));

  const grip = h('button', {
    class: 'grip',
    type: 'button',
    'aria-label': '並び替え。ドラッグするか、上下の矢印キーで動かします',
    title: 'ドラッグで並び替え',
    onpointerdown: (e) => { e.currentTarget.closest('.card').draggable = true; },
    onkeydown: (e) => {
      if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return;
      e.preventDefault();
      move(p.id, e.key === 'ArrowUp' ? -1 : 1);
    },
  }, icon('grip-vertical'));

  return h('li', { class: `card${live ? ' is-live' : ''}`, dataset: { id: p.id } },
    h('div', { class: 'card-main' },
      grip,
      h('div', { class: 'card-body' },
        h('div', { class: 'card-title-row' },
          titleView(p, p.count),
          live ? h('span', { class: 'live-label' }, h('span', { class: 'lamp', 'aria-hidden': 'true' }), '反映中') : null),
        h('div', { class: 'card-meta' },
          cat?.box_art_url ? img(boxArt(cat.box_art_url, 36, 48), 'art-sm') : null,
          h('span', { class: `card-category${cat ? '' : ' no-category'}`, text: cat?.name ?? 'カテゴリなし' }),
          p.tags.length ? h('span', { class: 'tags' }, p.tags.map((t) => h('span', { class: 'tag', text: t }))) : null),
        p.memo ? h('p', { class: 'card-memo', title: p.memo, text: p.memo }) : null)),
    h('div', { class: 'card-actions' },
      applyBtn,
      incBtn,
      h('span', { class: 'push' }),
      h('button', {
        class: 'icon-btn', type: 'button', dataset: { act: 'announce' },
        'aria-label': '告知文をコピー', title: '告知文をコピー',
        onclick: () => copyAnnounce(p.id),
      }, icon('copy')),
      h('button', {
        class: 'icon-btn', type: 'button', dataset: { act: 'edit' },
        'aria-label': '編集', title: '編集',
        onclick: () => openEditor(p),
      }, icon('pencil')),
      menuBtn));
}

function menuItems(p) {
  const i = state.presets.findIndex((x) => x.id === p.id);
  return [
    { label: 'X で投稿画面を開く', icon: 'external-link', onSelect: () => postToX(p.id) },
    { label: '複製', icon: 'copy-plus', onSelect: () => duplicatePreset(p.id) },
    { label: 'このプリセットを書き出す', icon: 'download', onSelect: () => exportPresets([p.id]) },
    'sep',
    { label: '上へ移動', icon: 'arrow-up', disabled: i === 0, onSelect: () => move(p.id, -1) },
    { label: '下へ移動', icon: 'arrow-down', disabled: i === state.presets.length - 1, onSelect: () => move(p.id, 1) },
    'sep',
    {
      label: '削除', icon: 'trash-2', danger: true,
      onSelect: async () => {
        const ok = await confirmDialog({ title: 'プリセットを削除しますか', text: `「${p.title}」を削除します。元に戻せません`, ok: '削除' });
        if (!ok) return;
        try {
          await deletePreset(p.id);
          toast('削除しました');
        } catch (e) {
          toast(errorText(e), 'error');
        }
      },
    },
  ];
}

/** ドラッグで並び替え。つかみ（grip）を押したときだけカードをドラッグできる */
export function wireDragSort() {
  const list = $('#preset-list');
  let dragging = null;
  let before = [];

  list.addEventListener('dragstart', (e) => {
    dragging = e.target.closest('.card');
    if (!dragging) return;
    before = [...list.children].map((c) => c.dataset.id);
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', dragging.dataset.id);
    requestAnimationFrame(() => dragging?.classList.add('is-dragging'));
  });

  list.addEventListener('dragover', (e) => {
    if (!dragging) return;
    e.preventDefault();
    const target = e.target.closest('.card');
    if (!target || target === dragging) return;
    const r = target.getBoundingClientRect();
    const cols = getComputedStyle(list).gridTemplateColumns.split(' ').length;
    const isBefore = cols > 1 ? e.clientX < r.left + r.width / 2 : e.clientY < r.top + r.height / 2;
    list.insertBefore(dragging, isBefore ? target : target.nextSibling);
  });

  list.addEventListener('drop', (e) => e.preventDefault());

  list.addEventListener('dragend', () => {
    if (!dragging) return;
    dragging.classList.remove('is-dragging');
    dragging.draggable = false;
    dragging = null;
    const after = [...list.children].map((c) => c.dataset.id);
    if (after.join() !== before.join()) reorder(after);
  });

  // つかまずに離したら、ドラッグできない状態に戻す
  list.addEventListener('pointerup', (e) => {
    const c = e.target.closest('.card');
    if (c && !dragging) c.draggable = false;
  });
}
