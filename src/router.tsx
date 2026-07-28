import {
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";

import { AppShell } from "@/components/layout/AppShell";
import { AlbumDetailView } from "@/components/library/AlbumDetailView";
import { AlbumsView } from "@/components/library/AlbumsView";
import { ArtistDetailView } from "@/components/library/ArtistDetailView";
import { ArtistsView } from "@/components/library/ArtistsView";
import { DiscoverView } from "@/components/discover/DiscoverView";
import { DoctorView } from "@/components/doctor/DoctorView";
import { FavoritesView } from "@/components/library/FavoritesView";
import { FoldersView } from "@/components/library/FoldersView";
import { GenreDetailView, GenresView } from "@/components/library/GenresView";
import { LogViewer } from "@/components/logs/LogViewer";
import { PlaylistDetailView } from "@/components/playlists/PlaylistDetailView";
import { PlaylistsView } from "@/components/playlists/PlaylistsView";
import { SearchView } from "@/components/search/SearchView";
import { SettingsView } from "@/components/settings/SettingsView";
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

const artistDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/artists/$artistId",
  component: ArtistDetailView,
});

export type FavoritesFilter = "all" | "fav" | "liked";

const favoritesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/favorites",
  component: FavoritesView,
  validateSearch: (
    search: Record<string, unknown>,
  ): { filter?: FavoritesFilter } =>
    search.filter === "fav" || search.filter === "liked"
      ? { filter: search.filter }
      : {},
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

const logsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/logs",
  component: LogViewer,
});

const doctorRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/doctor",
  component: DoctorView,
});

const discoverRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/discover",
  component: DiscoverView,
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SettingsView,
});

const foldersRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/folders",
  component: FoldersView,
});

const genresRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/genres",
  component: GenresView,
});

const genreDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/genres/$genreId",
  component: GenreDetailView,
});

const playlistsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/playlists",
  component: PlaylistsView,
});

const playlistDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/playlists/$kind/$playlistId",
  component: PlaylistDetailView,
});

const routeTree = rootRoute.addChildren([
  albumsRoute,
  albumDetailRoute,
  artistsRoute,
  artistDetailRoute,
  favoritesRoute,
  searchRoute,
  statsRoute,
  logsRoute,
  playlistsRoute,
  playlistDetailRoute,
  genresRoute,
  genreDetailRoute,
  foldersRoute,
  settingsRoute,
  doctorRoute,
  discoverRoute,
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
