import { create } from 'zustand'

interface OnboardingState {
  handle: string
  mnemonic: string[]
  setHandle: (handle: string) => void
  setMnemonic: (mnemonic: string[]) => void
  reset: () => void
}

export const useOnboardingStore = create<OnboardingState>((set) => ({
  handle: '',
  mnemonic: [],
  setHandle: (handle) => set({ handle }),
  setMnemonic: (mnemonic) => set({ mnemonic }),
  reset: () => set({ handle: '', mnemonic: [] })
}))