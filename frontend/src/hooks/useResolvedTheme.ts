"use client";

import { useEffect, useState } from "react";
import { useConfig } from "@/contexts/ConfigContext";

export function useResolvedTheme(): "light" | "dark" {
  const { themePreference } = useConfig();
  const [systemDark, setSystemDark] = useState(false);

  useEffect(() => {
    if (typeof window === "undefined") return;
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const updateSystemTheme = () => setSystemDark(mediaQuery.matches);

    updateSystemTheme();
    mediaQuery.addEventListener("change", updateSystemTheme);
    return () => mediaQuery.removeEventListener("change", updateSystemTheme);
  }, []);

  if (themePreference === "system") return systemDark ? "dark" : "light";
  return themePreference;
}
