import { Link } from '@tanstack/react-router'
import {
  IconChevronRight,
  IconFolder,
  IconHeart,
  IconHeartFilled,
  IconPlayerPlayFilled,
  IconPlayerStopFilled,
  IconRadio,
} from '@tabler/icons-react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useAtom, useAtomValue } from 'jotai'
import { addFavorite, fetchFavorites, isFolder, removeFavorite, type Station } from '../lib/api'
import { currentStationAtom, isPlayingAtom, providerAtom, type ProviderId } from '../state/atoms'

function EqBars() {
  return (
    <span className="flex h-3.5 items-end gap-[3px]" aria-label="Playing">
      <span className="w-[3px] origin-bottom animate-eq-1 rounded-full bg-violet-400" style={{ height: '100%' }} />
      <span className="w-[3px] origin-bottom animate-eq-2 rounded-full bg-fuchsia-400" style={{ height: '100%' }} />
      <span className="w-[3px] origin-bottom animate-eq-3 rounded-full bg-violet-400" style={{ height: '100%' }} />
    </span>
  )
}

function FavoriteButton({ station, provider }: { station: Station; provider: ProviderId }) {
  const queryClient = useQueryClient()
  const favorites = useQuery({ queryKey: ['favorites'], queryFn: fetchFavorites })
  const isFavorite =
    favorites.data?.some((f) => f.id === station.id && f.provider === provider) ?? false

  const toggle = useMutation({
    mutationFn: () =>
      isFavorite
        ? removeFavorite(station.id, provider)
        : addFavorite({ id: station.id, name: station.name, provider }),
    onSettled: () => queryClient.invalidateQueries({ queryKey: ['favorites'] }),
  })

  return (
    <button
      type="button"
      onClick={() => toggle.mutate()}
      disabled={toggle.isPending || !station.id}
      className="shrink-0 p-1 transition-transform hover:scale-110 disabled:opacity-50"
      aria-label={isFavorite ? 'Remove from favorites' : 'Add to favorites'}
    >
      {isFavorite ? (
        <IconHeartFilled size={18} className="text-fuchsia-400" />
      ) : (
        <IconHeart size={18} className="text-zinc-600 transition-colors hover:text-fuchsia-300" />
      )}
    </button>
  )
}

export function StationCard({
  station,
  provider: providerOverride,
}: {
  station: Station
  provider?: ProviderId
}) {
  const activeProvider = useAtomValue(providerAtom)
  const provider = providerOverride ?? activeProvider
  const [current, setCurrent] = useAtom(currentStationAtom)
  const [isPlaying, setIsPlaying] = useAtom(isPlayingAtom)

  if (isFolder(station, provider)) {
    return (
      <Link
        to="/browse/$category"
        params={{ category: station.id }}
        search={{ name: station.name }}
        className="group flex items-center gap-4 rounded-2xl border border-white/5 bg-white/[0.03] p-4 transition-all hover:border-violet-500/30 hover:bg-white/[0.06]"
      >
        <span className="flex size-12 shrink-0 items-center justify-center rounded-xl bg-zinc-800/80 text-zinc-400 transition-colors group-hover:text-violet-300">
          <IconFolder size={22} />
        </span>
        <span className="min-w-0 flex-1">
          <span className="block truncate font-medium text-zinc-100">{station.name}</span>
          <span className="block truncate text-sm text-zinc-500">Browse category</span>
        </span>
        <IconChevronRight
          size={18}
          className="shrink-0 text-zinc-600 transition-transform group-hover:translate-x-0.5 group-hover:text-violet-300"
        />
      </Link>
    )
  }

  const isCurrent = current?.id === station.id
  const isActive = isCurrent && isPlaying

  const toggle = () => {
    if (!station.id) return
    if (isCurrent) {
      setIsPlaying(!isPlaying)
      return
    }
    setCurrent({
      id: station.id,
      name: station.name,
      provider,
      subtitle: station.playing,
      streamUrl: station.streamUrl || undefined,
    })
    setIsPlaying(true)
  }

  return (
    <div
      className={`group flex items-center gap-2 rounded-2xl border p-4 transition-all ${
        isCurrent
          ? 'border-violet-500/40 bg-violet-500/10'
          : 'border-white/5 bg-white/[0.03] hover:border-violet-500/30 hover:bg-white/[0.06]'
      }`}
    >
      <button
        type="button"
        onClick={toggle}
        disabled={!station.id}
        className="flex min-w-0 flex-1 items-center gap-4 text-left disabled:cursor-not-allowed disabled:opacity-50"
      >
        <span className="relative flex size-12 shrink-0 items-center justify-center overflow-hidden rounded-xl bg-gradient-to-br from-violet-500/25 to-fuchsia-600/25 text-violet-300">
          {isActive ? (
            <EqBars />
          ) : (
            <>
              <IconRadio size={22} className="transition-opacity group-hover:opacity-0" />
              <IconPlayerPlayFilled
                size={20}
                className="absolute opacity-0 transition-opacity group-hover:opacity-100"
              />
            </>
          )}
        </span>

        <span className="min-w-0 flex-1">
          <span className={`block truncate font-medium ${isCurrent ? 'text-violet-200' : 'text-zinc-100'}`}>
            {station.name}
          </span>
          <span className="block truncate text-sm text-zinc-500">
            {station.playing || 'Live radio'}
          </span>
        </span>

        <span className="flex shrink-0 items-center gap-2">
          {station.codec && (
            <span className="rounded-md bg-white/5 px-1.5 py-0.5 text-[10px] font-semibold tracking-wide text-zinc-400 uppercase">
              {station.codec}
            </span>
          )}
          {station.bitrate > 0 && (
            <span className="rounded-md bg-white/5 px-1.5 py-0.5 text-[10px] font-semibold tracking-wide text-zinc-400">
              {station.bitrate}k
            </span>
          )}
          {isActive && <IconPlayerStopFilled size={16} className="text-violet-300" />}
        </span>
      </button>

      <FavoriteButton station={station} provider={provider} />
    </div>
  )
}

export function StationGrid({ stations, provider }: { stations: Station[]; provider?: ProviderId }) {
  return (
    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
      {stations.map((station, i) => (
        <StationCard key={`${station.id || station.name}-${i}`} station={station} provider={provider} />
      ))}
    </div>
  )
}

export function StationGridSkeleton({ count = 9 }: { count?: number }) {
  return (
    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
      {Array.from({ length: count }, (_, i) => (
        <div
          key={i}
          className="flex animate-pulse items-center gap-4 rounded-2xl border border-white/5 bg-white/[0.03] p-4"
        >
          <div className="size-12 shrink-0 rounded-xl bg-zinc-800/80" />
          <div className="flex-1 space-y-2">
            <div className="h-3.5 w-2/3 rounded bg-zinc-800/80" />
            <div className="h-3 w-1/3 rounded bg-zinc-800/60" />
          </div>
        </div>
      ))}
    </div>
  )
}
