use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy_egui::EguiPlugin;
use bevy_image_export::ImageExportPlugin;
use export::ExportState;
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::sound::FromFileError;
use serde::{Deserialize, Serialize};
use ui::NewProjectSaveFileDialog;
use std::io::Write;
use std::path::{PathBuf, Path};
use std::time::Duration;
use std::fs::File;
use kira::{
  DefaultBackend,
  AudioManagerSettings,
  Tween
};
use kira::AudioManager;
use bevy_file_dialog::prelude::*;

use bevy::render::view::RenderLayers;

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

#[derive(Component)]
struct LineText;

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
    .insert_non_send_resource(EditorState::default())
    .add_systems(Startup, setup)
    .add_systems(Update, update)
    //.add_systems(Update, center_text_hack)
    .add_systems(Update, handle_new_project_save_file_dialog)
    .add_systems(Update, (cleanup_preview, update_preview).chain())
    .add_plugins(TokioTasksPlugin::default())
    .add_plugins(FileDialogPlugin::new()
      .with_load_file::<LoadDialog>()
      .with_load_file::<ThumbnailFilePathDialog>()
      .with_pick_file::<crate::ui::NewProjectSongFileDialog>()
      .with_save_file::<crate::ui::NewProjectSaveFileDialog>()
      .with_load_file::<OpenProjectDialog>()
      .with_save_file::<SaveAsDialog>());

  ui::build(&mut app);
  sub_viewport::build(&mut app);
  export::build(&mut app);

  app.run();

  export_threads.finish();
}

fn setup(mut commands: Commands, mut editor_state: NonSendMut<EditorState>)
{
  // startup kira audio
  match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
  {
    Ok(m) => {
      editor_state.audio_manager = Some(m);
    },
    Err(e) => {
      error!("Error initializing Kira audio manager: {:?}", e); 
    }
  }

  // create a subviewport for the video preview
  commands.spawn(SubViewport::new(RenderLayers::layer(1)));
}

fn cleanup_preview(world: &mut World) {
  let line_texts = world.query_filtered::<Entity, With<LineText>>().iter(&world).collect::<Vec<_>>();
  for entity in line_texts.iter() {
    world.despawn(*entity);
  }

  let preview_texts = world.query_filtered::<Entity, With<PreviewText>>().iter(&world).collect::<Vec<_>>();
  for entity in preview_texts.iter() {
    world.despawn(*entity);
  }
}

fn update_preview(editor_state: NonSend<EditorState>,
  export_state: Res<ExportState>,
  mut commands: Commands,
)
{
  let song_position = if export_state.is_exporting() {
    Duration::from_secs_f64(export_state.frame_idx() as f64 / 12.)
  } else {
    editor_state.playhead_position()
  };

  let mut text: String = "".into();
  let mut chars_sung: usize = 0;
  if let Some(lyrics) = editor_state.parsed_lyrics.as_ref() {
    if let Some(block) = lyrics.get_block_at_time(&song_position, &Duration::from_secs(3)) {
      text = block.lyrics.clone();

      if let Some((ts1, ts2)) = block.get_timestamps_surrounding(&song_position) {
        if ts1.position <= ts2.position && ts1.time < ts2.time {
          let elapsed_in_syl = song_position - ts1.time;
          let total_syl_time = ts2.time - ts1.time;
          let chars_in_syl = ts2.position - ts1.position;
          let amount_sung = elapsed_in_syl.as_secs_f64() / total_syl_time.as_secs_f64();
          chars_sung = (amount_sung * chars_in_syl as f64) as usize + ts1.position;
        } else {
          warn!("non-sequential timestamps: {:?}, {:?}", ts1, ts2);
          chars_sung = 0
        }
      } else {
        chars_sung = 0
      }
    }
  }

  if text.len() > 2 && text.len() > chars_sung {
    let preview_text_ent = commands.spawn(
      (
        Text2d::default(), 
        TextLayout::new_with_justify(JustifyText::Center),
        RenderLayers::layer(1), 
        PreviewText
      )
    ).id();

    commands
      .spawn(
        (
          TextSpan::new(&text[0..chars_sung]),
          TextFont {
            font_size: 64.0,
            ..Default::default()
          }, 
          TextColor(
            editor_state.project_data.as_ref().unwrap().sung_color.unwrap_or_default()
          ), 
          LineText
        )
      )
      .set_parent(preview_text_ent);

    commands
      .spawn(
        (
          TextSpan::new(String::from(&text[chars_sung..]) + "\n"),
          TextFont {
            font_size: 64.0,
            ..Default::default()
          }, 
          TextColor(
            editor_state.project_data.as_ref().unwrap().unsung_color.unwrap_or_default()
          ), 
          LineText
        )
      )
      .set_parent(preview_text_ent);
  }
}

