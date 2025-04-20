use bevy::prelude::*;
use bevy_egui::egui;
use std::path::PathBuf;
use bevy_file_dialog::{prelude::*, FileDialog};
use std::fs::File;
use std::io::Write;
use serde::{Serialize, Deserialize};
use bevy_egui::EguiUserTextures;
use image::ImageReader;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_egui::egui::load::SizedTexture;

use crate::editor::EditorState;
use crate::stage::TitlecardUpdatedEvent;
use crate::export::ExportDialog;

pub struct ProjectPlugin;

impl Plugin for ProjectPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, handle_new_project_requested_events);
    app.add_systems(Update, handle_new_project_song_file_dialog_picked);
    app.add_systems(Update, handle_new_project_save_file_dialog);
    app.add_systems(Update, handle_new_project_dialog_submitted_events);
    app.add_systems(Update, handle_save_project_requested_event);
    app.add_systems(Update, handle_open_project_dialog);
    app.add_systems(Update, handle_thumbnail_file_path_dialog);
    app.add_event::<NewProjectRequestedEvent>();
    app.add_event::<NewProjectDialogSubmittedEvent>();
    app.add_event::<SaveProjectRequestedEvent>();
    app.add_event::<SaveAsRequestedEvent>();
  }
}

#[derive(Event, Default)]
pub struct NewProjectRequestedEvent;

fn handle_new_project_requested_events(mut events: EventReader<NewProjectRequestedEvent>, mut editor_state: NonSendMut<EditorState>) {
  for ev in events.read() {
    let mut new_file_dialog = NewProjectDialog::default();
    new_file_dialog.open();
    editor_state.new_file_dialog = Some(new_file_dialog);
  }
}

pub struct NewProjectDialog {
  is_open: bool,
  pub is_submitted: bool,
  pub artist: String,
  pub title: String,
  pub song_file: Option<PathBuf>,
  pub save_file: Option<PathBuf>,
}

impl Default for NewProjectDialog {
  fn default() -> Self {
      Self {
          is_open: false,
          is_submitted: false,
          artist: "glass beach".into(),
          title: "cul-de-sac".into(),
          song_file: None,
          save_file: None,
      }
  }
}

impl NewProjectDialog {
  pub fn open(&mut self) {
      self.is_open = true;
  }

  pub fn show(&mut self, ctx: &egui::Context, commands: &mut Commands) {
    if self.is_open {
      egui::Window::new("New Project").show(ctx, |ui| {
        let mut last_visited_path: Option<PathBuf> = None;
        ui.data_mut(|map| {
          last_visited_path = map.get_persisted("last_visited_path".into());
        });

        ui.horizontal(|ui| {
          ui.label("Artist");
          ui.text_edit_singleline(&mut self.artist)
        });

        ui.horizontal(|ui| {
          ui.label("Title");
          ui.text_edit_singleline(&mut self.title)
        });

        ui.horizontal(|ui| {
          ui.label("Song File");
          if let Some(song_file_path) = &self.song_file {
            ui.label(song_file_path.as_os_str().to_string_lossy());
          } else {
            ui.label("No file selected");
          }
          if ui.button("Browse...").clicked() {
            commands.dialog().pick_file_path::<NewProjectSongFileDialog>();
          }
        });

        let can_create = self.song_file != None;

        if ui.add_enabled(can_create, egui::Button::new("Create")).clicked() {
          commands.send_event(NewProjectDialogSubmittedEvent::default());
          self.is_open = false;
          self.is_submitted = true;
        }
      });
    }
  }
}

#[derive(Default)]
pub struct NewProjectSongFileDialog;

fn handle_new_project_song_file_dialog_picked(mut events: EventReader<DialogFilePicked<NewProjectSongFileDialog>>, mut editor_state: NonSendMut<EditorState>) {
  for ev in events.read() {
    if let Some(new_project_dialog) = &mut editor_state.new_file_dialog {
      new_project_dialog.song_file = Some(ev.path.clone());
    }
  }
}

#[derive(Event, Default)]
struct NewProjectDialogSubmittedEvent;

