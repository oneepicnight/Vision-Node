import { IncomingMessage, ServerResponse } from 'http'

type Balances = { [token: string]: number }

// In-memory mock state for dev sessions
const state: {
  receipts: any[]
  balances: Record<string, Balances>
  // exchange
  orderbooks: Record<string, { bids: number[][]; asks: number[][] }> // price, size
  orders: any[]
  trades: any[]
} = {
  receipts: [],
  balances: {},
  orderbooks: {},
  orders: [],
  trades: []
}

function sendJson(res: ServerResponse, obj: any, code = 200) {
  res.statusCode = code
  res.setHeader('Content-Type', 'application/json')
  res.end(JSON.stringify(obj))
}

function readBody(req: IncomingMessage): Promise<any> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = []
    req.on('data', (c) => chunks.push(Buffer.from(c)))
    req.on('end', () => {
      try {
        const s = Buffer.concat(chunks).toString() || ''
        if (!s) return resolve(null)
        resolve(JSON.parse(s))
      } catch (e) {
        reject(e)
      }
    })
    req.on('error', reject)
  })
}

export function createMockWalletMiddleware() {
  return async function mockWallet(req: IncomingMessage, res: ServerResponse, next: (...args: any[]) => void) {
    try {
      const rawUrl = req.url || '/'
      // parse path and query safely
      const parsed = new URL(rawUrl, 'http://localhost')
      const url = parsed.pathname
      const method = (req.method || 'GET').toUpperCase()

      // Simple routing for wallet endpoints used by the frontend
      if (method === 'GET' && (url === '/status' || url === '/status/')) {
        return sendJson(res, { node: 'mock', version: 'dev', time: Date.now() })
      }

      if (method === 'GET' && (url === '/keys' || url === '/keys/')) {
        // Return a simple keys response so tryKeysThenVault succeeds in dev
        return sendJson(res, { keys: [] })
      }

      if (method === 'GET' && (url === '/vault' || url === '/vault/')) {
        return sendJson(res, { receipts: state.receipts.slice(0, 50), mocked: true })
      }

      if (method === 'GET' && (url === '/supply' || url === '/supply/')) {
        return sendJson(res, { total: 1000000 })
      }

      if (method === 'GET' && url.startsWith('/receipts/latest')) {
        return sendJson(res, state.receipts.slice(0, 50))
      }

      if (method === 'GET' && url.startsWith('/balance/')) {
        const parts = url.split('/')
        const addr = parts[2] || 'unknown'
        const bal = state.balances[addr] || { LAND: 1, GAME: 250, CASH: 500 }
        return sendJson(res, bal)
      }

  // Exchange endpoints (simple mocks)
  // Support both direct `/exchange/...` and proxied `/api/market/exchange/...`
  let basePath = url
  if (basePath.startsWith('/api/market')) basePath = basePath.replace(/^\/api\/market/, '')
  if (basePath.startsWith('/market')) basePath = basePath.replace(/^\/market/, '')

  if (basePath.startsWith('/exchange') || basePath.startsWith('/market')) {
        // GET /exchange/book?chain=BTC&depth=200
        if (method === 'GET' && (parsed.pathname === '/exchange/book' || parsed.pathname === '/market/exchange/book' || basePath === '/exchange/book')) {
          const chain = parsed.searchParams.get('chain') || 'BTC'
          const depth = Number(parsed.searchParams.get('depth') || 50)
          // seed orderbook if missing
          if (!state.orderbooks[chain]) {
            const mid = 50000
            const bids: number[][] = []
            const asks: number[][] = []
            for (let i = 0; i < Math.min(depth, 100); i++) {
              bids.push([mid - i * 10, Math.max(1, Math.round(50 - i))])
              asks.push([mid + i * 10, Math.max(1, Math.round(10 + i))])
            }
            state.orderbooks[chain] = { bids, asks }
          }
          return sendJson(res, state.orderbooks[chain])
        }

        // GET /exchange/ticker?chain=BTC
        if (method === 'GET' && (parsed.pathname === '/exchange/ticker' || basePath === '/exchange/ticker')) {
          const chain = parsed.searchParams.get('chain') || 'BTC'
          const ob = state.orderbooks[chain]
          const last = ob ? (ob.asks[0][0] + ob.bids[0][0]) / 2 : 50000
          return sendJson(res, { chain, last, change24h: 1.2, volume24h: 12345 })
        }

        // GET /exchange/trades?chain=BTC&limit=50
        if (method === 'GET' && (parsed.pathname === '/exchange/trades' || basePath === '/exchange/trades')) {
          const limit = Number(parsed.searchParams.get('limit') || 50)
          return sendJson(res, state.trades.slice(0, limit))
        }

        // GET /exchange/my/orders?owner=...
        if (method === 'GET' && (parsed.pathname === '/exchange/my/orders' || basePath === '/exchange/my/orders')) {
          const owner = parsed.searchParams.get('owner') || ''
          const list = state.orders.filter((o) => o.owner === owner)
          return sendJson(res, list)
        }

        // POST /exchange/order -> create limit order
        if (method === 'POST' && (parsed.pathname === '/exchange/order' || basePath === '/exchange/order')) {
          const body = await readBody(req)
          const order = body || {}
          const id = `ord-${Date.now()}-${Math.floor(Math.random() * 10000)}`
          const o = { id, ...order, status: 'open', createdAt: Date.now() }
          state.orders.push(o)
          return sendJson(res, { ok: true, order: o })
        }

        // POST /exchange/buy -> market buy simulation
        if (method === 'POST' && (parsed.pathname === '/exchange/buy' || basePath === '/exchange/buy')) {
          const body = await readBody(req)
          const { chain = 'BTC', size = 1 } = body || {}
          // consume asks
          const ob = state.orderbooks[chain] || { asks: [[50000, 100]], bids: [[49900, 50]] }
          let remaining = Number(size)
          const trades: any[] = []
          while (remaining > 0 && ob.asks.length > 0) {
            const [price, qty] = ob.asks.shift()!
            const take = Math.min(remaining, qty)
            trades.push({ price, size: take, time: Date.now(), side: 'buy' })
            remaining -= take
            if (qty > take) {
              ob.asks.unshift([price, qty - take])
              break
            }
          }
          // record trades
          for (const t of trades) state.trades.unshift({ ...t, chain })
          return sendJson(res, { ok: true, trades })
        }

        // POST /exchange/order/:id/cancel
        if (method === 'POST' && ((parsed.pathname.startsWith('/exchange/order/') && parsed.pathname.endsWith('/cancel')) || basePath.startsWith('/exchange/order/') && basePath.endsWith('/cancel'))) {
          const parts = parsed.pathname.split('/')
          const id = parts[3]
          const order = state.orders.find((o) => o.id === id)
          if (order) {
            order.status = 'cancelled'
            return sendJson(res, { ok: true })
          }
          return sendJson(res, { ok: false, error: 'not_found' }, 404)
        }
      }

      if (method === 'POST' && (url === '/tx/submit' || url === '/tx/submit/')) {
        const body = await readBody(req)
        const payload = body || {}
        const tx = payload.tx || payload || {}
        const from = tx.from || 'unknown'
        const to = tx.to || 'unknown'
        const token = tx.token || 'CASH'
        const amount = Number(tx.amount || 0)

        // Update balances (best-effort)
        state.balances[from] = state.balances[from] || { LAND: 1, GAME: 250, CASH: 500 }
        state.balances[to] = state.balances[to] || { LAND: 0, GAME: 0, CASH: 0 }
        if (typeof state.balances[from][token] === 'number') {
          state.balances[from][token] = Math.max(0, state.balances[from][token] - amount)
        }
        if (typeof state.balances[to][token] === 'number') {
          state.balances[to][token] = (state.balances[to][token] || 0) + amount
        }

        const txid = `mock-${Date.now()}-${Math.floor(Math.random() * 10000)}`
        const receipt = { txid, from, to, token, amount, time: Date.now(), status: 'confirmed' }
        state.receipts.unshift(receipt)

        return sendJson(res, { ok: true, txid })
      }

      // Not one of our mock routes — continue to next middleware
      return next()
    } catch (e) {
      // If anything goes wrong, pass through so the dev server can handle it
      return next()
    }
  }
}

export default createMockWalletMiddleware
