import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useWalletStore } from '../state/wallet'
import { getExchangeBook, getExchangeTicker, getExchangeTrades, getMyOrders, createOrder } from '../lib/api'
import '../styles/wallet.css'

interface OrderBookEntry {
  price: number
  size: number
  total: number
}

interface Trade {
  time: string
  price: number
  size: number
  side: 'buy' | 'sell'
}

interface Order {
  id: string
  time: string
  pair: string
  side: 'buy' | 'sell'
  price: number
  amount: number
  filled: number
  status: 'open' | 'filled' | 'cancelled' | 'partial'
}

// Trading pairs configuration (LAND, BTC, BCH, DOGE only - no GAME)
const TRADING_PAIRS = [
  // CASH can only sell to LAND
  { base: 'CASH', quote: 'LAND', label: 'CASH/LAND' },
  // All other coins trade with LAND
  { base: 'LAND', quote: 'BTC', label: 'LAND/BTC' },
  { base: 'LAND', quote: 'BCH', label: 'LAND/BCH' },
  { base: 'LAND', quote: 'DOGE', label: 'LAND/DOGE' },
  // Other coins trade with each other (except CASH)
  { base: 'BTC', quote: 'BCH', label: 'BTC/BCH' },
  { base: 'BTC', quote: 'DOGE', label: 'BTC/DOGE' },
  { base: 'BCH', quote: 'DOGE', label: 'BCH/DOGE' },
]

