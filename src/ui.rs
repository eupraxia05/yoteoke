use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::RenderLayers;
use bevy_egui::egui::load::SizedTexture;
use bevy_egui::egui::Style;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiUserTextures};
use kira::command;
use kira::sound::static_sound::StaticSoundHandle;
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::sound::{FromFileError, PlaybackState, SoundData};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::ops::RangeInclusive;
use std::path::{PathBuf, Path};
use std::time::Duration;
use std::{fs::File, io::Read};
use kira::{
  AudioManager, 
  AudioManagerSettings, 
  DefaultBackend,
  Tween
};
use image::{ImageReader, ImageDecoder, DynamicImage};

use crate::export::{ExportInitiatedEvent, ExportState};
use crate::project::NewProjectRequestedEvent;
use crate::{editor::EditorState, project::OpenProjectDialog};
use crate::sub_viewport::SubViewport;
use bevy_file_dialog::{FileDialogExt, DialogFilePicked, DialogFileLoaded};
use crate::project::ThumbnailFilePathDialog;
use crate::stage::TitlecardUpdatedEvent;
use crate::project::ProjectData;

pub fn build(app: &mut App) {
  app.add_systems(Startup, startup);
  app.add_systems(Update, ui);
  app.add_systems(Update, handle_thumbnail_file_path_dialog);
  app.insert_resource(ExportDialog::default());
  app.add_systems(Update, handle_open_project_dialog);
  app.add_event::<SaveAsRequestedEvent>();
  app.add_event::<SaveProjectRequestedEvent>();
}

fn startup(mut commands: Commands) {

}

fn handle_open_project_dialog(
  mut events: EventReader<bevy_file_dialog::DialogFileLoaded<OpenProjectDialog>>, 
  mut editor_state: NonSendMut<EditorState>,
  mut titlecard_updated_events: EventWriter<TitlecardUpdatedEvent>,
  mut images: ResMut<Assets<Image>>,
  mut egui_user_textures: ResMut<EguiUserTextures>
) {
  for ev in events.read() { 
    editor_state.project_file_path = ev.path.clone();

    if let Ok(mut data) = serde_json::from_slice::<ProjectData>(ev.contents.as_slice()) {
      // hack: ensuring this field exists (should actually load projectdata 
      // from a separate deserialized struct)
      if data.titlecard_show_time.is_none() {
        data.titlecard_show_time = Some(10.);
      }
      editor_state.project_data = Some(data);
      editor_state.lyrics_dirty = true;
    } else {
        println!("couldn't deserialize file");
    }

    if let Some(titlecard_path) = &editor_state.project_data.as_ref().unwrap().thumbnail_path {
      let load_result = load_titlecard_image(&titlecard_path, images.as_mut(), egui_user_textures.as_mut());
      let Some((image_handle, egui_texture_id)) = load_result else {
        return;
      };
  
      editor_state.thumbnail_image = Some(image_handle);
      editor_state.thumbnail_egui_tex_id = Some(egui_texture_id);
      if let Some(project_data) = editor_state.project_data.as_mut() {
        project_data.thumbnail_path = Some(ev.path.clone());
      }  
  
      titlecard_updated_events.send_default();
    }
  }
}

fn ui(mut contexts: EguiContexts, mut editor_state: NonSendMut<EditorState>,
  camera_tex_query: Query<&SubViewport>, images: Res<Assets<Image>>,
  mut export_dialog: ResMut<ExportDialog>,
  mut export_state: ResMut<ExportState>,
  mut export_event_writer: EventWriter<ExportInitiatedEvent>,
  mut new_project_event_writer: EventWriter<NewProjectRequestedEvent>,
  mut save_requested_event_writer: EventWriter<SaveProjectRequestedEvent>,
  mut commands: Commands,
) {
  egui::TopBottomPanel::top("menu").show(contexts.ctx_mut(), |ui| {
    menu_ui(ui, editor_state.reborrow(), export_dialog.as_mut(), &mut export_event_writer, &mut new_project_event_writer, &mut save_requested_event_writer, &mut commands);
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
          let preview_img = camera_tex_query.single();
          preview_ui(ui, editor_state.reborrow(), preview_img, images.as_ref(), export_state.as_mut());
        });
      });
    }
  });

  file_dialog_ui(&mut contexts, editor_state.reborrow(), export_dialog.reborrow(), &mut commands);

  export_dialog.show(contexts.ctx_mut(), &mut export_event_writer, &mut commands);
}

