// Minimal Node host primitives for compiled PureScript in BSD VMs without an ARM64 FluentCI binary.
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
export const env = key => () => process.env[key] ?? '';
export const platform = () => process.platform;
export const arch = () => process.arch;
export const run = argv => () => {
  const result = spawnSync(argv[0], argv.slice(1), {stdio:'inherit',env:process.env});
  if (result.error) throw result.error;
  if (result.status !== 0) throw Error(`${argv[0]} failed (${result.status})`);
};
export const digest = path => () => createHash('sha256').update(readFileSync(path)).digest('hex');
export const writeText = path => text => () => writeFileSync(path,text);
export const setEnv = key => value => () => { process.env[key] = value; };
export const chdir = path => () => process.chdir(path);
