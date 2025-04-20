use std::time::Duration;

use regex::Regex;
use std::ops::Range;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::editor::EditorState;

pub struct LyricsPlugin;

impl Plugin for LyricsPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, update);
  }
}

fn update(mut editor_state: NonSendMut<EditorState>) {
  if editor_state.lyrics_dirty {
    info!("updating lyrics");
    editor_state.parsed_lyrics = None;
    match ParsedLyrics::parse(&editor_state.project_data.as_ref().unwrap().lyrics) {
      Ok(lyrics) => {
        editor_state.parsed_lyrics = Some(lyrics);
      },
      Err(err) => {
        error!("Error parsing lyrics: {:?}", err);
      }
    }
    editor_state.lyrics_dirty = false;
  }
}

pub struct ParsedLyrics {
  pub blocks: Vec<Block>,
}

impl ParsedLyrics {
    pub fn parse(lyrics: &String) -> Result<ParsedLyrics, String> {        
        let mut blocks = Vec::new();
        let normalized_lyrics = String::from_iter(normalize_line_endings::normalized(lyrics.chars()));
        let lines = normalized_lyrics.lines();

        let mut curr_block = Block::default();
        for line in lines {
            let line = line.trim();
            assert!(!line.contains("\r"));
            if !line.is_empty() {
                let (tags, line_without_tags) = Self::extract_tags(line);
                for tag in tags {
                    let timecode_regex = Regex::new(r"\[([0-9]+):([0-9]+).([0-9]+)\]").unwrap();
                    if let Some(captures) = timecode_regex.captures(&tag.tag) {
                        let minutes: u32 = captures.get(1).unwrap().as_str().parse().unwrap();
                        let seconds: u32 = captures.get(2).unwrap().as_str().parse().unwrap();
                        let millis: u32 = captures.get(3).unwrap().as_str().parse().unwrap();
                        let timestamp = Timestamp {
                            position: tag.position + curr_block.lyrics.len(),
                            time: Duration::from_secs_f32(minutes as f32 * 60. + seconds as f32 + millis as f32 / 1000.)
                        };
                        curr_block.timestamps.push(timestamp);
                    }
                }
                curr_block.lyrics.push_str(&line_without_tags);
                curr_block.lyrics.push_str("\n");
            } else {
                if curr_block.lyrics.len() > 0 {
                    blocks.push(curr_block.clone());
                    curr_block = Block::default();
                }
            }
        }
        
        Ok(ParsedLyrics {
            blocks,
        })
    }

    fn extract_tags(line: &str) -> (Vec<LyricTag>, String) {
        let mut tags = Vec::new();
        let mut stripped_line = "".to_string();

        let mut tag_start = None;
        let mut tag_ranges = Vec::new();
        line.char_indices().for_each(|(i, c)| {
            if c == '[' {
                tag_start = Some(i);
            }
            else if c == ']' && tag_start.is_some() {
                tag_ranges.push(tag_start.unwrap()..=i);
                tag_start = None;
            } else if tag_start.is_none() {
                stripped_line.push(c);
            }
        });

        let mut tag_len_so_far = 0;
        for range in tag_ranges {
            tags.push(LyricTag {
                position: *range.start() - tag_len_so_far + 1,
                tag: line[range.clone()].into()
            });
            tag_len_so_far += range.end() - range.start() + 1;
        }

        (tags, stripped_line)
    }

    pub fn get_block_at_time(&self, time: &Duration, lead_time: &Duration) -> Option<&Block> {
        for block in &self.blocks {
            if let Some(time_range) = block.get_time_range() {
                let check_range_min = *time;
                let check_range_max = *time + *lead_time;
                if time_range.start <= check_range_max && check_range_min <= time_range.end {
                    return Some(block);
                }
            }
        }
        
        None
    }
}

#[derive(Default, Clone)]
pub struct Block {
    pub lyrics: String,
    pub timestamps: Vec<Timestamp>,
}

impl Block {
    pub fn get_time_range(&self) -> Option<Range<Duration>> {
        let mut first_timestamp = None;
        let mut last_timestamp = None;

        for timestamp in &self.timestamps {
            if first_timestamp.is_none() {
                first_timestamp = Some(timestamp);
            } else {
                last_timestamp = Some(timestamp);
            }
        }

        if let Some(first_timestamp) = first_timestamp {
            if let Some(last_timestamp) = last_timestamp {
                return Some(first_timestamp.time..last_timestamp.time);
            }
        }
        None
    }

    pub fn start_time(&self) -> Option<Duration> {
        if self.timestamps.len() > 0 {
            return Some(self.timestamps[0].time)
        }

        None
    }

    pub fn end_time(&self) -> Option<Duration> {
        if self.timestamps.len() > 0 {
            return Some(self.timestamps[self.timestamps.len() - 1].time)
        }

        None
    }

    pub fn get_timestamps_surrounding(&self, time: &Duration) -> 
      Option<(Timestamp, Timestamp)> 
    {
        if let Some(start_time) = self.start_time() {
            if let Some(end_time) = self.end_time() {
                if time < &start_time || time > &end_time {
                    return None
                }

                for (idx, timestamp) in self.timestamps.iter().enumerate() {
                    if timestamp.time > *time  {
                        if idx == 0 {
                            return None
                        }

                        return Some((self.timestamps[idx - 1].clone(), 
                            self.timestamps[idx].clone()));
                    }
                }
            }
        }

        None
    }
}

#[derive(Debug)]
struct LyricTag {
    position: usize,
    tag: String
}

#[derive(Clone, Debug)]
pub struct Timestamp {
    pub position: usize,
    pub time: Duration
}

pub fn lyrics_edit_ui(mut ui: InMut<egui::Ui>, mut editor_state: NonSendMut<EditorState>) {
  let mut text_edit_changed = false;
  let mut cursor_pos = None;
  let mut insert_desired = false;
  let curr_time = if let Some(music_handle) = &editor_state.music_handle {
    Duration::from_secs_f64(music_handle.position())
  } else {
    Duration::default()
  };
  if let Some(project_data) = &mut editor_state.project_data {
    let title_str = format!("{} - {}", project_data.artist, project_data.title);
    ui.label(title_str);
    if ui.button("Insert").clicked() {
      insert_desired = true;
    }
    ui.separator();
    egui::ScrollArea::both().show(&mut ui, |ui| {
      let text_edit_response = ui.add_sized(ui.available_size(), 
        egui::TextEdit::multiline(&mut project_data.lyrics).code_editor());
      if text_edit_response.changed() {
        info!("text edit changed");
        text_edit_changed = true;
      }
      if let Some(text_edit_state) = egui::text_edit::TextEditState::load(ui.ctx(), 
        text_edit_response.id) 
      {
        if let Some(char_range) = text_edit_state.cursor.char_range() {
          cursor_pos = Some(char_range.primary);
        }
      }
    });
    if insert_desired {
      if let Some(cursor_pos) = cursor_pos {
        let str_to_insert = format!("[{:0>2}:{:0>2}.{:0>3}]", 
          curr_time.as_secs() / 60, curr_time.as_secs() % 60, curr_time.subsec_millis());
        project_data.lyrics.insert_str(cursor_pos.index, &str_to_insert);
        text_edit_changed = true
      }
    }
    // hack: keep carriage returns from entering lyrics
    project_data.lyrics = project_data.lyrics.replace("\r", "");
  }
  if text_edit_changed {
    info!("lyrics marked dirty");
    editor_state.lyrics_dirty = true;
  }
}