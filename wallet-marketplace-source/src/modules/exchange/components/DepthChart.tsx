import { useMemo } from "react";
import { AreaChart, Area, XAxis, YAxis, Tooltip, ReferenceLine, ResponsiveContainer } from "recharts";
import { useExchange } from "../store";

export default function DepthChart(){
  const { asks, bids } = useExchange();
  const data = useMemo(()=>{
    if (!asks.length || !bids.length) return [];
    const mid = (asks[0].price + bids[0].price)/2;
    // cumulative
    const left = [...bids].sort((a,b)=>a.price-b.price).reduce<{price:number, bid:number}[]>((acc,l)=>{
      const prev = acc.length? acc[acc.length-1].bid : 0;
      acc.push({ price:l.price, bid: +(prev + l.size).toFixed(3) }); return acc;
    },[]);
    const right = [...asks].sort((a,b)=>a.price-b.price).reduce<{price:number, ask:number}[]>((acc,l)=>{
      const prev = acc.length? acc[acc.length-1].ask : 0;
      acc.push({ price:l.price, ask: +(prev + l.size).toFixed(3) }); return acc;
    },[]);
    // merge around mid
    return [
      ...left.map(({price,bid})=>({ price, bid })),
      { price: mid, bid: left.length? left[left.length-1].bid:0, ask: right.length? right[0].ask:0, _mid:true },
      ...right.map(({price,ask})=>({ price, ask })),
    ];
  },[asks,bids]);

  if (!data.length) return <div className="h-48 border rounded grid place-items-center">No data</div>;

  const mid = data.find(d=> (d as any)._mid)?.price;

  return (
    <div className="h-72 border rounded p-2">
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={data}>
          <XAxis dataKey="price" tickFormatter={(v)=>v.toFixed?.(2) ?? v} />
          <YAxis />
          <Tooltip formatter={(v:any, n:any)=>[v, n.toUpperCase()]} />
          <Area type="step" dataKey="bid" fillOpacity={0.2} strokeOpacity={0.8} />
          <Area type="step" dataKey="ask" fillOpacity={0.2} strokeOpacity={0.8} />
          {mid && <ReferenceLine x={mid} strokeDasharray="3 3" />}
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