export default function Exchange() {
  const navigate = useNavigate()
  const { profile, balances } = useWalletStore()
  const [selectedPairIndex, setSelectedPairIndex] = useState(1) // Default to LAND/BTC
  const selectedPair = TRADING_PAIRS[selectedPairIndex]
  const [lastPrice, setLastPrice] = useState(0.00000042)
  const [priceChange24h, setPriceChange24h] = useState(0)
  const [volume24h, setVolume24h] = useState(0)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  
  const [orderSide, setOrderSide] = useState<'buy' | 'sell'>('buy')
  const [orderPrice, setOrderPrice] = useState('')
  const [orderAmount, setOrderAmount] = useState('')
  const [activeTab, setActiveTab] = useState<'open' | 'history'>('open')
  const [orderStatus, setOrderStatus] = useState('')

  const [bids, setBids] = useState<OrderBookEntry[]>([])
  const [asks, setAsks] = useState<OrderBookEntry[]>([])
  const [recentTrades, setRecentTrades] = useState<Trade[]>([])
  const [openOrders, setOpenOrders] = useState<Order[]>([])
  const [orderHistory, setOrderHistory] = useState<Order[]>([])

  // Load exchange data
  useEffect(() => {
    loadExchangeData()
    const interval = setInterval(loadExchangeData, 5000) // Refresh every 5 seconds
    return () => clearInterval(interval)
  }, [selectedPairIndex, selectedPair])

  // Load user orders
  useEffect(() => {
    if (profile?.address) {
      loadUserOrders()
      const interval = setInterval(loadUserOrders, 10000) // Refresh every 10 seconds
      return () => clearInterval(interval)
    }
  }, [profile, selectedPairIndex])

  async function loadExchangeData() {
    try {
      setError('')
      
      // Load ticker
      const ticker = await getExchangeTicker(selectedPair.quote)
      setLastPrice(ticker.last || 0.00000042)
      setPriceChange24h(ticker.change24h || 0)
      setVolume24h(ticker.vol24h || 0)

      // Load order book
      const book = await getExchangeBook(selectedPair.quote, 50)
      
      // Convert to OrderBookEntry format with cumulative totals
      let bidTotal = 0
      const formattedBids: OrderBookEntry[] = book.bids.map(([price, size]) => {
        bidTotal += price * size
        return { price, size, total: bidTotal }
      })
      
      let askTotal = 0
      const formattedAsks: OrderBookEntry[] = book.asks.map(([price, size]) => {
        askTotal += price * size
        return { price, size, total: askTotal }
      })
      
      setBids(formattedBids)
      setAsks(formattedAsks)

      // Load recent trades
      const trades = await getExchangeTrades(selectedPair.quote, 20)
      const formattedTrades: Trade[] = trades.map(t => ({
        time: new Date(t.ts).toLocaleTimeString(),
        price: t.price,
        size: t.size,
        side: t.side
      }))
      setRecentTrades(formattedTrades)

    } catch (err) {
      console.error('Failed to load exchange data:', err)
      setError(err instanceof Error ? err.message : 'Failed to load exchange data')
    }
  }

  async function loadUserOrders() {
    if (!profile?.address) return
    
    try {
      const orders = await getMyOrders(profile.address, selectedPair.quote)
      
      const formatted: Order[] = orders.map(o => ({
        id: o.id,
        time: new Date().toLocaleTimeString(), // TODO: Use actual timestamp when available
        pair: `LAND/${o.chain}`,
        side: o.side,
        price: o.price || 0,
        amount: o.size_total,
        filled: o.size_filled,
        status: o.status as any
      }))

      setOpenOrders(formatted.filter(o => o.status === 'open' || o.status === 'partial'))
      setOrderHistory(formatted.filter(o => o.status === 'filled' || o.status === 'cancelled'))
      
    } catch (err) {
      console.error('Failed to load user orders:', err)
    }
  }

  const orderTotal = orderPrice && orderAmount 
    ? (parseFloat(orderPrice) * parseFloat(orderAmount)).toFixed(8)
    : '0.00000000'

  const handlePlaceOrder = async () => {
    if (!profile?.address) {
      setOrderStatus('❌ Please connect wallet first')
      setTimeout(() => setOrderStatus(''), 3000)
      return
    }

    if (!orderPrice || !orderAmount) {
      setOrderStatus('❌ Please enter price and amount')
      setTimeout(() => setOrderStatus(''), 3000)
      return
    }

    const price = parseFloat(orderPrice)
    const size = parseFloat(orderAmount)

    if (price <= 0 || size <= 0) {
      setOrderStatus('❌ Invalid price or amount')
      setTimeout(() => setOrderStatus(''), 3000)
      return
    }

    try {
      setLoading(true)
      setOrderStatus('⏳ Placing order...')
      
      const result = await createOrder({
        owner: profile.address,
        chain: selectedPair.quote,
        side: orderSide,
        price,
        size,
        tif: 'GTC'
      })

      if (result.ok) {
        setOrderStatus(`✅ ${result.message}`)
        setOrderPrice('')
        setOrderAmount('')
        
        // Reload orders and book
        await loadUserOrders()
        await loadExchangeData()
        
        setTimeout(() => setOrderStatus(''), 5000)
      } else {
        setOrderStatus('❌ Order failed')
        setTimeout(() => setOrderStatus(''), 3000)
      }
    } catch (err) {
      console.error('Failed to place order:', err)
      setOrderStatus(`❌ ${err instanceof Error ? err.message : 'Order failed'}`)
      setTimeout(() => setOrderStatus(''), 3000)
    } finally {
      setLoading(false)
    }
  }

  const handleBookClick = (price: number) => {
    setOrderPrice(price.toFixed(8))
  }

  return (
    <div className="vw-shell">
      <div className="vw-main">
        {/* Trading Context Banner */}
        <div className="vw-card" style={{ marginBottom: '1rem', padding: '1rem 1.5rem' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '1rem' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '1.5rem', flexWrap: 'wrap' }}>
              <div>
                <span style={{ fontSize: '0.75rem', color: '#94a3b8', textTransform: 'uppercase' }}>Trading as:</span>
                <div style={{ fontSize: '0.9rem', fontWeight: 600, color: '#e2e8f0', marginTop: '0.25rem' }}>
                  {profile?.address ? `${profile.address.substring(0, 8)}...${profile.address.substring(profile.address.length - 6)}` : 'Not connected'}
                </div>
              </div>
              <div style={{ borderLeft: '1px solid rgba(148, 163, 184, 0.2)', paddingLeft: '1.5rem', display: 'flex', gap: '1.5rem' }}>
                <div>
                  <span style={{ fontSize: '0.75rem', color: '#94a3b8' }}>LAND:</span>
                  <span style={{ fontSize: '0.9rem', fontWeight: 600, color: '#10b981', marginLeft: '0.5rem' }}>
                    {balances.LAND.toFixed(2)}
                  </span>
                </div>
                <div>
                  <span style={{ fontSize: '0.75rem', color: '#94a3b8' }}>CASH:</span>
                  <span style={{ fontSize: '0.9rem', fontWeight: 600, color: '#3b82f6', marginLeft: '0.5rem' }}>
                    {balances.CASH.toFixed(0)}
                  </span>
                </div>
              </div>
            </div>
            <button 
              onClick={() => navigate('/command-center')}
              style={{
                padding: '0.5rem 1rem',
                background: 'rgba(59, 130, 246, 0.1)',
                border: '1px solid rgba(59, 130, 246, 0.3)',
                borderRadius: '8px',
                color: '#60a5fa',
                fontSize: '0.85rem',
                cursor: 'pointer',
                transition: 'all 0.2s ease'
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = 'rgba(59, 130, 246, 0.2)'
                e.currentTarget.style.borderColor = 'rgba(59, 130, 246, 0.5)'
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'rgba(59, 130, 246, 0.1)'
                e.currentTarget.style.borderColor = 'rgba(59, 130, 246, 0.3)'
              }}
            >
              View Full Command Center →
            </button>
          </div>
          <div style={{ marginTop: '0.75rem', fontSize: '0.75rem', color: '#64748b' }}>
            💡 Your node, mining, and Guardian status are monitored via the Mini Command Center at the top of the page.
          </div>
        </div>

        {/* Hero Bar */}
        <div className="vx-hero vw-card">
          <div className="vx-hero-left">
            <div className="vx-pair">
              <select 
                value={selectedPairIndex} 
                onChange={(e) => setSelectedPairIndex(parseInt(e.target.value))}
                style={{
                  background: 'rgba(138, 92, 255, 0.15)',
                  border: '1px solid rgba(162, 145, 255, 0.3)',
                  borderRadius: '8px',
                  padding: '0.5rem 1rem',
                  color: '#e4ddff',
                  fontSize: '1.2rem',
                  fontWeight: 700,
                  cursor: 'pointer'
                }}
              >
                {TRADING_PAIRS.map((pair, idx) => (
                  <option key={idx} value={idx} style={{ background: '#1a1425', color: '#e4ddff' }}>
                    {pair.label}
                  </option>
                ))}
              </select>
            </div>
            <div className="vx-stats">
              <div className="vx-stat">
                <div className="vx-stat-label">Last Price</div>
                <div className="vx-stat-value">{lastPrice.toFixed(8)}</div>
              </div>
              <div className="vx-stat">
                <div className="vx-stat-label">24h Change</div>
                <div className={`vx-stat-value ${priceChange24h >= 0 ? 'positive' : 'negative'}`}>
                  {priceChange24h >= 0 ? '+' : ''}{priceChange24h.toFixed(2)}%
                </div>
              </div>
              <div className="vx-stat">
                <div className="vx-stat-label">24h Volume</div>
                <div className="vx-stat-value">{volume24h.toLocaleString()} {selectedPair.base}</div>
              </div>
            </div>
          </div>
          <div className="vx-hero-right">
            <div className="vw-pill green">Live Trading</div>
          </div>
        </div>

        {error && (
          <div style={{
            padding: '1rem',
            background: 'rgba(239, 68, 68, 0.2)',
            border: '1px solid rgba(239, 68, 68, 0.5)',
            borderRadius: '12px',
            color: '#fca5a5',
            marginBottom: '1rem'
          }}>
            {error}
          </div>
        )}

        {/* Order Book / Chart / Trades Grid */}
        <div className="vw-section">
          <div className="vx-grid">
            {/* Order Book */}
            <div className="vw-card">
              <h3 className="vw-action-title" style={{ marginBottom: '1rem' }}>Order Book</h3>
              <div className="vx-order-book">
                {/* Asks (Sell Orders) */}
                <div className="vx-book-section">
                  <div className="vx-book-header">
                    <span>Price ({selectedPair.quote})</span>
                    <span>Size ({selectedPair.base})</span>
                    <span>Total</span>
                  </div>
                  {asks.length === 0 && (
                    <div style={{ padding: '1rem', textAlign: 'center', color: '#6b5f9a', fontSize: '0.85rem' }}>
                      No sell orders
                    </div>
                  )}
                  {asks.slice().reverse().map((ask, idx) => (
                    <div 
                      key={`ask-${idx}`} 
                      className="vx-book-row ask"
                      onClick={() => handleBookClick(ask.price)}
                    >
                      <span>{ask.price.toFixed(8)}</span>
                      <span>{ask.size.toFixed(2)}</span>
                      <span>{ask.total.toFixed(8)}</span>
                    </div>
                  ))}
                </div>

                {/* Current Price */}
                <div style={{ 
                  padding: '0.8rem', 
                  textAlign: 'center', 
                  fontSize: '1.1rem', 
                  fontWeight: 700,
                  color: '#e4ddff',
                  borderTop: '1px solid rgba(138, 92, 255, 0.2)',
                  borderBottom: '1px solid rgba(138, 92, 255, 0.2)',
                  margin: '0.5rem 0'
                }}>
                  {lastPrice.toFixed(8)}
                </div>

                {/* Bids (Buy Orders) */}
                <div className="vx-book-section">
                  <div className="vx-book-header">
                    <span>Price ({selectedPair.quote})</span>
                    <span>Size ({selectedPair.base})</span>
                    <span>Total</span>
                  </div>
                  {bids.length === 0 && (
                    <div style={{ padding: '1rem', textAlign: 'center', color: '#6b5f9a', fontSize: '0.85rem' }}>
                      No buy orders
                    </div>
                  )}
                  {bids.map((bid, idx) => (
                    <div 
                      key={`bid-${idx}`} 
                      className="vx-book-row bid"
                      onClick={() => handleBookClick(bid.price)}
                    >
                      <span>{bid.price.toFixed(8)}</span>
                      <span>{bid.size.toFixed(2)}</span>
                      <span>{bid.total.toFixed(8)}</span>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            {/* Price Chart Placeholder */}
            <div className="vx-chart-placeholder vw-card">
              <div className="vx-chart-title">Price Chart</div>
              <div className="vx-chart-subtitle">{selectedPair.label}</div>
              <div style={{ 
                marginTop: '2rem', 
                padding: '3rem', 
                border: '2px dashed rgba(138, 92, 255, 0.3)',
                borderRadius: '12px',
                color: '#6b5f9a',
                textAlign: 'center',
                fontSize: '0.9rem'
              }}>
                <div>Live trading chart coming soon</div>
                <div style={{ marginTop: '0.5rem', fontSize: '0.8rem' }}>
                  Order book and trades updating in real-time
                </div>
              </div>
            </div>

            {/* Recent Trades */}
            <div className="vw-card">
              <h3 className="vw-action-title" style={{ marginBottom: '1rem' }}>Recent Trades</h3>
              <div className="vx-trades">
                <div className="vx-trades-header">
                  <span>Time</span>
                  <span>Price</span>
                  <span>Size</span>
                  <span>Side</span>
                </div>
                {recentTrades.length === 0 && (
                  <div style={{ padding: '2rem', textAlign: 'center', color: '#6b5f9a', fontSize: '0.85rem' }}>
                    No recent trades
                  </div>
                )}
                {recentTrades.map((trade, idx) => (
                  <div key={`trade-${idx}`} className="vx-trade-row">
                    <span>{trade.time}</span>
                    <span>{trade.price.toFixed(8)}</span>
                    <span>{trade.size.toFixed(2)}</span>
                    <span className={`side ${trade.side}`}>
                      {trade.side.toUpperCase()}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Place Order / Order History */}
        <div className="vw-section">
          <div className="vx-bottom-grid">
            {/* Place Order */}
            <div className="vw-card">
              <h3 className="vw-action-title" style={{ marginBottom: '1rem' }}>Place Order</h3>
              <div className="vx-order-ticket">
                {/* Buy/Sell Toggle */}
                <div className="vx-order-toggle">
                  <button 
                    className={`vx-order-toggle-btn ${orderSide === 'buy' ? 'active buy' : ''}`}
                    onClick={() => setOrderSide('buy')}
                    disabled={loading}
                  >
                    Buy
                  </button>
                  <button 
                    className={`vx-order-toggle-btn ${orderSide === 'sell' ? 'active sell' : ''}`}
                    onClick={() => setOrderSide('sell')}
                    disabled={loading}
                  >
                    Sell
                  </button>
                </div>

                {/* Price */}
                <div className="vw-field">
                  <label className="vw-label">Price ({selectedPair.quote})</label>
                  <input 
                    type="text" 
                    className="vw-input"
                    placeholder="0.00000000"
                    value={orderPrice}
                    onChange={(e) => setOrderPrice(e.target.value)}
                    disabled={loading}
                  />
                </div>

                {/* Amount */}
                <div className="vw-field">
                  <label className="vw-label">Amount ({selectedPair.base})</label>
                  <input 
                    type="text" 
                    className="vw-input"
                    placeholder="0.00"
                    value={orderAmount}
                    onChange={(e) => setOrderAmount(e.target.value)}
                    disabled={loading}
                  />
                </div>

                {/* Total */}
                <div className="vx-order-summary">
                  <span className="vx-order-summary-label">Total</span>
                  <span className="vx-order-summary-value">{orderTotal} {selectedPair.quote}</span>
                </div>

                {/* Status Message */}
                {orderStatus && (
                  <div style={{
                    padding: '0.75rem',
                    background: orderStatus.includes('✅') ? 'rgba(34, 197, 94, 0.15)' : 
                                orderStatus.includes('❌') ? 'rgba(239, 68, 68, 0.15)' : 
                                'rgba(138, 92, 255, 0.15)',
                    border: `1px solid ${orderStatus.includes('✅') ? 'rgba(34, 197, 94, 0.4)' : 
                                          orderStatus.includes('❌') ? 'rgba(239, 68, 68, 0.4)' : 
                                          'rgba(138, 92, 255, 0.4)'}`,
                    borderRadius: '12px',
                    fontSize: '0.9rem',
                    textAlign: 'center'
                  }}>
                    {orderStatus}
                  </div>
                )}

                {/* Place Order Button */}
                <button 
                  className={orderSide === 'buy' ? 'vw-primary-btn' : 'vw-secondary-btn'}
                  onClick={handlePlaceOrder}
                  disabled={loading}
                  style={{ width: '100%', marginTop: '0.5rem' }}
                >
                  {loading ? 'Placing...' : orderSide === 'buy' ? 'Place Buy Order' : 'Place Sell Order'}
                </button>
              </div>
            </div>

            {/* Order History */}
            <div className="vw-card">
              <div className="vx-order-history">
                {/* Tabs */}
                <div className="vx-tabs">
                  <button 
                    className={`vx-tab ${activeTab === 'open' ? 'active' : ''}`}
                    onClick={() => setActiveTab('open')}
                  >
                    Open Orders ({openOrders.length})
                  </button>
                  <button 
                    className={`vx-tab ${activeTab === 'history' ? 'active' : ''}`}
                    onClick={() => setActiveTab('history')}
                  >
                    Order History
                  </button>
                </div>

                {/* Table */}
                <div className="vx-history-table">
                  <div className="vx-history-header">
                    <span>Time</span>
                    <span>Pair</span>
                    <span>Side</span>
                    <span>Price</span>
                    <span>Amount</span>
                  </div>
                  {activeTab === 'open' && openOrders.map((order) => (
                    <div key={order.id} className="vx-history-row">
                      <span>{order.time}</span>
                      <span>{order.pair}</span>
                      <span style={{ color: order.side === 'buy' ? '#5eead4' : '#fca5a5' }}>
                        {order.side.toUpperCase()}
                      </span>
                      <span>{order.price.toFixed(8)}</span>
                      <span>{order.amount.toLocaleString()}</span>
                    </div>
                  ))}
                  {activeTab === 'history' && orderHistory.map((order) => (
                    <div key={order.id} className="vx-history-row">
                      <span>{order.time}</span>
                      <span>{order.pair}</span>
                      <span style={{ color: order.side === 'buy' ? '#5eead4' : '#fca5a5' }}>
                        {order.side.toUpperCase()}
                      </span>
                      <span>{order.price.toFixed(8)}</span>
                      <span>{order.amount.toLocaleString()}</span>
                    </div>
                  ))}
                  {activeTab === 'open' && openOrders.length === 0 && (
                    <div style={{ 
                      padding: '2rem', 
                      textAlign: 'center', 
                      color: '#6b5f9a',
                      fontSize: '0.9rem'
                    }}>
                      No open orders
                    </div>
                  )}
                  {activeTab === 'history' && orderHistory.length === 0 && (
                    <div style={{ 
                      padding: '2rem', 
                      textAlign: 'center', 
                      color: '#6b5f9a',
                      fontSize: '0.9rem'
                    }}>
                      No order history
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
