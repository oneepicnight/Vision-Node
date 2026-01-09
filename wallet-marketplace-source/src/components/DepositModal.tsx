import { useEffect, useRef } from 'react'
import QRCode from 'qrcode'
import './DepositModal.css'

interface DepositModalProps {
  isOpen: boolean
  onClose: () => void
  coin: 'LAND' | 'CASH' | 'BTC' | 'BCH' | 'DOGE'
  address: string
  network?: string
}

export function DepositModal({ isOpen, onClose, coin, address, network = 'Mainnet' }: DepositModalProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    if (isOpen && canvasRef.current && address && !address.startsWith('Error')) {
      QRCode.toCanvas(canvasRef.current, address, {
        width: 256,
        margin: 2,
        color: {
          dark: '#000000',
          light: '#ffffff'
        }
      }).catch(err => {
        console.error('Failed to generate QR code:', err)
      })
    }
  }, [isOpen, address])

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onClose()
      }
    }

    if (isOpen) {
      document.addEventListener('keydown', handleEscape)
      document.body.style.overflow = 'hidden'
    }

    return () => {
      document.removeEventListener('keydown', handleEscape)
      document.body.style.overflow = 'unset'
    }
  }, [isOpen, onClose])

  if (!isOpen) return null

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(address)
      const btn = document.getElementById('qr-copy-btn')
      if (btn) {
        const originalText = btn.textContent
        btn.textContent = 'Copied!'
        setTimeout(() => {
          btn.textContent = originalText
        }, 1200)
      }
    } catch (error) {
      console.error('Copy failed:', error)
    }
  }

  return (
    <div className="deposit-modal-overlay" onClick={onClose}>
      <div className="deposit-modal-content" onClick={e => e.stopPropagation()}>
        <button className="deposit-modal-close" onClick={onClose}>
          ✕
        </button>
        
        <div className="deposit-modal-header">
          <h3 className="deposit-modal-title">{coin} Receive Address</h3>
          <p className="deposit-modal-subtitle">{network}</p>
        </div>

        <div className="deposit-modal-qr-container">
          <canvas ref={canvasRef} />
        </div>

        <div className="deposit-modal-address-display">
          <div className="deposit-modal-address-label">Address</div>
          <div className="deposit-modal-address-text">{address}</div>
        </div>

        <button 
          id="qr-copy-btn"
          className="deposit-modal-copy-btn"
          onClick={handleCopy}
        >
          Copy Address
        </button>
      </div>
    </div>
  )
}
