import {
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";

import { AppShell } from "@/components/layout/AppShell";
import { AlbumDetailView } from "@/components/library/AlbumDetailView";
import { AlbumsView } from "@/components/library/AlbumsView";
import { ArtistsView } from "@/components/library/ArtistsView";

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

const routeTree = rootRoute.addChildren([
  albumsRoute,
  albumDetailRoute,
  artistsRoute,
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
