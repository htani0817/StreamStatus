// 起動と全体の配線

import { applyThemeAttr, refreshChannel } from './actions.js';
import { errorText, invoke, isTauri, listen } from './api.js';
import { $ } from './dom.js';
import { openEditor, wireEditor } from './editor.js';
import { loginEvent, openLogin, wireLogin } from './login.js';
import { openSettings, wireSettings } from './settings.js';
import { notify, state, subscribe } from './state.js';
import { toast, wireDialogs, wireMenu } from './ui.js';
import { renderNow } from './view-now.js';
import { renderPresets, wireDragSort } from './view-presets.js';

function renderTopbar() {
  const u = state.user;
  const account = $('#account-button');
  account.hidden = !u;
  $('#login-button').hidden = Boolean(u) || !state.clientIdReady;
  if (u) {
    $('#account-name').textContent = u.display_name;
    account.title = `${u.display_name}（${u.login}）`;
    const avatar = $('#account-avatar');
    if (u.profile_image_url && avatar.getAttribute('src') !== u.profile_image_url) avatar.src = u.profile_image_url;
    if (!u.profile_image_url) avatar.removeAttribute('src');
  }
}

function render() {
  renderTopbar();
  renderNow();
  renderPresets();
}

function onAuth(p) {
  loginEvent(p);
  if (p.state === 'signed_in') {
    state.user = p.user;
    notify();
    toast(`${p.user.display_name} でログインしました`);
    refreshChannel();
  } else if (p.state === 'signed_out') {
    const was = state.user;
    state.user = null;
    state.channel = null;
    state.channelStatus = 'idle';
    notify();
    if (was && p.message) toast(p.message, 'error');
  }
}

const anyDialogOpen = () => Boolean(document.querySelector('dialog[open]'));

function wireShortcuts() {
  document.addEventListener('keydown', (e) => {
    // ページの再読み込みの代わりに、配信情報を読み直す
    if (e.key === 'F5' || (e.ctrlKey && e.key.toLowerCase() === 'r')) {
      e.preventDefault();
      if (!anyDialogOpen()) refreshChannel();
    } else if (e.ctrlKey && e.key.toLowerCase() === 'n' && !anyDialogOpen()) {
      e.preventDefault();
      openEditor(null);
    }
  });
  // 入力欄の外では、ブラウザの右クリックメニュー（戻る・再読み込み）を出さない
  document.addEventListener('contextmenu', (e) => {
    if (!e.target.closest('input, textarea, .now-title, .card-title, pre')) e.preventDefault();
  });
}

async function boot() {
  wireDialogs();
  wireMenu();
  wireDragSort();
  wireEditor();
  wireSettings();
  wireLogin();
  wireShortcuts();

  $('#settings-button').addEventListener('click', openSettings);
  $('#account-button').addEventListener('click', openSettings);
  $('#login-button').addEventListener('click', openLogin);
  $('#add-preset').addEventListener('click', () => openEditor(null));
  $('#empty-add').addEventListener('click', () => openEditor(null));
  $('#now-refresh').addEventListener('click', () => refreshChannel());
  $('#settings').dataset.backdropClose = '';
  $('#login').dataset.backdropClose = '';

  subscribe(render);

  try {
    const data = await invoke('get_bootstrap');
    Object.assign(state, {
      presets: data.presets,
      settings: data.settings,
      user: data.user,
      version: data.version,
      clientIdReady: data.client_id_ready,
    });
  } catch (e) {
    toast(errorText(e), 'error');
  }
  applyThemeAttr(state.settings.theme);
  render();
  await listen('auth-changed', onAuth);
  invoke('ui_ready');
  if (state.user) await refreshChannel({ quiet: true });
  if (!isTauri) openForPreview();
}

/** ブラウザで見た目を確かめるとき用（?open=editor など）。アプリでは動かない */
function openForPreview() {
  const open = new URLSearchParams(location.search).get('open');
  if (open === 'editor') openEditor(state.presets[0]);
  else if (open === 'new') openEditor(null, { fromChannel: state.channel });
  else if (open === 'settings') openSettings();
  else if (open === 'login') openLogin();
  else if (open === 'menu') document.querySelector('[data-act="menu"]')?.click();
}

boot();
