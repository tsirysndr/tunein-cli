// Build-time compatibility for FluentCI v0.17.3's PureScript inspector.
// Match the compiler-AST normalization already used by the shared module builder.
import {readFileSync, writeFileSync} from 'node:fs';

export function normalizePurescriptInspector(path) {
  const original = readFileSync(path, 'utf8');
  let source = original;
  for (const [declaration, handler] of [
    ['function resolve(type, stack = []) {', 'if (type.tag === "ParensInType") return resolve(type.contents, stack);'],
    ['function show(type) {', 'if (type.tag === "ParensInType") return show(type.contents);'],
  ]) {
    const start = source.indexOf(declaration);
    if (start < 0) throw Error(`Unsupported PureScript inspector layout: missing ${declaration}`);
    const body = start + declaration.length;
    if (/^\s*if\s*\(type\.tag\s*===\s*["']ParensInType["']\)/.test(source.slice(body))) continue;
    source = source.slice(0, body) + '\n    ' + handler + source.slice(body);
  }
  if (source !== original) writeFileSync(path, source);
}
