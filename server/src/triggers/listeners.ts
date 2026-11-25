import fs from "node:fs";
import path from "node:path";
import type { WorkflowDoc, NodeType, RunTrigger } from "@dataflow/shared";
import { workflows, type Stored } from "../db.js";
import { executeWorkflow } from "../engine/executor.js";
import { config } from "../config.js";

type StopFn = () => void | Promise<void>;
let stoppers: StopFn[] = [];

/** Last-seen RSS item guids/titles per workflow+url to avoid re-firing. */
const rssSeen = new Map<string, Set<string>>();

async function workflowsWithTrigger(type: NodeType): Promise<
  Array<{ workflow: Stored<WorkflowDoc>; node: WorkflowDoc["nodes"][0] }>
> {
  const list = await workflows()
    .find({ enabled: true, "nodes.type": type })
    .toArray();
  const out: Array<{
    workflow: Stored<WorkflowDoc>;
    node: WorkflowDoc["nodes"][0];
  }> = [];
  for (const wf of list) {
    for (const node of wf.nodes.filter((n) => n.type === type)) {
      out.push({ workflow: wf, node });
    }
  }
  return out;
}

async function fire(
  workflowId: string,
  trigger: RunTrigger,
  input: unknown,
): Promise<void> {
  try {
    await executeWorkflow({ workflowId, trigger, input });
  } catch (err) {
    console.error(`[trigger:${trigger}]`, err);
  }
}

async function startRedis(): Promise<void> {
  const url = config.redisUrl;
  if (!url) return;
  type RedisClient = {
    subscribe: (...channels: string[]) => Promise<unknown>;
    on: (event: "message", cb: (channel: string, message: string) => void) => void;
    quit: () => Promise<unknown>;
  };
  let RedisCtor: new (url: string) => RedisClient;
  try {
    const mod = await import("ioredis");
    RedisCtor = mod.default as unknown as new (url: string) => RedisClient;
  } catch {
    console.warn("ioredis not installed — Redis Trigger disabled");
    return;
  }
  const targets = await workflowsWithTrigger("redisTrigger");
  if (!targets.length) return;

  const sub = new RedisCtor(url);
  const channels = [
    ...new Set(
      targets.map((t) => String(t.node.parameters.channel ?? "dataflow")),
    ),
  ];
  await sub.subscribe(...channels);
  sub.on("message", (channel, message) => {
    let payload: unknown = message;
    try {
      payload = JSON.parse(message);
    } catch {
      /* keep string */
    }
    for (const t of targets) {
      const ch = String(t.node.parameters.channel ?? "dataflow");
      if (ch !== channel) continue;
      void fire(String(t.workflow._id), "redis", {
        channel,
        message: payload,
      });
    }
  });
  stoppers.push(async () => {
    await sub.quit();
  });
  console.log(`Redis Trigger listening on ${channels.join(", ")}`);
}

async function startRabbitmq(): Promise<void> {
  const url = config.rabbitmqUrl;
  if (!url) return;
  let amqp: typeof import("amqplib");
  try {
    amqp = await import("amqplib");
  } catch {
    console.warn("amqplib not installed — RabbitMQ Trigger disabled");
    return;
  }
  const targets = await workflowsWithTrigger("rabbitmqTrigger");
  if (!targets.length) return;

  const conn = await amqp.connect(url);
  const ch = await conn.createChannel();
  for (const t of targets) {
    const queue = String(t.node.parameters.queue ?? "dataflow");
    await ch.assertQueue(queue, { durable: true });
    await ch.consume(queue, (msg) => {
      if (!msg) return;
      const raw = msg.content.toString("utf8");
      let payload: unknown = raw;
      try {
        payload = JSON.parse(raw);
      } catch {
        /* keep */
      }
      void fire(String(t.workflow._id), "rabbitmq", {
        queue,
        message: payload,
        properties: msg.properties,
      }).finally(() => ch.ack(msg));
    });
  }
  stoppers.push(async () => {
    await ch.close();
    await conn.close();
  });
  console.log(`RabbitMQ Trigger listening (${targets.length} workflow(s))`);
}

