import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createStore, Provider } from 'jotai'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Station } from '../lib/api'
import { currentStationAtom, isPlayingAtom } from '../state/atoms'
import { StationCard } from './StationCard'

vi.mock('../lib/api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/api')>()),
  fetchFavorites: vi.fn(async () => []),
  addFavorite: vi.fn(async () => {}),
  removeFavorite: vi.fn(async () => {}),
}))

import { addFavorite, fetchFavorites } from '../lib/api'

const station: Station = {
  id: 's24939',
  name: 'BBC Radio 1',
  codec: 'MP3',
  bitrate: 128,
  streamUrl: 'https://example.com/stream.mp3',
  playing: 'The biggest new pop',
}

function renderCard(store = createStore()) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <Provider store={store}>
      <QueryClientProvider client={queryClient}>
        <StationCard station={station} provider="tunein" />
      </QueryClientProvider>
    </Provider>,
  )
  return store
}

describe('StationCard', () => {
  beforeEach(() => vi.clearAllMocks())

  it('renders name, subtitle and quality badges', () => {
    renderCard()
    expect(screen.getByText('BBC Radio 1')).toBeInTheDocument()
    expect(screen.getByText('The biggest new pop')).toBeInTheDocument()
    expect(screen.getByText('MP3')).toBeInTheDocument()
    expect(screen.getByText('128k')).toBeInTheDocument()
  })

  it('tunes in the station when clicked', async () => {
    const user = userEvent.setup()
    const store = renderCard()

    await user.click(screen.getByRole('button', { name: /BBC Radio 1/ }))

    expect(store.get(currentStationAtom)).toMatchObject({
      id: 's24939',
      name: 'BBC Radio 1',
      provider: 'tunein',
      streamUrl: 'https://example.com/stream.mp3',
    })
    expect(store.get(isPlayingAtom)).toBe(true)
  })

  it('pauses when the current station is clicked again', async () => {
    const user = userEvent.setup()
    const store = createStore()
    store.set(currentStationAtom, { id: 's24939', name: 'BBC Radio 1', provider: 'tunein' })
    store.set(isPlayingAtom, true)
    renderCard(store)

    await user.click(screen.getByRole('button', { name: /BBC Radio 1/ }))

    expect(store.get(isPlayingAtom)).toBe(false)
    expect(store.get(currentStationAtom)?.id).toBe('s24939')
  })

  it('adds the station to favorites via the heart button', async () => {
    const user = userEvent.setup()
    renderCard()

    await waitFor(() => expect(fetchFavorites).toHaveBeenCalled())
    await user.click(screen.getByRole('button', { name: 'Add to favorites' }))

    expect(addFavorite).toHaveBeenCalledWith({
      id: 's24939',
      name: 'BBC Radio 1',
      provider: 'tunein',
    })
  })
})
