import { loadConfig } from '../../lib/config';

export type Chain = "BTC"|"BCH"|"DOGE"|"LAND";

// Safe fetch helper that checks res.ok before parsing JSON
async function safeFetch(url: string, init?: RequestInit) {
  const res = await fetch(url, init);
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    throw new Error(`HTTP ${res.status} ${res.statusText} – ${text.slice(0, 200)}`);
  }
  const ct = res.headers.get('content-type') || '';
  if (ct.includes('application/json')) return res.json();
  try { return await res.json(); } catch { return await res.text(); }
}

export async function getBook(chain:Chain, depth=50){
  const { apiBase } = await loadConfig();
  return safeFetch(`${apiBase}/api/market/exchange/book?chain=${chain}&depth=${depth}`);
}

export async function getTicker(chain:Chain){
  const { apiBase } = await loadConfig();
  return safeFetch(`${apiBase}/api/market/exchange/ticker?chain=${chain}`);
}

export async function getMyOrders(owner:string){
  const { apiBase } = await loadConfig();
  return safeFetch(`${apiBase}/api/market/exchange/my/orders?owner=${encodeURIComponent(owner)}`);
}

export async function postOrder(params:{owner:string,chain:Chain,price:number,size:number,post_only:boolean,tif:string}){
  const { apiBase } = await loadConfig();
  return safeFetch(`${apiBase}/api/market/exchange/order`, { 
    method:"POST", 
    headers:{ "content-type":"application/json" }, 
    body: JSON.stringify(params) 
  });
}

export async function postBuy(params:{owner:string,chain:Chain,size?:number,spend?:number}){
  const { apiBase } = await loadConfig();
  return safeFetch(`${apiBase}/api/market/exchange/buy`, { 
    method:"POST", 
    headers:{ "content-type":"application/json" }, 
    body: JSON.stringify(params) 
  });
}

export async function cancelOrderReq(id:string, owner:string){
  const { apiBase } = await loadConfig();
  return safeFetch(`${apiBase}/api/market/exchange/order/${id}/cancel`, { 
    method:"POST", 
    headers:{ "content-type":"application/json" }, 
    body: JSON.stringify({ owner }) 
  });
}

export async function openStream(onMsg:(ev:any)=>void){
  const { wsBase } = await loadConfig();
  const url = `${wsBase}/api/market/exchange/stream`;
  try {
    const ws = new WebSocket(url);
    ws.onmessage = (e)=> { try { onMsg(JSON.parse(e.data)); } catch {} };
    ws.onopen = ()=>console.log('exchange ws open');
    ws.onclose = ()=>console.log('exchange ws closed');
    return ws;
  } catch (e) { return undefined as any; }
}

export async function fetchTrades(chain = 'BTC', limit = 100) {
  const { apiBase } = await loadConfig();
  return safeFetch(`${apiBase}/api/market/exchange/trades?chain=${chain}&limit=${limit}`);
}

// connectBookWS: lightweight Ws + polling fallback
export function connectBookWS(chain = 'BTC', onMsg: (m: any) => void) {
  let ws: WebSocket | null = null
  let fallback: number | null = null

  const start = async () => {
    try {
      const { wsBase } = await loadConfig();
      const url = `${wsBase}/api/market/exchange/stream?chain=${chain}`;
      ws = new WebSocket(url)
      ws.onopen = () => { if (fallback) { clearInterval(fallback); fallback = null } }
      ws.onmessage = (ev) => { try { onMsg(JSON.parse(ev.data)) } catch (_) {} }
      ws.onclose = () => schedulePoll()
      ws.onerror = () => schedulePoll()
    } catch (err) { schedulePoll() }
  }

  const schedulePoll = () => {
    if (fallback) return
    fallback = window.setInterval(async () => {
      try {
        const data = await getBook(chain as Chain, 30);
        onMsg({ type: 'book_snapshot', payload: data })
      } catch (e) { /* ignore */ }
    }, 2500)
  }

  start()

  return {
    close: () => { if (ws) ws.close(); if (fallback) { clearInterval(fallback); fallback = null } }
  }
}
