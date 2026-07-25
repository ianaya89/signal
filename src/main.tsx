import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";

import { SplashScreen } from "@/components/splash/SplashScreen";
import { bootstrapEvents } from "@/ipc/bootstrap";
import { router } from "@/router";

import "./styles.css";

const queryClient = new QueryClient();
bootstrapEvents(queryClient);

function Root() {
  const [splash, setSplash] = useState(true);
  return (
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
      {splash && <SplashScreen onDone={() => setSplash(false)} />}
    </QueryClientProvider>
  );
}

const rootEl = document.getElementById("root");
if (!rootEl) {
  throw new Error("missing #root element");
}

createRoot(rootEl).render(
  <StrictMode>
    <Root />
  </StrictMode>,
);
