import { GraphQLClient, gql } from 'graphql-request'
import type { ProviderId } from '../state/atoms'

// Same-origin in production (the actix server hosts both the SPA and the
// API); overridable via VITE_API_URL, proxied by vite in dev.
// graphql-request needs an absolute URL, so resolve against the page origin.
const endpoint: string = import.meta.env.VITE_API_URL ?? `${window.location.origin}/graphql`

export const client = new GraphQLClient(endpoint)

export interface Station {
  id: string
  name: string
  codec: string
  bitrate: number
  streamUrl: string
  playing: string | null
}

const STATION_FIELDS = gql`
  fragment StationFields on Station {
    id
    name
    codec
    bitrate
    streamUrl
    playing
  }
`

const SEARCH_QUERY = gql`
  ${STATION_FIELDS}
  query Search($query: String!, $provider: String) {
    search(query: $query, provider: $provider) {
      ...StationFields
    }
  }
`

const BROWSE_QUERY = gql`
  ${STATION_FIELDS}
  query Browse($category: String!, $offset: Int, $limit: Int, $provider: String) {
    browse(category: $category, offset: $offset, limit: $limit, provider: $provider) {
      ...StationFields
    }
  }
`

const CATEGORIES_QUERY = gql`
  query Categories($offset: Int, $limit: Int, $provider: String) {
    categories(offset: $offset, limit: $limit, provider: $provider)
  }
`

const STATION_QUERY = gql`
  ${STATION_FIELDS}
  query Station($id: ID!, $provider: String) {
    station(id: $id, provider: $provider) {
      ...StationFields
    }
  }
`

const NOW_PLAYING_QUERY = gql`
  query NowPlaying($stationId: ID!) {
    nowPlaying(stationId: $stationId)
  }
`

const FAVORITES_QUERY = gql`
  query Favorites {
    favorites {
      id
      name
      provider
    }
  }
`

const ADD_FAVORITE_MUTATION = gql`
  mutation AddFavorite($id: ID!, $name: String!, $provider: String) {
    addFavorite(id: $id, name: $name, provider: $provider) {
      id
    }
  }
`

const REMOVE_FAVORITE_MUTATION = gql`
  mutation RemoveFavorite($id: ID!, $provider: String) {
    removeFavorite(id: $id, provider: $provider)
  }
`

export async function searchStations(query: string, provider: ProviderId): Promise<Station[]> {
  const data = await client.request<{ search: Station[] }>(SEARCH_QUERY, { query, provider })
  return data.search
}

export async function browseCategory(
  category: string,
  provider: ProviderId,
  offset = 0,
  limit = 100,
): Promise<Station[]> {
  const data = await client.request<{ browse: Station[] }>(BROWSE_QUERY, {
    category,
    offset,
    limit,
    provider,
  })
  return data.browse
}

export async function fetchCategories(provider: ProviderId, limit = 60): Promise<string[]> {
  const data = await client.request<{ categories: string[] }>(CATEGORIES_QUERY, {
    offset: 0,
    limit,
    provider,
  })
  return data.categories
}

export async function fetchStation(id: string, provider: ProviderId): Promise<Station | null> {
  const data = await client.request<{ station: Station | null }>(STATION_QUERY, { id, provider })
  return data.station
}

export async function fetchNowPlaying(stationId: string): Promise<string> {
  const data = await client.request<{ nowPlaying: string }>(NOW_PLAYING_QUERY, { stationId })
  return data.nowPlaying
}

export interface Favorite {
  id: string
  name: string
  provider: string
}

export async function fetchFavorites(): Promise<Favorite[]> {
  const data = await client.request<{ favorites: Favorite[] }>(FAVORITES_QUERY)
  return data.favorites
}

export async function addFavorite(favorite: Favorite): Promise<void> {
  await client.request(ADD_FAVORITE_MUTATION, favorite)
}

export async function removeFavorite(id: string, provider: string): Promise<void> {
  await client.request(REMOVE_FAVORITE_MUTATION, { id, provider })
}

/**
 * TuneIn guide ids starting with "s" are stations; everything else ("c…",
 * "g…") is a browsable folder. Radio Browser ids are station uuids.
 */
export function isFolder(station: Station, provider: ProviderId): boolean {
  return provider === 'tunein' && !station.id.startsWith('s')
}
