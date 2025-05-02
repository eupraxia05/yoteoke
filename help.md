# YoteOke Lyric Editor

A karaoke video creator with manual lyric syncing.

## Creating a Project

- *File->New...* opens the dialog to create a new project. Enter the song's artist and title, select the song file, and click "Create".
- Use the file dialog to select where to save the project file.

## Editing Lyrics

- Use the text editor on the left to edit song lyrics.
- Lyrics are separated into **blocks** by empty lines. **Blocks** are the sections of text that appear at once.
- Timestamps use the syntax `[mm:ss.uuu]` and specify the time that the next character is sung.
- The "Insert" button above the text editor will insert a timestamp at the current playhead time.

# Playback

- Use the playback controls at the right to control preview video playback.

# Project Settings

*Project->Project Settings...* opens the project settings dialog. The following settings can be set from here:
- Background Color
- Lyric Color (sung/unsung)
- Titlecard
  - A titlecard image to show at the beginning.
- Titlecard Show Time
  - How long to show the titlecard for, in seconds.
- Song Pre-Delay
  - Delays the song by the given number of seconds before starting.

# Exporting

*Project->Export...* initiates project export. Use the file dialog that appears to select the output video path, then click "Save" to begin exporting.

The video will then begin playing back, saving to the specified file.