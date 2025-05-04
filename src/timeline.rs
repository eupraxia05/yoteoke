use bevy::prelude::*;

use std::time::Duration;
use kira::sound::PlaybackState;
use kira::Tween;
use std::ops::RangeInclusive;

use bevy_egui::egui;

use crate::editor::{self, EditorState};

pub struct TimelinePlugin;

impl Plugin for TimelinePlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, handle_pre_delay);
  }
}

pub fn timeline_ui(mut ui: InMut<egui::Ui>, mut editor_state: NonSendMut<EditorState>,
  mut audio_state: NonSendMut<crate::editor::AudioState>
) {
  egui::TopBottomPanel::new(egui::panel::TopBottomSide::Top, "timeline_header").exact_height(32.).show_inside(&mut ui, |ui| {
    timeline_header_ui(ui, editor_state.reborrow(), audio_state.reborrow());
  });

  egui::CentralPanel::default().show_inside(&mut ui, |ui| {
    timeline_blocks_ui(ui, editor_state.reborrow());
  });
}

fn seek_playhead_to(mut editor_state: Mut<EditorState>, mut audio_state: Mut<crate::editor::AudioState>, time: f64) {
  if let Some(project_data) = &editor_state.project_data {
    let delay_time = project_data.song_delay_time.unwrap() as f64;
    let t = time - delay_time;
    if t < 0. {
      info!("seek_playhead_to: entering pre delay");
      audio_state.music_handle.as_mut().unwrap().seek_to(0.);
      // pause the sound, we'll resume it after pre-delay
      audio_state.music_handle.as_mut().unwrap().pause(Tween::default());
      editor_state.is_in_pre_delay = true;
      editor_state.curr_pre_delay_time = time;
    } else {
      audio_state.music_handle.as_mut().unwrap().seek_to(t);
      editor_state.is_in_pre_delay = false;
      editor_state.curr_pre_delay_time = delay_time;
    }
  }
}

pub fn play_buttons_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>,
  curr_time: Duration, total_time: Duration,
  mut audio_state: Mut<crate::editor::AudioState>
) {
  ui.horizontal(|ui| {
    if ui.button("|<-").clicked() {
      seek_playhead_to(editor_state.reborrow(), audio_state.reborrow(), 0.);
    }
    if ui.button("<-").clicked() {
      let seek_duration = Duration::from_secs_f64(5.);
      let seek_to_time = if curr_time > seek_duration {
        (curr_time - Duration::from_secs_f64(5.)).as_secs_f64().max(0.)
      } else {
        0.
      };
      info!("Rewinding to {:?}", seek_to_time);
      seek_playhead_to(editor_state.reborrow(), audio_state.reborrow(), seek_to_time);
    }
    if editor_state.is_paused {
      if ui.button(">").clicked() {
        if !editor_state.is_in_pre_delay {
          audio_state.music_handle.as_mut().unwrap().resume(Tween::default());
        }
        editor_state.is_paused = false;
      }
    } else {
      if ui.button("||").clicked() {
        if !editor_state.is_in_pre_delay {
          audio_state.music_handle.as_mut().unwrap().pause(Tween::default());
        }
        editor_state.is_paused = true;
      }
    }
    if ui.button("->").clicked() {
      seek_playhead_to(editor_state.reborrow(), audio_state.reborrow(), (curr_time + Duration::from_secs_f64(5.)).min(total_time).as_secs_f64());
    }
    if ui.button("->|").clicked() {
      if audio_state.music_handle.as_mut().unwrap().state() == PlaybackState::Playing {
        audio_state.music_handle.as_mut().unwrap().pause(Tween::default());
      }
      seek_playhead_to(editor_state.reborrow(), audio_state.reborrow(), total_time.as_secs_f64());
    }
  });
}

pub fn timeline_header_ui(ui: &mut egui::Ui, 
  mut editor_state: Mut<EditorState>,
  mut audio_state: Mut<crate::editor::AudioState>
) {
  let Some(music_handle) = audio_state.music_handle.as_mut() else {
    return;
  };
  let pre_delay_time = if let Some(project_data) = &editor_state.project_data {
    project_data.song_delay_time.unwrap()
  } else {
    0.
  };
  let curr_time = audio_state.playhead_position() + Duration::from_secs_f64(editor_state.curr_pre_delay_time);
  let total_time = audio_state.duration.unwrap() + Duration::from_secs_f32(pre_delay_time);
  egui::SidePanel::new(egui::panel::Side::Left, "play_buttons").show_inside(ui, |ui| {
    crate::timeline::play_buttons_ui(ui, editor_state.reborrow(), curr_time, total_time, audio_state.reborrow());
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
      audio_state.music_handle.as_mut().unwrap().seek_to(mut_curr_time);
    }
  });
}

fn timeline_blocks_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>,) {
  egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
    if let Some(parsed_lyrics) = &mut editor_state.parsed_lyrics {
      ui.horizontal(|ui| {
        for block in &parsed_lyrics.blocks {
          egui::Frame::canvas(ui.style()).show(ui, |ui| {
            ui.vertical(|ui| {
              let lyrics = block.lyrics.clone().replace("\n", " ");
              let label = egui::Label::new(lyrics.clone())
                .wrap_mode(egui::TextWrapMode::Extend)
                .halign(egui::Align::Min);
              ui.add(label);
            });
          });
        }
      });
    }
  });
}

fn handle_pre_delay(mut editor_state: NonSendMut<EditorState>, 
  time: Res<Time>,
  mut audio_state: NonSendMut<crate::editor::AudioState>
) {
  let delay_time = if let Some(project_data) = &editor_state.project_data {
    project_data.song_delay_time.unwrap() as f64
  } else {
    return;
  };

  if !editor_state.is_paused && editor_state.is_in_pre_delay {
    editor_state.curr_pre_delay_time += time.delta_secs_f64();

    if editor_state.curr_pre_delay_time > delay_time {
      info!("handle_pre_delay: exiting pre delay");
      editor_state.curr_pre_delay_time = delay_time;
      audio_state.music_handle.as_mut().unwrap().resume(Tween::default());
      editor_state.is_paused = false;
      editor_state.is_in_pre_delay = false;
    }
  }
}