fn handle_new_project_dialog_submitted_events(
  mut events: EventReader<NewProjectDialogSubmittedEvent>,
  mut editor_state: NonSendMut<EditorState>,
  mut commands: Commands,
) {
  for _ in events.read() {
    if let Some(new_project_dialog) = editor_state.new_file_dialog.as_ref() {
      let mut project_data = ProjectData::default();
      project_data.artist = new_project_dialog.artist.clone();
      project_data.title = new_project_dialog.title.clone();
      project_data.song_file = new_project_dialog.song_file.clone();
      editor_state.project_data = Some(project_data);
      editor_state.new_file_dialog = None;
    
      let serialized = serde_json::to_vec_pretty(editor_state.project_data.as_ref().unwrap()).unwrap();
      commands.dialog().save_file::<NewProjectSaveFileDialog>(serialized);
    }
  }
}

#[derive(Default)]
pub struct NewProjectSaveFileDialog;

fn handle_new_project_save_file_dialog(
  mut events: EventReader<DialogFileSaved<NewProjectSaveFileDialog>>,
  mut editor_state: NonSendMut<EditorState>) 
{
  for ev in events.read() {
    editor_state.project_file_path = ev.path.clone();
  }
}

fn handle_save_project_requested_event(mut events: EventReader<SaveProjectRequestedEvent>, mut editor_state: NonSendMut<EditorState>) {
  for _ in events.read() {
    let mut vec = Vec::new();
    if let Some(project_data) = &editor_state.project_data {
        vec = serde_json::to_vec_pretty(&project_data).unwrap();
    }

    match File::create(editor_state.project_file_path.clone())
      .unwrap()
      .write_all(&vec[..])
    {
      Err(e) => {
        error!("Error saving to {:?}: {:?}", editor_state.project_file_path, e);
      },
      Ok(_) => {
        println!("Project saved to {:?}", editor_state.project_file_path);
      }
    }
  }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProjectData {
  pub lyrics: String,
  pub artist: String,
  pub title: String,
  pub song_file: Option<PathBuf>,
  pub background_color: Option<Color>,
  pub unsung_color: Option<Color>,
  pub sung_color: Option<Color>,
  pub thumbnail_path: Option<PathBuf>,
  pub titlecard_show_time: Option<f32>
}

impl Default for ProjectData {
  fn default() -> Self {
    Self {
      lyrics: default(),
      artist: default(),
      title: default(),
      song_file: None,
      background_color: Some(Color::BLACK),
      sung_color: Some(Color::WHITE),
      unsung_color: Some(Color::srgb(0.5, 0.5, 0.5)),
      thumbnail_path: None,
      titlecard_show_time: Some(10.)
    }
  }
}

pub struct OpenProjectDialog;

pub struct SaveAsDialog;

pub struct LoadDialog;

pub struct ThumbnailFilePathDialog;

pub fn configure_file_dialog_plugin(plugin: FileDialogPlugin) -> FileDialogPlugin {
  plugin.with_load_file::<crate::project::LoadDialog>()
  .with_pick_file::<crate::project::ThumbnailFilePathDialog>()
  .with_pick_file::<crate::project::NewProjectSongFileDialog>()
  .with_save_file::<crate::project::NewProjectSaveFileDialog>()
  .with_load_file::<crate::project::OpenProjectDialog>()
  .with_save_file::<crate::project::SaveAsDialog>()
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

pub fn file_ops_menu_ui(mut ui: InMut<egui::Ui>, 
  editor_state: NonSend<EditorState>,
  mut new_project_event_writer: EventWriter<NewProjectRequestedEvent>,
  mut save_requested_event_writer: EventWriter<SaveProjectRequestedEvent>,
  mut commands: Commands) {
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
}

pub fn project_menu_ui(mut ui: InMut<egui::Ui>,
  mut editor_state: NonSendMut<EditorState>,
  mut export_dialog: ResMut<ExportDialog>
) {
  if ui.button("Project Settings...").clicked() {
    editor_state.project_settings_dialog.open();
  }
  if ui.button("Export...").clicked() {
    info!("export button clicked");
    export_dialog.open();
  }
}

#[derive(Event, Default)]
pub struct SaveProjectRequestedEvent;

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

#[derive(Default, Event)]
struct SaveAsRequestedEvent;