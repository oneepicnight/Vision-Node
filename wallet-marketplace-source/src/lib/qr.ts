import * as QRCode from 'qrcode'

export async function makeQR(data: string): Promise<string> {
  try {
    const dataUrl = await QRCode.toDataURL(data, {
      width: 256,
      margin: 2,
      color: {
        dark: '#000000',
        light: '#FFFFFF'
      },
      errorCorrectionLevel: 'M'
    })
    return dataUrl
  } catch (error) {
    console.error('QR generation failed:', error)
    throw new Error('Failed to generate QR code')
  }
}