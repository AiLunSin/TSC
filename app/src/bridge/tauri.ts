import { IPC_COMMANDS, type IpcCommand } from "./types";

interface TauriCore {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
}

declare global {
  interface Window {
    __TAURI__?: {
      core?: TauriCore;
    };
  }
}

export function availableCommands(): readonly IpcCommand[] {
  return IPC_COMMANDS;
}

function coreBridge(): TauriCore {
  const bridge = window.__TAURI__?.core;
  if (!bridge) {
    throw new Error("桌面壳不可用");
  }
  return bridge;
}

export async function invokeCommand<T>(
  command: IpcCommand,
  args?: Record<string, unknown>,
): Promise<T> {
  return coreBridge().invoke<T>(command, args);
}