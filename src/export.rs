use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use bevy_image_export::{ImageExport, ImageExportSettings, ImageExportSource};
use directories::ProjectDirs;
use std::fs;
use bevy_tokio_tasks::TokioTasksRuntime;
use ffmpeg_cli::{FfmpegBuilder, File, Parameter};
use std::process::Stdio;
use bevy_egui::egui;
use bevy_file_dialog::prelude::*;
//use futures::{future::ready, StreamExt};

use crate::editor::EditorState;
use crate::sub_viewport::SubViewport;

pub fn build(app: &mut App) {
  app.add_systems(Startup, startup);
  app.insert_resource(ExportState::default());
  app.add_event::<ExportInitiatedEvent>();
  app.add_systems(Update, handle_export_initiated);
  app.add_systems(Update, update_export);
  app.add_systems(Update, handle_export_file_path_dialog);
}

fn handle_export_file_path_dialog(
  mut events: EventReader<DialogFileSaved<ExportFilePathDialog>>,
  mut export_state: ResMut<ExportState>,
  mut export_initiated_events: EventWriter<ExportInitiatedEvent>
) {
  for ev in events.read() {
    export_state.output_path = ev.path.clone();
    export_initiated_events.send_default();
  }
}

pub fn configure_file_dialog_plugin(plugin: FileDialogPlugin) -> FileDialogPlugin {
  plugin.with_save_file::<ExportFilePathDialog>()
}

fn startup() {

}

#[derive(Default, Resource, Debug)]
pub struct ExportState {
  pub is_exporting: bool,
  frame_idx: usize,
  export_ent: Option<Entity>,
  output_path: PathBuf,
}

impl ExportState {
  pub fn is_exporting(&self) -> bool {
    self.is_exporting
  }

  pub fn frame_idx(&self) -> usize {
    self.frame_idx
  }
}

#[derive(Event, Default)]
pub struct ExportInitiatedEvent;

fn handle_export_initiated(mut commands: Commands,
  mut event_reader: EventReader<ExportInitiatedEvent>,
  mut export_state: ResMut<ExportState>,
  mut export_sources: ResMut<Assets<ImageExportSource>>,
  sub_viewport_query: Query<&mut SubViewport>) 
{ 
  for _ in event_reader.read() {
    if !export_state.is_exporting {
      export_state.is_exporting = true;

      let img_dir = ProjectDirs::from("", "yoteoke", "yoteoke").unwrap().cache_dir().join(Path::new("export"));
      info!("image dir: {:?}", img_dir);
      fs::remove_dir_all(img_dir.clone()).unwrap();

      export_state.export_ent = Some(
        commands.spawn((
          ImageExport(
            export_sources.add(sub_viewport_query.single().image_handle())
          ),
          ImageExportSettings {
            output_dir: img_dir.as_os_str().to_string_lossy().into(),
            extension: "png".into()
          }
        ))
      .id());
    }
  }
}

fn update_export(mut export_state: ResMut<ExportState>, mut commands: Commands,
  editor_state: NonSend<EditorState>, audio_state: NonSend<crate::editor::AudioState>,
  tokio_runtime: Res<TokioTasksRuntime>) 
{
  if export_state.is_exporting {
    export_state.frame_idx += 1;
    if export_state.frame_idx as f64 / 12. > audio_state.duration.unwrap().as_secs_f64() + editor_state.project_data.as_ref().unwrap().song_delay_time.unwrap() as f64 {
      if let Some(export_ent) = export_state.export_ent {
        commands.entity(export_ent).despawn();
        export_state.export_ent = None;
        export_state.is_exporting = false;

        let img_dir = ProjectDirs::from("", "yoteoke", "yoteoke").unwrap().cache_dir().join(Path::new("export"));
        info!("image dir: {:?}", img_dir);

        let song_path: String = String::from(editor_state.project_data.as_ref().unwrap().song_file.as_ref().unwrap().as_os_str().to_string_lossy());

        let output_path = String::from(export_state.output_path.as_os_str().to_str().unwrap());

        let song_delay_time = editor_state.project_data.as_ref().unwrap().song_delay_time.unwrap().to_string();

        tokio_runtime.spawn_background_task(|_ctx| async move {
          let input_path: String = img_dir.join("%05d.png").as_os_str().to_string_lossy().into();

          let builder = FfmpegBuilder::new()
            .stderr(Stdio::piped())
            .option(Parameter::Single("nostdin"))
            // overwrite file if it exists
            .option(Parameter::Single("y"))
            .option(Parameter::KeyValue("r", "12"))
            .input(File::new(&input_path))
            .input(File::new(&song_path).option(Parameter::KeyValue("itsoffset", song_delay_time.as_str()))) 
            .output(
              // todo: get rid of unwrap
              File::new(output_path.as_str())
                .option(Parameter::KeyValue("vcodec", "libx264"))
                .option(Parameter::KeyValue("acodec", "mp3"))
                .option(Parameter::KeyValue("crf", "25"))
                .option(Parameter::KeyValue("map", "0:v:0"))  
                .option(Parameter::KeyValue("map", "1:a:0"))
          );

          let ffmpeg = builder.run().await.unwrap();

          /*ffmpeg
              .progress
              .for_each(|x| {
                  dbg!(x.unwrap());
                  ready(())
              })
              .await;*/

          let output = ffmpeg.process.wait_with_output().unwrap();

          println!(
              "{}\nstderr:\n{}",
              output.status,
              std::str::from_utf8(&output.stderr).unwrap()
          );
        });
      }
    }
  }
}
pub struct ExportFilePathDialog;
