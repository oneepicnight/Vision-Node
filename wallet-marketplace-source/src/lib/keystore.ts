import * as bip39 from 'bip39'
import * as secp256k1 from '@noble/secp256k1'
import { set, get } from 'idb-keyval'
import { Buffer } from 'buffer'
import { env } from '../utils/env'

// Make Buffer available globally for bip39
if (typeof window !== 'undefined') {
  window.Buffer = Buffer
}

// Utility functions for hex/bytes conversion
export function hexToBytes(hex: string): Uint8Array {
  const match = hex.match(/.{1,2}/g)
  return new Uint8Array(match ? match.map(byte => parseInt(byte, 16)) : [])
}

export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map(byte => byte.toString(16).padStart(2, '0'))
    .join('')
}

// Simple keccak256 substitute using Web Crypto (for demo purposes)
async function simpleHash(data: string): Promise<Uint8Array> {
  const encoder = new TextEncoder()
  const hash = await crypto.subtle.digest('SHA-256', encoder.encode(data))
  return new Uint8Array(hash)
}

// Generate 12-word mnemonic
export function generateMnemonic(): string[] {
  const mnemonic = bip39.generateMnemonic(128) // 128 bits = 12 words
  return mnemonic.split(' ')
}

// Derive keys from mnemonic
export async function deriveKeys(mnemonic: string[]): Promise<{
  privateKeyHex: string
  publicKeyHex: string
  address: string
}> {
  const mnemonicString = mnemonic.join(' ')
  
  // Validate mnemonic
  if (!bip39.validateMnemonic(mnemonicString)) {
    throw new Error('Invalid mnemonic')
  }
  
  // Use mnemonic as seed for private key generation
  const seed = await simpleHash(mnemonicString)
  const privateKey = seed.slice(0, 32) // Take first 32 bytes
  
  // Generate public key using secp256k1
  const publicKey = secp256k1.getPublicKey(privateKey, false) // uncompressed
  
  // Create address from public key (simplified - using last 20 bytes of hash)
  const pubKeyHash = await simpleHash(bytesToHex(publicKey))
  const address = '0x' + bytesToHex(pubKeyHash.slice(-20))
  
  return {
    privateKeyHex: bytesToHex(privateKey),
    publicKeyHex: bytesToHex(publicKey),
    address
  }
}

