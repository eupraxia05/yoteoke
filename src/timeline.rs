use bevy::prelude::*;

use std::time::Duration;
use kira::sound::PlaybackState;
use kira::Tween;
use std::ops::RangeInclusive;

use bevy_egui::egui;

use crate::editor::EditorState;

pub fn timeline_ui(mut ui: InMut<egui::Ui>, mut editor_state: NonSendMut<EditorState>) {
  egui::TopBottomPanel::new(egui::panel::TopBottomSide::Top, "timeline_header").exact_height(32.).show_inside(&mut ui, |ui| {
    timeline_header_ui(ui, editor_state.reborrow());
  });

  egui::CentralPanel::default().show_inside(&mut ui, |ui| {
    timeline_blocks_ui(ui, editor_state.reborrow());
  });
}

pub fn play_buttons_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>,
  curr_time: Duration, total_time: Duration) {
  ui.horizontal(|ui| {
    if ui.button("|<-").clicked() {
      editor_state.music_handle.as_mut().unwrap().seek_to(0.);
    }
    if ui.button("<-").clicked() {
      let seek_duration = Duration::from_secs_f64(5.);
      let seek_to_time = if curr_time > seek_duration {
        (curr_time - Duration::from_secs_f64(5.)).as_secs_f64().max(0.)
      } else {
        0.
      };
      info!("Rewinding to {:?}", seek_to_time);
      editor_state.music_handle.as_mut().unwrap().seek_to(seek_to_time);
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
}

pub fn timeline_header_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>) {
  let Some(music_handle) = editor_state.music_handle.as_mut() else {
    return;
  };
  let curr_time = Duration::from_secs_f64(music_handle.position());
  let total_time = editor_state.duration.unwrap();
  egui::SidePanel::new(egui::panel::Side::Left, "play_buttons").show_inside(ui, |ui| {
    crate::timeline::play_buttons_ui(ui, editor_state.reborrow(), curr_time, total_time);
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