use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy_image_export::{ImageExport, ImageExportSettings, ImageExportSource};
use directories::ProjectDirs;
use std::fs;
use bevy_tokio_tasks::TokioTasksRuntime;
use ffmpeg_cli::{FfmpegBuilder, File, Parameter};
use std::process::Stdio;
use bevy_egui::egui;
//use futures::{future::ready, StreamExt};

use crate::editor::EditorState;
use crate::sub_viewport::SubViewport;

pub fn build(app: &mut App) {
  app.add_systems(Startup, startup);
  app.insert_resource(ExportState::default());
  app.add_event::<ExportInitiatedEvent>();
  app.add_systems(Update, handle_export_initiated);
  app.add_systems(Update, update_export);
  app.insert_resource(ExportDialog::default());
}

fn startup() {

}

#[derive(Default, Resource, Debug)]
pub struct ExportState {
  is_exporting: bool,
  frame_idx: usize,
  export_ent: Option<Entity>
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
  editor_state: NonSend<EditorState>, tokio_runtime: Res<TokioTasksRuntime>) 
{
  if export_state.is_exporting {
    export_state.frame_idx += 1;
    if export_state.frame_idx as f64 / 12. > editor_state.duration.unwrap().as_secs_f64() {
      if let Some(export_ent) = export_state.export_ent {
        commands.entity(export_ent).despawn();
        export_state.export_ent = None;
        export_state.is_exporting = false;

        let img_dir = ProjectDirs::from("", "yoteoke", "yoteoke").unwrap().cache_dir().join(Path::new("export"));
        info!("image dir: {:?}", img_dir);

        let song_path: String = String::from(editor_state.project_data.as_ref().unwrap().song_file.as_ref().unwrap().as_os_str().to_string_lossy());

        tokio_runtime.spawn_background_task(|_ctx| async move {
          let input_path: String = img_dir.join("%05d.png").as_os_str().to_string_lossy().into();

          let builder = FfmpegBuilder::new()
            .stderr(Stdio::piped())
            .option(Parameter::Single("nostdin"))
            // overwrite file if it exists
            .option(Parameter::Single("y"))
            .option(Parameter::KeyValue("r", "12"))
            .input(File::new(&input_path))
            .input(File::new(&song_path)) 
            .output(
              File::new("output.mp4")
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

#[derive(Default, Resource)]
pub struct ExportDialog {
  is_open: bool,
  output_file: Option<PathBuf>,
}

impl ExportDialog {
  pub fn open(&mut self) {
    self.is_open = true;
  }

  pub fn show(&mut self, ctx: &egui::Context, export_event_writer: &mut EventWriter<ExportInitiatedEvent>) {
    if self.is_open {
      egui::Window::new("Export").show(ctx, |ui| {
        ui.horizontal(|ui| {
          ui.label("Output File");
          // todo: remove to_string_lossy
          ui.label(self.output_file.clone().unwrap_or("".into()).to_string_lossy());
          if ui.button("Browse...").clicked() {
            // todo: browse
          }
        });
        if ui.button("Export").clicked() {
          export_event_writer.send(ExportInitiatedEvent::default());
          self.is_open = false;
        }
      });
    }
  }
}