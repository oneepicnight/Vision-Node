import { useState, useEffect } from 'react'

export interface NodeStatus {
  online: boolean
  network: string
  height: number
  guardianMode: boolean
  peerCount: number
  p2pHealth: 'isolated' | 'weak' | 'ok' | 'stable' | 'immortal'
}

export function useNodeStatus(pollInterval = 5000) {
  const [nodeStatus, setNodeStatus] = useState<NodeStatus>({
    online: false,
    network: 'Constellation Testnet',
    height: 0,
    guardianMode: false,
    peerCount: 0,
    p2pHealth: 'isolated'
  })

  useEffect(() => {
    const fetchNodeStatus = async () => {
      try {
        // Fetch both status and constellation status
        const [statusRes, constellationRes] = await Promise.all([
          fetch('http://127.0.0.1:7070/api/status'),
          fetch('http://127.0.0.1:7070/constellation/status').catch(() => null)
        ])
        const data = await statusRes.json()
        const constellationData = constellationRes ? await constellationRes.json() : null
        
        setNodeStatus({
          online: data.live,
          network: data.guardian_mode ? 'Guardian Mode' : 'Constellation Testnet',
          height: data.chain_height,
          guardianMode: data.guardian_mode || false,
          peerCount: data.peers ? data.peers.length : 0,
          p2pHealth: constellationData?.p2p_health || 'isolated'
        })
      } catch (err) {
        console.debug('Failed to fetch node status:', err)
        setNodeStatus(prev => ({ ...prev, online: false }))
      }
    }

    fetchNodeStatus()
    const interval = setInterval(fetchNodeStatus, pollInterval)
    return () => clearInterval(interval)
  }, [pollInterval])

  return nodeStatus
}
