const $ = id => document.getElementById(id);
let state, selected, currentFile, revision, original = '', tab = 'overview', polling;
const icons = () => window.lucide?.createIcons();
const node = (tag, text, attrs = {}) => Object.assign(document.createElement(tag), {textContent:text ?? '', ...attrs});
async function api(path, data) {const response = await fetch(`/api/${path}`, data === undefined ? {} : {method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(data)}); const result = await response.json(); if (!response.ok) throw new Error(result.error); return result;}
const guard = callback => async event => {try {await callback(event); } catch(error) {$('status').textContent = error.message;}};
function showTab(name) {tab=name; for(const button of document.querySelectorAll('[data-tab]')) button.setAttribute('aria-selected', String(button.dataset.tab===name)); for(const view of document.querySelectorAll('.view')) view.hidden=view.id!==name;}
async function refresh() {
  state = await api('state');
  $('job-count').textContent = state.jobs.length;
  const query = $('search').value.toLowerCase(); $('projects').replaceChildren();
  for (const item of state.projects.filter(item => item.name.toLowerCase().includes(query))) {const button=node('button',item.name); button.setAttribute('aria-current',String(item.id===selected)); button.onclick=guard(()=>select(item.id)); $('projects').append(button);}
  renderJobs();
  if (!selected && state.projects.length) await select(state.projects.at(-1).id);
  const active = state.jobs.some(job=>job.code===null); for(const id of ['run','check','graph']) $(id).disabled=!selected||active||!state.sdk.ready;
  clearTimeout(polling); if(active) polling=setTimeout(guard(refresh),700);
}
async function select(id) {
  if($('source').value!==original && !confirm('放弃未保存的脚本修改？')) return;
  selected=id; const item=state.projects.find(item=>item.id===id);
  $('project-name').textContent=item.name; $('project-path').textContent=item.path;
  const files=await api(`scripts?id=${id}`); $('scripts').replaceChildren(...files.map(file=>node('option',file,{value:file}))); $('file-list').replaceChildren();
  for(const file of files){const button=node('button',file);button.onclick=guard(async()=>{showTab('script');$('scripts').value=file;await loadScript();});$('file-list').append(button);}
  $('preview').src=`/thumbnail?id=${id}`; $('preview').hidden=false; $('preview').onerror=()=>{$('preview').hidden=true;};
  document.querySelector('[name=output]').value=`${item.path}-build`;
  await loadScript(); await refresh();
}
async function loadScript() {if(!selected)return;const file=$('scripts').value;if(!file)return;const data=await api(`script?id=${selected}&file=${encodeURIComponent(file)}`);currentFile=file;revision=data.revision;original=data.text;$('source').value=original;$('dirty').textContent='';}
function renderJobs(){
  $('jobs-list').replaceChildren();
  if(!state.jobs.length){$('jobs-list').append(node('p','暂无任务'));return;}
  for(const job of state.jobs){const row=node('article',null,{className:'job'}),header=node('div',null,{className:'job-header'});header.append(node('span',job.name,{className:'job-name'}),node('span',job.code===null?'进行中':job.code===0?'完成':`失败 (${job.code})`,{className:`job-state ${job.code!==null&&job.code!==0?'failed':''}`}));if(job.code===null){const stop=node('button','取消');stop.onclick=guard(async()=>{await api('cancel',{id:job.id});await refresh();});header.append(stop);}row.append(header,node('pre',job.log||'…'));$('jobs-list').append(row);}
}
async function task(action,output){if(!selected)throw new Error('请先选择项目');await api('task',{id:selected,action,output});showTab('jobs');await refresh();}
function field(name,label,value='',type='text'){const wrapper=node('label',label),input=node('input',null,{name,value,type,required:true});wrapper.append(input);return wrapper;}
function dialog(title,fields,submit){$('dialog-title').textContent=title;$('dialog-fields').replaceChildren(...fields);$('dialog-error').textContent='';$('dialog-form').onsubmit=async event=>{event.preventDefault();$('dialog-submit').disabled=true;try{await submit(Object.fromEntries(new FormData(event.target)));$('dialog').close();await refresh();}catch(error){$('dialog-error').textContent=error.message;}finally{$('dialog-submit').disabled=false;}};$('dialog').showModal();icons();}
$('new').onclick=()=>{const template=node('label','模板'),select=node('select',null,{name:'template'});select.append(node('option','标准剧情',{value:'story'}),node('option','数据交互',{value:'inventory'}));template.append(select);dialog('新建项目',[field('title','项目名称'),field('projectId','项目 ID','org.renrs.my-story'),field('path','新项目目录'),template],async data=>{await api('create',data);showTab('jobs');});};
$('register').onclick=()=>dialog('添加已有项目',[field('path','项目目录')],async data=>{const item=await api('register',data);selected=undefined;state=await api('state');await select(item.id);});
$('sdk-button').onclick=()=>{const tools=node('div',null,{className:'sdk-tools'});for(const [name,ready]of Object.entries(state.sdk.tools))tools.append(node('span',`${ready?'✓':'×'} ${name}`,{className:ready?'':'missing'}));dialog('SDK 设置',[field('path','SDK 目录',state.sdk.path),tools],data=>api('sdk',data));};
$('dismiss').onclick=()=>$('dialog').close();$('search').oninput=guard(refresh);
$('run').onclick=guard(()=>task('run'));$('check').onclick=guard(()=>task('check'));$('graph').onclick=guard(()=>task('graph'));
$('remove').onclick=guard(async()=>{if(!selected)return;await api('remove',{id:selected});selected=undefined;location.reload();});
$('scripts').onchange=guard(async()=>{if($('source').value!==original&&!confirm('放弃未保存的脚本修改？')){$('scripts').value=currentFile;return;}await loadScript();});
$('reload-script').onclick=guard(async()=>{if($('source').value===original||confirm('放弃未保存的脚本修改？'))await loadScript();});
$('source').oninput=()=>{$('dirty').textContent=$('source').value===original?'':'未保存';};
$('save-script').onclick=guard(async()=>{if(!selected||!currentFile)return;const data=await api('script',{id:selected,file:currentFile,text:$('source').value,revision});revision=data.revision;original=$('source').value;$('dirty').textContent='已保存';await task('check');});
$('build-form').onsubmit=guard(async event=>{event.preventDefault();const data=Object.fromEntries(new FormData(event.target));await task(data.action,data.output);});
for(const button of document.querySelectorAll('[data-tab]'))button.onclick=()=>showTab(button.dataset.tab);
window.addEventListener('beforeunload',event=>{if($('source').value!==original){event.preventDefault();event.returnValue='';}});
await guard(refresh)();icons();
