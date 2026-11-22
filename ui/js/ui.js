// 通知・確認ダイアログ・ポップアップメニュー

import { $, h, icon } from './dom.js';

export function toast(text, kind = 'info') {
  const region = $('#toasts');
  const el = h('div', { class: `toast${kind === 'error' ? ' is-error' : ''}`, role: kind === 'error' ? 'alert' : 'status' },
    kind === 'error' ? null : icon('check'),
    h('span', { text }));
  region.append(el);
  while (region.children.length > 3) region.firstElementChild.remove();
  const ms = kind === 'error' ? 6000 : 2800;
  setTimeout(() => {
    el.classList.add('is-leaving');
    setTimeout(() => el.remove(), 200);
  }, ms);
}

/** 確認ダイアログ。OK なら true */
export function confirmDialog({ title, text, ok }) {
  const dlg = $('#confirm');
  $('#confirm-title').textContent = title;
  $('#confirm-text').textContent = text;
  $('#confirm-ok').textContent = ok;
  return new Promise((resolve) => {
    const done = (v) => {
      dlg.removeEventListener('close', onClose);
      dlg.close();
      resolve(v);
    };
    const onClose = () => done(false);
    $('#confirm-ok').onclick = () => done(true);
    $('#confirm-cancel').onclick = () => done(false);
    dlg.addEventListener('close', onClose);
    dlg.showModal();
    $('#confirm-cancel').focus();
  });
}

/** [data-close] で閉じる。data-backdrop-close を付けたダイアログは外側のクリックでも閉じる */
export function wireDialogs() {
  document.addEventListener('click', (e) => {
    const btn = e.target.closest('[data-close]');
    if (btn) btn.closest('dialog')?.close();
    if (e.target instanceof HTMLDialogElement && 'backdropClose' in e.target.dataset) {
      const r = e.target.getBoundingClientRect();
      const inside = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
      if (!inside) e.target.close();
    }
  });
}

let menuAnchor = null;

/**
 * ボタンの下にメニューを出す。
 * items: { label, icon, onSelect, danger, disabled } または 'sep'
 */
export function openMenu(anchor, items) {
  const menu = $('#card-menu');
  if (menu.matches(':popover-open')) {
    menu.hidePopover();
    if (menuAnchor === anchor) return;
  }
  menu.replaceChildren(...items.map((it) => {
    if (it === 'sep') return h('div', { class: 'menu-sep', role: 'separator' });
    return h('button', {
      class: `menu-item${it.danger ? ' is-danger' : ''}`,
      type: 'button',
      role: 'menuitem',
      disabled: it.disabled,
      onclick: () => {
        menu.hidePopover();
        it.onSelect();
      },
    }, icon(it.icon), it.label);
  }));
  menuAnchor = anchor;
  anchor.setAttribute('aria-expanded', 'true');
  menu.showPopover();

  // ボタンの右端にそろえ、下に入らなければ上に出す
  const r = anchor.getBoundingClientRect();
  const m = menu.getBoundingClientRect();
  const left = Math.max(8, Math.min(r.right - m.width, innerWidth - m.width - 8));
  const below = r.bottom + 4;
  const top = below + m.height > innerHeight - 8 ? Math.max(8, r.top - m.height - 4) : below;
  menu.style.left = `${left}px`;
  menu.style.top = `${top}px`;
  menu.querySelector('.menu-item:not(:disabled)')?.focus();
}

export function wireMenu() {
  const menu = $('#card-menu');
  menu.addEventListener('toggle', (e) => {
    if (e.newState === 'closed' && menuAnchor) {
      menuAnchor.setAttribute('aria-expanded', 'false');
      if (menu.contains(document.activeElement) || document.activeElement === document.body) menuAnchor.focus();
    }
  });
  menu.addEventListener('keydown', (e) => {
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
    e.preventDefault();
    const items = [...menu.querySelectorAll('.menu-item:not(:disabled)')];
    const i = items.indexOf(document.activeElement);
    const next = e.key === 'ArrowDown' ? (i + 1) % items.length : (i - 1 + items.length) % items.length;
    items[next]?.focus();
  });
}

/** ボタンを処理中の見た目にして、終わったら戻す */
export async function withBusy(btn, fn) {
  btn.classList.add('is-busy');
  btn.disabled = true;
  try {
    return await fn();
  } finally {
    btn.classList.remove('is-busy');
    btn.disabled = false;
  }
}
