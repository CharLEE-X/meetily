import { isTauri } from '@tauri-apps/api/core';

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
    __TAURI__?: unknown;
  }
}

export function isTauriRuntime(): boolean {
  if (typeof window === 'undefined') return false;
  return isTauri() || Boolean(window.__TAURI_INTERNALS__) || Boolean(window.__TAURI__);
}

export function noopUnlisten(): void {}
