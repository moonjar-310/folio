// Development-only SSR adapter. This is not the production Rust/Cloudflare backend.
import http from 'node:http';
import {readFile,mkdir,writeFile} from 'node:fs/promises';
import {randomUUID} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {dirname,resolve} from 'node:path';
import {home,todo,planner,notes,settings,goalForm,designSystem,dateKey} from '../ui/pages.mjs';
import {layout,escape} from '../ui/components.mjs';
import {seed} from './seed.mjs';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..'),store=resolve(root,'.local/preview.json');
await mkdir(resolve(root,'.local'),{recursive:true});
let state;try{state=JSON.parse(await readFile(store,'utf8'));}catch(error){if(error.code!=='ENOENT')throw error;state=seed();await writeFile(store,JSON.stringify(state,null,2));}
const port=Number(process.env.PORT||4173),host='127.0.0.1';let mutationQueue=Promise.resolve();
const redirect=(res,path)=>{res.writeHead(303,{Location:path.startsWith('/')&&!path.startsWith('//')?path:'/'});res.end();};
const text=(res,status,body,type='text/html; charset=utf-8')=>{res.writeHead(status,{'Content-Type':type,'X-Content-Type-Options':'nosniff','Cache-Control':'no-store'});res.end(body);};
async function form(req){let data='';for await(const part of req){data+=part;if(Buffer.byteLength(data)>150000)throw new Error('Request too large');}return new URLSearchParams(data);}
async function mutate(req,res,url){
 const origin=req.headers.origin;if(origin&&origin!==`http://${host}:${port}`&&origin!==`http://localhost:${port}`)return text(res,403,'Origin rejected.');
 const data=await form(req),before=structuredClone(state);let destination=data.get('returnTo')||'/';
 try {
 if(url.pathname==='/tasks'){const title=data.get('title')?.trim();if(!title||title.length>240)throw new Error('Enter a task title under 240 characters.');state.tasks.push({id:randomUUID(),title,date:data.get('date')||'',time:'',completed:false});}
 else if(/^\/tasks\/[^/]+\/(toggle|date)$/.test(url.pathname)){const task=state.tasks.find(t=>t.id===url.pathname.split('/')[2]);if(!task)throw new Error('Task not found.');if(url.pathname.endsWith('/toggle'))task.completed=!task.completed;else task.date=data.get('date')||'';}
 else if(url.pathname==='/notes/new'||url.pathname==='/notes/quick'){const body=data.get('body')||'',note={id:randomUUID(),title:body.trim().split('\n')[0].slice(0,80)||'Untitled note',body,notebook:'Personal',updated:new Date().toISOString()};state.notes.unshift(note);destination=`/notes?id=${note.id}`;}
 else if(/^\/notes\/[^/]+\/save$/.test(url.pathname)){const note=state.notes.find(n=>n.id===url.pathname.split('/')[2]);if(!note)throw new Error('Note not found.');const title=data.get('title')?.trim();if(!title||title.length>240)throw new Error('Enter a note title under 240 characters.');const notebook=data.get('notebook');if(!['Daily','Projects','Personal','Archive'].includes(notebook))throw new Error('Invalid notebook.');Object.assign(note,{title,body:data.get('body')||'',notebook,updated:new Date().toISOString()});destination=`/notes?id=${note.id}`;}
 else if(url.pathname==='/goals'){const title=data.get('title')?.trim();if(!title||title.length>240)throw new Error('Enter a goal title under 240 characters.');state.goals.push({id:randomUUID(),title,description:data.get('description')||'',status:'active'});}
 else if(/^\/goals\/[^/]+\/toggle$/.test(url.pathname)){const goal=state.goals.find(g=>g.id===url.pathname.split('/')[2]);if(!goal)throw new Error('Goal not found.');goal.status=goal.status==='active'?'completed':'active';}
 else return text(res,404,'Not found.');
 await writeFile(store,JSON.stringify(state,null,2));
 if(req.headers.accept==='application/json')text(res,200,JSON.stringify({saved:true}),'application/json');else redirect(res,destination);
 }catch(error){state=before;throw error;}
}
const server=http.createServer(async(req,res)=>{try{
 const url=new URL(req.url,`http://${host}:${port}`);
 if(req.method==='POST'){const operation=mutationQueue.then(()=>mutate(req,res,url));mutationQueue=operation.catch(()=>{});await operation;return;}
 if(req.method!=='GET')return text(res,405,'Method not allowed.');
 const assets={'/tokens.css':['public/tokens.css','text/css'],'/app.css':['public/app.css','text/css'],'/app.js':['public/app.js','text/javascript'],'/theme.js':['public/theme.js','text/javascript']};
 if(assets[url.pathname]){const [path,type]=assets[url.pathname];return text(res,200,await readFile(resolve(root,path)),type);}
 if(url.pathname==='/favicon.ico'){res.writeHead(204);return res.end();}
 if(url.pathname==='/health')return text(res,200,'ok','text/plain');
 if(url.pathname==='/export'){res.setHeader('Content-Disposition','attachment; filename="folio-notes.json"');return text(res,200,JSON.stringify(state.notes,null,2),'application/json');}
 if(url.pathname==='/daily'){let note=state.notes.find(n=>n.notebook==='Daily'&&n.title===dateKey());if(!note){note={id:randomUUID(),title:dateKey(),notebook:'Daily',body:'# '+dateKey()+'\n\n',updated:new Date().toISOString()};state.notes.unshift(note);await writeFile(store,JSON.stringify(state,null,2));}return redirect(res,`/notes?id=${note.id}`);}
 const routes={'/':()=>home(state),'/todo':()=>todo(state,url),'/planner':()=>planner(state,url),'/notes':()=>notes(state,url),'/settings':settings,'/goals/new':goalForm,'/design-system':()=>designSystem(state)};
 if(!routes[url.pathname])return text(res,404,layout({title:'Page not found',content:'<h1>This page is still blank.</h1><p><a href="/">Return home</a></p>'}));
 text(res,200,routes[url.pathname]());
 }catch(error){console.error(error.message);text(res,400,req.headers.accept==='application/json'?JSON.stringify({error:error.message}):layout({title:'Could not save',content:`<h1>Your change could not be saved.</h1><p>${escape(error.message)}</p><p>Go back to keep editing.</p>`}),req.headers.accept==='application/json'?'application/json':'text/html; charset=utf-8');}});
server.listen(port,host,()=>console.log(`Folio UI preview: http://${host}:${port}`));
