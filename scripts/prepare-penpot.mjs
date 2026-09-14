import {readFile,writeFile,mkdir} from 'node:fs/promises';
const base=new URL('../design/penpot/',import.meta.url),manifest=JSON.parse(await readFile(new URL('manifest.json',base),'utf8'));
const decode=s=>s.replaceAll('&amp;','&').replaceAll('&lt;','<').replaceAll('&gt;','>').replaceAll('&quot;','"').replaceAll('&#39;',"'");
const items=[];
for(const component of manifest.components){const source=await readFile(new URL(component.file,base),'utf8');const shapes=[...source.matchAll(/<(rect|text|circle|path)\b([^>]*?)(?:\/>|>([\s\S]*?)<\/\1>)/g)].map(([,type,attrs,text])=>({type,attrs:Object.fromEntries([...attrs.matchAll(/([\w-]+)="([^"]*)"/g)].map(([,k,v])=>[k,v])),text:decode(text||'')}));items.push({...component,shapes});}
const code=`const f=storage.folio; const items=${JSON.stringify(items)};
f.register=(board,name,theme)=>{const c=penpot.library.local.createComponent([board]);c.name=name.split('/').at(-1);c.path='Folio/'+theme+'/'+name.split('/').slice(0,-1).join('/');f.components[theme+'/'+name]=c;return c;};
f.instance=(parent,name,x,y,theme='light',overrides={})=>{const s=f.components[theme+'/'+name].instance();parent.appendChild(s);s.x=parent.x+x;s.y=parent.y+y;for(const child of penpotUtils.findShapes(sh=>sh.type==='text',s)){if(overrides[child.characters]!==undefined)child.characters=overrides[child.characters];}return s;};
let y=100;
for(const item of items){for(const [theme,x] of [['light',0],['dark',850]]){const board=f.board(item.name+' / '+theme,x,y,item.width,item.height,f.pages.components.root,theme,null);
for(const [i,shape] of item.shapes.entries()){const a=shape.attrs;const map=(color)=>{const v=(color||'#242424').toUpperCase();const key=Object.entries(f.colors.light).find(([,c])=>c===v)?.[0];return key||v;};
if(shape.type==='rect'){f.rect(board,'Surface '+i,+a.x,+a.y,+a.width,+a.height,map(a.fill),theme,+(a.rx||0),a.stroke?map(a.stroke):undefined);}
if(shape.type==='text'){const size=+(a['font-size']||14);const color=a.fill==='#fff'?(theme==='dark'?'#18281E':'#FFFFFF'):map(a.fill);f.text(board,'Label '+i,+a.x,+a.y-size,shape.text,size,color,theme,a['font-family']||'Geist',Math.max(20,item.width-(+a.x)-8));}
if(shape.type==='circle'){const s=penpot.createEllipse();s.name='Status marker';s.resize(+a.r*2,+a.r*2);s.x=board.x+(+a.cx)-(+a.r);s.y=board.y+(+a.cy)-(+a.r);s.fills=[{fillColor:f.colors[theme][map(a.fill)]||a.fill.toUpperCase(),fillOpacity:1}];board.appendChild(s);}
if(shape.type==='path'){const s=penpot.createPath();s.name='Rule';s.d=a.d;s.fills=[];s.strokes=[{strokeColor:f.colors[theme][map(a.stroke)]||a.stroke.toUpperCase(),strokeWidth:1,strokeStyle:'solid'}];s.x+=board.x;s.y+=board.y;board.appendChild(s);}
}f.register(board,item.name,theme);}y+=item.height+74;}
return {components:penpot.library.local.components.map(c=>({name:c.name,path:c.path})),count:penpot.library.local.components.length};`;
await mkdir(new URL('../.local/',import.meta.url),{recursive:true});await writeFile(new URL('../.local/penpot-components.json',import.meta.url),JSON.stringify({code}));console.log('Prepared native component payload.');
