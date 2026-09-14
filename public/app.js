const themeButtons=document.querySelectorAll('[data-theme-toggle]');
function updateThemeControls(){const dark=document.documentElement.dataset.theme==='dark';themeButtons.forEach(button=>{button.setAttribute('aria-label',dark?'Switch to light mode':'Switch to dark mode');button.querySelector('[data-theme-label]').textContent=dark?'Light mode':'Dark mode';});}
themeButtons.forEach(button=>button.addEventListener('click',()=>{const theme=document.documentElement.dataset.theme==='dark'?'light':'dark';document.documentElement.dataset.theme=theme;try{localStorage.setItem('folio-theme',theme);}catch{}updateThemeControls();}));
matchMedia('(prefers-color-scheme: dark)').addEventListener('change',event=>{let chosen;try{chosen=localStorage.getItem('folio-theme');}catch{}if(!chosen){document.documentElement.dataset.theme=event.matches?'dark':'light';updateThemeControls();}});
updateThemeControls();
if(location.hash==='#quick-note')document.querySelector('#quick-note textarea')?.focus();
const dialog=document.querySelector('#search-dialog');
document.querySelectorAll('[data-search]').forEach(button=>button.addEventListener('click',()=>dialog.showModal()));
document.querySelector('[data-close]')?.addEventListener('click',()=>dialog.close());
const menu=document.querySelector('[data-menu]');
menu?.addEventListener('click',()=>{const open=document.querySelector('#navigation').classList.toggle('open');menu.setAttribute('aria-expanded',String(open));});
document.addEventListener('keydown',event=>{if(event.key==='Escape'){document.querySelector('#navigation').classList.remove('open');menu?.setAttribute('aria-expanded','false');}if(!(event.ctrlKey||event.metaKey))return;
 if(event.key.toLowerCase()==='k'){event.preventDefault();dialog.showModal();}
 if(event.key.toLowerCase()==='n'){event.preventDefault();if(event.shiftKey){if(location.pathname==='/')document.querySelector('textarea[name=body]')?.focus();else location.href='/#quick-note';}else document.querySelector('form[action="/notes/new"]')?.requestSubmit();}
});
const editor=document.querySelector('#editor-form');
if(editor){
 const status=document.querySelector('#save-status'),body=editor.elements.body,title=editor.elements.title,notebook=editor.elements.notebook;
 const key=`folio-draft:${editor.action}`;let dirty=false,timer,saving=null,revision=0;
 const snapshot=()=>({title:title.value,body:body.value,notebook:notebook.value});
 const draft=(()=>{try{return JSON.parse(localStorage.getItem(key));}catch{return null;}})();
 if(draft&&typeof draft.body==='string'&&typeof draft.title==='string'){title.value=draft.title;body.value=draft.body;notebook.value=draft.notebook;dirty=true;revision++;status.textContent='Recovered unsaved draft';}
 function changed(){dirty=true;revision++;status.classList.remove('error');status.textContent='Unsaved changes';try{localStorage.setItem(key,JSON.stringify(snapshot()));}catch{status.textContent='Unsaved · draft backup unavailable';}document.querySelector('#word-count').textContent=`${body.value.trim().split(/\s+/).filter(Boolean).length} words · Markdown`;clearTimeout(timer);timer=setTimeout(save,3000);}
 async function save(){clearTimeout(timer);if(saving){await saving;if(dirty)return save();return !dirty;}if(!dirty)return true;const savingRevision=revision;status.textContent='Saving…';
 saving=(async()=>{try{const response=await fetch(editor.action,{method:'POST',headers:{Accept:'application/json'},body:new URLSearchParams(new FormData(editor))});if(!response.ok)throw new Error('Save failed');if(savingRevision===revision){dirty=false;try{localStorage.removeItem(key);}catch{}status.textContent='Saved locally';status.classList.remove('error');}else{status.textContent='Unsaved changes';timer=setTimeout(save,3000);}return true;}catch{status.textContent='Save failed · retry with ⌘ S';status.classList.add('error');return false;}finally{saving=null;}})();return saving;
 }
 editor.addEventListener('input',changed);notebook.addEventListener('change',changed);
 editor.addEventListener('submit',event=>{event.preventDefault();save();});
 editor.addEventListener('focusout',event=>{if(!editor.contains(event.relatedTarget))save();});
 document.addEventListener('keydown',event=>{if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='s'){event.preventDefault();save();}});
 document.addEventListener('click',async event=>{const link=event.target.closest('a[href]');if(!link||!dirty||event.ctrlKey||event.metaKey||event.shiftKey||link.target==='_blank')return;event.preventDefault();if(await save())location.href=link.href;});
 document.querySelector('form[action="/notes/new"]')?.addEventListener('submit',async event=>{if(!dirty)return;event.preventDefault();if(await save())event.target.submit();});
 window.addEventListener('beforeunload',event=>{if(dirty){event.preventDefault();event.returnValue='';}});
 document.addEventListener('visibilitychange',()=>{if(document.visibilityState==='hidden'&&dirty)save();});
}
