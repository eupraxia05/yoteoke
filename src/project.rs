use bevy::prelude::*;
use bevy_egui::egui;
use std::path::PathBuf;
use bevy_file_dialog::{prelude::*, FileDialog};
use std::fs::File;
use std::io::Write;
use serde::{Serialize, Deserialize};

use crate::editor::EditorState;

pub struct ProjectPlugin;

impl Plugin for ProjectPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, handle_new_project_requested_events);
    app.add_systems(Update, handle_new_project_song_file_dialog_picked);
    app.add_systems(Update, handle_new_project_save_file_dialog);
    app.add_systems(Update, handle_new_project_dialog_submitted_events);
    app.add_systems(Update, handle_save_project_requested_event);
    app.add_event::<NewProjectRequestedEvent>();
    app.add_event::<NewProjectDialogSubmittedEvent>();
  }
}

#[derive(Event, Default)]
pub struct NewProjectRequestedEvent;

fn handle_new_project_requested_events(mut events: EventReader<NewProjectRequestedEvent>, mut editor_state: NonSendMut<EditorState>) {
  for ev in events.read() {
    let mut new_file_dialog = NewProjectDialog::default();
    new_file_dialog.open();
    editor_state.new_file_dialog = Some(new_file_dialog);
  }
}

pub struct NewProjectDialog {
  is_open: bool,
  pub is_submitted: bool,
  pub artist: String,
  pub title: String,
  pub song_file: Option<PathBuf>,
  pub save_file: Option<PathBuf>,
}

impl Default for NewProjectDialog {
  fn default() -> Self {
      Self {
          is_open: false,
          is_submitted: false,
          artist: "glass beach".into(),
          title: "cul-de-sac".into(),
          song_file: None,
          save_file: None,
      }
  }
}

impl NewProjectDialog {
  pub fn open(&mut self) {
      self.is_open = true;
  }

  pub fn show(&mut self, ctx: &egui::Context, commands: &mut Commands) {
    if self.is_open {
      egui::Window::new("New Project").show(ctx, |ui| {
        let mut last_visited_path: Option<PathBuf> = None;
        ui.data_mut(|map| {
          last_visited_path = map.get_persisted("last_visited_path".into());
        });

        ui.horizontal(|ui| {
          ui.label("Artist");
          ui.text_edit_singleline(&mut self.artist)
        });

        ui.horizontal(|ui| {
          ui.label("Title");
          ui.text_edit_singleline(&mut self.title)
        });

        ui.horizontal(|ui| {
          ui.label("Song File");
          if let Some(song_file_path) = &self.song_file {
            ui.label(song_file_path.as_os_str().to_string_lossy());
          } else {
            ui.label("No file selected");
          }
          if ui.button("Browse...").clicked() {
            commands.dialog().pick_file_path::<NewProjectSongFileDialog>();
          }
        });

        let can_create = self.song_file != None;

        if ui.add_enabled(can_create, egui::Button::new("Create")).clicked() {
          commands.send_event(NewProjectDialogSubmittedEvent::default());
          self.is_open = false;
          self.is_submitted = true;
        }
      });
    }
  }
}

#[derive(Default)]
pub struct NewProjectSongFileDialog;

fn handle_new_project_song_file_dialog_picked(mut events: EventReader<DialogFilePicked<NewProjectSongFileDialog>>, mut editor_state: NonSendMut<EditorState>) {
  for ev in events.read() {
    if let Some(new_project_dialog) = &mut editor_state.new_file_dialog {
      new_project_dialog.song_file = Some(ev.path.clone());
    }
  }
}

#[derive(Event, Default)]
struct NewProjectDialogSubmittedEvent;

fn handle_new_project_dialog_submitted_events(
  mut events: EventReader<NewProjectDialogSubmittedEvent>,
  mut editor_state: NonSendMut<EditorState>,
  mut commands: Commands,
) {
  for _ in events.read() {
    if let Some(new_project_dialog) = editor_state.new_file_dialog.as_ref() {
      let mut project_data = ProjectData::default();
      project_data.artist = new_project_dialog.artist.clone();
      project_data.title = new_project_dialog.title.clone();
      project_data.song_file = new_project_dialog.song_file.clone();
      editor_state.project_data = Some(project_data);
      editor_state.new_file_dialog = None;
    
      let serialized = serde_json::to_vec_pretty(editor_state.project_data.as_ref().unwrap()).unwrap();
      commands.dialog().save_file::<NewProjectSaveFileDialog>(serialized);
    }
  }
}

#[derive(Default)]
pub struct NewProjectSaveFileDialog;

fn handle_new_project_save_file_dialog(
  mut events: EventReader<DialogFileSaved<NewProjectSaveFileDialog>>,
  mut editor_state: NonSendMut<EditorState>) 
{
  for ev in events.read() {
    editor_state.project_file_path = ev.path.clone();
  }
}

fn handle_save_project_requested_event(mut events: EventReader<crate::ui::SaveProjectRequestedEvent>, mut editor_state: NonSendMut<EditorState>) {
  for _ in events.read() {
    let mut vec = Vec::new();
    if let Some(project_data) = &editor_state.project_data {
        vec = serde_json::to_vec_pretty(&project_data).unwrap();
    }

    match File::create(editor_state.project_file_path.clone())
      .unwrap()
      .write_all(&vec[..])
    {
      Err(e) => {
        error!("Error saving to {:?}: {:?}", editor_state.project_file_path, e);
      },
      Ok(_) => {
        println!("Project saved to {:?}", editor_state.project_file_path);
      }
    }
  }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProjectData {
  pub lyrics: String,
  pub artist: String,
  pub title: String,
  pub song_file: Option<PathBuf>,
  pub background_color: Option<Color>,
  pub unsung_color: Option<Color>,
  pub sung_color: Option<Color>,
  pub thumbnail_path: Option<PathBuf>,
  pub titlecard_show_time: Option<f32>
}

impl Default for ProjectData {
  fn default() -> Self {
    Self {
      lyrics: default(),
      artist: default(),
      title: default(),
      song_file: None,
      background_color: Some(Color::BLACK),
      sung_color: Some(Color::WHITE),
      unsung_color: Some(Color::srgb(0.5, 0.5, 0.5)),
      thumbnail_path: None,
      titlecard_show_time: Some(10.)
    }
  }
}

pub struct OpenProjectDialog;

pub struct SaveAsDialog;

pub struct LoadDialog;

pub struct ThumbnailFilePathDialog;

pub fn configure_file_dialog_plugin(plugin: FileDialogPlugin) -> FileDialogPlugin {
  plugin.with_load_file::<crate::project::LoadDialog>()
  .with_pick_file::<crate::project::ThumbnailFilePathDialog>()
  .with_pick_file::<crate::project::NewProjectSongFileDialog>()
  .with_save_file::<crate::project::NewProjectSaveFileDialog>()
  .with_load_file::<crate::project::OpenProjectDialog>()
  .with_save_file::<crate::project::SaveAsDialog>()
}