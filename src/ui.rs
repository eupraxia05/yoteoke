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
use crate::EditorState;

pub fn build(app: &mut App) {
  app.add_systems(Update, ui);
}

fn ui(mut contexts: EguiContexts, mut editor_state: ResMut<EditorState>) {
  egui::TopBottomPanel::top("menu").show(contexts.ctx_mut(), |ui| {
    menu_ui(ui, editor_state.reborrow());
  });

  egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
    if editor_state.project_data.is_some() {
      egui::SidePanel::new(egui::panel::Side::Left, "main_left_panel")
        .default_width(512.)
        .show_inside(ui, |ui| 
        {
          lyrics_edit_ui(ui, editor_state.reborrow());
        }
      );
      egui::CentralPanel::default().show_inside(ui, |ui| {
        egui::TopBottomPanel::new(egui::panel::TopBottomSide::Bottom, "timeline_panel")
          .exact_height(256.)
          .show_inside(ui, |ui|
          {
            timeline_ui(ui, editor_state.reborrow());
          });
        egui::CentralPanel::default().show_inside(ui, |ui| {
          preview_ui(ui, editor_state.reborrow());
        });
      });
    }
  });

  file_dialog_ui(&mut contexts, editor_state.reborrow());
}

fn menu_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>) {
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
}

fn lyrics_edit_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>) {
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

fn timeline_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>) {
  egui::TopBottomPanel::new(egui::panel::TopBottomSide::Top, "timeline_header").exact_height(32.).show_inside(ui, |ui| {
    timeline_header_ui(ui, editor_state.reborrow());
  });

  egui::CentralPanel::default().show_inside(ui, |ui| {
    timeline_blocks_ui(ui, editor_state.reborrow());
  });
}

fn timeline_header_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>) {
  let curr_time = Duration::from_secs_f64(editor_state.music_handle.as_mut().unwrap().position());
  let total_time = editor_state.duration.unwrap();
  egui::SidePanel::new(egui::panel::Side::Left, "play_buttons").show_inside(ui, |ui| {
    play_buttons_ui(ui, editor_state.reborrow(), curr_time, total_time);
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

fn play_buttons_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>,
  curr_time: Duration, total_time: Duration) {
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
}

fn timeline_blocks_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>,) {
  egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
    if let Some(parsed_lyrics) = &mut editor_state.parsed_lyrics {
      ui.horizontal(|ui| {
        for block in &parsed_lyrics.blocks {
          egui::Frame::canvas(ui.style()).show(ui, |ui| {
            ui.vertical(|ui| {
              for line in &block.lines {
                let label = egui::Label::new(line.line.clone())
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
}

fn file_dialog_ui(contexts: &mut EguiContexts, mut editor_state: Mut<EditorState>,) {
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

fn preview_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>,) {
  if let Some(lyrics) = editor_state.parsed_lyrics.as_ref() {
    if let Some(music_handle) = editor_state.music_handle.as_ref() {
      if let Some(block) = lyrics.get_block_at_time(&Duration::from_secs_f64(music_handle.position()), &Duration::from_secs(3)) {
        for line in &block.lines {
          ui.label(line.line.clone());
        }
      }
    }
  }
}