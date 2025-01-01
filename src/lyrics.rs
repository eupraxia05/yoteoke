use std::time::Duration;

use regex::Regex;
use std::ops::Range;
use bevy::prelude::*;

pub struct ParsedLyrics {
    pub blocks: Vec<Block>,
}

impl ParsedLyrics {
    pub fn parse(lyrics: &String) -> Result<ParsedLyrics, String> {        
        let mut blocks = Vec::new();
        let lines = lyrics.lines();

        let mut curr_block = Block::default();
        for line in lines {
            let line = line.trim();
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
                position: *range.start() - tag_len_so_far,
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