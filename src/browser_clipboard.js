// Browser APIs cannot mark clipboard text private/exclude it from OS history.
// Do not claim cancellation after the item has been handed to the browser.
let active;
const error=code=>new Error(code);
export function clipboardSupported(){
  return isSecureContext && typeof navigator.clipboard?.write==='function' && typeof ClipboardItem==='function';
}
export function reserveClipboard(){
  if(!clipboardSupported())throw error('clipboard_unsupported');
  if(active)throw error('clipboard_busy');
  if(!navigator.userActivation?.isActive)throw error('clipboard_permission_denied');
  let deliver,abandon,resolve,reject;
  const content=new Promise((yes,no)=>{deliver=yes;abandon=no;});
  const result=new Promise((yes,no)=>{resolve=yes;reject=no;});
  // Attach handlers immediately: refusal can precede the Rust async consumer.
  content.catch(()=>{});result.catch(()=>{});
  const item={id:crypto.randomUUID(),deliver,abandon,result,resolve,reject,supplied:false,settled:false,cancelled:false};
  active=item;
  item.timer=setTimeout(()=>{
    item.cancelled=true;
    abandon(error('clipboard_unavailable'));reject(error('clipboard_unavailable'));
    if(item.settled&&active===item)active=undefined;
  },15000);
  try {
    navigator.clipboard.write([new ClipboardItem({'text/plain':content})]).then(resolve,reason=>{
      reject(error(reason?.name==='NotAllowedError'?'clipboard_permission_denied':'clipboard_unavailable'));
    }).finally(()=>{
      item.settled=true;
      if(item.supplied||item.cancelled){clearTimeout(item.timer);if(active===item)active=undefined;}
    });
  }catch{
    clearTimeout(item.timer);active=undefined;abandon(error('clipboard_unavailable'));
    throw error('clipboard_unavailable');
  }
  return item.id;
}
export function completeClipboard(id,text){
  const item=active;
  if(!item||item.id!==id||item.supplied||typeof text!=='string'||new TextEncoder().encode(text).length>8192||text.includes('\0'))throw error('clipboard_unavailable');
  item.supplied=true;
  if(item.settled){clearTimeout(item.timer);active=undefined;}
  else item.deliver(new Blob([text],{type:'text/plain'}));
  return item.result;
}
export function cancelClipboard(id){
  const item=active;
  if(item?.id===id&&!item.supplied){
    item.cancelled=true;
    item.abandon(error('clipboard_unavailable'));item.reject(error('clipboard_unavailable'));
    if(item.settled){clearTimeout(item.timer);active=undefined;}
  }
}
