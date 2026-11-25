import { useCallback, useEffect, useRef, useState } from "react";

import { invokeCommand } from "../bridge/tauri";
import type {
  AssistResultDto,
  CardDetailDto,
  ExplainResultDto,
  KeywordCardViewDto,
  SearchConfigDto,
  SearchHitDto,
  SearchResponseDto,
  TranscriptLineDto,
} from "../bridge/types";

export type ChannelState = "ready" | "degraded" | "off";

export interface ChannelView {
  name: string;
  state: ChannelState;
}

/** 状态灯与卡片的文字显示方式：悬停显示（默认）或常显。 */
export type LabelMode = "hover" | "always";

const DEFAULT_CHANNELS: readonly ChannelView[] = [
  { name: "字幕文本通道", state: "off" },
  { name: "系统音频通道", state: "off" },
  { name: "本地 ASR", state: "off" },
];

export const STATE_TEXT: Record<ChannelState, string> = {
  ready: "就绪",
  degraded: "降级",
  off: "未启用",
};

export const CHANNEL_DETAIL: Record<ChannelState, string> = {
  ready: "运行就绪",
  degraded: "运行中降级",
  off: "未启动或不可用",
};

export const SEARCH_PROVIDER_TEXT: Record<string, string> = {
  bocha: "博查 Bocha",
  tavily: "Tavily",
};

function toChannelState(value: string): ChannelState {
  if (value === "ready" || value === "degraded") {
    return value;
  }
  return "off";
}

/**
 * 识别状态位只回答"现在在做什么"。
 *
 * 停止转写后 `display_text` 还留着最后一句，早先直接铺在按钮右边，成了一行没有标签的
 * 会议内容（截图里像按钮说明）。停止态改为带前缀的"最后识别到「…」"，只留尾部：
 * partial 是累积增长的，尾部才是最新听到的几个字。
 */
export function asrHintText(running: boolean, text: string): string {
  if (running) {
    return text !== "" ? text : "系统音频实时识别，等待说话…";
  }
  if (text !== "") {
    const tail = text.length > 24 ? `…${text.slice(-24)}` : text;
    return `已停止 · 最后识别到「${tail}」`;
  }
  return "点「开始实时转写」，识别到的句子会逐条进入下方转写区。";
}

export function formatElapsed(totalSeconds: number): string {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  const pad = (value: number): string => String(value).padStart(2, "0");
  return hours > 0
    ? `${pad(hours)}:${pad(minutes)}:${pad(seconds)}`
    : `${pad(minutes)}:${pad(seconds)}`;
}

/** 后端把可读原因放在 IpcErrorDto.message 里，优先展示它而不是通用文案。 */
export function errorMessage(error: unknown, fallback: string): string {
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = String(error.message);
    if (message !== "") {
      return message;
    }
  }
  return fallback;
}

/**
 * 工作台状态机：转写、卡片、释义、检索、辅助回复与界面偏好都在这里。
 *
 * 两页共用一份状态：工作页只负责"看和用"，设置页负责"配"。这样切页不会丢正在进行的
 * 转写或已经选中的句子，也不需要把同一份轮询写两遍。
 */
