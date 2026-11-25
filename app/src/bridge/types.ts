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

export interface TranscriptLineDto {
  index: number;
  text: string;
  atMs: number;
}

export interface ExplainResultDto {
  text: string;
  explanation: string;
  source: string;
}

export interface SearchConfigDto {
  provider: string;
  hasKey: boolean;
  hasKeyBocha: boolean;
  hasKeyTavily: boolean;
  auto: boolean;
  ready: boolean;
  hourlyLimit: number;
  usedThisHour: number;
}

export interface SearchConfigInputDto {
  provider: string;
  key: string;
  auto: boolean;
  hourlyLimit: number;
  clearKey: boolean;
  keyBocha: string;
  keyTavily: string;
  clearKeyBocha: boolean;
  clearKeyTavily: boolean;
}

export interface AssistConfigDto {
  tipLines: number;
  contextLines: number;
}

export interface AssistConfigInputDto {
  tipLines: number;
  contextLines: number;
}

export interface SearchHitDto {
  title: string;
  site: string;
  snippet: string;
  url: string;
}

/** 界面偏好：状态灯与卡片的文字显示方式（`hover` / `always`）。 */
export interface UiPrefsDto {
  labelMode: string;
}

export interface UiPrefsInputDto {
  labelMode: string;
}

export interface SearchResponseDto {
  provider: string;
  hits: SearchHitDto[];
}

export interface AssistResultDto {
  source: string;
  points: string[];
  replies: string[];
  note: string;
}

export type ModelChoiceDto =
  | { preset: { displayName: string } }
  | { custom: { name: string } };

export type EndpointChoiceDto = { option: { displayName: string } } | "unavailable";

export type ConclusionDto =
  | { available: { text: string } }
  | "noReliableSource"
  | "absent";

export interface ProbeReportDto {
  succeeded: boolean;
  failure: string | null;
  message: string;
  elapsedMs: number;
  remainingMs: number | null;
}

export interface SettingsSnapshotDto {
  formState: string;
  hasKey: boolean;
  keyDisplay: string;
  modelChoice: ModelChoiceDto;
  endpointChoice: EndpointChoiceDto;
  inlineErrors: string[];
  canTest: boolean;
  canSave: boolean;
  sectionsOrder: string[];
  advancedCollapsed: boolean;
  probeReport: ProbeReportDto | null;
}

export interface SettingsSaveReceiptDto {
  persisted: boolean;
  fallback: string | null;
  hasKey: boolean;
  notice: string | null;
}

export interface SettingsClearReceiptDto {
  cleared: boolean;
  hasKey: boolean;
  notice: string | null;
}

export interface CardBodyDto {
  title: string;
  conclusion: ConclusionDto;
  confidence: number;
  referenceIds: string[];
}

export interface KeywordCardViewDto {
  cardId: string;
  keywordKind: string;
  trigger: string;
  triggeredAtMs: number;
  cardStatus: string;
  bodyVisible: boolean;
  body: CardBodyDto | null;
  clickState: string;
  disabledReason: string | null;
  badgeCount: number;
}

export interface CardDetailDto {
  cardId: string;
  title: string;
  conclusion: ConclusionDto;
  referenceIds: string[];
  citationAlignment: string;
}

export interface ProbeInputDto {
  key: string;
  modelIndex: number;
  endpointIndex: number;
  timeoutMs: number | null;
}

export interface SaveInputDto {
  key: string;
  modelIndex: number;
  endpointIndex: number;
  timeoutMs: number | null;
}

export interface ClearInputDto {
  confirm: boolean;
}

export interface OpenInputDto {
  cardId: string;
}

export const IPC_PROTOCOL_VERSION = 1;

export const IPC_COMMANDS = [
  "settings_snapshot",
  "settings_probe",
  "settings_save",
  "settings_clear",
  "keyword_cards_snapshot",
  "keyword_card_open",
  "open_log",
  "meeting_status",
  "capture_status",
  "ingest_text",
  "clear_cards",
  "start_asr",
  "stop_asr",
  "asr_status",
  "set_auto_cards",
  "transcript_snapshot",
  "explain_selection",
  "ingest_explanation",
  "search_config",
  "search_config_save",
  "search_clear_key",
  "web_search",
  "assist_reply",
  "assist_config",
  "assist_config_save",
  "ui_prefs",
  "ui_prefs_save",
] as const;

export type IpcCommand = (typeof IPC_COMMANDS)[number];

export interface IpcInvoker {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
}