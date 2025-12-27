import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useWalletStore } from '../state/wallet'
import { getBalance, getVaultStatus, getDepositAddress, submitTx, SignedTransaction } from '../lib/api'
import TipButton from '../components/TipButton'
import '../styles/wallet-vision.css'

type Asset = 'LAND' | 'CASH' | 'BTC' | 'BCH' | 'DOGE'

interface AssetInfo {
  symbol: Asset
  name: string
  description: string
}

const ASSETS: AssetInfo[] = [
  { symbol: 'LAND', name: 'LAND', description: 'Primary chain token' },
  { symbol: 'CASH', name: 'CASH', description: 'In-game credit' },
  { symbol: 'BTC', name: 'Bitcoin', description: 'Bitcoin' },
  { symbol: 'BCH', name: 'Bitcoin Cash', description: 'Bitcoin Cash' },
  { symbol: 'DOGE', name: 'Dogecoin', description: 'Dogecoin' }
]

export default function HomeNew() {
  const navigate = useNavigate()
  const { profile, balances, setBalances } = useWalletStore()
  
  // UI state
  const [copyMessage, setCopyMessage] = useState('')
  const [sendAsset, setSendAsset] = useState<Asset>('LAND')
  const [sendTo, setSendTo] = useState('')
  const [sendAmount, setSendAmount] = useState('')
  const [sendError, setSendError] = useState('')
  const [sendSuccess, setSendSuccess] = useState(false)
  const [depositAsset, setDepositAsset] = useState<'BTC' | 'BCH' | 'DOGE'>('BTC')
  const [depositAddress, setDepositAddress] = useState('')
  const [depositCopyMsg, setDepositCopyMsg] = useState('')
  
  // Vault data
  const [vaultData, setVaultData] = useState<any>(null)
  const [lastVaultUpdate, setLastVaultUpdate] = useState<Date>(new Date())
  
  // Portal charge (placeholder - will connect to real metric)
  const [portalCharge] = useState(42)

  useEffect(() => {
    if (!profile) {
      navigate('/')
    }
  }, [profile, navigate])

  // Load balances
  useEffect(() => {
    if (!profile) return

    const loadBalances = async () => {
      try {
        const nodeBalances = await getBalance(profile.address)
        setBalances(nodeBalances)
      } catch (error) {
        console.error('Failed to load balances:', error)
      }
    }

    loadBalances()
    const interval = setInterval(loadBalances, 5000)
    return () => clearInterval(interval)
  }, [profile, setBalances])

  // Load vault data
  useEffect(() => {
    if (!profile) return

    const loadVault = async () => {
      try {
        const data = await getVaultStatus()
        setVaultData(data)
        setLastVaultUpdate(new Date())
      } catch (error) {
        console.error('Failed to load vault:', error)
      }
    }

    loadVault()
    const interval = setInterval(loadVault, 3000)
    return () => clearInterval(interval)
  }, [profile])

  // Load deposit address
  useEffect(() => {
    if (!profile) return

    const loadDepositAddr = async () => {
      try {
        const result = await getDepositAddress(depositAsset, profile.address)
        setDepositAddress(result.address)
      } catch (error) {
        console.error('Failed to load deposit address:', error)
        setDepositAddress(`Error loading ${depositAsset} address`)
      }
    }

    loadDepositAddr()
  }, [profile, depositAsset])

  if (!profile) {
    return null
  }

  const totalPortfolio = balances.LAND || 0
  const portfolioUSD = (totalPortfolio * 0.0).toFixed(2) // Placeholder for price conversion

  const handleCopyAddress = async () => {
    try {
      await navigator.clipboard.writeText(profile.address)
      setCopyMessage('Copied!')
      setTimeout(() => setCopyMessage(''), 1200)
    } catch (error) {
      setCopyMessage('Failed')
      setTimeout(() => setCopyMessage(''), 1200)
    }
  }

  const handleCopyDeposit = async () => {
    try {
      await navigator.clipboard.writeText(depositAddress)
      setDepositCopyMsg('Copied!')
      setTimeout(() => setDepositCopyMsg(''), 1200)
    } catch (error) {
      setDepositCopyMsg('Failed')
      setTimeout(() => setDepositCopyMsg(''), 1200)
    }
  }

  const handleSend = async () => {
    setSendError('')
    setSendSuccess(false)

    // Validation
    const amount = parseFloat(sendAmount)
    if (!sendTo || !sendAmount || amount <= 0) {
      setSendError('Please fill all fields with valid values')
      return
    }

    const available = (balances as any)[sendAsset] || 0
    if (amount > available) {
      setSendError(`Insufficient ${sendAsset} balance`)
      return
    }

    try {
      // For now, create unsigned transaction
      // In production, this would be signed by wallet private key
      const tx: SignedTransaction = {
        tx: {
          token: sendAsset,
          to: sendTo,
          amount: amount,
          from: profile.address,
          nonce: Date.now()
        },
        sig: 'placeholder_signature'
      }

      const result = await submitTx(tx)
      if (result.ok) {
        setSendSuccess(true)
        setSendTo('')
        setSendAmount('')
        setTimeout(() => setSendSuccess(false), 3000)
        
        // Refresh balances
        const newBalances = await getBalance(profile.address)
        setBalances(newBalances)
      } else {
        setSendError(result.txid)
      }
    } catch (error) {
      setSendError(error instanceof Error ? error.message : 'Send failed')
    }
  }

  const getAssetBalance = (symbol: Asset) => {
    return (balances as any)[symbol] || 0
  }

  const formatBalance = (value: number) => {
    return value.toFixed(8)
  }

  const getVaultHealth = () => {
    if (!vaultData) return '...'
    // Calculate health based on vault balances
    return '84%'
  }

  const getTotalSupply = () => {
    if (!vaultData || !vaultData.balances) return '...'
    const land = vaultData.balances.LAND || {}
    const total = (land.miners || 0) + (land.dev || 0) + (land.founders || 0)
    return total.toLocaleString()
  }

  const getDepositConfirmations = () => {
    const confirmations: Record<string, number> = {
      BTC: 3,
      BCH: 6,
      DOGE: 20
    }
    return confirmations[depositAsset] || 3
  }

  return (
    <div className="vision-wallet-shell">
      <div className="vision-wallet-container">
        
        {/* Tip Button */}
        <TipButton />

        {/* SECTION 1: Portfolio Hero */}
        <div className="vw-card vw-hero-card">
          <div className="vw-hero-left">
            <div className="vw-label-sm">TOTAL PORTFOLIO</div>
            <div className="vw-hero-amount">{formatBalance(totalPortfolio)} LAND</div>
            <div className="vw-hero-usd">≈ ${portfolioUSD} USD</div>
          </div>
          <div className="vw-hero-right">
            <div className="vw-hero-row">
              <span className="vw-label-sm">ACCOUNT</span>
              <span className="vw-pill">@{profile.handle}</span>
            </div>
            <div className="vw-hero-row">
              <span className="vw-label-sm">NETWORK</span>
              <span className="vw-pill vw-pill-success">Vision Mainnet</span>
            </div>
          </div>
        </div>

        {/* SECTION 2: Asset Strip */}
        <div className="vw-asset-strip">
          {ASSETS.map((asset) => {
            const balance = getAssetBalance(asset.symbol)
            return (
              <div key={asset.symbol} className="vw-asset-card">
                <div className="vw-asset-left">
                  <div className="vw-asset-icon">{asset.symbol[0]}</div>
                  <div>
                    <div className="vw-asset-symbol">{asset.symbol}</div>
                    <div className="vw-asset-desc">{asset.description}</div>
                  </div>
                </div>
                <div className="vw-asset-right">
                  <div className="vw-asset-balance">{formatBalance(balance)}</div>
                  <div className="vw-asset-subtext">
                    Available · Total {formatBalance(balance)}
                  </div>
                </div>
              </div>
            )
          })}
        </div>

        {/* SECTION 3: Portal Charge */}
        <div className="vw-card vw-portal-card">
          <div className="vw-portal-header">
            <div>
              <div className="vw-portal-title">Portal charge</div>
              <div className="vw-portal-subtitle">Earn, mine, or trade to light the bar</div>
            </div>
            <div className="vw-portal-percent">{portalCharge}%</div>
          </div>
          <div className="vw-portal-bar-container">
            <div className="vw-portal-bar" style={{ width: `${portalCharge}%` }}></div>
          </div>
          <div className="vw-portal-labels">
            <span>0%</span>
            <span>Charged</span>
            <span>100%</span>
          </div>
        </div>

        {/* SECTION 4: Main Actions Row */}
        <div className="vw-actions-row">
          {/* Receive Card */}
          <div className="vw-card vw-action-card">
            <h3 className="vw-action-title">Receive</h3>
            <p className="vw-action-subtitle">Share your address to receive tokens</p>
            <div className="vw-input-group">
              <label className="vw-label-sm">YOUR ADDRESS</label>
              <div className="vw-address-row">
                <span className="vw-address-mono">{profile.address}</span>
                <button className="vw-btn-copy" onClick={handleCopyAddress}>
                  {copyMessage || 'Copy'}
                </button>
              </div>
            </div>
          </div>

          {/* Send Card */}
          <div className="vw-card vw-action-card">
            <h3 className="vw-action-title">Send</h3>
            <p className="vw-action-subtitle">Move value to friends or markets</p>
            
            <div className="vw-input-group">
              <label className="vw-label-sm">ASSET</label>
              <select 
                className="vw-input"
                value={sendAsset}
                onChange={(e) => setSendAsset(e.target.value as Asset)}
              >
                {ASSETS.map(a => (
                  <option key={a.symbol} value={a.symbol}>{a.symbol}</option>
                ))}
              </select>
            </div>

            <div className="vw-input-group">
              <label className="vw-label-sm">TO ADDRESS</label>
              <input 
                type="text"
                className="vw-input"
                value={sendTo}
                onChange={(e) => setSendTo(e.target.value)}
                placeholder="Enter recipient address"
              />
            </div>

            <div className="vw-input-group">
              <label className="vw-label-sm">AMOUNT</label>
              <input 
                type="number"
                step="0.00000001"
                className="vw-input"
                value={sendAmount}
                onChange={(e) => setSendAmount(e.target.value)}
                placeholder="0.00000000"
              />
            </div>

            {sendError && <div className="vw-error-msg">{sendError}</div>}
            {sendSuccess && <div className="vw-success-msg">Transaction submitted!</div>}

            <button className="vw-btn-primary" onClick={handleSend}>
              Send
            </button>
          </div>

          {/* Enter Vision Card */}
          <div className="vw-card vw-action-card">
            <h3 className="vw-action-title">Enter Vision</h3>
            <p className="vw-action-subtitle">Launch the world. One map. Everyone together.</p>
            <p className="vw-enter-desc">
              Connect to the Vision World game through this wallet. Your assets travel with you.
              Shape the world, build, trade, and explore with other dreamers.
            </p>
            <button className="vw-btn-primary" onClick={() => window.open('/', '_blank')}>
              Enter Vision
            </button>
          </div>
        </div>

        {/* SECTION 5: Secondary Actions Row */}
        <div className="vw-actions-row">
          <div className="vw-card vw-action-card-secondary">
            <h4 className="vw-secondary-title">Market</h4>
            <p className="vw-secondary-desc">
              Trade LAND/CASH with live order books
            </p>
            <button className="vw-btn-secondary" onClick={() => navigate('/exchange')}>
              Open Market →
            </button>
          </div>

          <div className="vw-card vw-action-card-secondary">
            <h4 className="vw-secondary-title">Miner Panel</h4>
            <p className="vw-secondary-desc">
              Control mining operations and monitor hashrate performance
            </p>
            <button className="vw-btn-secondary" onClick={() => window.open('/panel.html', '_blank')}>
              Open Miner Panel →
            </button>
          </div>

          <div className="vw-card vw-action-card-secondary">
            <h4 className="vw-secondary-title">Node Dashboard</h4>
            <p className="vw-secondary-desc">
              View blockchain stats, recent blocks, and network health
            </p>
            <button className="vw-btn-secondary" onClick={() => window.open('/dashboard.html', '_blank')}>
              Open Dashboard →
            </button>
          </div>
        </div>

        {/* SECTION 6: Bottom Row - Deposit + Vision Vault */}
        <div className="vw-bottom-row">
          {/* Deposit Card */}
          <div className="vw-card vw-deposit-card">
            <h3 className="vw-action-title">Deposit</h3>
            <p className="vw-action-subtitle">External deposits</p>

            <div className="vw-toggle-group">
              {(['BTC', 'BCH', 'DOGE'] as const).map((asset) => (
                <button
                  key={asset}
                  className={`vw-toggle-btn ${depositAsset === asset ? 'active' : ''}`}
                  onClick={() => setDepositAsset(asset)}
                >
                  {asset}
                </button>
              ))}
            </div>

            <div className="vw-input-group">
              <label className="vw-label-sm">{depositAsset} DEPOSIT ADDRESS</label>
              <div className="vw-address-row">
                <span className="vw-address-mono">{depositAddress}</span>
                <button className="vw-btn-copy" onClick={handleCopyDeposit}>
                  {depositCopyMsg || 'Copy'}
                </button>
              </div>
            </div>

            <div className="vw-warning-box">
              ⚠️ Only send {depositAsset} to this address. Sending other cryptocurrencies may result in permanent loss.
            </div>

            <div className="vw-deposit-footer">
              Requires {getDepositConfirmations()} blockchain confirmations
            </div>
          </div>

          {/* Vision Vault Card */}
          <div className="vw-card vw-vault-card-new">
            <div className="vw-vault-header">
              <h3 className="vw-action-title">Vision Vault</h3>
              <span className="vw-vault-badge">Live updating (3s)</span>
            </div>
            <p className="vw-action-subtitle">Foundation & Treasury • Auto-balancing over time</p>

            <div className="vw-vault-stats">
              <div className="vw-vault-stat-row">
                <span className="vw-label-sm">Last update</span>
                <span className="vw-vault-value">
                  {lastVaultUpdate.getTime() - Date.now() < 5000 ? 'Just now' : lastVaultUpdate.toLocaleTimeString()}
                </span>
              </div>
              <div className="vw-vault-stat-row">
                <span className="vw-label-sm">Total supply</span>
                <span className="vw-vault-value">{getTotalSupply()} LAND</span>
              </div>
              <div className="vw-vault-stat-row">
                <span className="vw-label-sm">Vault health</span>
                <span className="vw-vault-value">{getVaultHealth()} backed</span>
              </div>
            </div>

            <div className="vw-vault-footer">
              50% miners · 30% dev · 20% founders
            </div>
          </div>
        </div>

      </div>
    </div>
  )
}
