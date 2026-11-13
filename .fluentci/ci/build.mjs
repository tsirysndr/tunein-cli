import {buildPlugin} from '@fluentci/inspection';
import {build} from 'esbuild';
import {mkdirSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
const result = spawnSync('spago',['build'],{stdio:'inherit'});
if(result.error)throw result.error;
if(result.status!==0)throw Error(`spago failed (${result.status})`);
mkdirSync('dist',{recursive:true});
// The BSD guest has the same PureScript policy and only minimal Node host bindings.
await build({stdin:{contents:"import {main} from './output/CI.BsdGuest/index.js'; main();",resolveDir:process.cwd(),loader:'js'},bundle:true,platform:'node',format:'cjs',target:'node22',outfile:'dist/bsd-guest.cjs'});
await buildPlugin({sdk:'purescript',runtime:'component'});
