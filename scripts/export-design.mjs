// Editable SVG handoff assets; native Penpot component registration needs a live Penpot connection.
import {mkdir,writeFile} from 'node:fs/promises';
import {escape} from '../ui/components.mjs';
const output=new URL('../design/penpot/',import.meta.url);await mkdir(output,{recursive:true});
const colors={paper:'#fcf9f8',sheet:'#ffffff',well:'#f6f3f2',ink:'#242424',muted:'#716e68',line:'#e5dfda',sage:'#476552',terracotta:'#8c4a2f'};
const r=(x,y,w,h,fill=colors.sheet,stroke=colors.line,rx=6)=>`<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${rx}" fill="${fill}" stroke="${stroke}"/>`;
const t=(x,y,text,size=14,fill=colors.ink,font='Geist')=>`<text x="${x}" y="${y}" font-family="${font}" font-size="${size}" fill="${fill}">${escape(text)}</text>`;
const line=(x,y,w)=>`<path d="M${x} ${y}h${w}" stroke="${colors.line}"/>`;
const svg=(name,w,h,body)=>`<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}"><title>${escape(name)}</title>${body}</svg>`;
const components=[
 ['Action/Primary',150,40,r(0.5,0.5,149,39,colors.sage,colors.sage)+t(27,25,'Create note',13,'#fff')],
 ['Action/Secondary',120,40,r(.5,.5,119,39)+t(35,25,'Cancel',13)],
 ['Action/Text',120,32,t(0,21,'All notes ↗',12,colors.sage)],
 ['Action/Disabled',150,40,r(.5,.5,149,39,colors.well)+t(39,25,'Saving…',13,colors.muted)],
 ['Navigation/Item',216,40,r(0,0,216,40,colors.well,colors.well,4)+t(38,26,'Home',13,colors.muted)],
 ['Navigation/Selected',216,40,r(0,0,216,40,'#edf1ed','#edf1ed',4)+r(0,0,2,40,colors.sage,colors.sage,0)+t(38,26,'Home',13,colors.sage)],
 ['Input/Search',216,38,r(.5,.5,215,37)+t(12,24,'Search your notes',11,colors.muted)+r(178,10,28,18,colors.sheet,colors.line,3)+t(182,22,'⌘ K',9,colors.muted,'JetBrains Mono')],
 ['Task/Open',600,56,r(0,0,600,56,colors.sheet,colors.sheet,0)+r(14,20,16,16,colors.sheet,'#9a9c92',3)+t(44,33,'Review the authentication architecture',13)+t(527,33,'09:00',10,colors.muted,'JetBrains Mono')+line(0,55.5,600)],
 ['Task/Completed',600,56,r(0,0,600,56,colors.sheet,colors.sheet,0)+r(14,20,16,16,colors.sage,colors.sage,3)+t(16,32,'✓',13,'#fff')+t(44,33,'Morning pages & a slow coffee',13,colors.muted)+`<path d="M44 28h191" stroke="${colors.muted}"/>`+t(527,33,'07:30',10,colors.muted,'JetBrains Mono')+line(0,55.5,600)],
 ['Task/Add',600,48,r(0,0,600,48,colors.paper,colors.paper,0)+t(14,30,'＋',16,colors.muted)+t(44,29,'Add an intention…',12,colors.muted)+t(546,29,'Add ↵',10,colors.sage,'JetBrains Mono')+line(0,47.5,600)],
 ['Content/Goal',250,212,r(.5,.5,249,211)+t(18,26,'DIRECTION 01',9,colors.terracotta,'JetBrains Mono')+`<circle cx="227" cy="23" r="3" fill="${colors.sage}"/>`+t(18,63,'Grow as an engineer',21,colors.ink,'Newsreader')+t(18,93,'Make time for deeper understanding,',11,colors.muted)+t(18,113,'careful practice, and the work',11,colors.muted)+t(18,133,'that stretches me.',11,colors.muted)+line(18,173,214)+t(18,196,'ACTIVE',9,colors.muted,'JetBrains Mono')+t(178,196,'Complete ↗',9,colors.sage)],
 ['Content/Note',382,133,r(.5,.5,381,132)+t(18,32,'A quieter kind of notebook',19,colors.ink,'Newsreader')+t(18,59,'A personal space for clear thinking, daily intentions,',11,colors.muted)+t(18,77,'and ideas worth returning to.',11,colors.muted)+t(18,111,'Projects · Sep 14',9,colors.muted,'JetBrains Mono')+t(350,32,'↗',14,colors.muted)],
 ['Content/QuickNote',600,190,r(.5,.5,599,189)+t(18,33,'Quick note',23,colors.ink,'Newsreader')+r(18,50,564,78,colors.well,colors.well,4)+t(32,78,'Let a thought land here…',12,colors.muted)+t(18,166,'Saved as a new note in Personal',9,colors.muted,'JetBrains Mono')+r(447,143,135,32,colors.sage,colors.sage,6)+t(465,164,'Create note →',12,'#fff')],
 ['Navigation/ViewSwitch',148,36,r(.5,.5,147,35,colors.well)+r(4,4,66,28,colors.sheet,colors.sheet,4)+t(20,23,'Daily',11,colors.sage)+t(86,23,'Weekly',11,colors.muted)],
 ['Status/Saved',160,25,`<circle cx="5" cy="12" r="3" fill="${colors.sage}"/>`+t(16,16,'Saved locally',10,colors.muted,'JetBrains Mono')],
 ['Status/SaveFailed',250,25,t(0,16,'Save failed · retry with ⌘ S',10,'#ba1a1a','JetBrains Mono')],
 ['Planner/Day',152,290,r(.5,.5,151,289,'#edf1ed')+t(13,27,'MON',10,colors.muted,'JetBrains Mono')+t(119,29,'14',17,colors.sage,'JetBrains Mono')+r(13,58,13,13,colors.sheet,'#9a9c92',3)+t(35,68,'Review the',11)+t(35,86,'architecture',11)+t(35,109,'09:00',9,colors.muted,'JetBrains Mono')+line(13,125,126)+t(27,154,'＋ Add intention',9,colors.sage,'JetBrains Mono')],
 ['Overlay/Search',560,172,r(.5,.5,559,171,colors.paper,colors.ink,8)+t(24,41,'Find a thought.',27,colors.ink,'Newsreader')+t(525,40,'×',24)+t(24,92,'Search titles and words…',14,colors.muted)+r(443,67,91,39,colors.sage,colors.sage)+t(466,93,'Search',12,'#fff')+t(24,145,'Search your notes by title or content. Esc to close.',10,colors.muted,'JetBrains Mono')]
];
let sheet=r(0,0,1600,2450,colors.paper,colors.paper,0)+t(64,74,'Folio',48,colors.ink,'Newsreader')+t(64,111,'FOUNDATIONS & REUSABLE COMPONENTS / STITCH → PENPOT',12,colors.muted,'JetBrains Mono');
Object.entries(colors).forEach(([name,color],i)=>{const x=64+i*186;sheet+=r(x,151,164,80,color,colors.line)+t(x,255,name,12)+t(x,275,color,10,colors.muted,'JetBrains Mono');});
sheet+=t(64,347,'Room to think.',40,colors.ink,'Newsreader')+t(650,325,'Geist · Interface & prose',17)+t(650,358,'JetBrains Mono · METADATA',12,colors.muted,'JetBrains Mono');
let y=420;for(let i=0;i<components.length;i++){const [name,w,h,body]=components[i],x=i%2?840:64;if(i%2===0&&i>0)y+=Math.max(components[i-2][2],components[i-1][2])+64;sheet+=`<g id="${name.replaceAll('/','-')}" transform="translate(${x} ${y})">${t(0,-15,name,12,colors.muted,'JetBrains Mono')}${body}</g>`;await writeFile(new URL(name.replaceAll('/','-').toLowerCase()+'.svg',output),svg(name,w,h,body));}
await writeFile(new URL('component-library.svg',output),svg('Folio component library',1600,y+340,sheet));
const tokens={global:{color:Object.fromEntries(Object.entries(colors).map(([key,value])=>[key,{$type:'color',$value:value}])),spacing:Object.fromEntries([4,8,12,16,24,32,40,48].map(n=>[String(n),{$type:'dimension',$value:`${n}px`}])),radius:{control:{$type:'borderRadius',$value:'4px'},container:{$type:'borderRadius',$value:'6px'},dialog:{$type:'borderRadius',$value:'8px'}},font:{heading:{$type:'fontFamily',$value:'Newsreader'},body:{$type:'fontFamily',$value:'Geist'},metadata:{$type:'fontFamily',$value:'JetBrains Mono'}}}};
await writeFile(new URL('tokens.json',output),JSON.stringify(tokens,null,2));
const dark={paper:'#141312',sheet:'#1c1b1a',well:'#211f1e',ink:'#ede8e1',muted:'#b9aaa3',line:'#403731',sage:'#a8c5b0',terracotta:'#d67754'};
let darkSheet=sheet;for(const [key,value] of Object.entries(colors))darkSheet=darkSheet.replaceAll(value,`COLOR_${key}`);for(const [key,value] of Object.entries(dark))darkSheet=darkSheet.replaceAll(`COLOR_${key}`,value);darkSheet=darkSheet.replaceAll('#fff','#18281e').replaceAll('#edf1ed','#26332a').replaceAll('#ba1a1a','#ffb4ab');
await writeFile(new URL('component-library-dark.svg',output),svg('Folio dark component library',1600,y+340,darkSheet));
tokens.light={color:Object.fromEntries(Object.entries(colors).map(([key,value])=>[key,{$type:'color',$value:value}]))};
tokens.dark={color:Object.fromEntries(Object.entries(dark).map(([key,value])=>[key,{$type:'color',$value:value}]))};
await writeFile(new URL('tokens.json',output),JSON.stringify(tokens,null,2));
await writeFile(new URL('manifest.json',output),JSON.stringify({status:'Editable SVG assets for native Penpot component registration.',themes:['light','dark'],navigationLabel:'Folders',components:components.map(([name,width,height])=>({name,width,height,file:name.replaceAll('/','-').toLowerCase()+'.svg'}))},null,2));
console.log(`Exported ${components.length} editable components, component sheet, and token JSON to design/penpot.`);
