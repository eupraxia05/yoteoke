#![cfg_attr(
  all(
    target_os = "windows",
  ),
  windows_subsystem = "windows"
)]

use bevy::log::{tracing_subscriber, BoxedLayer};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy_egui::EguiPlugin;
use bevy_image_export::ImageExportPlugin;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use lyrics::LyricsPlugin;
use project::ProjectPlugin;
use bevy_file_dialog::prelude::*;
use std::env::{set_current_dir, current_exe};

mod lyrics;
use crate::lyrics::ParsedLyrics;

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

mod crash_handling;

mod timeline;
use timeline::TimelinePlugin;

mod help;
use help::HelpPlugin;

use bevy_tokio_tasks::TokioTasksPlugin;

fn main() {
  #[cfg(not(debug_assertions))]
  {
    println!("Setting cwd...");
    let mut exe_dir = current_exe().unwrap();
    exe_dir.pop();
    set_current_dir(exe_dir).expect("couldn't set cwd!");
  }

  let _guard = crash_handling::run_handler();

  let mut app = App::new();

  let export_plugin = ImageExportPlugin::default();
  let export_threads = export_plugin.threads.clone();
    
  println!("Configuring plugins...");
  app
    .add_plugins(DefaultPlugins
      .set(RenderPlugin {
        synchronous_pipeline_compilation: true,
        ..default()
      })
      .set(WindowPlugin {
        close_when_requested: false,
        primary_window: Some(Window {
          title: "YoteOke Lyric Editor".into(),
          ..default()
        }),
        ..default()
      })
    )
    .add_plugins(export_plugin)
    .add_plugins(EguiPlugin)
    .add_plugins(TokioTasksPlugin::default())
    .add_plugins(
      export::configure_file_dialog_plugin(
        project::configure_file_dialog_plugin(FileDialogPlugin::new())
      )
    )
    .add_plugins(EditorPlugin)
    .add_plugins(ProjectPlugin)
    .add_plugins(AudioPlugin)
    .add_plugins(LyricsPlugin)
    .add_plugins(StagePlugin)
    .add_plugins(TimelinePlugin)
    .add_plugins(HelpPlugin)
    .add_plugins(DefaultInspectorConfigPlugin);


  sub_viewport::build(&mut app);
  export::build(&mut app);

  println!("Running app...");
  app.run();

  export_threads.finish();
}
