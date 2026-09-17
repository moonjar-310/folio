import assert from 'node:assert/strict';
const base=process.argv[2], phase=process.argv[3];
if(!/^http:\/\/(127\.0\.0\.1|localhost):\d+$/.test(base??''))throw new Error('Use a disposable loopback path-storage test server.');
if(!['prepare','verify-loss'].includes(phase))throw new Error('Expected prepare or verify-loss');
let cookie='',csrf='',access='';
async function call(method,path,body={}) {
 const r=await fetch(base+path,{method,signal:AbortSignal.timeout(30000),headers:{'content-type':'application/json',origin:base,authorization:'Bearer '+access,cookie,'x-csrf-token':csrf,...(process.env.FOLIO_TEST_ACCESS_TOKEN?{'Cf-Access-Jwt-Assertion':process.env.FOLIO_TEST_ACCESS_TOKEN}:{})},body:method==='GET'?undefined:JSON.stringify(body)});
 const data=await r.json();assert.equal(r.status,200,method+' '+path+': '+JSON.stringify(data));
 if(r.headers.get('set-cookie'))cookie=r.headers.get('set-cookie').split(';')[0];
 if(data?.access_token){access=data.access_token;csrf=data.csrf;}
 return data;
}
await call('POST','/api/auth/login',{username:'folio-test',password:'Folio-local-test-2026!'});
assert.ok(access);assert.ok(csrf);
await call('POST','/api/auth/refresh');
if(phase==='prepare') {
 await call('PUT','/api/notes/storage-recovery',{markdown:'# Recovery original\n\n- [ ] Portable checkbox\n',folder:'Recovery/Deep'});
 await call('POST','/api/folders',{path:'Recovery/Empty/Nested'});
 await call('POST','/api/tasks',{title:'Portable standalone task'});
 await call('POST','/api/goals',{title:'Portable goal'});
 console.log('Prepared canonical path and independent planner originals.');
} else if(phase==='verify-loss') {
 // Confirm the harness actually cleared derived rows before any read/edit repairs them.
 assert.equal((await call('GET','/api/notes?q=Recovery')).items.length,0);
 assert.equal((await call('GET','/api/tasks')).items.length,0);
 assert.equal((await call('GET','/api/goals')).length,0);
 const folders=await call('GET','/api/folders');assert.ok(folders.some(f=>f.path==='Recovery/Deep'));
 const list=await call('GET','/api/notes?folder=Recovery%2FDeep&exact=true');assert.equal(list.items.length,1);
 const note=await call('GET','/api/notes/'+list.items[0].id);assert.match(note.markdown,/Portable checkbox/);
 await call('PUT','/api/notes/'+note.id,{...note,markdown:note.markdown+'\nEdited without index.'});
 let input={};let done=false;let skipped=0;
 for(let i=0;i<200;i++) {const next=await call('POST','/api/storage/rebuild',input);skipped+=(next.warnings??[]).length;if(next.done){done=true;break;} input=next;}
 assert.ok(done);assert.equal(skipped,0);
 const tasks=await call('GET','/api/tasks');assert.ok(tasks.items.some(t=>t.title==='Portable standalone task'));assert.ok(tasks.items.some(t=>t.title==='Portable checkbox'));
 const goals=await call('GET','/api/goals');assert.ok(goals.some(g=>g.title==='Portable goal'));
 const search=await call('GET','/api/notes?q=Edited%20without%20index');assert.equal(search.items.length,1);
 let rename=await call('PUT','/api/folders',{path:'Recovery',name:'Recovered'});
 for(let i=0;!rename.done&&i<20;i++)rename=await call('PUT','/api/folders',{path:'Recovery',name:'Recovered'});
 assert.equal(rename.done,true);
 const moved=await call('GET','/api/notes?folder=Recovered%2FDeep&exact=true');assert.equal(moved.items.length,1);
 assert.match((await call('GET','/api/notes/'+moved.items[0].id)).markdown,/Edited without index/);
 assert.ok((await call('GET','/api/folders')).some(f=>f.path==='Recovered/Empty/Nested'));
 console.log('PASS: index loss -> raw discovery/open/edit -> search/checkbox/tasks/goals rebuild -> folder/file move.');
}
await call('POST','/api/auth/logout');
