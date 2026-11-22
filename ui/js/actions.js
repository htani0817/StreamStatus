// Rust 側を呼んで状態を更新する処理

import { errorText, invoke } from './api.js';
import { notify, replacePreset, state } from './state.js';
import { safeFileName, today } from './text.js';
import { toast } from './ui.js';

export async function refreshChannel({ quiet = false } = {}) {
  if (!state.user) return;
  state.channelStatus = 'loading';
  notify();
  try {
    state.channel = await invoke('fetch_channel');
    state.channelStatus = 'ok';
  } catch (e) {
    state.channelStatus = 'error';
    state.channelError = errorText(e);
    if (!quiet) toast(errorText(e), 'error');
  }
  notify();
}

export async function applyPreset(id, increment) {
  const key = `${id}:${increment ? 'inc' : 'set'}`;
  if (state.busy.size) return;
  state.busy.add(key);
  notify();
  try {
    const r = await invoke('apply_preset', { id, increment });
    replacePreset(r.preset);
    state.channel = r.channel;
    state.channelStatus = 'ok';
    toast(increment ? `配信回数を ${r.preset.count} にして反映しました` : 'Twitch に反映しました');
  } catch (e) {
    toast(errorText(e), 'error');
  } finally {
    state.busy.delete(key);
    notify();
  }
}

export async function savePreset(input) {
  const p = await invoke('save_preset', { input });
  replacePreset(p);
  notify();
  return p;
}

export async function deletePreset(id) {
  await invoke('delete_preset', { id });
  state.presets = state.presets.filter((p) => p.id !== id);
  notify();
}

export async function duplicatePreset(id) {
  try {
    const copy = await invoke('duplicate_preset', { id });
    const i = state.presets.findIndex((p) => p.id === id);
    state.presets.splice(i + 1, 0, copy);
    notify();
    toast('複製しました');
  } catch (e) {
    toast(errorText(e), 'error');
  }
}

export async function reorder(ids) {
  const byId = new Map(state.presets.map((p) => [p.id, p]));
  state.presets = ids.map((id) => byId.get(id)).filter(Boolean);
  notify();
  try {
    await invoke('reorder_presets', { ids });
  } catch (e) {
    toast(errorText(e), 'error');
  }
}

export function move(id, delta) {
  const ids = state.presets.map((p) => p.id);
  const i = ids.indexOf(id);
  const j = i + delta;
  if (i < 0 || j < 0 || j >= ids.length) return false;
  [ids[i], ids[j]] = [ids[j], ids[i]];
  reorder(ids);
  return true;
}

async function announce(id) {
  return invoke('announce_text', { id: id ?? null });
}

export async function copyAnnounce(id) {
  try {
    const text = await announce(id);
    await navigator.clipboard.writeText(text);
    toast('告知文をコピーしました');
  } catch (e) {
    toast(errorText(e), 'error');
  }
}

export async function postToX(id) {
  try {
    const text = await announce(id);
    await invoke('open_x_intent', { text });
  } catch (e) {
    toast(errorText(e), 'error');
  }
}

export async function exportPresets(ids) {
  const single = ids?.length === 1 ? state.presets.find((p) => p.id === ids[0]) : null;
  const file_name = single ? `${safeFileName(single.title)}.tsp` : `streamstatus-backup-${today()}.tsb`;
  try {
    const path = await invoke('export_presets', { ids: ids ?? null, file_name });
    if (path) toast('書き出しました');
  } catch (e) {
    toast(errorText(e), 'error');
  }
}

export async function importPresets(mode) {
  try {
    const r = await invoke('import_presets', { mode });
    if (!r) return;
    state.presets = r.presets;
    notify();
    const skipped = r.skipped ? `（読めなかったもの ${r.skipped} 件）` : '';
    toast(`${r.added} 件を読み込みました${skipped}`);
  } catch (e) {
    toast(errorText(e), 'error');
  }
}

export async function saveSettings(patch) {
  const next = { ...state.settings, ...patch };
  try {
    state.settings = await invoke('save_settings', { settings: next });
    applyThemeAttr(state.settings.theme);
    notify();
  } catch (e) {
    toast(errorText(e), 'error');
  }
}

export function applyThemeAttr(theme) {
  const root = document.documentElement;
  if (theme === 'light' || theme === 'dark') root.dataset.theme = theme;
  else delete root.dataset.theme;
}

export async function logout() {
  try {
    await invoke('logout');
  } catch (e) {
    toast(errorText(e), 'error');
  }
}
