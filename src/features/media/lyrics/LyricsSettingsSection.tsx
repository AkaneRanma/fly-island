import { SettingsGroup } from "../../../components/settings/SettingsGroup";
import { SettingsInput } from "../../../components/settings/SettingsInput";
import { SettingsRow } from "../../../components/settings/SettingsRow";
import { SettingsToggleRow } from "../../../components/settings/SettingsToggleRow";
import type { IslandSettings } from "../../appearance/types";
import { useTranslation } from "../../../i18n";

/**
 * Settings → Music → Lyrics: the music folder whose songs are asked for the lyrics they carry
 * themselves, looking lyrics up online, and the Chinese sources with them. Timing is fixed per song
 * on the music page itself (LyricsOffsetControl).
 */
export function LyricsSettingsSection({
  settings,
  onSettingsChange,
}: {
  settings: IslandSettings;
  onSettingsChange: (settings: IslandSettings) => void;
}) {
  const text = useTranslation().settings.lyrics;

  return (
    <SettingsGroup title={text.title} footer={text.hint}>
      <SettingsRow title={text.musicFolder} description={text.musicFolderHint}>
        <SettingsInput
          monospace
          autoComplete="off"
          aria-label={text.musicFolder}
          value={settings.lyricsMusicFolder}
          placeholder={text.musicFolderPlaceholder}
          onChange={(event) => {
            const lyricsMusicFolder = event.currentTarget.value;
            onSettingsChange({ ...settings, lyricsMusicFolder });
          }}
        />
      </SettingsRow>
      <SettingsToggleRow
        title={text.online}
        description={text.onlineHint}
        checked={settings.lyricsOnline}
        onChange={(lyricsOnline) => onSettingsChange({ ...settings, lyricsOnline })}
      />
      {settings.lyricsOnline && (
        <SettingsToggleRow
          indent
          title={text.chinese}
          description={text.chineseHint}
          checked={settings.lyricsChinese}
          onChange={(lyricsChinese) => onSettingsChange({ ...settings, lyricsChinese })}
        />
      )}
    </SettingsGroup>
  );
}
