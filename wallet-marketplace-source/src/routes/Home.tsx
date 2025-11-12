import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useWalletStore } from '../state/wallet'
import { getBalance, submitTx, type Transaction, type SignedTransaction } from '../lib/api'
import { loadAndDecrypt } from '../lib/keystore'
import { isValidAddress, isPositiveAmount } from '../lib/guards'
import * as secp256k1 from '@noble/secp256k1'
import ExchangePage from '../modules/exchange/Exchange'
import { useExchange } from '../modules/exchange/store'

export default function Home() {
  const navigate = useNavigate()
  const { profile, balances, setBalances } = useWalletStore()
  const [sendToken, setSendToken] = useState('LAND')
  const [sendTo, setSendTo] = useState('')
  const [sendAmount, setSendAmount] = useState('')
  const [sendMessage, setSendMessage] = useState('')
  const [sendLoading, setSendLoading] = useState(false)
  const [copyMessage, setCopyMessage] = useState('')

  // Redirect to splash if no profile
  useEffect(() => {
    if (!profile) {
      navigate('/')
    }
  }, [profile, navigate])

  // Load balances from node and poll every 5 seconds
  useEffect(() => {
    if (!profile) return

    const loadBalances = async () => {
      try {
        const nodeBalances = await getBalance(profile.address)
        setBalances(nodeBalances)
      } catch (error) {
        console.error('Failed to load balances:', error)
        // Keep using local balances on error
      }
    }

    // Load immediately
    loadBalances()
    
    // Then poll every 5 seconds
    const interval = setInterval(loadBalances, 5000)
    
    return () => clearInterval(interval)
  }, [profile, setBalances])

  const handleCopyAddress = async () => {
    if (!profile) return

    try {
      await navigator.clipboard.writeText(profile.address)
      setCopyMessage('Copied!')
      setTimeout(() => setCopyMessage(''), 1200)
    } catch (error) {
      console.error('Copy failed:', error)
      setCopyMessage('Copy failed')
      setTimeout(() => setCopyMessage(''), 1200)
    }
  }

  const handleSend = async () => {
    if (!profile || sendLoading) return

    // Validate inputs
    const toAddress = sendTo.trim()
    const amount = parseInt(sendAmount, 10)

    if (!isValidAddress(toAddress)) {
      setSendMessage('Invalid address')
      return
    }

    if (!isPositiveAmount(amount)) {
      setSendMessage('Enter amount > 0')
      return
    }

    if (balances[sendToken as keyof typeof balances] < amount) {
      setSendMessage('Insufficient balance')
      return
    }

    setSendLoading(true)
    setSendMessage('Preparing transaction...')

    try {
      // Load private key
      const keystore = await loadAndDecrypt()
      if (!keystore) {
        throw new Error('Could not decrypt keystore')
      }

      // Create transaction
      const tx: Transaction = {
        token: sendToken,
        to: toAddress,
        amount,
        from: profile.address,
        nonce: Date.now()
      }

      // Sign transaction hash
      const txHash = JSON.stringify(tx)
      const encoder = new TextEncoder()
      const txBytes = encoder.encode(txHash)
      const hashBytes = await crypto.subtle.digest('SHA-256', txBytes)
      const privateKeyBytes = new Uint8Array(keystore.privateKeyHex.match(/.{1,2}/g)!.map(byte => parseInt(byte, 16)))
      
      const signature = await secp256k1.sign(new Uint8Array(hashBytes), privateKeyBytes)
      const sigBytes = signature.toCompactRawBytes()
      const signedTx: SignedTransaction = {
        tx,
        sig: Array.from(sigBytes).map(b => b.toString(16).padStart(2, '0')).join('')
      }

      // Submit to node
      setSendMessage('Submitting transaction...')
      const result = await submitTx(signedTx)

      if (result.ok) {
        // Optimistic update of balance
        setBalances({ [sendToken]: balances[sendToken as keyof typeof balances] - amount })
        setSendMessage(`Submitted ✔ TX: ${result.txid}`)
        setSendTo('')
        setSendAmount('')
      } else {
        setSendMessage(`Failed: ${result.txid}`)
      }
    } catch (error) {
      console.error('Send failed:', error)
      setSendMessage(`Error: ${error instanceof Error ? error.message : 'Unknown error'}`)
    } finally {
      setSendLoading(false)
    }
  }

  const handleEnterVision = () => {
    if (!profile) return

    // Try deep link first
    const deepLink = `vision://enter?address=${encodeURIComponent(profile.address)}&handle=${encodeURIComponent(profile.handle)}`
    
    // Show connecting toast
    const showToast = (message: string) => {
      // Simple toast implementation
      const toast = document.createElement('div')
      toast.textContent = message
      toast.className = 'fixed top-16 right-4 bg-accent/90 text-white px-4 py-2 rounded-lg shadow-lg z-50'
      document.body.appendChild(toast)
      setTimeout(() => toast.remove(), 3000)
    }

    showToast('Connecting to portal...')
    
    try {
      // Try the deep link
      window.location.href = deepLink
      
      // Fallback after a short delay if deep link doesn't work
      setTimeout(() => {
        const fallbackUrl = 'http://127.0.0.1:5173/vision'
        window.open(fallbackUrl, '_blank')
      }, 1000)
    } catch (error) {
      // Immediate fallback
      const fallbackUrl = 'http://127.0.0.1:5173/vision'
      window.open(fallbackUrl, '_blank')
    }
  }

  if (!profile) {
    return null // Will redirect in useEffect
  }

  return (
    <div className="home-container">
      <div className="home-content">
        {/* Header */}
        <div className="home-section">
          <div className="card header-card">
            <div className="header-left">
              <div className="wallet-icon">V</div>
              <span style={{ fontWeight: 'bold' }}>Vision Wallet</span>
            </div>
            <div className="handle-text">@{profile.handle}</div>
          </div>
        </div>

        {/* Balance Orbs */}
        <div className="home-section">
          <div className="balance-grid">
            <div className="balance-orb land">
              <div className="balance-value">{balances.LAND}</div>
              <div className="balance-label">LAND</div>
            </div>
            <div className="balance-orb cash">
              <div className="balance-value">{balances.CASH}</div>
              <div className="balance-label">CASH</div>
            </div>
          </div>
        </div>

        {/* Mission Progress */}
        <div className="home-section">
          <div className="card">
            <div className="progress-section">
              <div className="progress-title">Portal charge</div>
              <div className="progress-subtitle">Earn, mine, or trade to light the bar.</div>
            </div>
            <div className="progress-bar">
              <div className="progress-fill"></div>
            </div>
          </div>
        </div>

          {/* Action Cards */}
        <div className="home-section">
          <div className="action-grid">
            {/* Receive */}
            <div className="action-card">
              <h3 className="action-title">Receive</h3>
              <p className="action-description">Share your address to receive tokens.</p>
              <div className="address-section">
                <div className="address-display">{profile.address}</div>
                <button onClick={handleCopyAddress} className="copy-button">
                  {copyMessage || 'Copy'}
                </button>
              </div>
            </div>

            {/* Send */}
            <div className="action-card">
              <h3 className="action-title">Send</h3>
              <p className="action-description">Move value to friends or markets.</p>
              <div className="send-form">
                <select 
                  value={sendToken}
                  onChange={(e) => setSendToken(e.target.value)}
                  className="send-select"
                  disabled={sendLoading}
                >
                  <option value="LAND">LAND</option>
                  <option value="CASH">CASH</option>
                </select>
                <input 
                  placeholder="to: addr..." 
                  value={sendTo}
                  onChange={(e) => setSendTo(e.target.value)}
                  className="send-input" 
                  disabled={sendLoading}
                />
                <input 
                  placeholder="amount" 
                  type="number" 
                  min="0" 
                  step="1"
                  value={sendAmount}
                  onChange={(e) => setSendAmount(e.target.value)}
                  className="send-input" 
                  disabled={sendLoading}
                />
                <button 
                  onClick={handleSend}
                  disabled={sendLoading}
                  className="send-button"
                >
                  {sendLoading ? 'Sending...' : 'Send'}
                </button>
              </div>
              {sendMessage && (
                <div className={`send-message ${sendMessage.includes('✔') ? 'success' : sendMessage.includes('Error') || sendMessage.includes('Failed') ? 'error' : 'warning'}`}>
                  {sendMessage}
                </div>
              )}
            </div>

            {/* Enter Vision */}
            <div className="action-card">
              <h3 className="action-title">Enter Vision</h3>
              <p className="action-description">Launch the world. One map. Everyone together.</p>
              <button onClick={handleEnterVision} className="enter-vision-button">
                Enter Vision
              </button>
            </div>
          </div>
        </div>

        {/* Embedded Market (mini exchange inside wallet) */}
        <div className="home-section">
          <div className="card">
            <h3 className="action-title">Market</h3>
            <p className="action-description">Quick market view — orderbook, depth chart and trade ticket.</p>
            <div style={{ marginTop: 12 }}>
              <ExchangePage onPickPrice={(p)=>{
                // Use exchange chain/quote to map clicked price into the send form
                const { quote } = useExchange.getState();
                // If quote is CASH, user will spend CASH to buy the base asset
                setSendToken(quote);
                // For LAND we prefer integer amounts; otherwise round to 2
                const amt = quote === 'CASH' ? Math.round(p) : Math.round(p * 100) / 100;
                setSendAmount(String(amt));
              }} />
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}