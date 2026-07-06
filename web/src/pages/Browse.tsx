import { IconArrowLeft, IconMoodEmpty } from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { useAtomValue } from 'jotai'
import { browseCategory } from '../lib/api'
import { providerAtom } from '../state/atoms'
import { StationGrid, StationGridSkeleton } from '../components/StationCard'
import { browseRoute } from '../router'
import { ErrorNote } from './Home'

export function BrowsePage() {
  const { category } = browseRoute.useParams()
  const { name } = browseRoute.useSearch()
  const provider = useAtomValue(providerAtom)

  const stations = useQuery({
    queryKey: ['browse', provider, category],
    queryFn: () => browseCategory(category, provider),
  })

  return (
    <div className="pt-10">
      <Link
        to="/"
        className="inline-flex items-center gap-1.5 text-sm text-zinc-400 transition-colors hover:text-violet-300"
      >
        <IconArrowLeft size={16} /> Back to search
      </Link>

      <h1 className="mt-4 mb-8 text-3xl font-bold tracking-tight text-white capitalize">
        {name ?? category}
      </h1>

      {stations.isLoading ? (
        <StationGridSkeleton count={12} />
      ) : stations.isError ? (
        <ErrorNote message={(stations.error as Error).message} />
      ) : stations.data && stations.data.length > 0 ? (
        <StationGrid stations={stations.data} />
      ) : (
        <div className="flex flex-col items-center gap-3 py-16 text-zinc-500">
          <IconMoodEmpty size={40} />
          <p>Nothing to browse here.</p>
        </div>
      )}
    </div>
  )
}
