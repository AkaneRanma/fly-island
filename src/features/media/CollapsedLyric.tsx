import type { MediaState } from "./api";
import { songOffsetKey } from "./lyrics/lrc";
import { SungWords } from "./lyrics/SungWords";
import { useLyricClock } from "./lyrics/useLyricClock";
import { useLyrics, type LyricsSettings } from "./lyrics/useLyrics";
import { useSongOffset } from "./lyrics/useSongOffset";
import "./CollapsedLyric.css";

/**
 * The line being sung, for the collapsed island, word by word when the source times words and the
 * setting asks for it; the song's title and artist when it has no lyrics or between lines. It wakes
 * on its own for each line, so only this leaf re-renders while a song plays; a new line replaces the
 * last at once (the music page lets it rise in). Keep prefetching while expanded; only the visible
 * leaf drives the line-change clock.
 */
export function CollapsedLyric({
  mediaState,
  lyricsSettings,
  active,
}: {
  mediaState: MediaState;
  lyricsSettings: LyricsSettings & { lyricsOffsetMs: number };
  active: boolean;
}) {
  const { lines } = useLyrics(mediaState, lyricsSettings);
  const [songOffsetMs] = useSongOffset(lines ? songOffsetKey(mediaState) : null);
  const offsetMs = lyricsSettings.lyricsOffsetMs + songOffsetMs;
  const { index, position, playing } = useLyricClock(lines, mediaState, offsetMs, active);
  const song = [mediaState.title, mediaState.artist].filter(Boolean).join(" · ");
  const line = lines && index >= 0 ? lines[index] : null;
  const words = lyricsSettings.lyricWordFill === true && line?.text ? line.words : undefined;
  const running = playing && active;

  // Re-keyed on a seek, a calibration or a pause, so the fill restarts from the true position.
  return (
    <span className="collapsed-lyric">
      {words
        ? <SungWords key={`${index}:${mediaState.updatedAt}:${offsetMs}:${running}`} words={words} positionMs={position} playing={running} />
        : line?.text || song}
    </span>
  );
}
