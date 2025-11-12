import { describe, it, expect } from 'vitest'
import { generateMnemonic, deriveKeys, hexToBytes, bytesToHex, isValidHex, isValidAddress } from '../lib/keystore'
import { isValidHandle, isValidAddress as guardIsValidAddress, isPositiveAmount } from '../lib/guards'
import { getBaseUrl, setBaseUrl } from '../lib/api'

describe('Keystore', () => {
  it('generateMnemonic returns 12 words', () => {
    const mnemonic = generateMnemonic()
    expect(mnemonic).toHaveLength(12)
    expect(mnemonic.every(word => typeof word === 'string')).toBe(true)
  })

  it('deriveKeys produces stable output', async () => {
    const mnemonic = ['test', 'test', 'test', 'test', 'test', 'test', 'test', 'test', 'test', 'test', 'test', 'junk']
    const keys = await deriveKeys(mnemonic)
    
    expect(keys.privateKeyHex).toMatch(/^[0-9a-f]{64}$/)
    expect(keys.publicKeyHex).toMatch(/^[0-9a-f]+$/)
    expect(keys.address).toMatch(/^0x[0-9a-f]{40}$/)
  })

  it('hex conversion utilities work', () => {
    const hex = 'deadbeef'
    const bytes = hexToBytes(hex)
    const backToHex = bytesToHex(bytes)
    
    expect(backToHex).toBe(hex)
    expect(bytes).toHaveLength(4)
  })

  it('isValidHex validates hex strings', () => {
    expect(isValidHex('deadbeef')).toBe(true)
    expect(isValidHex('DEADBEEF')).toBe(true)
    expect(isValidHex('123456')).toBe(true)
    expect(isValidHex('xyz123')).toBe(false)
    expect(isValidHex('')).toBe(false) // Empty string is not valid hex
  })

  it('isValidAddress validates addresses', () => {
    expect(isValidAddress('0x1234567890123456789012345678901234567890')).toBe(true)
    expect(isValidAddress('0x123')).toBe(false)
    expect(isValidAddress('1234567890123456789012345678901234567890')).toBe(false)
  })
})

describe('Guards', () => {
  it('isValidHandle validates handles', () => {
    expect(isValidHandle('test')).toBe(true)
    expect(isValidHandle('test-123')).toBe(true)
    expect(isValidHandle('test_user.name')).toBe(true)
    expect(isValidHandle('ab')).toBe(false) // too short
    expect(isValidHandle('TEST')).toBe(false) // uppercase not allowed
    expect(isValidHandle('test@user')).toBe(false) // invalid character
  })

  it('guardIsValidAddress validates addresses', () => {
    expect(guardIsValidAddress('0x1234567890123456789012345678901234567890')).toBe(true)
    expect(guardIsValidAddress('0x123')).toBe(false)
    expect(guardIsValidAddress('invalid')).toBe(false)
  })

  it('isPositiveAmount validates amounts', () => {
    expect(isPositiveAmount(10)).toBe(true)
    expect(isPositiveAmount('10')).toBe(true)
    expect(isPositiveAmount('10.5')).toBe(true)
    expect(isPositiveAmount(0)).toBe(false)
    expect(isPositiveAmount(-5)).toBe(false)
    expect(isPositiveAmount('not-a-number')).toBe(false)
  })
})

describe('API', () => {
  it('baseUrl getter/setter works', async () => {
    const originalUrl = await getBaseUrl()
    
    setBaseUrl('http://test.local:8080')
    expect(await getBaseUrl()).toBe('http://test.local:8080')
    
    // Restore original
    setBaseUrl(originalUrl)
    expect(await getBaseUrl()).toBe(originalUrl)
  })
})