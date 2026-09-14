const f = storage.folio;
if (penpot.currentPage.name !== '02 — Screens') throw Error('Activate Screens first');
f.screenBoards = [];
for (const [theme, x] of [['light', 0], ['dark', 1440]]) {
  const home = f.screen('Home', x, 0, theme, 1470);
  f.screenBoards.push(home);
  f.text(home, 'Date', 330, 111, 'MONDAY, SEPTEMBER 14', 10, 'terracotta', theme, 'JetBrains Mono', 500);
  f.rect(home, 'Date rule', 330, 145, 786, 1, 'line', theme);
  f.text(home, 'Greeting', 330, 175, 'Good afternoon.', 44, 'ink', theme, 'Newsreader', 700);
  f.text(home, 'Epigraph', 330, 234, 'Make room for what matters.', 20, 'muted', theme, 'Newsreader', 600);
  f.text(home, 'Goals heading', 330, 305, 'WHAT MATTERS · LONG-TERM GOALS', 10, 'muted', theme, 'JetBrains Mono', 500);
  for (const [i,title] of ['Grow as an engineer','Build a personal notebook','Read, reflect, repeat'].entries()) f.instance(home,'Content/Goal',330+i*262,340,theme,{'Grow as an engineer':title,'DIRECTION 01':'DIRECTION 0'+(i+1)});
  f.text(home,'Task heading',330,596,"Today's intentions",25,'ink',theme,'Newsreader',500);
  for(const [i,text] of ['Review the authentication architecture','Refine the Paper & Folio design system','Morning pages & a slow coffee','Write the first page of the notebook','A walk, with room to think'].entries())f.row(home,330,645+i*56,text,theme,i===2,['09:00','11:30','07:30','14:00','16:30'][i]);
  f.instance(home,'Task/Add',330,925,theme);
  f.instance(home,'Content/QuickNote',330,1010,theme);
  f.text(home,'Recent heading',330,1242,'Recently edited',25,'ink',theme,'Newsreader',500);
  f.instance(home,'Content/Note',330,1290,theme);
  f.instance(home,'Content/Note',728,1290,theme,{'A quieter kind of notebook':'Notes from the margins'});
  const todo=f.screen('Todo',x,1650,theme,1040);f.screenBoards.push(todo);
  f.text(todo,'Eyebrow',330,115,'ONE THING AT A TIME',10,'terracotta',theme,'JetBrains Mono',500);
  f.text(todo,'Title',330,151,'Tasks & intentions',44,'ink',theme,'Newsreader',750);
  f.text(todo,'Description',330,219,'A place for everything on your mind. A little less to carry.',14,'muted',theme,'Geist',780);
  f.text(todo,'Task groups',330,291,'Today     Upcoming     Someday     Completed',14,'muted',theme,'Geist',780);
  f.rect(todo,'Active tab',330,327,54,2,'sage',theme);
  for(const [i,title] of ['Review the authentication architecture','Refine the Paper & Folio design system','Write the first page of the notebook','A walk, with room to think'].entries())f.row(todo,330,371+i*56,title,theme,false,['09:00','11:30','14:00','16:30'][i]);
  f.instance(todo,'Task/Add',330,595,theme);
  f.text(todo,'Completed heading',330,708,'A little progress.',25,'ink',theme,'Newsreader',600);
  f.row(todo,330,761,'Morning pages & a slow coffee',theme,true,'07:30');
  const planner=f.screen('Planner',x,2850,theme,1060);f.screenBoards.push(planner);
  f.text(planner,'Eyebrow',280,111,'A LITTLE PERSPECTIVE',10,'terracotta',theme,'JetBrains Mono',750);
  f.text(planner,'Title',280,151,'The weekly horizon.',44,'ink',theme,'Newsreader',900);
  f.text(planner,'Week',280,226,'September 14 – 20, 2026',13,'muted',theme,'JetBrains Mono',600);
  f.instance(planner,'Navigation/ViewSwitch',1055,214,theme,{'Daily':'Weekly','Weekly':'Daily'});
  for(const [i,day] of ['MON','TUE','WED','THU','FRI','SAT','SUN'].entries()){
    const dayBoard=f.instance(planner,'Planner/Day',280+i*136,291,theme,{MON:day,'14':String(14+i),'Review the':['Review the','Read and','Write a','A walk','Finish the','A quiet','Reflect on'][i],architecture:['architecture','take notes','new page','outside','small things','morning','the week'][i]});
    dayBoard.resize(128,350);
    for(const child of penpotUtils.findShapes(s=>s.type==='rectangle'&&s.width>140,dayBoard))child.resize(127,child.height);
    for(const child of penpotUtils.findShapes(s=>s.type==='text'&&s.characters===String(14+i),dayBoard))child.x=dayBoard.x+98;
  }
  f.text(planner,'Backlog heading',280,714,'Not yet scheduled',25,'ink',theme,'Newsreader',700);
  f.row(planner,280,770,'Organize the reading list',theme,false,'');
  const notes=f.screen('Notes',x,4100,theme,1240);f.screenBoards.push(notes);
  f.rect(notes,'Folder note list',240,68,254,1172,'well',theme);
  f.text(notes,'Note list heading',263,97,'All notes',23,'ink',theme,'Newsreader',210);
  f.instance(notes,'Input/Search',258,147,theme);
  for(const [i,title] of ['A quieter kind of notebook','A new page','Notes from the margins','Storage & small boundaries'].entries()){
    f.rect(notes,'Note list row '+i,254,207+i*116,226,105,i===0?'sheet':'well',theme,4);
    f.text(notes,'Note title '+i,270,222+i*116,title,17,'ink',theme,'Newsreader',196);
    f.text(notes,'Preview '+i,270,255+i*116,'A thought worth returning to.',11,'muted',theme,'Geist',196);
    f.text(notes,'Path '+i,270,284+i*116,'Projects · Sep 14',9,'muted',theme,'JetBrains Mono',196);
  }
  f.text(notes,'Editor path',534,102,'Projects / a-quieter-kind-of-notebook.md',9,'muted',theme,'JetBrains Mono',650);
  f.instance(notes,'Navigation/ViewSwitch',1065,89,theme,{'Daily':'Edit','Weekly':'Preview'});
  f.text(notes,'Note title',534,163,'A quieter kind of notebook',37,'ink',theme,'Newsreader',670);
  f.text(notes,'Metadata',534,229,'PROJECTS  ·  SAVED LOCALLY  ·  MARKDOWN',9,'muted',theme,'JetBrains Mono',650);
  f.text(notes,'Section title',534,285,'The underlying idea',26,'ink',theme,'Newsreader',650);
  f.text(notes,'Prose',534,334,'Our tools shape our attention. This notebook should feel like opening a clean page: enough structure to begin, enough space to think.',15,'ink',theme,'Geist',638);
  f.rect(notes,'Quote well',534,429,638,85,'well',theme,4);
  f.rect(notes,'Quote rule',534,429,2,85,'sage',theme);
  f.text(notes,'Quote',555,449,'Simplicity is about making room for what matters.',22,'muted',theme,'Newsreader',590);
  f.text(notes,'Checklist heading',534,556,'Principles to keep close',26,'ink',theme,'Newsreader',600);
  f.text(notes,'Checklist',534,609,'☐  Markdown is the source of truth.\n☑  Keep the reading experience calm.\n☐  Make everyday actions feel effortless.',15,'ink',theme,'Geist',638);
  f.text(notes,'Code heading',534,727,'Small, clear boundaries',26,'ink',theme,'Newsreader',600);
  f.rect(notes,'Code surface',534,780,638,133,theme==='dark'?'#0F0E0D':'#242424',theme,4);
  f.text(notes,'Code',554,802,'trait NoteStorage {\n    async fn get(&self, path: &str)\n        -> Result<String, Error>;\n}',13,'#EDE8E1',theme,'JetBrains Mono',585);
  f.text(notes,'Table heading',534,957,'Storage responsibilities',26,'ink',theme,'Newsreader',600);
  for(const [i,row] of ['LAYER                    RESPONSIBILITY','R2                       Canonical Markdown','D1                       Metadata and search'].entries()){f.rect(notes,'Table rule '+i,534,1046+i*42,638,1,'line',theme);f.text(notes,'Table row '+i,546,1018+i*42,row,12,'muted',theme,'JetBrains Mono',610);}
}
return {boards:f.screenBoards.map(b=>({id:b.id,name:b.name})),count:f.screenBoards.length};
