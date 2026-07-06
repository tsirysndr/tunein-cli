import { IconLoader2, IconMoodEmpty, IconSearch } from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { useAtomValue } from 'jotai'
import { useForm } from 'react-hook-form'
import { useDebounce } from '../hooks/useDebounce'
import { fetchCategories, searchStations } from '../lib/api'
import { providerAtom } from '../state/atoms'
import { StationGrid, StationGridSkeleton } from '../components/StationCard'

interface SearchForm {
  query: string
}

export function HomePage() {
  const provider = useAtomValue(providerAtom)
  const { register, handleSubmit, watch } = useForm<SearchForm>({
    defaultValues: { query: '' },
  })
  const query = useDebounce(watch('query').trim(), 300)
  const searching = query.length >= 2

  const results = useQuery({
    queryKey: ['search', provider, query],
    queryFn: () => searchStations(query, provider),
    enabled: searching,
    placeholderData: keepPreviousData,
  })

  const categories = useQuery({
    queryKey: ['categories', provider],
    queryFn: () => fetchCategories(provider),
    enabled: !searching,
  })

  return (
    <div className="pt-14 sm:pt-20">
      <section className="mx-auto max-w-2xl text-center">
        <h1 className="text-4xl font-bold tracking-tight text-white sm:text-5xl">
          Listen to the{' '}
          <span className="bg-gradient-to-r from-violet-400 to-fuchsia-400 bg-clip-text text-transparent">
            world's radio
          </span>
        </h1>
        <p className="mt-4 text-lg text-zinc-400">
          Thousands of live stations across the globe — search, browse and tune in.
        </p>

        {/* Instant search: results update as you type. */}
        <form onSubmit={handleSubmit(() => {})} className="relative mt-8" role="search">
          <IconSearch
            size={20}
            className="pointer-events-none absolute top-1/2 left-5 -translate-y-1/2 text-zinc-500"
          />
          <input
            {...register('query')}
            type="search"
            autoFocus
            autoComplete="off"
            placeholder="Search stations, e.g. jazz, BBC, lofi…"
            className="w-full rounded-2xl border border-white/10 bg-white/5 py-4 pr-14 pl-13 text-base text-white placeholder-zinc-500 shadow-2xl shadow-violet-500/5 backdrop-blur transition-all outline-none focus:border-violet-500/50 focus:bg-white/[0.07] focus:ring-4 focus:ring-violet-500/15"
          />
          {results.isFetching && (
            <IconLoader2
              size={20}
              className="absolute top-1/2 right-5 -translate-y-1/2 animate-spin text-violet-400"
            />
          )}
        </form>
      </section>

      <section className="mt-14">
        {searching ? (
          <>
            <h2 className="mb-4 text-sm font-semibold tracking-widest text-zinc-500 uppercase">
              Results for “{query}”
            </h2>
            {results.isLoading ? (
              <StationGridSkeleton />
            ) : results.isError ? (
              <ErrorNote message={(results.error as Error).message} />
            ) : results.data && results.data.length > 0 ? (
              <StationGrid stations={results.data} />
            ) : (
              <div className="flex flex-col items-center gap-3 py-16 text-zinc-500">
                <IconMoodEmpty size={40} />
                <p>No stations found — try another search.</p>
              </div>
            )}
          </>
        ) : (
          <>
            <h2 className="mb-4 text-sm font-semibold tracking-widest text-zinc-500 uppercase">
              Browse by category
            </h2>
            {categories.isLoading ? (
              <div className="flex flex-wrap gap-2.5">
                {Array.from({ length: 18 }, (_, i) => (
                  <div
                    key={i}
                    className="h-10 w-28 animate-pulse rounded-full border border-white/5 bg-white/[0.03]"
                  />
                ))}
              </div>
            ) : categories.isError ? (
              <ErrorNote message={(categories.error as Error).message} />
            ) : (
              <div className="flex flex-wrap gap-2.5">
                {categories.data?.map((category) => (
                  <Link
                    key={category}
                    to="/browse/$category"
                    params={{ category }}
                    search={{ name: category }}
                    className="rounded-full border border-white/10 bg-white/5 px-4 py-2 text-sm font-medium text-zinc-300 transition-all hover:border-violet-500/40 hover:bg-violet-500/10 hover:text-violet-200"
                  >
                    {category}
                  </Link>
                ))}
              </div>
            )}
          </>
        )}
      </section>
    </div>
  )
}

export function ErrorNote({ message }: { message: string }) {
  return (
    <div className="rounded-2xl border border-amber-500/20 bg-amber-500/5 p-6 text-center text-amber-300">
      Something went wrong: {message.split(':')[0]}
    </div>
  )
}