fn menu_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>, 
  export_dialog: &mut ExportDialog, 
  export_event_writer: &mut EventWriter<ExportInitiatedEvent>,
  new_project_event_writer: &mut EventWriter<NewProjectRequestedEvent>,
  save_requested_event_writer: &mut EventWriter<SaveProjectRequestedEvent>,
  commands: &mut Commands) 
{
  egui::menu::bar(ui, |ui| {
    ui.menu_button("File", |ui| {
      if ui.button("New...").clicked() {
        new_project_event_writer.send_default();
      }
      if ui.button("Open...").clicked() {
        commands.dialog().load_file::<crate::project::OpenProjectDialog>();
      }
      if ui.button("Save").clicked() {
        save_requested_event_writer.send_default();
      }
      if ui.button("Save As...").clicked() {
        if let Some(project_data) = editor_state.project_data.as_ref() {
          let serialized = serde_json::to_vec_pretty(project_data).unwrap();
          commands.dialog().save_file::<crate::project::SaveAsDialog>(serialized);
        }

      }
    });
    ui.menu_button("Project", |ui| {
      if ui.button("Project Settings...").clicked() {
        editor_state.project_settings_dialog.open();
      }
      if ui.button("Export...").clicked() {
        info!("export button clicked");
        export_dialog.open();
      }
    });
  });
}

#[derive(Default, Event)]
struct SaveAsRequestedEvent;

fn lyrics_edit_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>) {
  let mut text_edit_changed = false;
  let mut cursor_pos = None;
  let mut insert_desired = false;
  let curr_time = if let Some(music_handle) = &editor_state.music_handle {
    Duration::from_secs_f64(music_handle.position())
  } else {
    Duration::default()
  };
  if let Some(project_data) = &mut editor_state.project_data {
    let title_str = format!("{} - {}", project_data.artist, project_data.title);
    ui.label(title_str);
    if ui.button("Insert").clicked() {
      insert_desired = true;
    }
    ui.separator();
    egui::ScrollArea::both().show(ui, |ui| {
      let text_edit_response = ui.add_sized(ui.available_size(), 
        egui::TextEdit::multiline(&mut project_data.lyrics).code_editor());
      if text_edit_response.changed() {
        info!("text edit changed");
        text_edit_changed = true;
      }
      if let Some(text_edit_state) = egui::text_edit::TextEditState::load(ui.ctx(), 
        text_edit_response.id) 
      {
        if let Some(char_range) = text_edit_state.cursor.char_range() {
          cursor_pos = Some(char_range.primary);
        }
      }
    });
    if insert_desired {
      if let Some(cursor_pos) = cursor_pos {
        let str_to_insert = format!("[{:0>2}:{:0>2}.{:0>3}]", 
          curr_time.as_secs() / 60, curr_time.as_secs() % 60, curr_time.subsec_millis());
        project_data.lyrics.insert_str(cursor_pos.index, &str_to_insert);
        text_edit_changed = true
      }
    }
    // hack: keep carriage returns from entering lyrics
    project_data.lyrics = project_data.lyrics.replace("\r", "");
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
  let Some(music_handle) = editor_state.music_handle.as_mut() else {
    return;
  };
  let curr_time = Duration::from_secs_f64(music_handle.position());
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

fn timeline_blocks_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>,) {
  egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
    if let Some(parsed_lyrics) = &mut editor_state.parsed_lyrics {
      ui.horizontal(|ui| {
        for block in &parsed_lyrics.blocks {
          if let Some(time_range) = block.get_time_range() {
            let block_duration = if time_range.end > time_range.start {
              time_range.end - time_range.start
            } else {
              warn!("non-sequential time range: {:?}", time_range);
              Duration::default()
            };
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
        }
      });
    }
  });
}

fn file_dialog_ui(contexts: &mut EguiContexts, mut editor_state: Mut<EditorState>,
  mut export_dialog: Mut<ExportDialog>, commands: &mut Commands
) {

  if let Some(new_project_dialog) = &mut editor_state.new_file_dialog {
    new_project_dialog.show(contexts.ctx_mut(), commands);
  }

  // todo: this is gross
  let mut project_data_temp = ProjectData::default();
  if let Some(project_data) = &mut editor_state.project_data {
    project_data_temp = project_data.clone();
  }
  let thumbnail_egui_tex_id = editor_state.thumbnail_egui_tex_id.clone();
  editor_state.project_settings_dialog.show(contexts.ctx_mut(), &mut project_data_temp, commands, thumbnail_egui_tex_id);

  if let Some(project_data) = &mut editor_state.project_data {
    *project_data = project_data_temp;
  }
}

