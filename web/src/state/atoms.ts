import { atom } from 'jotai'
import { atomWithStorage } from 'jotai/utils'

export type ProviderId = 'tunein' | 'radiobrowser'

/** Which radio directory backs the UI. */
export const providerAtom = atomWithStorage<ProviderId>('tunein.provider', 'tunein')

export interface TunedStation {
  id: string
  name: string
  provider: ProviderId
  subtitle?: string | null
  /** Already-resolved stream url (radiobrowser results carry it); skips the lookup. */
  streamUrl?: string
}

export const currentStationAtom = atom<TunedStation | null>(null)
export const isPlayingAtom = atom(false)
export const volumeAtom = atomWithStorage('tunein.volume', 0.8)
