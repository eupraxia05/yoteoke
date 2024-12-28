use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy_egui::egui::Style;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_image_export::ImageExportPlugin;
use egui_file::FileDialog;
use export::ExportState;
use kira::sound::static_sound::StaticSoundHandle;
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::sound::{FromFileError, PlaybackState, SoundData};
use kira::tween::Tween;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::ops::RangeInclusive;
use std::path::{PathBuf, Path};
use std::time::Duration;
use std::{fs::File, io::Read};
use kira::{
  manager::{
    AudioManager, 
    AudioManagerSettings, 
    backend::DefaultBackend
  },
  sound::static_sound::{
    StaticSoundData, 
    StaticSoundSettings
  }, 
};

use bevy::render::view::RenderLayers;
use bevy::text::TextLayoutInfo;
use std::collections::HashMap;

mod lyrics;
use crate::lyrics::ParsedLyrics;

mod ui;
use crate::ui::NewProjectDialog;
use crate::ui::ProjectSettingsDialog;

mod sub_viewport;
use crate::sub_viewport::SubViewport;

mod export;

use bevy_tokio_tasks::TokioTasksPlugin;

#[derive(Component)]
struct PreviewText;

fn main() {
  let mut app = App::new();

  let export_plugin = ImageExportPlugin::default();
  let export_threads = export_plugin.threads.clone();
    
  app
    .add_plugins(DefaultPlugins
      .set(RenderPlugin {
        synchronous_pipeline_compilation: true,
        ..default()
      })
    )
    .add_plugins(export_plugin)
    .add_plugins(EguiPlugin)
    .insert_resource(EditorState::default())
    .add_systems(Startup, setup)
    .add_systems(Update, update)
    .add_systems(Update, center_text_hack)
    .add_systems(Update, update_preview)
    .add_plugins(TokioTasksPlugin::default());

  ui::build(&mut app);
  sub_viewport::build(&mut app);
  export::build(&mut app);

  app.run();

  export_threads.finish();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, 
  mut editor_state: ResMut<EditorState>)
{
  match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
  {
    Ok(m) => {
      editor_state.audio_manager = Some(m);
    },
    Err(e) => {
      error!("Error initializing Kira audio manager: {:?}", e); 
    }
  }

  commands.spawn(SubViewport::new(RenderLayers::layer(1)));
  commands.spawn(
    (
      Text2d("".into()), 
      TextFont {
        font_size: 64.0,
        ..Default::default()
      },
      TextLayout::new_with_justify(JustifyText::Right),
      RenderLayers::layer(1), 
      PreviewText
    )
  );
}

fn update_preview(mut editor_state: ResMut<EditorState>,
  mut preview_text_query: Query<(&PreviewText, &mut Text2d)>,
  export_state: Res<ExportState>)
{
  let song_position = if export_state.is_exporting() {
    Duration::from_secs_f64(export_state.frame_idx() as f64 / 1.)
  } else {
    editor_state.playhead_position()
  };

  if let Some(lyrics) = editor_state.parsed_lyrics.as_ref() {
    if let Some(music_handle) = editor_state.music_handle.as_ref() {
      if let Some(block) = lyrics.get_block_at_time(&song_position, &Duration::from_secs(3)) {
        let mut text = "".to_string();
        for line in &block.lines {
          text += &line.line.clone();
          text += "\n"
        }
        preview_text_query.single_mut().1.0 = text.clone();
      } else {
        preview_text_query.single_mut().1.0 = "".to_string();
      }
    }
  }
}

fn update(mut editor_state: ResMut<EditorState>, asset_server: Res<AssetServer>,
  mut preview_text_query: Query<(&PreviewText, &mut Text2d)>,
  mut camera_tex_query: Query<&mut SubViewport>) 
{
  editor_state.update(asset_server.as_ref());

  if let Some(project_data) = &editor_state.project_data {
    camera_tex_query.single_mut().clear_color = ClearColorConfig::Custom(project_data.background_color.unwrap_or_default());
  }
}

#[derive(Resource, Default)]
struct EditorState {
  project_file_path: PathBuf,
  text: String,
  file_dialog: Option<FileDialog>,
  project_data: Option<ProjectData>,
  new_file_dialog: Option<NewProjectDialog>,
  open_dialog: Option<FileDialog>,
  music_handle: Option<StreamingSoundHandle<FromFileError>>,
  audio_manager: Option<AudioManager>,
  duration: Option<Duration>,
  parsed_lyrics: Option<ParsedLyrics>,
  lyrics_dirty: bool,
  project_settings_dialog: ProjectSettingsDialog,
}

impl EditorState {
    fn set_project_file_path(&mut self, path: PathBuf) {
        self.project_file_path = path
    }

