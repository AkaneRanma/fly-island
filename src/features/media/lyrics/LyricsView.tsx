import type { ReactNode } from "react";
import { useTranslation } from "../../../i18n";
import type { MediaState } from "../api";
import type { LyricLine } from "./lrc";
import { SungWords } from "./SungWords";
import { useLyricClock } from "./useLyricClock";
import "./LyricsView.css";
import { hasTimeline } from "../trackClock";

/**
 * Three lines of synced lyrics: the one being sung, large and bright, between
 * the one before and the one after. A line with a translation shows it under itself in place
 * of the next line. `offsetMs` moves the lyrics later (positive) or earlier.
 */
export function LyricsView({ lines, mediaState, offsetMs, onPosition, wordFillEnabled = true }: {
  lines: LyricLine[]; mediaState: MediaState; offsetMs: number; onPosition?: (ms: number) => void; wordFillEnabled?: boolean;
}) {
  const t = useTranslation();
  const { index, position, playing } = useLyricClock(lines, mediaState, offsetMs);
  const words = wordFillEnabled && index >= 0 ? lines[index].words : undefined;
  const text = (at: number) => (at >= 0 && at < lines.length ? lines[at].text : "");
  // Before the first line (the intro) and in breaks, a note holds the middle row, so the first
  // line waits below it rather than standing alone.
  const current = (index >= 0 && text(index)) || "♪";
  const row = (at: number, className: string, content: ReactNode, translation = false) => {
    const line = lines[at];
    const clickable = onPosition && line?.text;
    // 校准落在歌词时刻加偏移的位置，和当前歌词时钟保持同一坐标系。
    return clickable ? <button type="button" key={`${at}:${translation}`} className={`lyrics-view__line ${className} lyrics-view__line--clickable`}
      aria-label={(hasTimeline(mediaState) ? t.music.seekLine : t.music.calibrateLine)(line.text)}
      onClick={() => onPosition(Math.max(0, line.timeMs + offsetMs))}>{content}</button>
      : <span key={`${at}:${translation}`} className={`lyrics-view__line ${className}`} aria-hidden={className.endsWith("--side") ? true : undefined}>{content}</span>;
  };

  return (
    <div className="lyrics-view" aria-label={t.music.lyricsLabel}>
      {row(index - 1, "lyrics-view__line--side", text(index - 1))}
      {row(index, "lyrics-view__line--current", words ? <SungWords key={`${mediaState.updatedAt}:${offsetMs}`} words={words} positionMs={position} playing={playing} /> : current)}
      {index >= 0 && lines[index].translation ? (
        row(index, "lyrics-view__line--translation", lines[index].translation, true)
      ) : (
        row(index + 1, "lyrics-view__line--side", text(index + 1))
      )}
    </div>
  );
}
