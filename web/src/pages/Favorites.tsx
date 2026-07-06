import { IconArrowLeft, IconHeart } from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchFavorites, type Favorite, type Station } from '../lib/api'
import type { ProviderId } from '../state/atoms'
import { StationCard, StationGridSkeleton } from '../components/StationCard'
import { ErrorNote } from './Home'

function toStation(favorite: Favorite): Station {
  return {
    id: favorite.id,
    name: favorite.name,
    codec: '',
    bitrate: 0,
    streamUrl: '',
    playing: null,
  }
}

export function FavoritesPage() {
  const favorites = useQuery({ queryKey: ['favorites'], queryFn: fetchFavorites })

  return (
    <div className="pt-10">
      <Link
        to="/"
        className="inline-flex items-center gap-1.5 text-sm text-zinc-400 transition-colors hover:text-violet-300"
      >
        <IconArrowLeft size={16} /> Back to search
      </Link>

      <h1 className="mt-4 mb-8 flex items-center gap-3 text-3xl font-bold tracking-tight text-white">
        <IconHeart size={28} className="text-fuchsia-400" /> Favorites
      </h1>

      {favorites.isLoading ? (
        <StationGridSkeleton count={6} />
      ) : favorites.isError ? (
        <ErrorNote message={(favorites.error as Error).message} />
      ) : favorites.data && favorites.data.length > 0 ? (
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {favorites.data.map((favorite) => (
            <StationCard
              key={`${favorite.provider}-${favorite.id}`}
              station={toStation(favorite)}
              provider={favorite.provider as ProviderId}
            />
          ))}
        </div>
      ) : (
        <div className="flex flex-col items-center gap-3 py-16 text-zinc-500">
          <IconHeart size={40} />
          <p>No favorites yet — tap the heart on any station to save it here.</p>
        </div>
      )}
    </div>
  )
}
