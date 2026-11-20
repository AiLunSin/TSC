export interface SessionSummaryDto {
  sessionId: string;
  platform: string;
  state: string;
  startedAtMs: number;
  channel: string;
}

export interface ChannelStatusDto {
  name: string;
  state: string;
  detail: string;
}

export interface SuggestionCardDto {
  cardId: string;
  kind: string;
  title: string;
  conclusion: string | null;
  trust: string;
  sourceLabel: string;
  shownAtMs: number;
}

export interface IpcErrorDto {
  code: string;
  message: string;
  retryable: boolean;
}

export const IPC_PROTOCOL_VERSION = 1;

export interface IpcInvoker {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
}