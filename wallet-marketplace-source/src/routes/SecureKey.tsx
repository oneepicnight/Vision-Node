import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useOnboardingStore } from '../state/onboarding'
import { useWalletStore } from '../state/wallet'
import { deriveKeys, encryptAndSave } from '../lib/keystore'
import { tryKeysThenVault } from '../lib/api'
import { env } from '../utils/env'
import { makeQR } from '../lib/qr'

export default function SecureKey() {
  const navigate = useNavigate()
  const { handle, mnemonic, reset } = useOnboardingStore()
  const { setProfile } = useWalletStore()
  const [confirmed, setConfirmed] = useState(false)
  const [qrDataUrl, setQrDataUrl] = useState('')
  const [loading, setLoading] = useState(false)

  // Redirect if no mnemonic
  useEffect(() => {
    if (!mnemonic.length || !handle) {
      navigate('/handle')
      return
    }

    // Generate QR code
    const generateQR = async () => {
      try {
        console.log('Generating QR code...')
        const keys = await deriveKeys(mnemonic)
        const backupData = `vision:backup:${keys.address}:${handle}`
        console.log('QR data:', backupData)
        const qrUrl = await makeQR(backupData)
        console.log('QR generated successfully')
        setQrDataUrl(qrUrl)
      } catch (error) {
        console.error('QR generation failed:', error)
        // Create a simple text-based QR fallback
        const fallbackData = `Wallet: ${handle}\nAddress: Loading...`
        try {
          const fallbackQR = await makeQR(fallbackData)
          setQrDataUrl(fallbackQR)
        } catch (fallbackError) {
          console.error('Fallback QR also failed:', fallbackError)
        }
      }
    }

    generateQR()
  }, [mnemonic, handle, navigate])

  const handleContinue = async () => {
    if (!confirmed) return
    
    setLoading(true)
    
    try {
      console.log('=== WALLET CREATION STARTED ===')
      console.log('Mnemonic length:', mnemonic.length)
      console.log('Handle:', handle)
      
      // Derive keys from mnemonic
      const keys = await deriveKeys(mnemonic)
      console.log('Keys generated:', { address: keys.address })
      
      // Encrypt and save keystore
      await encryptAndSave({
        mnemonic,
        privateKeyHex: keys.privateKeyHex
      })
      console.log('Keystore saved')
      
      // Create profile
      const profile = {
        handle,
        address: keys.address,
        createdAt: Date.now()
      }
      
      // Save profile to wallet store
      setProfile(profile)
      console.log('Profile set:', profile)
      
      // Verify profile was saved
      const { profile: savedProfile } = useWalletStore.getState()
      console.log('Saved profile verification:', savedProfile)
      
      // Give starter airdrop
      const { setBalances } = useWalletStore.getState()
      setBalances({ LAND: 1, GAME: 250, CASH: 500 })
      console.log('Balances set')
      
      // Navigate to Command Center after wallet creation
      navigate('/command-center')
      setTimeout(() => reset(), 100) // Delay reset to ensure navigation completes
      
      // Probe node in background
      if (!env.WALLET_DEV_BYPASS) {
        try {
          await tryKeysThenVault()
        } catch (probeErr) {
          console.warn('Node probe failed after wallet creation:', probeErr)
          window.pushToast?.('Node appears offline — running in DEV fallback. Balances/receipts may be stale.', 'info')
        }
      }
    } catch (error) {
      console.error('Wallet creation failed:', error)
      window.pushToast?.(`Failed to create wallet: ${error instanceof Error ? error.message : 'Unknown error'}`, 'error')
    } finally {
      setLoading(false)
    }
  }

  if (!mnemonic.length || !handle) {
    return null // Will redirect in useEffect
  }

  return (
    <div className="page-container">
      <div className="form-container-wide">
        <div>
          <h2 className="form-title">
            Secure your <span className="form-accent">soul key</span>
          </h2>
          <p className="form-subtitle">
            Write these 12 words down. They recover your wallet.
          </p>
        </div>

        <div className="seed-grid">
          {mnemonic.map((word, index) => (
            <div key={index} className="seed-word">
              {index + 1}. {word}
            </div>
          ))}
        </div>

        <div className="qr-section">
          <div className="qr-container">
            {qrDataUrl ? (
              <img src={qrDataUrl} alt="Backup QR" className="qr-image" />
            ) : (
              <div className="qr-loading">Loading...</div>
            )}
          </div>
          <div className="qr-description">
            <p>Encrypted backup QR (save it somewhere safe)</p>
          </div>
        </div>

        <label className="checkbox-container">
          <input 
            type="checkbox" 
            className="checkbox"
            checked={confirmed}
            onChange={(e) => setConfirmed(e.target.checked)}
          />
          <span>I saved my recovery words and QR.</span>
        </label>

        <div>
          <button 
            onClick={handleContinue}
            disabled={!confirmed || loading}
            className="primary-button"
          >
            {loading ? 'Creating Wallet...' : 'Continue'}
          </button>
          
          <button 
            onClick={() => navigate('/handle')}
            disabled={loading}
            className="secondary-button"
          >
            Back
          </button>
        </div>
      </div>
    </div>
  )
}