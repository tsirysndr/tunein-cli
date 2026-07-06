import { useEffect, useRef, useState } from 'react'
import {
  IconAlertTriangle,
  IconLoader2,
  IconPlayerPlayFilled,
  IconPlayerStopFilled,
  IconRadio,
  IconVolume,
  IconVolumeOff,
  IconX,
} from '@tabler/icons-react'
import { useQuery } from '@tanstack/react-query'
import { useAtom } from 'jotai'
import { fetchNowPlaying, fetchStation } from '../lib/api'
import { currentStationAtom, isPlayingAtom, volumeAtom } from '../state/atoms'

export function PlayerBar() {
  const [current, setCurrent] = useAtom(currentStationAtom)
  const [isPlaying, setIsPlaying] = useAtom(isPlayingAtom)
  const [volume, setVolume] = useAtom(volumeAtom)
  const audioRef = useRef<HTMLAudioElement>(null)
  const [buffering, setBuffering] = useState(false)
  const [failed, setFailed] = useState(false)

  const needsResolve = !!current && !current.streamUrl
  const details = useQuery({
    queryKey: ['station', current?.provider, current?.id],
    queryFn: () => fetchStation(current!.id, current!.provider),
    enabled: needsResolve,
    staleTime: Infinity,
  })

  const streamUrl = current?.streamUrl ?? details.data?.streamUrl

  const nowPlaying = useQuery({
    queryKey: ['nowPlaying', current?.id],
    queryFn: () => fetchNowPlaying(current!.id),
    enabled: !!current && current.provider === 'tunein' && isPlaying,
    refetchInterval: 30_000,
    retry: false,
  })

  useEffect(() => {
    const audio = audioRef.current
    if (!audio || !streamUrl) return
    setFailed(false)
    audio.src = streamUrl
    if (isPlaying) {
      audio.play().catch(() => setFailed(true))
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [streamUrl])

  useEffect(() => {
    const audio = audioRef.current
    if (!audio || !streamUrl) return
    if (isPlaying) {
      audio.play().catch(() => setFailed(true))
    } else {
      // Live streams cannot be paused/resumed meaningfully — stop buffering.
      audio.pause()
      setBuffering(false)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isPlaying])

  useEffect(() => {
    if (audioRef.current) audioRef.current.volume = volume
  }, [volume, streamUrl])

  if (!current) return null

  const resolving = needsResolve && details.isLoading
  const error = failed || details.isError || (needsResolve && !details.isLoading && !streamUrl)
  const subtitle = nowPlaying.data || current.subtitle || 'Live radio'

  const close = () => {
    audioRef.current?.pause()
    setIsPlaying(false)
    setCurrent(null)
  }

  return (
    <div className="fixed inset-x-0 bottom-0 z-50 border-t border-white/10 bg-zinc-950/85 pb-[env(safe-area-inset-bottom)] backdrop-blur-2xl">
      <audio
        ref={audioRef}
        onWaiting={() => setBuffering(true)}
        onPlaying={() => setBuffering(false)}
        onError={() => {
          setBuffering(false)
          setFailed(true)
        }}
      />
      <div className="mx-auto flex h-20 w-full max-w-6xl items-center gap-4 px-4 sm:px-6">
        <div className="flex size-12 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br from-violet-500 to-fuchsia-600 shadow-lg shadow-violet-500/25">
          <IconRadio size={24} className="text-white" />
        </div>

        <div className="min-w-0 flex-1">
          <p className="truncate font-semibold text-white">{current.name}</p>
          <p className="truncate text-sm text-zinc-400">
            {error ? (
              <span className="flex items-center gap-1.5 text-amber-400">
                <IconAlertTriangle size={14} /> Stream unavailable
              </span>
            ) : resolving ? (
              'Tuning in…'
            ) : (
              subtitle
            )}
          </p>
        </div>

        <div className="hidden w-36 items-center gap-2 sm:flex">
          <button
            type="button"
            onClick={() => setVolume(volume > 0 ? 0 : 0.8)}
            className="text-zinc-400 transition-colors hover:text-white"
            aria-label={volume > 0 ? 'Mute' : 'Unmute'}
          >
            {volume > 0 ? <IconVolume size={20} /> : <IconVolumeOff size={20} />}
          </button>
          <input
            type="range"
            min={0}
            max={1}
            step={0.02}
            value={volume}
            onChange={(e) => setVolume(Number(e.target.value))}
            aria-label="Volume"
          />
        </div>

        <button
          type="button"
          onClick={() => setIsPlaying(!isPlaying)}
          disabled={resolving || error}
          className="flex size-12 shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-violet-500 to-fuchsia-600 text-white shadow-lg shadow-violet-500/30 transition-transform hover:scale-105 disabled:opacity-60 disabled:hover:scale-100"
          aria-label={isPlaying ? 'Stop' : 'Play'}
        >
          {resolving || buffering ? (
            <IconLoader2 size={22} className="animate-spin" />
          ) : isPlaying ? (
            <IconPlayerStopFilled size={20} />
          ) : (
            <IconPlayerPlayFilled size={20} />
          )}
        </button>

        <button
          type="button"
          onClick={close}
          className="shrink-0 text-zinc-500 transition-colors hover:text-white"
          aria-label="Close player"
        >
          <IconX size={20} />
        </button>
      </div>
    </div>
  )
}
