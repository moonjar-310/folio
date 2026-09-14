const f=storage.folio;
if(penpot.currentPage.name!=='00 — Foundations')throw Error('Activate Foundations first');
for(const [theme,x] of [['light',0],['dark',1400]]){
  const board=f.board('Foundations / '+theme,x,0,1280,1100,penpot.root,theme);
  f.text(board,'Brand',64,55,'Paper & Folio',54,'ink',theme,'Newsreader',1100);
  f.text(board,'Description',64,135,'A shared language for quiet, intentional planning.',18,'muted',theme,'Geist',1100);
  f.text(board,'Color heading',64,217,'COLOR / '+theme.toUpperCase(),12,'terracotta',theme,'JetBrains Mono',1100);
  Object.entries(f.colors[theme]).forEach(([name,color],i)=>{const px=64+(i%5)*226,py=268+Math.floor(i/5)*151;f.rect(board,'Swatch / '+name,px,py,202,79,name,theme,6,'line');f.text(board,'Color name / '+name,px,py+90,name,13,'ink',theme,'Geist',202);f.text(board,'Color value / '+name,px,py+114,color,10,'muted',theme,'JetBrains Mono',202);});
  f.text(board,'Type heading',64,599,'TYPOGRAPHY',12,'terracotta',theme,'JetBrains Mono',1000);
  let y=653;for(const [name,sample] of [['Display','Room to think.'],['Heading','A thought worth keeping.'],['Body','Make room for what matters. Keep the words at the center.'],['Metadata','MONDAY, SEPTEMBER 14 · SAVED LOCALLY']]){const text=f.text(board,'Type / '+name,64,y,sample,14,'ink',theme,'Geist',1100);text.applyTypography(f.typos[name]);y+=name==='Display'?75:55;}
  f.text(board,'Spacing',64,933,'SPACING  4 · 8 · 12 · 16 · 24 · 32 · 40 · 48',11,'muted',theme,'JetBrains Mono',1100);
  f.text(board,'Geometry',64,978,'RADII  4 / 6 / 8 px     BORDERS  1 px     NAVIGATION  Folders',11,'muted',theme,'JetBrains Mono',1100);
}
return {boards:penpot.root.children.map(s=>({id:s.id,name:s.name}))};
