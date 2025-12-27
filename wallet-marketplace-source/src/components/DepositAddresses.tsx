import { useState, useEffect } from 'react'
import { useWalletStore } from '../state/wallet'
import { getDepositAddress } from '../lib/api'
import QRCode from 'qrcode'
import { Copy, Check, Download } from 'lucide-react'

type Currency = 'BTC' | 'BCH' | 'DOGE'

export function DepositAddresses() {
  const { profile } = useWalletStore()
  const [selectedCurrency, setSelectedCurrency] = useState<Currency>('BTC')
  const [depositAddress, setDepositAddress] = useState<string>('')
  const [network, setNetwork] = useState<string>('')
  const [confirmations, setConfirmations] = useState<number>(6)
  const [qrDataUrl, setQrDataUrl] = useState<string>('')
  const [copied, setCopied] = useState(false)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    if (!profile) return

    const loadAddress = async () => {
      setLoading(true)
      try {
        const data = await getDepositAddress(selectedCurrency, profile.address)
        setDepositAddress(data.address)
        setNetwork(selectedCurrency)
        setConfirmations(6)

        // Generate QR code
        try {
          // Create a canvas element for QR code generation
          const canvas = document.createElement('canvas')
          await QRCode.toCanvas(canvas, data.address, {
            width: 256,
            margin: 2,
            color: {
              dark: '#000000',
              light: '#FFFFFF'
            }
          })
          // Convert canvas to data URL
          const qr = canvas.toDataURL('image/png')
          setQrDataUrl(qr)
        } catch (qrError) {
          console.error('Failed to generate QR code:', qrError)
          // Set a fallback or empty string to prevent crashes
          setQrDataUrl('')
        }
      } catch (error) {
        console.error('Failed to load deposit address:', error)
      } finally {
        setLoading(false)
      }
    }

    loadAddress()
  }, [profile, selectedCurrency])

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(depositAddress)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (error) {
      console.error('Copy failed:', error)
    }
  }

  const handleDownloadQR = () => {
    const link = document.createElement('a')
    link.href = qrDataUrl
    link.download = `${selectedCurrency}-deposit-${Date.now()}.png`
    link.click()
  }

  if (!profile) return null

  const currencies: { key: Currency; name: string; symbol: string }[] = [
    { key: 'BTC', name: 'Bitcoin', symbol: '₿' },
    { key: 'BCH', name: 'Bitcoin Cash', symbol: 'BCH' },
    { key: 'DOGE', name: 'Dogecoin', symbol: 'Ð' }
  ]

  return (
    <div className="card">
      <h3 className="text-lg font-semibold mb-4">Deposit Addresses</h3>

      {/* Currency Selector */}
      <div className="flex gap-2 mb-6">
        {currencies.map(({ key, name, symbol }) => (
          <button
            key={key}
            onClick={() => setSelectedCurrency(key)}
            className={`flex-1 px-4 py-2 rounded-lg border transition-all ${
              selectedCurrency === key
                ? 'bg-accent/20 border-accent text-accent font-semibold'
                : 'bg-black/20 border-white/10 hover:border-white/20'
            }`}
          >
            <div className="text-lg">{symbol}</div>
            <div className="text-xs opacity-70">{name}</div>
          </button>
        ))}
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-8">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-accent"></div>
        </div>
      ) : (
        <>
          {/* QR Code */}
          <div className="flex justify-center mb-6">
            <div className="bg-white p-4 rounded-lg">
              {qrDataUrl ? (
                <img src={qrDataUrl} alt={`${selectedCurrency} deposit address QR`} className="w-64 h-64" />
              ) : (
                <div className="w-64 h-64 flex items-center justify-center bg-gray-100 rounded">
                  <span className="text-gray-500 text-sm">QR Code Unavailable</span>
                </div>
              )}
            </div>
          </div>

          {/* Address Display */}
          <div className="bg-black/30 rounded-lg p-4 mb-4">
            <div className="text-xs opacity-70 mb-2">Deposit Address</div>
            <div className="font-mono text-sm break-all mb-3">{depositAddress}</div>
            
            <div className="flex gap-2">
              <button
                onClick={handleCopy}
                className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-accent/20 hover:bg-accent/30 border border-accent/30 rounded-lg transition-all"
              >
                {copied ? (
                  <>
                    <Check size={16} />
                    <span>Copied!</span>
                  </>
                ) : (
                  <>
                    <Copy size={16} />
                    <span>Copy Address</span>
                  </>
                )}
              </button>
              
              <button
                onClick={handleDownloadQR}
                className="px-4 py-2 bg-black/30 hover:bg-black/40 border border-white/10 rounded-lg transition-all"
                title="Download QR Code"
              >
                <Download size={16} />
              </button>
            </div>
          </div>

          {/* Network Info */}
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="opacity-70">Network:</span>
              <span className="font-semibold">{network}</span>
            </div>
            <div className="flex justify-between">
              <span className="opacity-70">Confirmations Required:</span>
              <span className="font-semibold">{confirmations}</span>
            </div>
          </div>

          {/* Warning */}
          <div className="mt-4 p-3 bg-yellow-500/10 border border-yellow-500/30 rounded-lg text-xs">
            <strong>⚠️ Important:</strong> Only send {selectedCurrency} to this address. Sending other cryptocurrencies may result in permanent loss.
          </div>
        </>
      )}
    </div>
  )
}
