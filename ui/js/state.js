// 画面全体で共有する状態。変えたら notify() で描き直す

export const state = {
  presets: [],
  settings: { theme: 'system', announce_template: '', close_to_tray: true },
  user: null,
  channel: null,
  /** 'idle' | 'loading' | 'ok' | 'error' */
  channelStatus: 'idle',
  channelError: '',
  version: '',
  clientIdReady: true,
  /** 処理中のボタン（`${id}:set` など） */
  busy: new Set(),
};

const subscribers = new Set();

export function subscribe(fn) {
  subscribers.add(fn);
}

export function notify() {
  for (const fn of subscribers) fn();
}

export function replacePreset(p) {
  const i = state.presets.findIndex((x) => x.id === p.id);
  if (i >= 0) state.presets[i] = p;
  else state.presets.push(p);
}
