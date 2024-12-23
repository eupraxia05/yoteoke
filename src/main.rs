use bevy::prelude::*;
use bevy_egui::egui::Style;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui_file::FileDialog;
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

mod lyrics;
use crate::lyrics::ParsedLyrics;

mod ui;

fn main() {
  let mut app = App::new();
    
  app.add_plugins(DefaultPlugins)
    .add_plugins(EguiPlugin)
    .insert_resource(EditorState::default())
    .add_systems(Startup, setup)
    .add_systems(Update, update);

  ui::build(&mut app);

  app.run();
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
}

fn update(mut editor_state: ResMut<EditorState>, asset_server: Res<AssetServer>) {
  editor_state.update(asset_server.as_ref());
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

#[derive(Serialize, Deserialize, Default)]
struct ProjectData {
    lyrics: String,
    artist: String,
    title: String,
    song_file: Option<PathBuf>
}

struct NewProjectDialog {
    is_open: bool,
    is_submitted: bool,
    artist: String,
    title: String,
    save_file_dialog: Option<FileDialog>,
    song_file_dialog: Option<FileDialog>,
    song_file: Option<PathBuf>,
    save_file: Option<PathBuf>,
}

impl Default for NewProjectDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            is_submitted: false,
            artist: "glass beach".into(),
            title: "cul-de-sac".into(),
            song_file_dialog: None,
            save_file_dialog: None,
            song_file: None,
            save_file: None,
        }
    }
}

impl NewProjectDialog {
  fn open(&mut self) {
      self.is_open = true;
  }

  fn show(&mut self, ctx: &egui::Context) {
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
            let mut song_file_dialog = FileDialog::open_file(None);
            song_file_dialog.open();
            self.song_file_dialog = Some(song_file_dialog);
          }
        });

        ui.horizontal(|ui| {
            ui.label("Project File");
            if let Some(project_file_path) = &self.save_file {
              ui.label(project_file_path.as_os_str().to_string_lossy());
            } else {
              ui.label("No file selected");
            }
            if ui.button("Browse...").clicked() {
              let mut project_file_dialog = FileDialog::save_file(None);
              project_file_dialog.open();
              self.save_file_dialog = Some(project_file_dialog);
            }
        });

        let can_create = self.save_file != None && self.song_file != None;

        if ui.add_enabled(can_create, egui::Button::new("Create")).clicked() {
          self.is_open = false;
          self.is_submitted = true;
        }
      });

      if let Some(song_file_dialog) = &mut self.song_file_dialog {
        song_file_dialog.show(ctx);
        if song_file_dialog.selected() {
          if let Some(path) = song_file_dialog.path() {
            self.song_file = Some(PathBuf::from(path));
            ctx.data_mut(|map| {
              *map.get_persisted_mut_or_default("last_visited_path".into()) 
                = Some(PathBuf::from(song_file_dialog.directory()));
            });
          }
        }
      }

      if let Some(save_file_dialog) = &mut self.save_file_dialog {
        save_file_dialog.show(ctx);
        if save_file_dialog.selected() {
          if let Some(path) = save_file_dialog.path() {
            self.save_file = Some(PathBuf::from(path));
            ctx.data_mut(|map| {
              *map.get_persisted_mut_or_default("last_visited_path".into()) 
                = Some(PathBuf::from(save_file_dialog.directory()));
            });
          }
        }
      }
    }
  }
}
