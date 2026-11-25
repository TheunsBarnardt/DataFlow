import { useCallback, useEffect, useMemo, useState } from "react";
import {
  BotIcon,
  HistoryIcon,
  PanelLeftCloseIcon,
  SquarePenIcon,
  WrenchIcon,
} from "lucide-react";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Bubble, BubbleContent } from "@/components/ui/bubble";
import { Button } from "@/components/ui/button";
import {
  Message,
  MessageAvatar,
  MessageContent,
  MessageHeader,
} from "@/components/ui/message";
import {
  MessageScroller,
  MessageScrollerButton,
  MessageScrollerContent,
  MessageScrollerItem,
  MessageScrollerProvider,
  MessageScrollerViewport,
} from "@/components/ui/message-scroller";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { Textarea } from "@/components/ui/textarea";
import {
  Trace,
  TraceCollapsible,
  TraceCollapsibleContent,
  TraceCollapsibleTrigger,
  TraceIcon,
  TraceLine,
  TraceTitle,
} from "@/components/ui/trace";
import { cn } from "@/lib/utils";
import { api } from "@/lib/api";
import { toast } from "sonner";

interface ChatMessage {
  id: string;
  role: string;
  content: string;
  toolName?: string;
}

interface Thread {
  _id: string;
  title: string;
  messages: ChatMessage[];
  updatedAt?: string;
  createdAt?: string;
}

type ThreadSummary = {
  _id: string;
  title: string;
  updatedAt?: string;
  createdAt?: string;
};

type Turn =
  | { kind: "user"; message: ChatMessage }
  | { kind: "assistant"; tools: ChatMessage[]; message: ChatMessage | null };

function groupTurns(messages: ChatMessage[]): Turn[] {
  const turns: Turn[] = [];
  let tools: ChatMessage[] = [];

  for (const message of messages) {
    if (message.role === "user") {
      tools = [];
      turns.push({ kind: "user", message });
      continue;
    }
    if (message.role === "tool") {
      tools.push(message);
      continue;
    }
    if (message.role === "assistant") {
      turns.push({ kind: "assistant", tools, message });
      tools = [];
    }
  }

  if (tools.length) {
    turns.push({ kind: "assistant", tools, message: null });
  }

  return turns;
}

function formatToolTitle(name?: string) {
  if (!name) return "Ran a tool";
  return `Ran ${name}`;
}

function formatToolResult(content: string) {
  try {
    return JSON.stringify(JSON.parse(content), null, 2);
  } catch {
    return content;
  }
}

