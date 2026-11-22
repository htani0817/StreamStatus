// プリセットの追加・編集ダイアログ

import { deletePreset, savePreset } from './actions.js';
import { errorText, invoke } from './api.js';
import { $, h, icon, img } from './dom.js';
import { state } from './state.js';
import { MEMO_MAX, TAGS_MAX, TITLE_MAX, boxArt, charCount, cleanTag, fillCounter, hasCounter, splitCounter, tagProblem } from './text.js';
import { confirmDialog, toast, withBusy } from './ui.js';

/** 編集中の内容（入力欄に無いもの） */
let editing = { id: null, tags: [], category: null };
let results = [];
let active = -1;
let searchTimer = 0;
let searchSeq = 0;

export function openEditor(preset, { fromChannel } = {}) {
  const src = preset
    ?? (fromChannel
      ? { title: fromChannel.title, category: fromChannel.category, tags: fromChannel.tags, memo: '', count: 0 }
      : { title: '', category: null, tags: [], memo: '', count: 0 });

  editing = { id: preset?.id ?? null, tags: [...src.tags], category: src.category ? { ...src.category } : null };
  $('#editor-title').textContent = preset ? 'プリセットを編集' : fromChannel ? 'いまの配信をプリセットにする' : 'プリセットを追加';
  $('#editor-save').textContent = preset ? '保存' : '追加';
  $('#editor-delete').hidden = !preset;
  $('#f-title').value = src.title;
  $('#f-count').value = String(src.count ?? 0);
  $('#f-memo').value = src.memo ?? '';
  $('#f-tag-input').value = '';

  const searchInput = $('#f-category');
  searchInput.value = '';
  searchInput.disabled = !state.user;
  searchInput.placeholder = state.user ? 'カテゴリを検索' : 'ログインすると検索できます';

  closeList();
  setError('');
  tagHint(null);
  renderCategory();
  renderTags();
  updateTitle();
  updateMemo();
  $('#editor').showModal();
  $('#f-title').focus();
}

function setError(msg) {
  const el = $('#editor-error');
  el.textContent = msg;
  el.hidden = !msg;
}

// ---- タイトルと回数 ----

function currentCount() {
  const n = Number.parseInt($('#f-count').value, 10);
  return Number.isFinite(n) && n >= 0 ? n : 0;
}

function updateTitle() {
  const title = $('#f-title').value;
  const counter = hasCounter(title);
  const n = currentCount();
  const len = charCount(fillCounter(title.trim(), n));
  const meta = $('#f-title-count');
  meta.textContent = `${len}/${TITLE_MAX}`;
  meta.classList.toggle('is-over', len > TITLE_MAX);
  $('#f-count-field').hidden = !counter;

  const hint = $('#f-title-hint');
  if (counter) {
    hint.replaceChildren('反映されるタイトル: ',
      ...splitCounter(title.trim(), n).map((p) => ('count' in p ? h('span', { class: 'counter', text: String(p.count) }) : p.text)));
  } else {
    hint.replaceChildren(h('code', { text: '{n}' }), ' を入れると、その場所に配信回数が入ります');
  }
}

function updateMemo() {
  const len = charCount($('#f-memo').value);
  const meta = $('#f-memo-count');
  meta.textContent = `${len}/${MEMO_MAX}`;
  meta.classList.toggle('is-over', len > MEMO_MAX);
}

// ---- カテゴリ ----

function renderCategory() {
  const picked = $('#f-category-picked');
  const combo = $('#f-category-combo');
  const c = editing.category;
  picked.hidden = !c;
  combo.hidden = Boolean(c);
  if (!c) return;
  picked.replaceChildren(
    c.box_art_url ? img(boxArt(c.box_art_url, 48, 64), 'art-sm') : null,
    h('span', { text: c.name }),
    h('button', {
      class: 'icon-btn',
      type: 'button',
      'aria-label': 'カテゴリを外す',
      title: 'カテゴリを外す',
      onclick: () => {
        editing.category = null;
        renderCategory();
        $('#f-category').focus();
      },
    }, icon('x')));
}

function closeList() {
  const list = $('#f-category-list');
  list.hidden = true;
  list.replaceChildren();
  $('#f-category').setAttribute('aria-expanded', 'false');
  $('#f-category').removeAttribute('aria-activedescendant');
}

function showNote(text) {
  const list = $('#f-category-list');
  list.replaceChildren(h('li', { class: 'combo-note', text }));
  list.hidden = false;
  $('#f-category').setAttribute('aria-expanded', 'true');
}

function renderList() {
  if (!results.length) {
    showNote('見つかりませんでした');
    return;
  }
  const list = $('#f-category-list');
  list.replaceChildren(...results.map((c, i) => h('li', {
    class: 'combo-option',
    role: 'option',
    id: `cat-opt-${i}`,
    'aria-selected': String(i === active),
    // フォーカスを入力欄から外さずに選ぶ
    onpointerdown: (e) => {
      e.preventDefault();
      pick(c);
    },
  }, img(boxArt(c.box_art_url, 36, 48), 'art-sm'), h('span', { text: c.name }))));
  list.hidden = false;
  const input = $('#f-category');
  input.setAttribute('aria-expanded', 'true');
  if (active >= 0) {
    input.setAttribute('aria-activedescendant', `cat-opt-${active}`);
    $(`#cat-opt-${active}`)?.scrollIntoView({ block: 'nearest' });
  }
}

