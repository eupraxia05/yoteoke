use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy_egui::EguiPlugin;
use bevy_image_export::ImageExportPlugin;
use lyrics::LyricsPlugin;
use project::ProjectPlugin;
use bevy_file_dialog::prelude::*;

mod lyrics;
use crate::lyrics::ParsedLyrics;

mod ui;
use crate::ui::ProjectSettingsDialog;

mod sub_viewport;
use crate::sub_viewport::SubViewport;

mod export;

mod project;
use crate::project::NewProjectDialog;

mod stage;
use crate::stage::StagePlugin;

mod audio;
use crate::audio::AudioPlugin;

mod editor;
use crate::editor::EditorPlugin;

use bevy_tokio_tasks::TokioTasksPlugin;

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
    .add_plugins(TokioTasksPlugin::default())
    .add_plugins(project::configure_file_dialog_plugin(FileDialogPlugin::new()))
    .add_plugins(EditorPlugin)
    .add_plugins(ProjectPlugin)
    .add_plugins(AudioPlugin)
    .add_plugins(LyricsPlugin)
    .add_plugins(StagePlugin);


  ui::build(&mut app);
  sub_viewport::build(&mut app);
  export::build(&mut app);

  app.run();

  export_threads.finish();
}