fn update(mut editor_state: NonSendMut<EditorState>,
  mut camera_tex_query: Query<&mut SubViewport>,
  mut commands: Commands) 
{
  editor_state.update(&mut commands);

  if let Some(project_data) = &editor_state.project_data {
    camera_tex_query.single_mut().clear_color = ClearColorConfig::Custom(project_data.background_color.unwrap_or_default());  
  }
}

#[derive(Default)]
struct EditorState {
  project_file_path: PathBuf,
  project_data: Option<ProjectData>,
  new_file_dialog: Option<NewProjectDialog>,
  music_handle: Option<StreamingSoundHandle<FromFileError>>,
  audio_manager: Option<AudioManager>,
  duration: Option<Duration>,
  parsed_lyrics: Option<ParsedLyrics>,
  lyrics_dirty: bool,
  project_settings_dialog: ProjectSettingsDialog,
  curr_thumbnail_path: Option<PathBuf>,
}

impl EditorState {
    fn set_project_file_path(&mut self, path: PathBuf) {
        self.project_file_path = path
    }

    fn new(&mut self) {
        let mut new_file_dialog = NewProjectDialog::default();
        new_file_dialog.open();
        self.new_file_dialog = Some(new_file_dialog);
    }

    fn save(&mut self) {
        let mut vec = Vec::new();
        if let Some(project_data) = &self.project_data {
            vec = serde_json::to_vec_pretty(&project_data).unwrap();
        }

        match File::create(self.project_file_path.clone())
          .unwrap()
          .write_all(&vec[..])
        {
          Err(e) => {
            error!("Error saving to {:?}: {:?}", self.project_file_path, e);
          },
          Ok(_) => {
            println!("Project saved to {:?}", self.project_file_path);
          }
        }
        
    }

    fn serialize_project(&self) -> Vec<u8> {
      let mut vec = Vec::new();
      if let Some(project_data) = &self.project_data {
          vec = serde_json::to_vec_pretty(&project_data).unwrap();
      }

      vec
    }

    fn open(&mut self, file_contents: &Vec<u8>) {
      if let Ok(data) = serde_json::from_slice::<ProjectData>(file_contents.as_slice()) {
          self.project_data = Some(data);
          self.lyrics_dirty = true;
      } else {
          println!("couldn't deserialize file");
      }
    }

    fn playhead_position(&self) -> Duration {
      if let Some(music_handle) = &self.music_handle {
        return Duration::from_secs_f64(music_handle.position());
      } else {
        return Duration::default();
      }
    }

    // todo: break this up
    fn update(&mut self, commands: &mut Commands) {
      if let Some(new_file_dialog) = &self.new_file_dialog {
        if new_file_dialog.is_submitted {
          let mut project_data = ProjectData::default();
          project_data.artist = new_file_dialog.artist.clone();
          project_data.title = new_file_dialog.title.clone();
          project_data.song_file = new_file_dialog.song_file.clone();
          self.project_data = Some(project_data);
          self.new_file_dialog = None;

          commands.dialog().save_file::<NewProjectSaveFileDialog>(self.serialize_project());
          //self.project_file_path = new_file_dialog.save_file.clone().unwrap();
          //self.save();
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

      if let Some(project_data) = &self.project_data {
        if self.curr_thumbnail_path != project_data.thumbnail_path {
          // load thumbnail image
        }
      }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct ProjectData {
    lyrics: String,
    artist: String,
    title: String,
    song_file: Option<PathBuf>,
    background_color: Option<Color>,
    unsung_color: Option<Color>,
    sung_color: Option<Color>,
    thumbnail_path: Option<PathBuf>,
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
    }
  }
}

struct SaveAsDialog;

struct LoadDialog;

pub struct ThumbnailFilePathDialog;

pub struct OpenProjectDialog;

fn handle_new_project_save_file_dialog(
  mut events: EventReader<DialogFileSaved<crate::NewProjectSaveFileDialog>>,
  mut editor_state: NonSendMut<EditorState>) 
{
  for ev in events.read() {
    editor_state.project_file_path = ev.path.clone();
  }
}