async function startLocalFiles(): Promise<void> {
  const targets = await workflowsWithTrigger("localFileTrigger");
  for (const t of targets) {
    const watchPath = String(t.node.parameters.path ?? "");
    if (!watchPath || !fs.existsSync(watchPath)) continue;
    const watcher = fs.watch(watchPath, { recursive: true }, (event, file) => {
      void fire(String(t.workflow._id), "localFile", {
        event,
        file: file ? path.join(watchPath, file) : watchPath,
      });
    });
    stoppers.push(() => watcher.close());
  }
  if (targets.length) {
    console.log(`Local File Trigger watching ${targets.length} path(s)`);
  }
}

function parseRssItems(xml: string): Array<{ id: string; title: string; link?: string }> {
  const items: Array<{ id: string; title: string; link?: string }> = [];
  const blocks = xml.match(/<item[\s>][\s\S]*?<\/item>|<entry[\s>][\s\S]*?<\/entry>/gi) ?? [];
  for (const block of blocks) {
    const title =
      block.match(/<title[^>]*>(?:<!\[CDATA\[)?([\s\S]*?)(?:\]\]>)?<\/title>/i)?.[1]?.trim() ??
      "";
    const guid =
      block.match(/<guid[^>]*>(?:<!\[CDATA\[)?([\s\S]*?)(?:\]\]>)?<\/guid>/i)?.[1]?.trim() ||
      block.match(/<id[^>]*>([\s\S]*?)<\/id>/i)?.[1]?.trim() ||
      block.match(/<link[^>]*href=["']([^"']+)["']/i)?.[1]?.trim() ||
      block.match(/<link[^>]*>([\s\S]*?)<\/link>/i)?.[1]?.trim() ||
      title;
    const link =
      block.match(/<link[^>]*href=["']([^"']+)["']/i)?.[1]?.trim() ||
      block.match(/<link[^>]*>([\s\S]*?)<\/link>/i)?.[1]?.trim();
    if (!guid && !title) continue;
    items.push({ id: guid || title, title, link });
  }
  return items;
}

async function startRssPollers(): Promise<void> {
  const targets = await workflowsWithTrigger("rssFeedTrigger");
  if (!targets.length) return;

  const pollMs = 60_000;
  const tick = async () => {
    for (const t of targets) {
      const url = String(t.node.parameters.url ?? "").trim();
      if (!url) continue;
      const key = `${t.workflow._id}:${url}`;
      try {
        const res = await fetch(url, {
          headers: { Accept: "application/rss+xml, application/atom+xml, application/xml, text/xml" },
          signal: AbortSignal.timeout(20_000),
        });
        if (!res.ok) continue;
        const xml = await res.text();
        const items = parseRssItems(xml);
        let seen = rssSeen.get(key);
        if (!seen) {
          seen = new Set(items.map((i) => i.id));
          rssSeen.set(key, seen);
          continue; // seed without firing
        }
        for (const item of items) {
          if (seen.has(item.id)) continue;
          seen.add(item.id);
          void fire(String(t.workflow._id), "rss", {
            url,
            title: item.title,
            link: item.link,
            guid: item.id,
          });
        }
        // Cap memory
        if (seen.size > 500) {
          const trimmed = [...seen].slice(-250);
          rssSeen.set(key, new Set(trimmed));
        }
      } catch (err) {
        console.warn(`[rss] poll failed for ${url}:`, err);
      }
    }
  };

  void tick();
  const handle = setInterval(() => void tick(), pollMs);
  stoppers.push(() => clearInterval(handle));
  console.log(`RSS Feed Trigger polling ${targets.length} feed(s)`);
}

export async function stopTriggerListeners(): Promise<void> {
  const current = stoppers;
  stoppers = [];
  for (const stop of current) {
    try {
      await stop();
    } catch (err) {
      console.warn("[trigger] stop failed", err);
    }
  }
}

export async function startTriggerListeners(): Promise<void> {
  await Promise.all([
    startRedis(),
    startRabbitmq(),
    startLocalFiles(),
    startRssPollers(),
  ]);
}

/** Restart all listeners after workflow save (Redis/Rabbit/file/RSS). */
export async function refreshTriggerListeners(): Promise<void> {
  await stopTriggerListeners();
  await startTriggerListeners();
}
