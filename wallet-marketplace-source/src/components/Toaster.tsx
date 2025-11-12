import { useEffect, useState } from 'react'

type Toast = { id: number; text: string; type?: 'info'|'success'|'error' }

const subscribers: ((t: Toast) => void)[] = []
let lastId = 0

export function pushToast(text: string, type: Toast['type'] = 'info'){
  const t = { id: ++lastId, text, type }
  subscribers.forEach(s=>s(t))
}

// expose globally for non-React callers (simple bridge)
;(window as any).pushToast = pushToast
// Also expose on global window for typings
declare global {
  interface Window { pushToast?: (text:string, type?: 'info'|'success'|'error') => void }
}

export default function Toaster(){
  const [toasts, setToasts] = useState<Toast[]>([])

  useEffect(()=>{
    const sub = (t: Toast) => {
      setToasts(s=>[t, ...s])
      setTimeout(()=>{ setToasts(s=>s.filter(x=>x.id!==t.id)) }, 3500)
    }
    subscribers.push(sub)
    return ()=>{
      const idx = subscribers.indexOf(sub)
      if(idx>=0) subscribers.splice(idx,1)
    }
  }, [])

  return (
    <div style={{ position:'fixed', right:12, top:64, zIndex:9999, display:'flex', flexDirection:'column', gap:8 }}>
      {toasts.map(t=> (
        <div key={t.id} style={{ minWidth:220, padding:10, borderRadius:8, background: t.type==='error'?'#3b0b0b': t.type==='success' ? '#05331a' : 'rgba(0,0,0,0.6)', color:'white', boxShadow:'0 6px 18px rgba(0,0,0,0.4)' }}>
          {t.text}
        </div>
      ))}
    </div>
  )
}
