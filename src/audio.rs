use bevy::prelude::*;
use kira::sound::streaming::StreamingSoundData;
use kira::{
  DefaultBackend,
  AudioManagerSettings,
  Tween
};
use kira::AudioManager;

use crate::editor::{show_and_log_error, EditorState};

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
  fn build(&self, app: &mut App) {
   app.add_systems(Startup, startup);
   app.add_systems(Update, update); 
  }
}

fn startup(mut editor_state: NonSendMut<EditorState>, 
  mut audio_state: NonSendMut<crate::editor::AudioState>
) {
  // startup kira audio
  match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
  {
    Ok(m) => {
      audio_state.audio_manager = Some(m);
    },
    Err(e) => {
      show_and_log_error(editor_state.as_mut(), 
        format!("Error initializing Kira audio manager: {:?}", e));
    }
  }
}

fn update(mut editor_state: NonSendMut<EditorState>, 
  mut audio_state: NonSendMut<crate::editor::AudioState>
) {
  if audio_state.music_handle.is_none() {
    let mut music = None;
    if editor_state.project_data.is_some() {
      if editor_state.project_data.as_ref().unwrap().song_file.is_some() {
        let song_file = editor_state.project_data.as_ref().unwrap().song_file.as_ref().unwrap().clone();
          match StreamingSoundData::from_file(song_file.clone()) {
              Ok(data) => {
                  music = Some(data);
              },
              Err(e) => {
                  show_and_log_error(editor_state.as_mut(), 
                    format!("Failed to load music file {:?}: {:?}", 
                      song_file, e)
                  );
              }
          }
      }
    }

    if let Some(music) = music {
      audio_state.duration = Some(music.duration());
      let play_result = audio_state.audio_manager.as_mut().unwrap().play(music);
      match play_result {
          Ok(mut handle) => {
              info!("sound played successfully");
              handle.pause(Tween::default());
              handle.set_loop_region(..);
              audio_state.music_handle = Some(handle);
          },
          Err(e) => {
            show_and_log_error(editor_state.as_mut(), 
              format!("Failed to play sound: {:?}", e));
          }
      }
    }
  }
}