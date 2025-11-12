import Pane from "../../../components/Pane";
import { useMemo } from "react";
import { AreaChart, Area, XAxis, YAxis, Tooltip, ReferenceLine, ResponsiveContainer } from "recharts";
import { useExchange } from "../store";

export default function DepthChartPro(){
  const { asks, bids } = useExchange();
  const { data, mid } = useMemo(()=>{
    if (!asks.length || !bids.length) return { data:[], mid:undefined as number|undefined };
    const m = (asks[0].price + bids[0].price)/2;
    const left = [...bids].sort((a,b)=>a.price-b.price).reduce<any[]>((acc,l)=>{
      const prev = acc.length? acc[acc.length-1].bid:0;
      acc.push({price:l.price, bid:+(prev+l.size).toFixed(3)}); return acc;
    },[]);
    const right = [...asks].sort((a,b)=>a.price-b.price).reduce<any[]>((acc,l)=>{
      const prev = acc.length? acc[acc.length-1].ask:0;
      acc.push({price:l.price, ask:+(prev+l.size).toFixed(3)}); return acc;
    },[]);
    return { data:[...left, {price:m, _mid:true}, ...right], mid:m };
  },[asks,bids]);

  return (
    <Pane title="Depth" className="h-full">
      <div className="h-[300px]">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data}>
            <defs>
              <linearGradient id="gBuy" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="rgb(16 185 129 / .6)" />
                <stop offset="100%" stopColor="rgb(16 185 129 / .05)" />
              </linearGradient>
              <linearGradient id="gSell" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="rgb(239 68 68 / .6)" />
                <stop offset="100%" stopColor="rgb(239 68 68 / .05)" />
              </linearGradient>
            </defs>
            <XAxis dataKey="price" tick={{fill:"#9aa4b2"}} />
            <YAxis tick={{fill:"#9aa4b2"}} />
            <Tooltip contentStyle={{background:"#11161c", border:"1px solid rgba(255,255,255,.06)", borderRadius:8}} />
            <Area type="step" dataKey="bid" stroke="rgb(16 185 129)" fill="url(#gBuy)" />
            <Area type="step" dataKey="ask" stroke="rgb(239 68 68)" fill="url(#gSell)" />
            {mid && <ReferenceLine x={mid} stroke="#3b82f6" strokeDasharray="4 4" />}
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </Pane>
  );
}
