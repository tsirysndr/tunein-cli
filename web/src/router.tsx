import { createRootRoute, createRoute, createRouter } from '@tanstack/react-router'
import { Layout } from './components/Layout'
import { HomePage } from './pages/Home'
import { BrowsePage } from './pages/Browse'
import { FavoritesPage } from './pages/Favorites'

const rootRoute = createRootRoute({ component: Layout })

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: HomePage,
})

export const browseRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/browse/$category',
  component: BrowsePage,
  validateSearch: (search: Record<string, unknown>): { name?: string } =>
    typeof search.name === 'string' ? { name: search.name } : {},
})

const favoritesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/favorites',
  component: FavoritesPage,
})

const routeTree = rootRoute.addChildren([indexRoute, browseRoute, favoritesRoute])

export const router = createRouter({ routeTree })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
