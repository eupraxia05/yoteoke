use bevy::prelude::*;
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::sound::FromFileError;
use kira::{
  DefaultBackend,
  AudioManagerSettings,
  Tween
};
use kira::AudioManager;

use crate::editor::EditorState;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
  fn build(&self, app: &mut App) {
   app.add_systems(Startup, startup);
   app.add_systems(Update, update); 
  }
}

fn startup(mut commands: Commands, mut editor_state: NonSendMut<EditorState>)
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
}

fn update(mut editor_state: NonSendMut<EditorState>) {
  if editor_state.music_handle.is_none() {
    let mut music = None;
    if let Some(project_data) = &editor_state.project_data {
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
      editor_state.duration = Some(music.duration());
      let play_result = editor_state.audio_manager.as_mut().unwrap().play(music);
      match play_result {
          Ok(mut handle) => {
              info!("sound played successfully");
              handle.pause(Tween::default());
              handle.set_loop_region(..);
              editor_state.music_handle = Some(handle);
          },
          Err(e) => {
              error!("Failed to play sound: {:?}", e);
          }
      }
    }
  }
}