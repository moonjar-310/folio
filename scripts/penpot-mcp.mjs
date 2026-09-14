// Small CLI bridge for this session; reads the registered endpoint without printing credentials.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {homedir} from 'node:os';
import {resolve} from 'node:path';
const config=await readFile(resolve(homedir(),'.codex/config.toml'),'utf8');
const section=config.match(/\[mcp_servers\.penpot\]([\s\S]*?)(?=\n\[|$)/)?.[1];
const endpoint=section?.match(/^url\s*=\s*"([^"]+)"/m)?.[1];
if(!endpoint)throw new Error('Penpot MCP is not registered.');
let session,id=0;
async function rpc(method,params,notify=false){
 const headers={'Content-Type':'application/json',Accept:'application/json, text/event-stream','MCP-Protocol-Version':'2024-11-05'};if(session)headers['Mcp-Session-Id']=session;
 const response=await fetch(endpoint,{method:'POST',headers,body:JSON.stringify({jsonrpc:'2.0',...(notify?{}:{id:++id}),method,params}),signal:AbortSignal.timeout(120000)});
 if(response.headers.get('mcp-session-id'))session=response.headers.get('mcp-session-id');
 if(!response.ok)throw new Error(`Penpot MCP HTTP ${response.status}`);
 const content=await response.text();if(!content)return {};
 let value;if(response.headers.get('content-type')?.includes('text/event-stream')){const messages=content.split('\n').filter(line=>line.startsWith('data:')).map(line=>JSON.parse(line.slice(5)));value=messages.find(message=>message.id===id)||messages.at(-1);}else value=JSON.parse(content);
 if(value?.error)throw new Error(JSON.stringify(value.error));return value?.result;
}
try{
 await rpc('initialize',{protocolVersion:'2024-11-05',capabilities:{},clientInfo:{name:'folio-design-handoff',version:'1.0.0'}});
 await rpc('notifications/initialized',{},true);
 const [name,argsFile,outFile]=process.argv.slice(2);
 const args=argsFile?(argsFile.endsWith('.js')?{code:await readFile(argsFile,'utf8')}:JSON.parse(await readFile(argsFile,'utf8'))):{};
 const result=name?await rpc('tools/call',{name,arguments:args}):await rpc('tools/list',{});
 if(outFile){await mkdir(resolve(outFile,'..'),{recursive:true});const image=result.content?.find(c=>c.type==='image');await writeFile(outFile,image&&outFile.endsWith('.png')?Buffer.from(image.data,'base64'):JSON.stringify(result,null,2));console.log(`Saved MCP result to ${outFile}`);}else console.log(JSON.stringify(result,null,2));
}catch(error){console.error(error.message.replaceAll(endpoint,'[redacted endpoint]'));process.exitCode=1;}
