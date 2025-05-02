use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui_commonmark::{CommonMarkCache as BaseCommonMarkCache, CommonMarkViewer, commonmark_str};

pub struct HelpPlugin;

impl Plugin for HelpPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, handle_help_dialog_open_requested);
    app.add_systems(Update, handle_about_dialog_open_requested);
    app.insert_resource(HelpDialogState::default());
    app.insert_resource(AboutDialogState::default());
    app.add_event::<HelpDialogOpenRequested>();
    app.add_event::<AboutDialogOpenRequested>();
    app.insert_resource(CommonMarkCache::default());
  }
}

#[derive(Event, Default)]
pub struct HelpDialogOpenRequested;

#[derive(Event, Default)]
pub struct AboutDialogOpenRequested;

#[derive(Resource, Default)]
pub struct CommonMarkCache(BaseCommonMarkCache);

fn handle_help_dialog_open_requested(mut events: EventReader<HelpDialogOpenRequested>, mut help_dialog_state: ResMut<HelpDialogState>) {
  for ev in events.read() {
    help_dialog_state.is_open = true;
  }
}

fn handle_about_dialog_open_requested(mut events: EventReader<AboutDialogOpenRequested>, mut about_dialog_state: ResMut<AboutDialogState>) {
  for ev in events.read() {
    about_dialog_state.is_open = true;
  }
}

#[derive(Default, Resource)]
pub struct HelpDialogState {
  is_open: bool
}

#[derive(Default, Resource)]
pub struct AboutDialogState {
  is_open: bool
}

pub fn help_dialog_ui(mut egui_contexts: EguiContexts, mut help_dialog_state: ResMut<HelpDialogState>,
  mut commonmark_cache: ResMut<CommonMarkCache>
) {
  if help_dialog_state.is_open {
    egui::Window::new("Help").show(egui_contexts.ctx_mut(), |ui| {
      egui::ScrollArea::new([false, true]).max_height(512.).show(ui, |ui| {
        commonmark_str!(ui, &mut commonmark_cache.0, "help.md");
      });
      if ui.button("Close").clicked() {
        help_dialog_state.is_open = false;
      }
    });
  }
}

pub fn about_dialog_ui(mut egui_contexts: EguiContexts, mut about_dialog_state: ResMut<AboutDialogState>) {
  if about_dialog_state.is_open {
    egui::Window::new("About").show(egui_contexts.ctx_mut(), |ui| {
      ui.heading("YoteOke Lyric Editor");
      ui.label("Made by Cassie G. for YoteOke");
      ui.label("Special thanks to the diveBar karaoke community!");
      ui.separator();
      egui::ScrollArea::new([false, true])
        .auto_shrink([false, false])
        .max_height(128.)
        .show(ui, |ui| {
          ui.heading("Dependencies");
          ui.label("Bevy Engine");
          ui.label("serde");
          ui.label("kira");
          ui.label("regex");
          ui.label("ffmpeg-cli");
          ui.label("bevy-tokio-tasks");
          ui.label("bevy_image_export");
          ui.label("directories");
          ui.label("normalize-line-endings");
          ui.label("bevy_file_dialog");
          ui.label("image");
          ui.label("sadness-generator");
          ui.label("minidump-writer");
          ui.label("minidumper-child");
          ui.label("minidump");
        }
      );
      if ui.button("Close").clicked() {
        about_dialog_state.is_open = false;
      }
    });
  }
}