export function useWorkbench() {
  const [cards, setCards] = useState<readonly KeywordCardViewDto[]>([]);
  const [cardError, setCardError] = useState<string | null>(null);
  const [detail, setDetail] = useState<CardDetailDto | null>(null);
  const [meeting, setMeeting] = useState<{ app: string; title: string } | null>(null);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const [channels, setChannels] = useState<readonly ChannelView[]>(DEFAULT_CHANNELS);
  const meetingStartedAt = useRef<number | null>(null);
  const [ingestText, setIngestText] = useState("");
  const [ingestBusy, setIngestBusy] = useState(false);
  const [ingestNotice, setIngestNotice] = useState<string | null>(null);
  const [asrRunning, setAsrRunning] = useState(false);
  const [asrText, setAsrText] = useState("");
  const [asrError, setAsrError] = useState("");
  const [asrStats, setAsrStats] = useState("");
  const [autoCards, setAutoCards] = useState(false);
  const [lines, setLines] = useState<TranscriptLineDto[]>([]);
  const [selection, setSelection] = useState("");
  const [explainBusy, setExplainBusy] = useState(false);
  const [explainResult, setExplainResult] = useState<ExplainResultDto | null>(null);
  const [explainNotice, setExplainNotice] = useState<string | null>(null);
  const [searchConfig, setSearchConfig] = useState<SearchConfigDto | null>(null);
  const [searchProvider, setSearchProvider] = useState("bocha");
  const [searchKeyBocha, setSearchKeyBocha] = useState("");
  const [searchKeyTavily, setSearchKeyTavily] = useState("");
  const [searchAuto, setSearchAuto] = useState(false);
  const [searchLimit, setSearchLimit] = useState(20);
  const [searchBusy, setSearchBusy] = useState(false);
  const [searchNotice, setSearchNotice] = useState<string | null>(null);
  const [searchConfigNotice, setSearchConfigNotice] = useState<string | null>(null);
  const [cardBusy, setCardBusy] = useState(false);
  const [searchHits, setSearchHits] = useState<SearchHitDto[]>([]);
  const [searchHitProvider, setSearchHitProvider] = useState("");
  const [assistBusy, setAssistBusy] = useState(false);
  const [assist, setAssist] = useState<AssistResultDto | null>(null);
  const [assistNotice, setAssistNotice] = useState<string | null>(null);
  const [labelMode, setLabelMode] = useState<LabelMode>("hover");
  const [labelNotice, setLabelNotice] = useState<string | null>(null);

  const loadCards = useCallback(async () => {
    try {
      const next = await invokeCommand<KeywordCardViewDto[]>("keyword_cards_snapshot");
      setCards(next);
      setCardError(null);
    } catch {
      setCardError("卡片暂不可用，请稍后重试。");
    }
  }, []);

  useEffect(() => {
    void loadCards();
    const id = window.setInterval(() => {
      void loadCards();
    }, 1000);
    return () => {
      window.clearInterval(id);
    };
  }, [loadCards]);

  const loadTranscript = useCallback(async () => {
    try {
      const next = await invokeCommand<TranscriptLineDto[]>("transcript_snapshot");
      setLines(next);
    } catch {
      // 保留上一次内容
    }
  }, []);

  useEffect(() => {
    void loadTranscript();
    const id = window.setInterval(() => {
      void loadTranscript();
    }, 1200);
    return () => {
      window.clearInterval(id);
    };
  }, [loadTranscript]);

  useEffect(() => {
    let cancelled = false;
    async function loadSearch(): Promise<void> {
      try {
        const config = await invokeCommand<SearchConfigDto>("search_config");
        if (!cancelled) {
          setSearchConfig(config);
          setSearchProvider(config.provider);
          setSearchAuto(config.auto);
          setSearchLimit(config.hourlyLimit);
        }
      } catch {
        // 读不到就按"未配置"展示
      }
    }
    void loadSearch();
    return () => {
      cancelled = true;
    };
  }, []);

  // 显示方式存在后端 `ui.json`：前端不落盘（门禁禁止），换台机器也还是同一个习惯。
  useEffect(() => {
    let cancelled = false;
    async function loadPrefs(): Promise<void> {
      try {
        const prefs = await invokeCommand<{ labelMode: string }>("ui_prefs");
        if (!cancelled) {
          setLabelMode(prefs.labelMode === "always" ? "always" : "hover");
        }
      } catch {
        // 读不到就按悬停显示
      }
    }
    void loadPrefs();
    return () => {
      cancelled = true;
    };
  }, []);

  async function saveLabelMode(next: LabelMode): Promise<void> {
    setLabelMode(next);
    setLabelNotice(null);
    try {
      const prefs = await invokeCommand<{ labelMode: string }>("ui_prefs_save", {
        input: { labelMode: next },
      });
      setLabelMode(prefs.labelMode === "always" ? "always" : "hover");
    } catch (error) {
      setLabelNotice(errorMessage(error, "显示方式没存下来，重启后会回到悬停显示。"));
    }
  }

  useEffect(() => {
    let cancelled = false;
    async function refreshMeeting(): Promise<void> {
      try {
        const info = await invokeCommand<[string, string] | null>("meeting_status");
        if (!cancelled) {
          setMeeting(info === null ? null : { app: info[0], title: info[1] });
        }
      } catch {
        if (!cancelled) {
          setMeeting(null);
        }
      }
    }
    void refreshMeeting();
    const id = window.setInterval(() => {
      void refreshMeeting();
    }, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    async function refreshCapture(): Promise<void> {
      try {
        const rows = await invokeCommand<Array<[string, string]>>("capture_status");
        if (!cancelled) {
          setChannels(rows.map(([name, state]) => ({ name, state: toChannelState(state) })));
        }
      } catch {
        // 保留上一次状态
      }
    }
    void refreshCapture();
    const id = window.setInterval(() => {
      void refreshCapture();
    }, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, []);

  useEffect(() => {
    if (meeting === null) {
      meetingStartedAt.current = null;
      setElapsedSeconds(0);
      return;
    }
    if (meetingStartedAt.current === null) {
      meetingStartedAt.current = Date.now();
    }
    const id = window.setInterval(() => {
      if (meetingStartedAt.current !== null) {
        setElapsedSeconds(Math.floor((Date.now() - meetingStartedAt.current) / 1000));
      }
    }, 1000);
    return () => {
      window.clearInterval(id);
    };
  }, [meeting]);

  useEffect(() => {
    let cancelled = false;
    async function refreshAsr(): Promise<void> {
      try {
        const [running, text, error, stats, autoCardsOn] = await invokeCommand<
          [boolean, string, string, string, boolean]
        >("asr_status");
        if (!cancelled) {
          setAsrRunning(running);
          setAsrText(text);
          setAsrError(error);
          setAsrStats(stats);
          setAutoCards(autoCardsOn);
        }
      } catch {
        // 保留上一次状态
      }
    }
    void refreshAsr();
    const id = window.setInterval(() => {
      void refreshAsr();
    }, 700);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, []);

  async function openCard(cardId: string): Promise<void> {
    try {
      const next = await invokeCommand<CardDetailDto>("keyword_card_open", {
        input: { cardId },
      });
      setDetail(next);
    } catch {
      setCardError("详情暂不可用。");
    }
  }

  // 只接受转写区内部的选区，避免把整页文字误当成选中。
  function captureSelection(container: HTMLElement): void {
    const active = window.getSelection();
    if (active === null || active.rangeCount === 0) {
      return;
    }
    if (!container.contains(active.getRangeAt(0).commonAncestorContainer)) {
      return;
    }
    const text = active.toString().trim().replace(/\s+/g, " ");
    if (text === "") {
      return;
    }
    setSelection(text.slice(0, 200));
    setExplainResult(null);
    setExplainNotice(null);
  }

  function clearSelection(): void {
    setSelection("");
    setExplainResult(null);
    setExplainNotice(null);
  }

  async function explainSelection(): Promise<void> {
    const text = selection.trim();
    if (text === "") {
      return;
    }
    setExplainBusy(true);
    setExplainNotice(null);
    try {
      const result = await invokeCommand<ExplainResultDto>("explain_selection", {
        input: { text, context: "" },
      });
      setExplainResult(result);
    } catch (error) {
      setExplainResult(null);
      setExplainNotice(errorMessage(error, "查询失败，请稍后重试。"));
    } finally {
      setExplainBusy(false);
    }
  }

  async function cardFromExplanation(): Promise<void> {
    if (explainResult === null || cardBusy) {
      return;
    }
    setCardBusy(true);
    try {
      const added = await invokeCommand<number>("ingest_explanation", {
        input: {
          word: explainResult.text,
          explanation: explainResult.explanation,
          sources: searchHits.map((hit) => hit.site).filter((site) => site !== ""),
        },
      });
      setExplainNotice(added > 0 ? "已生成卡片。" : "这条内容未通过卡片生成条件。");
      await loadCards();
    } catch {
      setExplainNotice("生成卡片失败，请稍后重试。");
    } finally {
      setCardBusy(false);
    }
  }

  async function saveSearchConfig(): Promise<void> {
    setSearchBusy(true);
    setSearchConfigNotice(null);
    try {
      const config = await invokeCommand<SearchConfigDto>("search_config_save", {
        input: {
          provider: searchProvider,
          key: "",
          auto: searchAuto,
          hourlyLimit: searchLimit,
          clearKey: false,
          keyBocha: searchKeyBocha,
          keyTavily: searchKeyTavily,
          clearKeyBocha: false,
          clearKeyTavily: false,
        },
      });
      setSearchConfig(config);
      setSearchProvider(config.provider);
      setSearchAuto(config.auto);
      setSearchLimit(config.hourlyLimit);
      setSearchKeyBocha("");
      setSearchKeyTavily("");
      setSearchConfigNotice(
        config.ready ? "已保存，联网查可用。" : "已保存，但当前构建或 Key 还不足以联网。",
      );
    } catch (error) {
      setSearchConfigNotice(errorMessage(error, "保存失败，请稍后重试。"));
    } finally {
      setSearchBusy(false);
    }
  }

  // 清除指定服务的检索凭据：配置（服务名、限额）保留，Key 从系统凭据里删掉。
  // 清当前服务走 search_clear_key（不要求仍有 Key）；清另一家走保存并只清那一家的 Key。
  async function clearSearchKeyFor(which: "bocha" | "tavily"): Promise<void> {
    setSearchBusy(true);
    setSearchConfigNotice(null);
    try {
      const config =
        which === searchProvider
          ? await invokeCommand<SearchConfigDto>("search_clear_key")
          : await invokeCommand<SearchConfigDto>("search_config_save", {
              input: {
                provider: searchProvider,
                key: "",
                auto: searchAuto,
                hourlyLimit: searchLimit,
                clearKey: false,
                keyBocha: "",
                keyTavily: "",
                clearKeyBocha: which === "bocha",
                clearKeyTavily: which === "tavily",
              },
            });
      setSearchConfig(config);
      if (which === "bocha") {
        setSearchKeyBocha("");
      } else {
        setSearchKeyTavily("");
      }
      setSearchConfigNotice(which === "bocha" ? "已清除博查 Key。" : "已清除 Tavily Key。");
    } catch (error) {
      setSearchConfigNotice(errorMessage(error, "清除失败，请稍后重试。"));
    } finally {
      setSearchBusy(false);
    }
  }

  // 只把选中的一小段发出去；查不到就如实说，不猜。
  async function searchSelection(): Promise<void> {
    const query = selection.trim();
    if (query === "") {
      return;
    }
    setSearchBusy(true);
    setSearchNotice(null);
    setSearchHits([]);
    try {
      const response = await invokeCommand<SearchResponseDto>("web_search", {
        input: { query },
      });
      setSearchHits(response.hits);
      setSearchHitProvider(response.provider);
      if (response.hits.length === 0) {
        setSearchNotice("没查到结果，换个说法再试。");
      }
    } catch (error) {
      setSearchNotice(errorMessage(error, "联网查失败，请稍后重试。"));
    } finally {
      setSearchBusy(false);
      // 用完一次就刷新计数，让"本小时 N/上限"跟着变。
      try {
        setSearchConfig(await invokeCommand<SearchConfigDto>("search_config"));
      } catch {
        // 计数刷不到不影响查询结果
      }
    }
  }

  async function askAssist(): Promise<void> {
    setAssistBusy(true);
    setAssistNotice(null);
    setAssist(null);
    try {
      const result = await invokeCommand<AssistResultDto>("assist_reply", {
        input: { focus: selection },
      });
      setAssist(result);
      if (result.points.length === 0 && result.replies.length === 0) {
        setAssistNotice("模型没按格式返回，原文见下方。");
      }
    } catch (error) {
      setAssistNotice(errorMessage(error, "生成失败，请稍后重试。"));
    } finally {
      setAssistBusy(false);
    }
  }

  async function copyReply(text: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(text);
      setAssistNotice("已复制，可以直接粘贴到会议里。");
    } catch {
      setAssistNotice("当前窗口不允许写剪贴板，请手动选中这段文字。");
    }
  }

  async function toggleAutoCards(): Promise<void> {
    try {
      const next = await invokeCommand<boolean>("set_auto_cards", { enabled: !autoCards });
      setAutoCards(next);
    } catch {
      setCardError("自动出卡开关暂不可用。");
    }
  }

  function toggleSearchAuto(): void {
    setSearchAuto((prev) => !prev);
  }

  async function clearCards(): Promise<void> {
    try {
      await invokeCommand<number>("clear_cards");
      setDetail(null);
      await loadCards();
    } catch {
      setCardError("清空卡片失败，请稍后重试。");
    }
  }

  async function toggleAsr(): Promise<void> {
    try {
      await invokeCommand(asrRunning ? "stop_asr" : "start_asr");
      if (!asrRunning) {
        setAsrError("");
      }
      setAsrRunning(!asrRunning);
    } catch (error) {
      // 后端已把可读原因放在 IpcErrorDto.message 里，直接展示，别再吞成通用文案。
      setAsrError(errorMessage(error, "实时转写启动失败，请稍后重试。"));
    }
  }

  async function ingest(): Promise<void> {
    if (ingestBusy || ingestText.trim() === "") {
      return;
    }
    setIngestBusy(true);
    try {
      const added = await invokeCommand<number>("ingest_text", { text: ingestText });
      setIngestText("");
      setIngestNotice(`已生成 ${String(added)} 张卡片。`);
      await loadCards();
    } catch {
      setIngestNotice(null);
      setCardError("生成卡片失败，请稍后重试。");
    } finally {
      setIngestBusy(false);
    }
  }

  return {
    cards,
    cardError,
    detail,
    setDetail,
    meeting,
    elapsedSeconds,
    channels,
    ingestText,
    setIngestText,
    ingestBusy,
    ingestNotice,
    asrRunning,
    asrText,
    asrError,
    asrStats,
    autoCards,
    lines,
    selection,
    explainBusy,
    explainResult,
    explainNotice,
    searchConfig,
    searchProvider,
    setSearchProvider,
    searchKeyBocha,
    setSearchKeyBocha,
    searchKeyTavily,
    setSearchKeyTavily,
    searchAuto,
    searchLimit,
    setSearchLimit,
    searchBusy,
    searchNotice,
    searchConfigNotice,
    cardBusy,
    searchHits,
    searchHitProvider,
    assistBusy,
    assist,
    assistNotice,
    labelMode,
    labelNotice,
    saveLabelMode,
    openCard,
    captureSelection,
    clearSelection,
    explainSelection,
    cardFromExplanation,
    saveSearchConfig,
    clearSearchKeyFor,
    searchSelection,
    askAssist,
    copyReply,
    toggleAutoCards,
    toggleSearchAuto,
    clearCards,
    toggleAsr,
    ingest,
  };
}

export type Workbench = ReturnType<typeof useWorkbench>;
