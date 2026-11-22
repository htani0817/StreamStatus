// Rust 側のコマンドを呼ぶ。ブラウザで開いたとき（見た目の確認用）は mock に切り替える

const T = window.__TAURI__;
export const isTauri = Boolean(T?.core);

let mockModule = null;
async function mock() {
  mockModule ??= await import('./mock.js');
  return mockModule;
}

export async function invoke(cmd, args = {}) {
  if (isTauri) return T.core.invoke(cmd, args);
  return (await mock()).invoke(cmd, args);
}

export async function listen(event, cb) {
  if (isTauri) return T.event.listen(event, (e) => cb(e.payload));
  return (await mock()).listen(event, cb);
}

/** エラーを画面に出せる文にする */
export function errorText(e) {
  if (e && typeof e === 'object' && 'message' in e) return String(e.message);
  return String(e ?? '不明なエラーが起きました');
}

export const errorCode = (e) => (e && typeof e === 'object' && 'code' in e ? e.code : '');