function relativeTime(iso?: string): string {
  if (!iso) return "";
  const diff = Date.now() - new Date(iso).getTime();
  if (Number.isNaN(diff) || diff < 0) return "";
  const minutes = Math.floor(diff / 60_000);
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d`;
  return `${Math.floor(days / 7)}w`;
}

function startOfDay(d: Date) {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

function historyGroupLabel(iso?: string): string {
  if (!iso) return "Earlier";
  const day = startOfDay(new Date(iso));
  const today = startOfDay(new Date());
  const yesterday = today - 86_400_000;
  if (day === today) return "Today";
  if (day === yesterday) return "Yesterday";
  return "Earlier";
}

export function ChatPanel({
  onWorkflowChanged,
  compact = false,
  className,
}: {
  onWorkflowChanged: () => void;
  compact?: boolean;
  className?: string;
}) {
  const [thread, setThread] = useState<Thread | null>(null);
  const [history, setHistory] = useState<ThreadSummary[]>([]);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);

  const refreshHistory = useCallback(async () => {
    const res = await api<{ threads: ThreadSummary[] }>("/api/threads");
    setHistory(res.threads);
  }, []);

  async function createThread() {
    const created = await api<{ thread: Thread }>("/api/threads", {
      method: "POST",
    });
    setThread(created.thread);
    setText("");
    await refreshHistory().catch(() => undefined);
  }

  async function openThread(id: string) {
    const res = await api<{ thread: Thread }>(`/api/threads/${id}`);
    setThread(res.thread);
    setText("");
  }

  useEffect(() => {
    void (async () => {
      try {
        const res = await api<{ threads: ThreadSummary[] }>("/api/threads");
        setHistory(res.threads);
        if (res.threads[0]?._id) {
          await openThread(res.threads[0]._id);
        } else {
          await createThread();
        }
      } catch (err) {
        toast.error(err instanceof Error ? err.message : String(err));
      }
    })();
  }, []);

  const turns = useMemo(
    () => groupTurns(thread?.messages ?? []),
    [thread?.messages],
  );

  const historyGroups = useMemo(() => {
    const groups: { label: string; items: ThreadSummary[] }[] = [];
    for (const item of history) {
      const label = historyGroupLabel(item.updatedAt ?? item.createdAt);
      const last = groups[groups.length - 1];
      if (last?.label === label) last.items.push(item);
      else groups.push({ label, items: [item] });
    }
    return groups;
  }, [history]);

  async function send() {
    if (!thread || !text.trim() || busy) return;
    setBusy(true);
    try {
      const res = await api<{ thread: Thread }>("/api/threads/" + thread._id + "/messages", {
        method: "POST",
        body: JSON.stringify({ content: text }),
      });
      setThread(res.thread);
      setText("");
      onWorkflowChanged();
      await refreshHistory().catch(() => undefined);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className={cn("flex h-full min-h-0 flex-col", compact ? "gap-2 p-0" : "gap-3 p-3", className)}>
      <div className={cn("flex items-center justify-between gap-2", compact && "px-3 pt-1")}>
        <div className="min-w-0">
          <h2 className="text-sm font-semibold">Agent</h2>
          <p className="truncate text-xs text-muted-foreground">
            {thread?.title ?? "…"}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          <Button
            type="button"
            size="sm"
            variant={historyOpen ? "secondary" : "outline"}
            aria-pressed={historyOpen}
            title={historyOpen ? "Hide chat history" : "Show chat history"}
            onClick={() => {
              const next = !historyOpen;
              setHistoryOpen(next);
              if (next) {
                void refreshHistory().catch((err) =>
                  toast.error(err instanceof Error ? err.message : String(err)),
                );
              }
            }}
          >
            {historyOpen ? (
              <PanelLeftCloseIcon data-icon="inline-start" />
            ) : (
              <HistoryIcon data-icon="inline-start" />
            )}
            History
          </Button>
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={() =>
              void createThread().catch((err) =>
                toast.error(err instanceof Error ? err.message : String(err)),
              )
            }
          >
            <SquarePenIcon data-icon="inline-start" />
            New chat
          </Button>
        </div>
      </div>

      <div className={cn("flex min-h-0 flex-1", compact ? "px-0" : "")}>
        <ResizablePanelGroup
          orientation="horizontal"
          className="min-h-0 flex-1"
          id="agent-chat"
        >
          {historyOpen ? (
            <>
              <ResizablePanel
                id="agent-history"
                defaultSize="32%"
                minSize="18%"
                maxSize="48%"
                className="min-h-0"
              >
                <aside
                  className={cn(
                    "flex h-full min-h-0 flex-col border-r bg-background/80",
                    compact ? "border-border" : "rounded-l-md border border-r-0",
                  )}
                >
                  <div className="min-h-0 flex-1 overflow-y-auto px-2 py-2">
                    {historyGroups.length === 0 ? (
                      <p className="px-2 py-3 text-xs text-muted-foreground">
                        No chats yet
                      </p>
                    ) : (
                      historyGroups.map((group) => (
                        <div key={group.label} className="mb-3">
                          <div className="px-2 pb-1.5 text-[11px] font-medium text-muted-foreground">
                            {group.label}
                          </div>
                          <div className="flex flex-col gap-0.5">
                            {group.items.map((item) => {
                              const active = thread?._id === item._id;
                              return (
                                <button
                                  key={item._id}
                                  type="button"
                                  onClick={() =>
                                    void openThread(item._id).catch((err) =>
                                      toast.error(
                                        err instanceof Error
                                          ? err.message
                                          : String(err),
                                      ),
                                    )
                                  }
                                  className={cn(
                                    "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors",
                                    active
                                      ? "bg-muted text-foreground"
                                      : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
                                  )}
                                >
                                  <span
                                    className={cn(
                                      "h-1.5 w-1.5 shrink-0 rounded-full",
                                      active
                                        ? "bg-sky-400"
                                        : "bg-muted-foreground/50",
                                    )}
                                  />
                                  <span className="min-w-0 flex-1 truncate">
                                    {item.title}
                                  </span>
                                  <span className="shrink-0 text-[11px] text-muted-foreground">
                                    {relativeTime(
                                      item.updatedAt ?? item.createdAt,
                                    )}
                                  </span>
                                </button>
                              );
                            })}
                          </div>
                        </div>
                      ))
                    )}
                  </div>
                </aside>
              </ResizablePanel>
              <ResizableHandle withHandle />
            </>
          ) : null}

          <ResizablePanel
            id="agent-main"
            defaultSize={historyOpen ? "68%" : "100%"}
            minSize="40%"
            className="min-h-0"
          >
            <ResizablePanelGroup
              orientation="vertical"
              className={cn(
                "h-full min-h-0 min-w-0",
                compact && "px-3 pb-3",
              )}
              id="agent-main-vertical"
            >
              <ResizablePanel
                id="agent-messages"
                defaultSize="72%"
                minSize="35%"
                className="min-h-0"
              >
                <MessageScrollerProvider
                  autoScroll
                  defaultScrollPosition="last-anchor"
                >
                  <MessageScroller
                    className={cn(
                      "h-full min-h-0 bg-card",
                      compact
                        ? "rounded-md border"
                        : historyOpen
                          ? "rounded-r-md border border-l-0"
                          : "rounded-md border",
                    )}
                  >
                    <MessageScrollerViewport>
                      <MessageScrollerContent className="gap-4 p-3">
                        {!turns.length && !busy ? (
                          <MessageScrollerItem messageId="empty">
                            <Message>
                              <MessageAvatar>
                                <Avatar size="sm">
                                  <AvatarFallback>
                                    <BotIcon />
                                  </AvatarFallback>
                                </Avatar>
                              </MessageAvatar>
                              <MessageContent>
                                <MessageHeader>Agent</MessageHeader>
                                <Bubble variant="muted">
                                  <BubbleContent>
                                    Try: &quot;create webhook http flow&quot; or
                                    paste workflow JSON.
                                  </BubbleContent>
                                </Bubble>
                              </MessageContent>
                            </Message>
                          </MessageScrollerItem>
                        ) : null}

                        {turns.map((turn) => {
                          if (turn.kind === "user") {
                            return (
                              <MessageScrollerItem
                                key={turn.message.id}
                                messageId={turn.message.id}
                                scrollAnchor
                              >
                                <Message align="end">
                                  <MessageContent>
                                    <Bubble variant="tinted" align="end">
                                      <BubbleContent className="whitespace-pre-wrap">
                                        {turn.message.content}
                                      </BubbleContent>
                                    </Bubble>
                                  </MessageContent>
                                </Message>
                              </MessageScrollerItem>
                            );
                          }

                          const id =
                            turn.message?.id ??
                            turn.tools.map((t) => t.id).join("-") ??
                            "assistant";

                          return (
                            <MessageScrollerItem key={id} messageId={id}>
                              <div className="flex flex-col gap-2">
                                {turn.tools.length ? (
                                  <Trace>
                                    {turn.tools.map((tool) => (
                                      <TraceCollapsible key={tool.id}>
                                        <TraceCollapsibleTrigger>
                                          <TraceLine>
                                            <TraceIcon>
                                              <WrenchIcon />
                                            </TraceIcon>
                                            <TraceTitle>
                                              {formatToolTitle(tool.toolName)}
                                            </TraceTitle>
                                          </TraceLine>
                                        </TraceCollapsibleTrigger>
                                        <TraceCollapsibleContent>
                                          <pre className="overflow-x-auto whitespace-pre-wrap text-xs text-foreground">
                                            {formatToolResult(tool.content)}
                                          </pre>
                                        </TraceCollapsibleContent>
                                      </TraceCollapsible>
                                    ))}
                                  </Trace>
                                ) : null}

                                {turn.message ? (
                                  <Message>
                                    <MessageAvatar>
                                      <Avatar size="sm">
                                        <AvatarFallback>
                                          <BotIcon />
                                        </AvatarFallback>
                                      </Avatar>
                                    </MessageAvatar>
                                    <MessageContent>
                                      <MessageHeader>Agent</MessageHeader>
                                      <Bubble variant="muted">
                                        <BubbleContent className="whitespace-pre-wrap">
                                          {turn.message.content}
                                        </BubbleContent>
                                      </Bubble>
                                    </MessageContent>
                                  </Message>
                                ) : null}
                              </div>
                            </MessageScrollerItem>
                          );
                        })}

                        {busy ? (
                          <MessageScrollerItem
                            messageId="working"
                            scrollAnchor={false}
                          >
                            <Trace>
                              <TraceLine status="running">
                                <TraceIcon>
                                  <WrenchIcon />
                                </TraceIcon>
                                <TraceTitle>Working on your request</TraceTitle>
                              </TraceLine>
                            </Trace>
                          </MessageScrollerItem>
                        ) : null}
                      </MessageScrollerContent>
                    </MessageScrollerViewport>
                    <MessageScrollerButton />
                  </MessageScroller>
                </MessageScrollerProvider>
              </ResizablePanel>

              <ResizableHandle withHandle />

              <ResizablePanel
                id="agent-composer"
                defaultSize="28%"
                minSize="18%"
                maxSize="55%"
                className="min-h-0"
              >
                <div className="flex h-full min-h-0 flex-col gap-2 pt-2">
                  <Textarea
                    value={text}
                    onChange={(e) => setText(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" && !e.shiftKey) {
                        e.preventDefault();
                        void send();
                      }
                    }}
                    placeholder='Try: "create webhook http flow" or paste workflow JSON'
                    className="min-h-0 flex-1 resize-none"
                    disabled={busy || !thread}
                  />
                  <Button
                    onClick={() => void send()}
                    disabled={busy || !thread || !text.trim()}
                    className="shrink-0"
                  >
                    {busy ? "Working…" : "Send"}
                  </Button>
                </div>
              </ResizablePanel>
            </ResizablePanelGroup>
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    </div>
  );
}
