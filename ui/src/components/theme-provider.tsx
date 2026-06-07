import { useEffect, useState } from "react";
import type { ReactNode } from "react";

import { ThemeProviderContext, type Theme } from "@/lib/theme";

type ThemeProviderProps = {
  children: ReactNode;
  defaultTheme?: Theme;
  storageKey?: string;
};

export function ThemeProvider({
  children,
  defaultTheme = "system",
  storageKey = "vite-ui-theme",
}: ThemeProviderProps) {
  const [theme, setThemeState] = useState<Theme>(
    () => (localStorage.getItem(storageKey) as Theme | null) ?? defaultTheme,
  );

  useEffect(() => {
    const root = window.document.documentElement;
    const systemThemeQuery = window.matchMedia("(prefers-color-scheme: dark)");

    function applyTheme() {
      root.classList.remove("light", "dark");

      const resolvedTheme =
        theme === "system"
          ? systemThemeQuery.matches
            ? "dark"
            : "light"
          : theme;

      root.classList.add(resolvedTheme);
      root.style.colorScheme = resolvedTheme;
    }

    applyTheme();

    if (theme !== "system") {
      return;
    }

    systemThemeQuery.addEventListener("change", applyTheme);
    return () => systemThemeQuery.removeEventListener("change", applyTheme);
  }, [theme]);

  const value = {
    theme,
    setTheme: (theme: Theme) => {
      localStorage.setItem(storageKey, theme);
      setThemeState(theme);
    },
  };

  return (
    <ThemeProviderContext.Provider value={value}>
      {children}
    </ThemeProviderContext.Provider>
  );
}
