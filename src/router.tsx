import {
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";

import { AppShell } from "@/components/layout/AppShell";
import { AlbumDetailView } from "@/components/library/AlbumDetailView";
import { AlbumsView } from "@/components/library/AlbumsView";
import { ArtistsView } from "@/components/library/ArtistsView";
import { SearchView } from "@/components/search/SearchView";
import { StatsView } from "@/components/stats/StatsView";

const rootRoute = createRootRoute({ component: AppShell });

const albumsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: AlbumsView,
});

const albumDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/albums/$albumId",
  component: AlbumDetailView,
});

const artistsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/artists",
  component: ArtistsView,
});

const searchRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/search",
  component: SearchView,
});

const statsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/stats",
  component: StatsView,
});

const routeTree = rootRoute.addChildren([
  albumsRoute,
  albumDetailRoute,
  artistsRoute,
  searchRoute,
  statsRoute,
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
