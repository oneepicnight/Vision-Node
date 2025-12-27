import { useState, useEffect } from 'react'

export interface MiningStatus {
  mode: 'solo' | 'pool' | 'off'
  hashrate: number
  active: boolean
}

export function useMiningStatus(pollInterval = 3000) {
  const [miningStatus, setMiningStatus] = useState<MiningStatus>({
    mode: 'off',
    hashrate: 0,
    active: false
  })

  useEffect(() => {
    const fetchMiningStatus = async () => {
      try {
        const response = await fetch('http://127.0.0.1:7070/api/miner/status')
        const data = await response.json()
        setMiningStatus({
          mode: data.pool_mining ? 'pool' : (data.active ? 'solo' : 'off'),
          hashrate: data.hashrate || 0,
          active: data.active
        })
      } catch (err) {
        console.debug('Failed to fetch mining status:', err)
      }
    }

    fetchMiningStatus()
    const interval = setInterval(fetchMiningStatus, pollInterval)
    return () => clearInterval(interval)
  }, [pollInterval])

  return miningStatus
}
