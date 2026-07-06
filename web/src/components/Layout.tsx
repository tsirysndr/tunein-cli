import { Link, Outlet } from '@tanstack/react-router'
import { IconBroadcast, IconHeart } from '@tabler/icons-react'
import { useAtom } from 'jotai'
import { providerAtom, type ProviderId } from '../state/atoms'
import { PlayerBar } from './PlayerBar'

const PROVIDERS: { id: ProviderId; label: string }[] = [
  { id: 'tunein', label: 'TuneIn' },
  { id: 'radiobrowser', label: 'Radio Browser' },
]

export function Layout() {
  const [provider, setProvider] = useAtom(providerAtom)

  return (
    <div className="flex min-h-svh flex-col">
      <header className="sticky top-0 z-40 border-b border-white/5 bg-zinc-950/70 backdrop-blur-xl">
        <div className="mx-auto flex h-16 w-full max-w-6xl items-center justify-between px-4 sm:px-6">
          <Link to="/" className="group flex items-center gap-2.5">
            <span className="flex size-9 items-center justify-center rounded-xl bg-gradient-to-br from-violet-500 to-fuchsia-600 shadow-lg shadow-violet-500/25 transition-transform group-hover:scale-105">
              <IconBroadcast size={22} className="text-white" />
            </span>
            <span className="hidden text-lg font-semibold tracking-tight text-white min-[400px]:block">
              tunein
              <span className="bg-gradient-to-r from-violet-400 to-fuchsia-400 bg-clip-text text-transparent">
                .radio
              </span>
            </span>
          </Link>

          <div className="flex items-center gap-2 sm:gap-3">
            <Link
              to="/favorites"
              className="flex items-center gap-1.5 rounded-full border border-white/10 bg-white/5 px-3 py-2 text-sm font-medium text-zinc-400 transition-colors hover:text-fuchsia-300"
              activeProps={{ className: 'text-fuchsia-300 border-fuchsia-500/40' }}
            >
              <IconHeart size={16} />
              <span className="hidden sm:inline">Favorites</span>
            </Link>

            <nav className="flex items-center rounded-full border border-white/10 bg-white/5 p-1 text-sm">
            {PROVIDERS.map(({ id, label }) => (
              <button
                key={id}
                type="button"
                onClick={() => setProvider(id)}
                className={`rounded-full px-2.5 py-1.5 font-medium whitespace-nowrap transition-colors sm:px-3.5 ${
                  provider === id
                    ? 'bg-violet-500/90 text-white shadow shadow-violet-500/30'
                    : 'text-zinc-400 hover:text-zinc-200'
                }`}
              >
                {label}
              </button>
            ))}
            </nav>
          </div>
        </div>
      </header>

      <main className="mx-auto w-full max-w-6xl flex-1 px-4 pb-36 sm:px-6">
        <Outlet />
      </main>

      <PlayerBar />
    </div>
  )
}
