use bevy::prelude::*;
use std::path::PathBuf;
use std::time::Duration;
use kira::sound::streaming::StreamingSoundHandle;
use kira::AudioManager;
use kira::sound::FromFileError;
use bevy_egui::egui;

use crate::NewProjectDialog;
use crate::ParsedLyrics;
use crate::ProjectSettingsDialog;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
  fn build(&self, app: &mut App) {
    app.insert_non_send_resource(EditorState::default());
  }
}

#[derive(Default)]
pub struct EditorState {
  pub project_file_path: PathBuf,
  pub project_data: Option<crate::project::ProjectData>,
  pub new_file_dialog: Option<NewProjectDialog>,
  pub music_handle: Option<StreamingSoundHandle<FromFileError>>,
  pub audio_manager: Option<AudioManager>,
  pub duration: Option<Duration>,
  pub parsed_lyrics: Option<ParsedLyrics>,
  pub lyrics_dirty: bool,
  pub project_settings_dialog: ProjectSettingsDialog,
  pub thumbnail_image: Option<Handle<Image>>,
  pub thumbnail_egui_tex_id: Option<egui::TextureId>,
}

impl EditorState {
    pub fn playhead_position(&self) -> Duration {
      if let Some(music_handle) = &self.music_handle {
        return Duration::from_secs_f64(music_handle.position());
      } else {
        return Duration::default();
      }
    }
}