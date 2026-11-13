// SDK/toolchain build glue only. CI policy and environment persistence are PureScript.
import {mkdtempSync, cpSync, rmSync} from 'node:fs';
import {tmpdir, homedir} from 'node:os';
import {join, dirname, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
const root = dirname(fileURLToPath(import.meta.url));
const home = homedir();
const toolchain = join(home,'.local/share/fluentci/purescript/0.15.16');
const node = join(home,'.local/share/fluentci/node/24.22.2/bin/node');
const env = {...process.env, PATH:[dirname(node),join(toolchain,'node_modules/.bin'),join(home,'.local/share/fluentci/bun/1.4.3/bin'),process.env.PATH].join(':')};
function command(program,args,cwd) {
  const result=spawnSync(program,args,{cwd,env,stdio:'inherit'});
  if(result.error)throw result.error;
  if(result.status!==0)throw Error(`${program} failed (${result.status})`);
}
const temporary=mkdtempSync(join(tmpdir(),'tunein-sdk-'));
try {
  command('fluentci',['init','seed','--sdk','purescript','--runtime','component'],temporary);
  cpSync(join(temporary,'.fluentci/seed/.sdk'),join(root,'.sdk'),{recursive:true});
} finally {rmSync(temporary,{recursive:true,force:true});}
command(node,[join(toolchain,'node_modules/npm/bin/npm-cli.js'),'ci','--no-audit','--no-fund'],root);
command('fluentci',['run','--work-dir',resolve(root,'../..'),'ci','prepare'],resolve(root,'../..'));
