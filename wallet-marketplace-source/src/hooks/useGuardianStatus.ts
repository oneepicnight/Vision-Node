import { useState, useEffect } from 'react'

export interface GuardianStatus {
  beaconConnected: boolean
  guardianOnline: boolean
  lastHeartbeat?: number
}

export function useGuardianStatus(pollInterval = 10000) {
  const [guardianStatus, setGuardianStatus] = useState<GuardianStatus>({
    beaconConnected: false,
    guardianOnline: false
  })

  useEffect(() => {
    const fetchGuardianStatus = async () => {
      try {
        const moodResponse = await fetch('http://127.0.0.1:7070/api/mood')
        const moodData = await moodResponse.json()
        
        setGuardianStatus({
          beaconConnected: moodData.details?.guardian_active || false,
          guardianOnline: moodData.details?.guardian_active || false,
          lastHeartbeat: undefined
        })
      } catch (err) {
        console.debug('Guardian/Beacon fetch failed:', err)
      }
    }

    fetchGuardianStatus()
    const interval = setInterval(fetchGuardianStatus, pollInterval)
    return () => clearInterval(interval)
  }, [pollInterval])

  return guardianStatus
}
