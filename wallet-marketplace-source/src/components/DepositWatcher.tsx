import { useEffect, useState } from 'react'
import './DepositWatcher.css'

type DepositStatus = 'WAITING' | 'DETECTED' | 'CONFIRMING' | 'CONFIRMED'

interface DepositInfo {
  status: DepositStatus
  amount?: number
  confirmations?: number
  requiredConfirmations: number
  txid?: string
}

interface DepositWatcherProps {
  coin: 'BTC' | 'BCH' | 'DOGE'
  address: string
  isActive: boolean
}

const CONFIRMATIONS_REQUIRED = {
  BTC: 3,
  BCH: 6,
  DOGE: 20
}

export function DepositWatcher({ coin, address, isActive }: DepositWatcherProps) {
  const [depositInfo, setDepositInfo] = useState<DepositInfo>({
    status: 'WAITING',
    requiredConfirmations: CONFIRMATIONS_REQUIRED[coin]
  })

  useEffect(() => {
    if (!isActive || !address || address.startsWith('Error')) {
      return
    }

    let cancelled = false

    const checkDeposit = async () => {
      try {
        // TODO: Replace with actual API call to check deposit status
        // For now, this is a placeholder that would call something like:
        // const response = await fetch(`/api/wallet/deposit/status?address=${address}&coin=${coin}`)
        // const data = await response.json()
        
        // Mock response structure for development:
        // {
        //   has_pending: boolean,
        //   amount: number,
        //   confirmations: number,
        //   txid: string
        // }
        
        if (!cancelled) {
          // Placeholder - in production this would update based on real blockchain data
          setDepositInfo(prev => ({
            ...prev,
            requiredConfirmations: CONFIRMATIONS_REQUIRED[coin]
          }))
        }
      } catch (error) {
        console.error(`Failed to check ${coin} deposit:`, error)
      }
    }

    // Initial check
    checkDeposit()

    // Poll every 15 seconds
    const interval = setInterval(checkDeposit, 15000)

    return () => {
      cancelled = true
      clearInterval(interval)
    }
  }, [coin, address, isActive])

  if (depositInfo.status === 'WAITING') {
    return (
      <div className="deposit-watcher state-waiting">
        <div className="deposit-watcher-header">
          <div className="deposit-watcher-icon">⏳</div>
          <h4 className="deposit-watcher-title state-waiting">Waiting for deposit…</h4>
        </div>
        <p className="deposit-watcher-message state-waiting">
          No incoming transactions detected yet.
        </p>
      </div>
    )
  }

  if (depositInfo.status === 'DETECTED' || depositInfo.status === 'CONFIRMING') {
    const progress = ((depositInfo.confirmations || 0) / depositInfo.requiredConfirmations) * 100

    return (
      <div className="deposit-watcher state-confirming">
        <div className="deposit-watcher-header">
          <div className="deposit-watcher-icon">📥</div>
          <h4 className="deposit-watcher-title state-confirming">Deposit detected!</h4>
        </div>
        
        {depositInfo.amount && (
          <p className="deposit-watcher-message state-confirming">
            <span className="deposit-watcher-amount">{depositInfo.amount} {coin}</span>
          </p>
        )}

        <div className="deposit-watcher-progress">
          <div className="deposit-watcher-progress-label">
            <span>Confirmations</span>
            <span>{depositInfo.confirmations || 0} / {depositInfo.requiredConfirmations}</span>
          </div>
          <div className="deposit-watcher-progress-bar">
            <div 
              className="deposit-watcher-progress-fill" 
              style={{ width: `${Math.min(progress, 100)}%` }}
            />
          </div>
        </div>

        {depositInfo.txid && (
          <div className="deposit-watcher-tx">
            Txid: <a href="#" className="deposit-watcher-tx-link">{depositInfo.txid.slice(0, 8)}...{depositInfo.txid.slice(-8)}</a>
          </div>
        )}
      </div>
    )
  }

  if (depositInfo.status === 'CONFIRMED') {
    return (
      <div className="deposit-watcher state-confirmed">
        <div className="deposit-watcher-header">
          <div className="deposit-watcher-icon">✅</div>
          <h4 className="deposit-watcher-title state-confirmed">Deposit confirmed!</h4>
        </div>
        {depositInfo.amount && (
          <p className="deposit-watcher-message state-confirmed">
            <span className="deposit-watcher-amount">{depositInfo.amount} {coin}</span> available in wallet
          </p>
        )}
      </div>
    )
  }

  return null
}
