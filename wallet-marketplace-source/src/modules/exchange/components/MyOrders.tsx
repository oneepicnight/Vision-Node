import { useExchange } from "../store";

export default function MyOrders(){
  const { myOrders, cancelOrder } = useExchange();
  return (
    <div className="card-exchange my-orders">
      <div className="text-sm font-semibold mb-2">My Orders</div>
      <div style={{ maxHeight: 240, overflow: 'auto' }}>
      <table className="w-full text-sm">
        <thead><tr><th>ID</th><th>Price</th><th>Filled</th><th>Size</th><th>Status</th><th></th></tr></thead>
        <tbody>
          {myOrders.map(o=>(
            <tr key={o.id}>
              <td className="font-mono text-xs">{o.id.slice(0,8)}</td>
              <td>{o.price.toFixed(2)}</td>
              <td>
                <div className="w-full bg-gray-200 rounded h-2">
                  <div className="bg-blue-600 h-2 rounded" style={{ width: `${(o.sizeFilled/o.sizeTotal)*100}%` }} />
                </div>
                <div className="text-xs text-gray-600">{o.sizeFilled.toFixed(3)} / {o.sizeTotal.toFixed(3)}</div>
              </td>
              <td className="uppercase">{o.status}</td>
              <td className="text-right">
                {(o.status==="open"||o.status==="partial") &&
                  <button className="px-2 py-1 border rounded hover:bg-gray-50" onClick={()=>cancelOrder(o.id)}>Cancel</button>}
              </td>
            </tr>
          ))}
          {!myOrders.length && <tr><td colSpan={6} className="text-center text-gray-500 py-4">No orders yet</td></tr>}
        </tbody>
      </table>
      </div>
    </div>
  );
}
