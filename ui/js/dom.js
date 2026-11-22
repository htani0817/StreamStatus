// DOM を組み立てる小さな道具

export function h(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props ?? {})) {
    if (v == null || v === false) continue;
    if (k === 'class') el.className = v;
    else if (k === 'text') el.textContent = v;
    else if (k === 'dataset') Object.assign(el.dataset, v);
    else if (k.startsWith('on') && typeof v === 'function') el.addEventListener(k.slice(2), v);
    else if (v === true) el.setAttribute(k, '');
    else el.setAttribute(k, v);
  }
  for (const c of children.flat()) {
    if (c == null || c === false) continue;
    el.append(c instanceof Node ? c : String(c));
  }
  return el;
}

const SVG_NS = 'http://www.w3.org/2000/svg';

export function icon(name, cls = 'i') {
  const svg = document.createElementNS(SVG_NS, 'svg');
  svg.setAttribute('class', cls);
  svg.setAttribute('aria-hidden', 'true');
  const use = document.createElementNS(SVG_NS, 'use');
  use.setAttribute('href', `icons.svg#${name}`);
  svg.append(use);
  return svg;
}

export const $ = (sel, root = document) => root.querySelector(sel);

export function img(src, cls, alt = '') {
  const el = h('img', { class: cls, alt, loading: 'lazy', decoding: 'async', referrerpolicy: 'no-referrer' });
  // 読めなかったら空の枠だけ残す
  el.addEventListener('error', () => el.removeAttribute('src'), { once: true });
  if (src) el.src = src;
  return el;
}
