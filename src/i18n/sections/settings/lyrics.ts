export type SettingsLyricsText = {
  title: string;
  hint: string;
  musicFolder: string;
  musicFolderHint: string;
  musicFolderPlaceholder: string;
  online: string;
  onlineHint: string;
  chinese: string;
  chineseHint: string;
};

export const zhCN: SettingsLyricsText = {
  title: "歌词",
  hint: "找到带时间轴的歌词时，音乐页会跟着播放滚动显示。某首歌的歌词对不上时，可以在音乐页上单独调它的快慢。",
  musicFolder: "音乐文件夹",
  musicFolderHint: "填上放歌的文件夹，岛屿先读歌曲文件里自带的歌词：同名的 .lrc 文件优先，其次读文件里写入的歌词（FLAC 的 LYRICS 标签、MP3 的 USLT 帧），逐字歌词也能用。子文件夹都会找，不联网。",
  musicFolderPlaceholder: "D:\\音乐",
  online: "在线查找歌词",
  onlineHint: "查 LRCLIB（lrclib.net，免费开源的歌词库），会把歌名、歌手、专辑和时长发过去。每首歌只查一次，结果存在本机。中文歌收录较少。",
  chinese: "也查酷狗和网易云",
  chineseHint: "中文歌收录多，有翻译。这两个是非官方接口，可能随时失效；会把歌名、歌手和时长发给它们。装了网易云的 InfLink 插件时按歌曲直接查，更准。",
};

export const en: SettingsLyricsText = {
  title: "Lyrics",
  hint: "When synced lyrics are found, the music page scrolls them along with the song. A song whose lyrics run out of step can be moved on its own on the music page.",
  musicFolder: "Music folder",
  musicFolderHint: "Point this at the folder your songs are in and the island reads the lyrics the files carry: a .lrc file beside a track wins, then the lyrics written into the file itself (a FLAC LYRICS tag, an MP3 USLT frame), word timing included. Subfolders are searched; nothing goes online.",
  musicFolderPlaceholder: "D:\\Music",
  online: "Find lyrics online",
  onlineHint: "Asks LRCLIB (lrclib.net, a free and open lyrics library), sending the title, artist, album and length. Each song is asked about once and the answer is kept on this computer.",
  chinese: "Also ask Kugou and NetEase",
  chineseHint: "Many more Chinese songs, with translations. These are unofficial interfaces that may stop working at any time; the title, artist and length are sent to them. With NetEase's InfLink plugin the song is looked up by its id, which is more exact.",
};
