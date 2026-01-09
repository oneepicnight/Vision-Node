import { useState, useEffect } from 'react'

interface ExchangeStatus {
  enabled: boolean
  reason?: string
  needs?: {
    asset: string
    min_confirmations: number
    detected: boolean
  }
}

const POLL_INTERVAL = 15000 // 15 seconds

export function useExchangeStatus(walletAddress: string | null) {
  const [status, setStatus] = useState<ExchangeStatus>({
    enabled: false,
    reason: 'Loading...'
  })
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    if (!walletAddress) {
      setStatus({
        enabled: false,
        reason: 'No wallet connected'
      })
      setIsLoading(false)
      return
    }

    let isMounted = true
    let pollTimer: NodeJS.Timeout

    const fetchStatus = async () => {
      try {
        const response = await fetch(
          `http://localhost:7070/api/exchange_status?user_id=${encodeURIComponent(walletAddress)}`
        )
        
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`)
        }

        const data = await response.json()
        
        if (isMounted) {
          setStatus(data)
          setIsLoading(false)
        }
      } catch (error) {
        console.error('Failed to fetch exchange status:', error)
        if (isMounted) {
          setStatus({
            enabled: false,
            reason: 'Exchange unlocks after your deposit is confirmed.'
          })
          setIsLoading(false)
        }
      }
    }

    // Initial fetch
    fetchStatus()

    // Poll every 15 seconds
    pollTimer = setInterval(fetchStatus, POLL_INTERVAL)

    return () => {
      isMounted = false
      clearInterval(pollTimer)
    }
  }, [walletAddress])

  return { status, isLoading }
}
