import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App.tsx";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ThemeProvider } from "./components/theme-provider";
import { ThemedToaster } from "./components/themed-toaster";

const queryClient = new QueryClient();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider defaultTheme="system" storageKey="remote-yt-theme">
      <QueryClientProvider client={queryClient}>
        <ThemedToaster />
        <App />
      </QueryClientProvider>
    </ThemeProvider>
  </StrictMode>
);
