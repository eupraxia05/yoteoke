use bevy::prelude::*;
use std::path::PathBuf;
use std::time::Duration;
use kira::sound::streaming::StreamingSoundHandle;
use kira::AudioManager;
use kira::sound::FromFileError;
use bevy_egui::egui;
use bevy::ecs::system::SystemState;
use bevy_egui::EguiContexts;

use crate::export::ExportInitiatedEvent;
use crate::NewProjectDialog;
use crate::ParsedLyrics;
use crate::project::ProjectSettingsDialog;
use crate::project::ProjectData;
use crate::export::ExportDialog;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
  fn build(&self, app: &mut App) {
    app.insert_non_send_resource(EditorState::default());
    app.add_systems(Update, ui);
  }
}

#[derive(Default)]
pub struct EditorState {
  pub project_file_path: PathBuf,
  pub project_data: Option<crate::project::ProjectData>,
  pub new_file_dialog: Option<NewProjectDialog>,
  pub music_handle: Option<StreamingSoundHandle<FromFileError>>,
  pub audio_manager: Option<AudioManager>,
  pub duration: Option<Duration>,
  pub parsed_lyrics: Option<ParsedLyrics>,
  pub lyrics_dirty: bool,
  pub project_settings_dialog: ProjectSettingsDialog,
  pub thumbnail_image: Option<Handle<Image>>,
  pub thumbnail_egui_tex_id: Option<egui::TextureId>,
}

impl EditorState {
    pub fn playhead_position(&self) -> Duration {
      if let Some(music_handle) = &self.music_handle {
        return Duration::from_secs_f64(music_handle.position());
      } else {
        return Duration::default();
      }
    }
}

fn ui(world: &mut World) {
  let ctx = {
    let mut contexts_state = SystemState::<EguiContexts>::new(world);
    let mut contexts = contexts_state.get_mut(world);
    contexts.ctx_mut().clone()
  };
  egui::TopBottomPanel::top("menu").show(&ctx, |ui| {
    world.run_system_cached_with(menu_ui, ui);
  });

  let project_loaded = world.get_non_send_resource::<EditorState>().unwrap().project_data.is_some();

  egui::CentralPanel::default().show(&ctx, |ui| {
    if project_loaded {
      egui::SidePanel::new(egui::panel::Side::Left, "main_left_panel")
        .default_width(512.)
        .show_inside(ui, |ui| 
        {
          world.run_system_cached_with(crate::lyrics::lyrics_edit_ui, ui);
        }
      );
      egui::CentralPanel::default().show_inside(ui, |ui| {
        egui::TopBottomPanel::new(egui::panel::TopBottomSide::Bottom, "timeline_panel")
          .exact_height(256.)
          .show_inside(ui, |ui|
          {
            world.run_system_cached_with(crate::timeline::timeline_ui, ui);
          });
        egui::CentralPanel::default().show_inside(ui, |ui| {
          world.run_system_cached_with(crate::stage::preview_ui, ui);
        });
      });
    }
  });

  world.run_system_cached(file_dialog_ui);
}

fn menu_ui(ui: InMut<egui::Ui>, world: &mut World) 
{
  egui::menu::bar(ui.0, |ui| {
    ui.menu_button("File", |ui| {
      world.run_system_cached_with(crate::project::file_ops_menu_ui, ui);
    });
    ui.menu_button("Project", |ui| {
      world.run_system_cached_with(crate::project::project_menu_ui, ui);
    });

    ui.menu_button("Debug", |ui| {
      if ui.button("Sadness").clicked() {
        unsafe {
          sadness_generator::raise_segfault();
        }
      }
    });
  });
}

fn file_dialog_ui(mut contexts: EguiContexts, mut editor_state: NonSendMut<EditorState>, mut commands: Commands, mut export_dialog: ResMut<ExportDialog>,
  mut export_event_writer: EventWriter<ExportInitiatedEvent>
) {
  if let Some(new_project_dialog) = &mut editor_state.new_file_dialog {
    new_project_dialog.show(contexts.ctx_mut(), &mut commands);
  }

  // todo: this is gross
  let mut project_data_temp = ProjectData::default();
  if let Some(project_data) = &mut editor_state.project_data {
    project_data_temp = project_data.clone();
  }
  let thumbnail_egui_tex_id = editor_state.thumbnail_egui_tex_id.clone();
  editor_state.project_settings_dialog.show(contexts.ctx_mut(), &mut project_data_temp, &mut commands, thumbnail_egui_tex_id);

  if let Some(project_data) = &mut editor_state.project_data {
    *project_data = project_data_temp;
  }

  export_dialog.show(contexts.ctx_mut(), &mut export_event_writer);
}