async function search(q) {
  const seq = ++searchSeq;
  showNote('検索しています');
  try {
    const r = await invoke('search_categories', { query: q });
    if (seq !== searchSeq) return;
    results = r;
    active = r.length ? 0 : -1;
    renderList();
  } catch (e) {
    if (seq === searchSeq) showNote(errorText(e));
  }
}

function pick(c) {
  editing.category = c;
  $('#f-category').value = '';
  closeList();
  renderCategory();
}

// ---- タグ ----

function renderTags() {
  const box = $('#f-tags');
  const input = $('#f-tag-input');
  box.querySelectorAll('.tag-chip').forEach((c) => c.remove());
  editing.tags.forEach((t, i) => {
    box.insertBefore(h('span', { class: 'tag-chip' }, t, h('button', {
      type: 'button',
      'aria-label': `${t} を外す`,
      onclick: () => {
        editing.tags.splice(i, 1);
        renderTags();
        tagHint(null);
        input.focus();
      },
    }, icon('x'))), input);
  });
  const full = editing.tags.length >= TAGS_MAX;
  $('#f-tags-count').textContent = `${editing.tags.length}/${TAGS_MAX}`;
  input.disabled = full;
  input.placeholder = full ? '' : editing.tags.length ? '' : 'タグを入力';
}

function tagHint(problem) {
  const hint = $('#f-tags-hint');
  hint.textContent = problem ?? 'Enter かカンマで追加。空白と半角記号は使えません';
  hint.classList.toggle('is-error', Boolean(problem));
}

/** カンマ区切りの文字をタグとして足す。問題があれば false */
function addTags(raw) {
  let problem = null;
  for (const part of raw.split(/[,、，]/)) {
    const t = cleanTag(part);
    if (!t) continue;
    problem = tagProblem(t);
    if (problem) break;
    if (editing.tags.some((x) => x.toLowerCase() === t.toLowerCase())) continue;
    if (editing.tags.length >= TAGS_MAX) {
      problem = `タグは${TAGS_MAX}個までです`;
      break;
    }
    editing.tags.push(t);
  }
  renderTags();
  tagHint(problem);
  return !problem;
}

// ---- 配線 ----

export function wireEditor() {
  $('#f-title').addEventListener('input', updateTitle);
  $('#f-count').addEventListener('input', updateTitle);
  $('#f-memo').addEventListener('input', updateMemo);

  const searchInput = $('#f-category');
  searchInput.addEventListener('input', () => {
    clearTimeout(searchTimer);
    const q = searchInput.value.trim();
    if (!q) {
      searchSeq++;
      closeList();
      return;
    }
    searchTimer = setTimeout(() => search(q), 300);
  });
  searchInput.addEventListener('keydown', (e) => {
    const open = !$('#f-category-list').hidden;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      if (!results.length) return;
      e.preventDefault();
      active = e.key === 'ArrowDown' ? (active + 1) % results.length : (active - 1 + results.length) % results.length;
      renderList();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (open && active >= 0 && results[active]) pick(results[active]);
    } else if (e.key === 'Escape' && open) {
      // ダイアログごと閉じないようにする
      e.preventDefault();
      e.stopPropagation();
      closeList();
    }
  });
  searchInput.addEventListener('blur', () => setTimeout(closeList, 150));

  const tagInput = $('#f-tag-input');
  $('#f-tags').addEventListener('click', (e) => {
    if (e.target === e.currentTarget) tagInput.focus();
  });
  tagInput.addEventListener('keydown', (e) => {
    // 日本語の変換中の Enter はタグにしない
    if (e.isComposing || e.keyCode === 229) return;
    if (e.key === 'Enter' || e.key === ',') {
      e.preventDefault();
      if (tagInput.value.trim() && addTags(tagInput.value)) tagInput.value = '';
    } else if (e.key === 'Backspace' && !tagInput.value && editing.tags.length) {
      editing.tags.pop();
      renderTags();
      tagHint(null);
    }
  });
  tagInput.addEventListener('input', () => {
    const v = tagInput.value;
    if (!/[,、，]/.test(v)) return;
    const parts = v.split(/[,、，]/);
    const rest = parts.pop();
    if (addTags(parts.join(','))) tagInput.value = rest;
  });

  $('#editor-form').addEventListener('submit', async (e) => {
    e.preventDefault();
    const pendingTag = tagInput.value.trim();
    if (pendingTag) {
      if (!addTags(pendingTag)) return;
      tagInput.value = '';
    }
    const title = $('#f-title').value.trim();
    if (!title) {
      setError('タイトルを入れてください');
      $('#f-title').focus();
      return;
    }
    const isNew = !editing.id;
    const input = {
      id: editing.id,
      title,
      category: editing.category,
      tags: editing.tags,
      memo: $('#f-memo').value,
      count: currentCount(),
    };
    await withBusy($('#editor-save'), async () => {
      try {
        await savePreset(input);
        $('#editor').close();
        toast(isNew ? 'プリセットを追加しました' : '保存しました');
      } catch (err) {
        setError(errorText(err));
      }
    });
  });

  $('#editor-delete').addEventListener('click', async () => {
    const id = editing.id;
    if (!id) return;
    const ok = await confirmDialog({ title: 'プリセットを削除しますか', text: `「${$('#f-title').value}」を削除します。元に戻せません`, ok: '削除' });
    if (!ok) return;
    try {
      await deletePreset(id);
      $('#editor').close();
      toast('削除しました');
    } catch (err) {
      setError(errorText(err));
    }
  });
}
