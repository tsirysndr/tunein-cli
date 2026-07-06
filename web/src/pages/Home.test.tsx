import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Station } from '../lib/api'

vi.mock('../lib/api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/api')>()),
  fetchCategories: vi.fn(async () => ['Music', 'Talk', 'Sports']),
  searchStations: vi.fn(async (): Promise<Station[]> => [
    {
      id: 's8439',
      name: 'Jazz24',
      codec: 'MP3',
      bitrate: 128,
      streamUrl: '',
      playing: 'Smooth jazz all day',
    },
  ]),
  fetchFavorites: vi.fn(async () => []),
}))

import { searchStations } from '../lib/api'
import { router } from '../router'
import { RouterProvider } from '@tanstack/react-router'

function renderApp() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
}

describe('HomePage', () => {
  it('shows browsable categories by default', async () => {
    renderApp()
    expect(await screen.findByText('Music')).toBeInTheDocument()
    expect(screen.getByText('Talk')).toBeInTheDocument()
    expect(screen.getByText('Sports')).toBeInTheDocument()
  })

  it('performs a debounced instant search as the user types', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.type(screen.getByRole('searchbox'), 'jazz')

    expect(await screen.findByText('Jazz24', {}, { timeout: 2000 })).toBeInTheDocument()
    expect(screen.getByText('Smooth jazz all day')).toBeInTheDocument()
    expect(searchStations).toHaveBeenCalledWith('jazz', 'tunein')
  })
})
