import { escape } from './components.mjs';
function inline(text) {
  return escape(text).replace(/`([^`]+)`/g,'<code>$1</code>').replace(/\*\*([^*]+)\*\*/g,'<strong>$1</strong>').replace(/\*([^*]+)\*/g,'<em>$1</em>').replace(/\[([^\]]+)\]\(([^\s)]+)\)/g,(_,label,url)=> /^(https?:\/\/|\/[^/]|#)/i.test(url)?`<a href="${url}" rel="noreferrer">${label}</a>`:label);
}
export function markdown(source) {
  const lines = source.split('\n'); let output='',code=null,list=false;
  const closeList=()=>{if(list){output+='</ul>';list=false;}};
  for(let i=0;i<lines.length;i++) {
    const line=lines[i];
    if(line.startsWith('```')) {closeList();if(code!==null){output+=`<pre><code>${escape(code.join('\n'))}</code></pre>`;code=null;}else code=[];continue;}
    if(code!==null){code.push(line);continue;}
    if(/^[-*] /.test(line)){if(!list){output+='<ul>';list=true;}const m=line.match(/^[-*] \[([ xX])\] (.*)/);output+=m?`<li class="md-task"><span class="md-check">${m[1]===' '?'☐':'☑'}</span>${inline(m[2])}</li>`:`<li>${inline(line.slice(2))}</li>`;continue;}
    closeList();
    if(line.includes('|') && /^\s*\|?\s*:?-{3}/.test(lines[i+1]||'')) {
      const cells=s=>s.trim().replace(/^\||\|$/g,'').split('|').map(x=>x.trim());
      output+='<table><thead><tr>'+cells(line).map(x=>`<th>${inline(x)}</th>`).join('')+'</tr></thead><tbody>';i++;
      while(lines[i+1]?.includes('|')){i++;output+='<tr>'+cells(lines[i]).map(x=>`<td>${inline(x)}</td>`).join('')+'</tr>';}
      output+='</tbody></table>';continue;
    }
    const heading=line.match(/^(#{1,6}) (.+)/);
    if(heading){output+=`<h${heading[1].length}>${inline(heading[2])}</h${heading[1].length}>`;continue;}
    if(line.startsWith('> ')){output+=`<blockquote>${inline(line.slice(2))}</blockquote>`;continue;}
    if(line.trim())output+=`<p>${inline(line)}</p>`;
  }
  closeList();if(code!==null)output+=`<pre><code>${escape(code.join('\n'))}</code></pre>`;return output;
}
