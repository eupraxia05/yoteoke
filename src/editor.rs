use bevy::prelude::*;
use std::path::PathBuf;
use std::time::Duration;
use kira::sound::streaming::StreamingSoundHandle;
use kira::AudioManager;
use kira::sound::FromFileError;
use bevy_egui::egui;
use bevy::ecs::system::SystemState;
use bevy_egui::EguiContexts;
use bevy::window::WindowCloseRequested;

use crate::export::ExportInitiatedEvent;
use crate::project::ProjectSavedEvent;
use crate::project::SaveProjectRequestedEvent;
use crate::NewProjectDialog;
use crate::ParsedLyrics;
use crate::project::ProjectSettingsDialog;
use crate::project::ProjectData;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
  fn build(&self, app: &mut App) {
    app.insert_non_send_resource(EditorState::default());
    app.insert_non_send_resource(AudioState::default());
    app.add_systems(Update, ui);
    app.add_systems(Update, handle_window_close_requested);
    app.insert_resource(ExitConfirmDialog::default());
    app.add_systems(Update, exit_confirm_dialog_ui);
    app.add_systems(Update, finish_save_and_exit);
    app.insert_resource(TitlecardState::default());
  }
}

#[derive(Default)]
pub struct EditorState {
  pub project_file_path: PathBuf,
  pub project_data: Option<crate::project::ProjectData>,
  pub new_file_dialog: Option<NewProjectDialog>,
  pub parsed_lyrics: Option<ParsedLyrics>,
  pub lyrics_dirty: bool,
  pub needs_save_before_exit: bool,
  pub is_in_pre_delay: bool,
  pub curr_pre_delay_time: f64,
  pub is_paused: bool,
}

#[derive(Default)]
pub struct AudioState {
  pub music_handle: Option<StreamingSoundHandle<FromFileError>>,
  pub audio_manager: Option<AudioManager>,
  pub duration: Option<Duration>,
}

#[derive(Default, Resource)]
pub struct TitlecardState {
  pub titlecard_image: Option<Handle<Image>>,
  pub titlecard_egui_tex_id: Option<egui::TextureId>,
}

impl AudioState {
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
    world.run_system_cached_with(menu_ui, ui).expect("Couldn't run menu_ui system!");
  });

  let project_loaded = world.get_non_send_resource::<EditorState>().unwrap().project_data.is_some();

  egui::CentralPanel::default().show(&ctx, |ui| {
    if project_loaded {
      egui::SidePanel::new(egui::panel::Side::Left, "main_left_panel")
        .default_width(512.)
        .show_inside(ui, |ui| 
        {
          world.run_system_cached_with(crate::lyrics::lyrics_edit_ui, ui).expect("Couldn't run lyrics_edit_ui system!");
        }
      );
      egui::CentralPanel::default().show_inside(ui, |ui| {
        egui::TopBottomPanel::new(egui::panel::TopBottomSide::Bottom, "timeline_panel")
          .exact_height(256.)
          .show_inside(ui, |ui|
          {
            world.run_system_cached_with(crate::timeline::timeline_ui, ui).expect("Couldn't run timeline_ui system!");
          });
        egui::CentralPanel::default().show_inside(ui, |ui| {
          world.run_system_cached_with(crate::stage::preview_ui, ui).expect("Couldn't run preview_ui system!");
        });
      });
    }
  });

  world.run_system_cached(file_dialog_ui).expect("Couldn't run file_dialog_ui system!");
  world.run_system_cached(crate::help::help_dialog_ui).expect("Couldn't run help_dialog_ui system!");
  world.run_system_cached(crate::help::about_dialog_ui).expect("Couldn't run about_dialog_ui system!");
  world.run_system_cached(crate::project::project_settings_dialog_ui).expect("Couldn't run project_settings_dialog_ui system!");
}

fn menu_ui(ui: InMut<egui::Ui>, world: &mut World) 
{
  egui::menu::bar(ui.0, |ui| {
    ui.menu_button("File", |ui| {
      world.run_system_cached_with(crate::project::file_ops_menu_ui, ui).expect("Couldn't run file_ops_menu_ui system!");
    });
    ui.menu_button("Project", |ui| {
      world.run_system_cached_with(crate::project::project_menu_ui, ui).expect("Couldn't run project_menu_ui system!");
    });
    ui.menu_button("Help", |ui| {
      if ui.button("Help").clicked() {
        world.send_event_default::<crate::help::HelpDialogOpenRequested>();
      }
      if ui.button("About").clicked() {
        world.send_event_default::<crate::help::AboutDialogOpenRequested>();
      }
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

fn file_dialog_ui(mut contexts: EguiContexts, 
  mut editor_state: NonSendMut<EditorState>, 
  mut commands: Commands,
  mut export_event_writer: EventWriter<ExportInitiatedEvent>,
  type_registry: Res<AppTypeRegistry>,
  titlecard_state: Res<TitlecardState>
) {
  if let Some(new_project_dialog) = &mut editor_state.new_file_dialog {
    new_project_dialog.show(contexts.ctx_mut(), &mut commands);
  }

  // todo: this is gross
  let mut project_data_temp = ProjectData::default();
  if let Some(project_data) = &mut editor_state.project_data {
    project_data_temp = project_data.clone();
  }

  if let Some(project_data) = &mut editor_state.project_data {
    *project_data = project_data_temp;
  }
}

fn handle_window_close_requested(mut events: EventReader<WindowCloseRequested>,
  mut exit_confirm_dialog: ResMut<ExitConfirmDialog>,
  editor_state: NonSend<EditorState>,
  mut exit_events: EventWriter<AppExit>,
) {
  for ev in events.read() {
    if editor_state.needs_save_before_exit {
      exit_confirm_dialog.is_active = true;
    } else {
      exit_events.send(AppExit::Success);
    }
  } 
}

#[derive(Resource, Default)]
struct ExitConfirmDialog {
  is_active: bool,
  is_saving_before_exit: bool,
}

fn exit_confirm_dialog_ui(mut exit_confirm_dialog: ResMut<ExitConfirmDialog>,
  mut egui_contexts: EguiContexts,
  mut exit_events: EventWriter<AppExit>,
  mut save_events: EventWriter<SaveProjectRequestedEvent>
) {
  if exit_confirm_dialog.is_active {
    egui::Window::new("Exit?").show(egui_contexts.ctx_mut(), |ui| {
      ui.label("You have unsaved changes. Are you sure you want to exit?");
      ui.horizontal(|ui| {
        if ui.button("Cancel").clicked() {
          exit_confirm_dialog.is_active = false;
        }
        if ui.button("Save and exit").clicked() {
          save_events.send_default();
          exit_confirm_dialog.is_saving_before_exit = true;
        }
        if ui.button("Exit without saving").clicked() {
          exit_events.send(AppExit::Success);
        }
      });
    });
  }
}

fn finish_save_and_exit(exit_confirm_dialog: Res<ExitConfirmDialog>, 
  mut exit_events: EventWriter<AppExit>,
  project_saved_events: EventReader<ProjectSavedEvent>
) {
  if exit_confirm_dialog.is_saving_before_exit {
    if !project_saved_events.is_empty() {
      exit_events.send(AppExit::Success);
    }
  }
}