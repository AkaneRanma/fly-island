import "../../src/styles/tokens.css";
import { act, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { mockIPC } from "@tauri-apps/api/mocks";
import { CollapsedLyric } from "../../src/features/media/CollapsedLyric";
import { DEFAULT_MEDIA_STATE } from "../../src/features/media/mediaState";

Object.assign(globalThis, { isTauri: true, IS_REACT_ACT_ENVIRONMENT: true });
const root = createRoot(document.getElementById("fixture")!);
const results = document.getElementById("results")!;
const passed: string[] = [];
const originalNow = Date.now;
let now = originalNow();
Date.now = () => now;
const anchor = now;
let finish: ((value: unknown) => void) | undefined;
const calls: string[] = [];
const lrc = "[00:01.00]first\n[00:05.00]second\n[00:09.00]third";
// A line whose source times each word (enhanced LRC), for the word-by-word view.
const wordedLrc = "[00:59.00]<00:59.00>one <00:59.50>two<01:00.00>";
mockIPC((command, payload) => {
  if (command !== "get_lyrics") return null;
  const { title } = (payload as { query: { title: string } }).query;
  calls.push(title);
  if (title === "late") return new Promise((resolve) => { finish = resolve; });
  if (title === "failed") return Promise.reject(new Error("lookup failed fixture"));
  if (title === "worded") return { lrc: wordedLrc };
  return { lrc };
});
async function render(title: string, active: boolean, durationMs = 0) {
  await act(async () => {
    root.render(<StrictMode><CollapsedLyric active={active}
      mediaState={{ ...DEFAULT_MEDIA_STATE, title, artist: "artist", available: true, playbackStatus: "playing",
        durationMs, positionMs: 0, updatedAt: anchor }}
      lyricsSettings={{ lyricsFolder: "", lyricsMusicFolder: "", lyricsOnline: true, lyricsOffsetMs: 0 }} /></StrictMode>);
  });
}
function check(condition: unknown, message: string) {
  if (!condition) throw new Error(message);
  passed.push(`PASS ${message}`);
}
const text = () => document.querySelector(".collapsed-lyric")?.textContent;
try {
  await render("prefetched", false);
  check(calls.length === 1, "展开且未进入音乐页时预取，StrictMode 不重复请求");
  now += 6100;
  await render("prefetched", true, 60_000);
  check(text() === "second" && calls.length === 1, "收起立即按已流逝时间显示正确句子，0 时长变真实时长不重查");
  await render("late", true);
  now += 3000;
  await act(async () => { finish?.({ lrc }); });
  check(text() === "third", "歌词迟到时读取当前时间，不退回歌名或旧句");
  await render("failed", false);
  await render("prefetched", false);
  await render("failed", true);
  check(calls.filter((title) => title === "failed").length === 1, "失败结果 60 秒内复用");
  now += 60_001;
  await render("prefetched", false);
  await render("failed", false);
  check(calls.filter((title) => title === "failed").length === 2, "失败缓存过期后可重试");
  await render("worded", true, 60_000);
  const words = [...document.querySelectorAll<HTMLElement>(".collapsed-lyric .sung-word")];
  check(words.length === 2 && words.every((word) => word.style.animationDuration !== ""), "收起态逐字：源给了词时间就按词高亮");
  check(text() === "one two", "逐字高亮时整行文字不变");
  results.dataset.result = "passed";
  results.textContent = passed.join("\n");
} catch (error) {
  results.dataset.result = "failed";
  results.textContent = `${passed.join("\n")}\nFAIL ${String(error)}`;
} finally {
  await act(async () => root.unmount());
  Date.now = originalNow;
}