    fn new(&mut self) {
        let mut new_file_dialog = NewProjectDialog::default();
        new_file_dialog.open();
        self.new_file_dialog = Some(new_file_dialog);
        self.project_data = Some(ProjectData::default());
    }

    fn save(&mut self) {
        let mut vec = Vec::new();
        if let Some(project_data) = &self.project_data {
            vec = serde_json::to_vec_pretty(&project_data).unwrap();
        }

        println!("Saving to {:?}", self.project_file_path);

        File::create(self.project_file_path.clone())
            .unwrap()
            .write_all(&vec[..]);
    }

    fn open(&mut self, path: &Path) {
        self.project_file_path = PathBuf::from(path);
        if let Ok(file) = File::open(path) {
            if let Ok(data) = serde_json::from_reader::<_, ProjectData>(file) {
                self.project_data = Some(data);
                self.lyrics_dirty = true;
            } else {
                println!("couldn't deserialize file");
            }
        } else {
            println!("couldn't open file {:?}", path);
        }
    }

    fn playhead_position(&self) -> Duration {
      if let Some(music_handle) = &self.music_handle {
        return Duration::from_secs_f64(music_handle.position());
      } else {
        return Duration::default();
      }
    }

    fn update(&mut self, asset_server: &AssetServer) {
      if let Some(new_file_dialog) = &self.new_file_dialog {
        if new_file_dialog.is_submitted {
          let mut project_data = ProjectData::default();
          project_data.artist = new_file_dialog.artist.clone();
          project_data.title = new_file_dialog.title.clone();
          project_data.song_file = new_file_dialog.song_file.clone();
          self.project_data = Some(project_data);
          self.project_file_path = new_file_dialog.save_file.clone().unwrap();
          self.save();
        }
      }

      if self.lyrics_dirty {
        info!("updating lyrics");
        self.parsed_lyrics = None;
        match ParsedLyrics::parse(&self.project_data.as_ref().unwrap().lyrics) {
          Ok(lyrics) => {
            self.parsed_lyrics = Some(lyrics);
          },
          Err(err) => {
            error!("Error parsing lyrics: {:?}", err);
          }
        }
        self.lyrics_dirty = false;
      }

      if self.music_handle.is_none() {
        let mut music = None;
        if let Some(project_data) = &self.project_data {
          if let Some(song_file) = &project_data.song_file {
              match StreamingSoundData::from_file(song_file) {
                  Ok(data) => {
                      music = Some(data);
                  },
                  Err(e) => {
                      error!("Failed to load music file {:?}: {:?}", 
                          project_data.song_file, e);
                  }
              }
          }
        }

        if let Some(music) = music {
          self.duration = Some(music.duration());
          let play_result = self.audio_manager.as_mut().unwrap().play(music);
          match play_result {
              Ok(mut handle) => {
                  info!("sound played successfully");
                  handle.pause(Tween::default());
                  handle.set_loop_region(..);
                  self.music_handle = Some(handle);
              },
              Err(e) => {
                  error!("Failed to play sound: {:?}", e);
              }
          }
        }
      }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct ProjectData {
    lyrics: String,
    artist: String,
    title: String,
    song_file: Option<PathBuf>,
    background_color: Option<Color>,
}

pub(crate) fn center_text_hack(
    mut query: Query<(&TextLayout, &mut TextLayoutInfo), (Changed<TextLayoutInfo>, With<Text2d>)>,
) {
    for (_, mut text_info) in query
        .iter_mut()
        .filter(|(layout, _)| layout.justify == JustifyText::Center)
    {
        // find max x position for each text section
        let mut text_section_max_pos: HashMap<usize, f32> = HashMap::new();
        for positioned_glyph in text_info.glyphs.iter() {
            text_section_max_pos
                .entry(positioned_glyph.span_index)
                .and_modify(|value| {
                    if *value < positioned_glyph.position.x {
                        *value = positioned_glyph.position.x;
                    }
                })
                .or_insert(positioned_glyph.position.x);
        }

        // find max x for whole text
        let max_pos = text_section_max_pos
            .values()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Less));

        if let Some(max_pos) = max_pos {
            // calculate correction for x for each section if needed
            let to_correct: HashMap<usize, f32> =
                HashMap::from_iter(text_section_max_pos.iter().filter_map(|(k, v)| {
                    if v < max_pos {
                        Some((*k, (max_pos - v) / 2.))
                    } else {
                        None
                    }
                }));

            // apply x correction
            for positioned_glyph in text_info.glyphs.iter_mut() {
                if let Some(x_fix) = to_correct.get(&positioned_glyph.span_index) {
                    positioned_glyph.position.x += x_fix;
                }
            }
        }
    }
}