fn preview_ui(ui: &mut egui::Ui, mut editor_state: Mut<EditorState>,
  camera_tex: &SubViewport, images: &Assets<Image>, export_state: &mut ExportState) 
{
  egui::TopBottomPanel::top("preview_header").show_inside(ui, |ui| {
    if export_state.is_exporting() {
      ui.label("Exporting...");
    }
  });
  egui::CentralPanel::default().show_inside(ui, |ui| {
    camera_tex.show(ui, images);
  });
}

#[derive(Default)]
pub struct ProjectSettingsDialog {
  is_open: bool,
}

impl ProjectSettingsDialog {
  pub fn open(&mut self) {
    self.is_open = true;
  }

  fn color_property(ui: &mut egui::Ui, label_text: &str, color: &mut Option<Color>) {
    ui.horizontal(|ui| {
      ui.label(label_text);
      let c = color.unwrap_or_default().to_linear();
      let mut color_temp = [c.red, c.green, c.blue];
      ui.color_edit_button_rgb(&mut color_temp);
      *color = Some(Color::linear_rgb(color_temp[0], color_temp[1], color_temp[2]));
    });
  }

  pub fn show(&mut self, ctx: &egui::Context, 
    project_data: &mut ProjectData, commands: &mut Commands,
    thumbnail_egui_tex_id: Option<egui::TextureId>) 
  {
    if self.is_open {
      egui::Window::new("Project Settings").show(ctx, |ui| {
        Self::color_property(ui, "Background color", &mut project_data.background_color);
        Self::color_property(ui, "Text color (unsung)", &mut project_data.unsung_color);
        Self::color_property(ui, "Text color (sung)", &mut project_data.sung_color);
        ui.horizontal(|ui| {
          ui.label("Thumbnail");
          if ui.button("Set").clicked() {
            commands.dialog().set_directory("/").set_title("Select Thumbnail Image").pick_file_path::<ThumbnailFilePathDialog>();
          }
          if let Some(img) = thumbnail_egui_tex_id {
            ui.image(egui::ImageSource::Texture(SizedTexture::new(img, [128., 72.])));
          }
        });

        ui.horizontal(|ui| {
          ui.label("Titlecard show time");
          ui.add(egui::DragValue::new(project_data.titlecard_show_time.as_mut().unwrap()).speed(0.1));
        });
      });
    }
  }
}

fn load_titlecard_image(titlecard_path: &PathBuf, images: &mut Assets<Image>,
  egui_user_textures: &mut EguiUserTextures) 
  -> Option<(Handle<Image>, egui::TextureId)>
{
  let reader_result = ImageReader::open(titlecard_path.clone());
  let Ok(reader) = reader_result else {
    error!("error reading titlecard image: {:?}", reader_result.err().unwrap());
    return None;
  };

  let decode_result = reader.decode();
  let Ok(decode) = decode_result else {
    error!("error decoding titlecard image: {:?}", decode_result.err().unwrap());
    return None;
  };

  let converted_img = decode.to_rgba8();
  let image = Image::new(
    Extent3d {
      width: decode.width(),
      height: decode.height(),
      depth_or_array_layers: 1
    }, 
    TextureDimension::D2,
    converted_img.into_vec(),
    TextureFormat::Rgba8Unorm,
    RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD);
  let image_handle = images.add(image);
  let egui_texture_id = egui_user_textures.add_image(image_handle.clone());

  Some((image_handle, egui_texture_id))
}

fn handle_thumbnail_file_path_dialog(
  mut events: EventReader<DialogFilePicked<ThumbnailFilePathDialog>>,
  asset_server: Res<AssetServer>,
  mut egui_user_textures: ResMut<EguiUserTextures>,
  mut editor_state: NonSendMut<EditorState>,
  mut images: ResMut<Assets<Image>>,
  mut titlecard_updated_events: EventWriter<TitlecardUpdatedEvent>
) {
  for ev in events.read() {
    let load_result = load_titlecard_image(&ev.path, images.as_mut(), egui_user_textures.as_mut());
    let Some((image_handle, egui_texture_id)) = load_result else {
      return;
    };

    editor_state.thumbnail_image = Some(image_handle);
    editor_state.thumbnail_egui_tex_id = Some(egui_texture_id);
    if let Some(project_data) = editor_state.project_data.as_mut() {
      project_data.thumbnail_path = Some(ev.path.clone());
    }
    titlecard_updated_events.send_default();    
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

  pub fn show(&mut self, ctx: &egui::Context, export_event_writer: &mut EventWriter<ExportInitiatedEvent>, commands: &mut Commands) {
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

#[derive(Event, Default)]
pub struct SaveProjectRequestedEvent;