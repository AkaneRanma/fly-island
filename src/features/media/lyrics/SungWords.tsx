import { useState, type CSSProperties } from "react";
import { wordFill, type LyricWord } from "./lrc";
import "./SungWords.css";

/**
 * A line timed word by word: each word fills from dim to bright as it is sung. The timings are
 * taken once, when the line appears or the player reports a new position (the caller re-keys it),
 * and the browser runs them from there; pausing pauses them. The music page and the collapsed
 * island both show a line this way.
 */
export function SungWords({ words, positionMs, playing }: { words: LyricWord[]; positionMs: number; playing: boolean }) {
  const [start] = useState(positionMs);
  return words.map((word, index) => {
    const { delayMs, durationMs } = wordFill(word, start);
    const style = {
      animationDelay: `${delayMs}ms`,
      animationDuration: `${durationMs}ms`,
      animationPlayState: playing ? "running" : "paused",
    } as CSSProperties;
    return <span className="sung-word" key={index} style={style}>{word.text}</span>;
  });
}
