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

fn main() {
  App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(EguiPlugin)
    .insert_resource(EditorState::default())
    .add_systems(Startup, setup)
    .add_systems(Update, update)
    .add_systems(Update, ui)
    .run();
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

fn ui(mut contexts: EguiContexts, mut editor_state: ResMut<EditorState>) {
  egui::TopBottomPanel::top("menu").show(contexts.ctx_mut(), |ui| {
      egui::menu::bar(ui, |ui| {
          ui.menu_button("File", |ui| {
              if ui.button("New...").clicked() {
                editor_state.new();
              }
              if ui.button("Open...").clicked() {
                println!("open button clicked");
                let mut dialog = FileDialog::open_file(None);
                dialog.open();
                editor_state.open_dialog = Some(dialog);
              }
              if ui.button("Save").clicked() {
                editor_state.save();
              }
              if ui.button("Save As...").clicked() {
                let mut dialog = FileDialog::save_file(None);
                dialog.open();
                editor_state.file_dialog = Some(dialog)
              }
          });
      });
  });

  egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
    if editor_state.project_data.is_some() {
      egui::SidePanel::new(egui::panel::Side::Left, "main_left_panel")
        .default_width(512.)
        .show_inside(ui, |ui| 
        {
          let mut text_edit_changed = false;
          if let Some(project_data) = &mut editor_state.project_data {
            let title_str = format!("{} - {}", project_data.artist, project_data.title);
            ui.label(title_str);
            ui.separator();
            egui::ScrollArea::both().show(ui, |ui| {
              let text_edit_response = ui.add_sized(ui.available_size(), 
                egui::TextEdit::multiline(&mut project_data.lyrics).code_editor());
              if text_edit_response.changed() {
                info!("text edit changed");
                text_edit_changed = true;
              }
            });
          }
          if text_edit_changed {
            info!("lyrics marked dirty");
            editor_state.lyrics_dirty = true;
          }
        }
      );
      egui::CentralPanel::default().show_inside(ui, |ui| {
        egui::TopBottomPanel::new(egui::panel::TopBottomSide::Bottom, "timeline_panel")
          .exact_height(256.)
          .show_inside(ui, |ui|
          {
            egui::TopBottomPanel::new(egui::panel::TopBottomSide::Top, "timeline_header").exact_height(32.).show_inside(ui, |ui| {
              let curr_time = Duration::from_secs_f64(editor_state.music_handle.as_mut().unwrap().position());
              let total_time = editor_state.duration.unwrap();
              egui::SidePanel::new(egui::panel::Side::Left, "play_buttons").show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                  if ui.button("|<-").clicked() {
                    editor_state.music_handle.as_mut().unwrap().seek_to(0.);
                  }
                  if ui.button("<-").clicked() {
                    editor_state.music_handle.as_mut().unwrap().seek_to((curr_time - Duration::from_secs_f64(5.)).as_secs_f64().max(0.));
                  }
                  if editor_state.music_handle.as_ref().unwrap().state() == PlaybackState::Paused {
                    if ui.button(">").clicked() {
                      editor_state.music_handle.as_mut().unwrap().resume(Tween::default());
                    }
                  } else {
                    if ui.button("||").clicked() {
                      editor_state.music_handle.as_mut().unwrap().pause(Tween::default());
                    }
                  }
                  if ui.button("->").clicked() {
                    editor_state.music_handle.as_mut().unwrap().seek_to((curr_time + Duration::from_secs_f64(5.)).min(total_time).as_secs_f64());
                  }
                  if ui.button("->|").clicked() {
                    if editor_state.music_handle.as_mut().unwrap().state() == PlaybackState::Playing {
                      editor_state.music_handle.as_mut().unwrap().pause(Tween::default());
                    }
                    editor_state.music_handle.as_mut().unwrap().seek_to(total_time.as_secs_f64());
                  }
                });
              });
              let curr_time_str = format!("{:0>2}:{:0>2}.{:0>3}", curr_time.as_secs() / 60, curr_time.as_secs() % 60, curr_time.subsec_millis());
              let total_time_str = format!("{:0>2}:{:0>2}.{:0>3}", total_time.as_secs() / 60, total_time.as_secs() % 60, total_time.subsec_millis());
              egui::SidePanel::new(egui::panel::Side::Right, "timecode").show_inside(ui, |ui| {
                ui.label(format!("{} / {}", curr_time_str, total_time_str));
              });
              egui::CentralPanel::default().show_inside(ui, |ui| {
                ui.style_mut().spacing.slider_width = ui.available_width();
                let mut mut_curr_time = curr_time.as_secs_f64();
                let slider_response = ui.add(egui::Slider::new(&mut mut_curr_time, RangeInclusive::new(0., total_time.as_secs_f64())).show_value(false));
                if slider_response.changed() {
                  editor_state.music_handle.as_mut().unwrap().seek_to(mut_curr_time);
                }
              });
            });
            egui::CentralPanel::default().show_inside(ui, |ui| {
              egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                if let Some(parsed_lyrics) = &mut editor_state.parsed_lyrics {
                  ui.horizontal(|ui| {
                    for block in &parsed_lyrics.blocks {
                      egui::Frame::canvas(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                          for line in &block.lines {
                            let label = egui::Label::new(line.clone())
                              .wrap_mode(egui::TextWrapMode::Extend)
                              .halign(egui::Align::Center);
                            ui.add(label);
                          }
                        });
                      });
                    }
                  });
                }
              });
            });
          });
      });
    }
  });

  let mut file_to_save = None;

  if let Some(file_dialog) = &mut editor_state.file_dialog {
    if file_dialog.show(contexts.ctx_mut()).selected() {
      if let Some(file) = file_dialog.path() {
        file_to_save = Some(PathBuf::from(file));
      }
    }
  }

  if let Some(file_to_save) = file_to_save {
    editor_state.set_project_file_path(file_to_save);
    editor_state.save();
  }

  if let Some(new_project_dialog) = &mut editor_state.new_file_dialog {
    new_project_dialog.show(contexts.ctx_mut());
  }

  let mut file_to_open = None;
  if let Some(open_file_dialog) = &mut editor_state.open_dialog {
    if open_file_dialog.show(contexts.ctx_mut()).selected() {
      file_to_open = Some(PathBuf::from(open_file_dialog.path().unwrap()));
    }
  }

  if let Some(file_to_open) = file_to_open {
    editor_state.open(&file_to_open);
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