// Encryption/storage using Web Crypto API
export async function encryptAndSave(data: {
  mnemonic: string[]
  privateKeyHex: string
}): Promise<void> {
  try {
    // Dev bypass: store keystore in plaintext in IndexedDB/local dev storage so onboarding can continue
    if (env.WALLET_DEV_BYPASS) {
      // Try IndexedDB first (idb-keyval). If that fails (common when extensions block storage),
      // fall back to localStorage so the dev flow still works.
      try {
        await set('vision.keystore', { plaintext: true, data })
        return
      } catch (e) {
        console.warn('Failed to write dev-bypass keystore to IndexedDB, attempting localStorage fallback', e)
        try {
          localStorage.setItem('vision.keystore', JSON.stringify({ plaintext: true, data }))
          return
        } catch (e2) {
          console.warn('Failed to write dev-bypass keystore to localStorage as well', e2)
          // fallthrough to normal path (attempt proper encrypted save)
        }
      }
    }
    // Get or create device secret
    let deviceSecret = await get('vision.device.secret') as string
    if (!deviceSecret) {
      const secretBytes = crypto.getRandomValues(new Uint8Array(32))
      deviceSecret = bytesToHex(secretBytes)
      await set('vision.device.secret', deviceSecret)
    }
    
    // Import device secret as encryption key
    const secretBytes = new Uint8Array(hexToBytes(deviceSecret))
    let key: CryptoKey
    try {
      key = await crypto.subtle.importKey(
        'raw',
        secretBytes,
        { name: 'AES-GCM' },
        false,
        ['encrypt']
      )
    } catch (err: any) {
      console.error('crypto.subtle.importKey failed', {
        name: err?.name,
        message: err?.message,
        stack: err?.stack,
        deviceSecretLength: deviceSecret?.length,
        isSecureContext: (typeof window !== 'undefined') ? (window.isSecureContext ?? false) : undefined,
        userAgent: (typeof navigator !== 'undefined') ? navigator.userAgent : undefined,
      })
      throw err
    }

    // Encrypt the data
    const plaintext = JSON.stringify(data)
    const encoder = new TextEncoder()
    const iv = crypto.getRandomValues(new Uint8Array(12))

    let ciphertext: ArrayBuffer
    try {
      ciphertext = await crypto.subtle.encrypt(
        { name: 'AES-GCM', iv },
        key,
        encoder.encode(plaintext)
      )
    } catch (err: any) {
      console.error('crypto.subtle.encrypt failed', {
        name: err?.name,
        message: err?.message,
        stack: err?.stack,
        deviceSecretLength: deviceSecret?.length,
        isSecureContext: (typeof window !== 'undefined') ? (window.isSecureContext ?? false) : undefined,
        userAgent: (typeof navigator !== 'undefined') ? navigator.userAgent : undefined,
      })
      throw err
    }
    
    // Store encrypted data with IV
    const encryptedData = {
      iv: bytesToHex(iv),
      ciphertext: bytesToHex(new Uint8Array(ciphertext))
    }
    
    await set('vision.keystore', encryptedData)
  } catch (error) {
    // Provide richer logging to help diagnose WebCrypto issues in the wild
    console.error('Encryption failed:', {
      name: (error as any)?.name,
      message: (error as any)?.message,
      stack: (error as any)?.stack,
      isSecureContext: (typeof window !== 'undefined') ? (window.isSecureContext ?? false) : undefined,
      userAgent: (typeof navigator !== 'undefined') ? navigator.userAgent : undefined,
    })
    // Rethrow a specific error so callers can show a friendly message
    throw new Error('Failed to encrypt and save keystore')
  }
}

// Decrypt and load keystore
export async function loadAndDecrypt(): Promise<{
  mnemonic: string[]
  privateKeyHex: string
} | null> {
  try {
    const deviceSecret = await get('vision.device.secret') as string
    let encryptedData: any = null

    try {
      encryptedData = await get('vision.keystore')
    } catch (e) {
      console.warn('IndexedDB read failed for vision.keystore, attempting localStorage fallback', e)
      try {
        const raw = localStorage.getItem('vision.keystore')
        if (raw) encryptedData = JSON.parse(raw)
      } catch (e2) {
        console.warn('localStorage read failed for vision.keystore', e2)
      }
    }

    if (!encryptedData) return null

    // If stored as plaintext under dev bypass, return directly
    if (encryptedData && encryptedData.plaintext && encryptedData.data) {
      return encryptedData.data as { mnemonic: string[]; privateKeyHex: string }
    }

    if (!deviceSecret) {
      return null
    }
    
    // Import device secret as decryption key
    const secretBytes = new Uint8Array(hexToBytes(deviceSecret))
    const key = await crypto.subtle.importKey(
      'raw',
      secretBytes,
      { name: 'AES-GCM' },
      false,
      ['decrypt']
    )
    
    // Decrypt the data
    const iv = new Uint8Array(hexToBytes(encryptedData.iv))
    const ciphertext = new Uint8Array(hexToBytes(encryptedData.ciphertext))
    
    const plaintext = await crypto.subtle.decrypt(
      { name: 'AES-GCM', iv },
      key,
      ciphertext
    )
    
    const decoder = new TextDecoder()
    const data = JSON.parse(decoder.decode(plaintext))
    
    return data
  } catch (error) {
    console.error('Decryption failed:', error)
    return null
  }
}

// Validation helpers
export function isValidHex(hex: string): boolean {
  return /^[0-9a-fA-F]+$/.test(hex)
}

export function isValidAddress(address: string): boolean {
  return /^0x[0-9a-fA-F]{40}$/.test(address)
}