import { useState, useEffect } from 'react'
import "../styles/wallet-aaa.css"

interface LinkDiscordStepProps {
  walletAddress: string
  onSkip: () => void
  onLinked: () => void
}

interface DiscordStatus {
  linked: boolean
  discord_user_id?: string
  discord_username?: string
}

export default function LinkDiscordStep({ walletAddress, onSkip, onLinked }: LinkDiscordStepProps) {
  const [status, setStatus] = useState<DiscordStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Check Discord link status on mount
  useEffect(() => {
    checkStatus()
  }, [walletAddress])

  const checkStatus = async () => {
    try {
      setLoading(true)
      setError(null)
      
      const response = await fetch(
        `/api/discord/status?wallet_address=${encodeURIComponent(walletAddress)}`
      )
      
      if (!response.ok) {
        throw new Error('Failed to check Discord status')
      }
      
      const data: DiscordStatus = await response.json()
      setStatus(data)
      
      // If already linked, auto-continue after a moment
      if (data.linked) {
        setTimeout(() => {
          onLinked()
        }, 2000)
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error')
    } finally {
      setLoading(false)
    }
  }

  const handleLinkDiscord = async () => {
    try {
      setLoading(true)
      setError(null)
      
      // Get Discord OAuth URL from backend
      const response = await fetch(
        `/api/discord/login?wallet_address=${encodeURIComponent(walletAddress)}`
      )
      
      if (!response.ok) {
        throw new Error('Failed to initiate Discord linking')
      }
      
      const data = await response.json()
      
      // Redirect to Discord OAuth
      window.location.href = data.url
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error')
      setLoading(false)
    }
  }

  const shortenAddress = (addr: string) => {
    if (addr.length <= 20) return addr
    return `${addr.slice(0, 12)}...${addr.slice(-8)}`
  }

  if (loading && !status) {
    return (
      <div className="vision-landing">
        <div className="vision-landing--inner">
          <div className="import-panel" style={{ textAlign: 'center' }}>
            <div className="vision-title-sub-small" style={{ marginBottom: '2rem' }}>
              Checking Discord Status...
            </div>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="vision-landing">
      <div className="vision-landing--inner">
        <div className="import-panel">
          <div className="vision-title-sub-small" style={{ marginBottom: '1rem' }}>
            {status?.linked ? "You're all set, Dreamer." : "You're ready, Dreamer."}
          </div>

          {!status?.linked && (
            <p style={{ 
              fontSize: '0.9rem', 
              color: 'rgba(255,255,255,0.6)', 
              marginBottom: '2rem',
              lineHeight: 1.6
            }}>
              Optionally connect your Discord so the Guardian can salute your node and track events like First Contact.
            </p>
          )}

          {/* Status Card */}
          <div style={{
            background: 'rgba(0,0,0,0.3)',
            border: '1px solid rgba(255,255,255,0.1)',
            borderRadius: '8px',
            padding: '1.5rem',
            marginBottom: '2rem'
          }}>
            <div style={{ marginBottom: '1rem' }}>
              <div style={{ 
                fontSize: '0.75rem', 
                textTransform: 'uppercase', 
                letterSpacing: '0.1em',
                color: 'rgba(255,255,255,0.5)',
                marginBottom: '0.5rem'
              }}>
                Wallet Address
              </div>
              <div style={{ 
                fontFamily: 'monospace', 
                fontSize: '0.9rem',
                color: 'rgba(255,255,255,0.9)'
              }}>
                {shortenAddress(walletAddress)}
              </div>
            </div>

            <div>
              <div style={{ 
                fontSize: '0.75rem', 
                textTransform: 'uppercase', 
                letterSpacing: '0.1em',
                color: 'rgba(255,255,255,0.5)',
                marginBottom: '0.5rem'
              }}>
                Discord Status
              </div>
              <div style={{ 
                fontSize: '0.9rem',
                color: status?.linked ? '#4ade80' : 'rgba(255,255,255,0.5)'
              }}>
                {status?.linked ? (
                  <>
                    ✅ Linked to {status.discord_username}
                  </>
                ) : (
                  '❌ Not linked'
                )}
              </div>
            </div>
          </div>

          {/* Error Message */}
          {error && (
            <div style={{
              background: 'rgba(239, 68, 68, 0.1)',
              border: '1px solid rgba(239, 68, 68, 0.3)',
              borderRadius: '6px',
              padding: '1rem',
              marginBottom: '1.5rem',
              color: '#ef4444',
              fontSize: '0.85rem'
            }}>
              {error}
            </div>
          )}

          {/* Buttons */}
          <div className="import-buttons">
            {status?.linked ? (
              <button
                type="button"
                className="btn-primary"
                onClick={onLinked}
                style={{ width: 'auto', minWidth: '200px' }}
              >
                Continue to Wallet
              </button>
            ) : (
              <>
                <button
                  type="button"
                  onClick={onSkip}
                  style={{
                    background: 'transparent',
                    border: 'none',
                    color: 'rgba(255,255,255,0.5)',
                    cursor: 'pointer',
                    fontSize: '0.9rem',
                    textDecoration: 'underline'
                  }}
                >
                  Skip for now
                </button>
                <button
                  type="button"
                  className="btn-primary"
                  onClick={handleLinkDiscord}
                  disabled={loading}
                  style={{ width: 'auto', minWidth: '200px' }}
                >
                  {loading ? 'Connecting...' : 'LINK DISCORD'}
                </button>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
