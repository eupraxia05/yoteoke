use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use bevy_egui::{egui, EguiContexts};
use bevy_inspector_egui::inspector_egui_impls::{InspectorEguiImpl, InspectorPrimitive};
use bevy_inspector_egui::reflect_inspector::{Context, InspectorUi};
use std::path::PathBuf;
use bevy_file_dialog::prelude::*;
use std::fs::File;
use std::io::Write;
use serde::{Serialize, Deserialize};
use bevy_egui::EguiUserTextures;
use image::ImageReader;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{AsBindGroupShaderType, Extent3d, TextureDimension, TextureFormat};
use bevy_egui::egui::load::SizedTexture;
use bevy_file_dialog::prelude::*;
use bevy::ecs::world::CommandQueue;
use kira::Tween;

use crate::editor::{AudioState, EditorState, show_and_log_error, show_and_log_info};
use crate::stage::TitlecardUpdatedEvent;

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
    app.add_systems(Update, handle_song_file_path_dialog);
    app.add_event::<NewProjectRequestedEvent>();
    app.add_event::<NewProjectDialogSubmittedEvent>();
    app.add_event::<SaveProjectRequestedEvent>();
    app.add_event::<SaveAsRequestedEvent>();
    app.add_event::<ProjectSavedEvent>();
    app.register_type::<TitlecardPath>();
    app.register_type_data::<TitlecardPath, InspectorEguiImpl>();
    app.insert_resource(ProjectSettingsDialog::default());
    app.register_type::<SongFilePath>();
    app.register_type_data::<SongFilePath, InspectorEguiImpl>();
  }
}

#[derive(Event, Default)]
pub struct NewProjectRequestedEvent;

fn handle_new_project_requested_events(mut events: EventReader<NewProjectRequestedEvent>, mut editor_state: NonSendMut<EditorState>) {
  for _ in events.read() {
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
}

impl Default for NewProjectDialog {
  fn default() -> Self {
      Self {
          is_open: false,
          is_submitted: false,
          artist: "glass beach".into(),
          title: "cul-de-sac".into(),
          song_file: None,
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
            commands.dialog().add_filter("Audio file", &["mp3", "wav", "ogg", "flac"]).pick_file_path::<NewProjectSongFileDialog>();
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
      commands.dialog().add_filter("YoteOke Lyric Editor Project", &["yoke"]).save_file::<NewProjectSaveFileDialog>(serialized);
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

fn handle_save_project_requested_event(mut events: EventReader<SaveProjectRequestedEvent>, 
  mut editor_state: NonSendMut<EditorState>,
  mut project_saved_events: EventWriter<ProjectSavedEvent>
) {
  for _ in events.read() {
    let mut vec = Vec::new();
    if let Some(project_data) = &editor_state.project_data {
        vec = serde_json::to_vec_pretty(&project_data).unwrap();
    }

    let project_file_path = editor_state.project_file_path.clone();
    match File::create(project_file_path.clone())
      .unwrap()
      .write_all(&vec[..])
    {
      Err(e) => {
        show_and_log_error(editor_state.as_mut(), 
          format!("Error saving to {:?}: {:?}", project_file_path.clone(), e));
      },
      Ok(_) => {
        show_and_log_info(editor_state.as_mut(), 
          format!("Project saved to {:?}", project_file_path));
        editor_state.needs_save_before_exit = false;
        project_saved_events.send_default();
      }
    }
  }
}

#[derive(Event, Default)]
pub struct ProjectSavedEvent;

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
  pub titlecard_show_time: Option<f32>,
  pub song_delay_time: Option<f32>,
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
      titlecard_show_time: Some(10.),
      song_delay_time: Some(0.)
    }
  }
}

pub struct OpenProjectDialog;

pub struct SaveAsDialog;

pub struct LoadDialog;

pub struct TitlecardFilePathDialog;

pub fn configure_file_dialog_plugin(plugin: FileDialogPlugin) -> FileDialogPlugin {
  plugin.with_load_file::<crate::project::LoadDialog>()
  .with_pick_file::<crate::project::TitlecardFilePathDialog>()
  .with_pick_file::<crate::project::NewProjectSongFileDialog>()
  .with_save_file::<crate::project::NewProjectSaveFileDialog>()
  .with_load_file::<crate::project::OpenProjectDialog>()
  .with_save_file::<crate::project::SaveAsDialog>()
  .with_pick_file::<SongFilePathDialog>()
}

fn handle_open_project_dialog(
  mut events: EventReader<bevy_file_dialog::DialogFileLoaded<OpenProjectDialog>>, 
  mut editor_state: NonSendMut<EditorState>,
  mut titlecard_updated_events: EventWriter<TitlecardUpdatedEvent>,
  mut images: ResMut<Assets<Image>>,
  mut egui_user_textures: ResMut<EguiUserTextures>,
  mut titlecard_state: ResMut<crate::editor::TitlecardState>,
) {
  for ev in events.read() { 
    editor_state.project_file_path = ev.path.clone();

    if let Ok(mut data) = serde_json::from_slice::<ProjectData>(ev.contents.as_slice()) {
      // hack: ensuring this field exists (should actually load projectdata 
      // from a separate deserialized struct)
      if data.titlecard_show_time.is_none() {
        data.titlecard_show_time = Some(10.);
      }
      if data.song_delay_time.is_none() {
        data.song_delay_time = Some(10.);
      }
      editor_state.project_data = Some(data);
      editor_state.lyrics_dirty = true;
      editor_state.is_paused = true;
      editor_state.is_in_pre_delay = true;
    } else {
        println!("couldn't deserialize file");
    }

    if let Some(titlecard_path) = editor_state.project_data.as_ref().unwrap().thumbnail_path.clone() {
      let load_result = load_titlecard_image(&titlecard_path, images.as_mut(), egui_user_textures.as_mut(), editor_state.as_mut());
      let Some((image_handle, egui_texture_id)) = load_result else {
        return;
      };
  
      titlecard_state.titlecard_image = Some(image_handle);
      titlecard_state.titlecard_egui_tex_id = Some(egui_texture_id);
      if let Some(project_data) = editor_state.project_data.as_mut() {
        project_data.thumbnail_path = Some(ev.path.clone());
      }  
  
      titlecard_updated_events.send_default();
    }
  }
}

fn load_titlecard_image(titlecard_path: &PathBuf, images: &mut Assets<Image>,
  egui_user_textures: &mut EguiUserTextures, editor_state: &mut EditorState) 
  -> Option<(Handle<Image>, egui::TextureId)>
{
  let reader_result = ImageReader::open(titlecard_path.clone());
  let Ok(reader) = reader_result else {
    show_and_log_error(editor_state, 
      format!("Error reading titlecard image: {:?}", reader_result.err().unwrap()));
    return None;
  };

  let decode_result = reader.decode();
  let Ok(decode) = decode_result else {
    show_and_log_error(editor_state, 
      format!("Error decoding titlecard image: {:?}", decode_result.err().unwrap()));
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
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD);
  let image_handle = images.add(image);
  let egui_texture_id = egui_user_textures.add_image(image_handle.clone());

  Some((image_handle, egui_texture_id))
}

fn handle_thumbnail_file_path_dialog(
  mut events: EventReader<DialogFilePicked<TitlecardFilePathDialog>>,
  mut egui_user_textures: ResMut<EguiUserTextures>,
  mut editor_state: NonSendMut<EditorState>,
  mut images: ResMut<Assets<Image>>,
  mut titlecard_updated_events: EventWriter<TitlecardUpdatedEvent>,
  mut titlecard_state: ResMut<crate::editor::TitlecardState>
) {
  for ev in events.read() {
    let load_result = load_titlecard_image(&ev.path, images.as_mut(), egui_user_textures.as_mut(), editor_state.as_mut());
    let Some((image_handle, egui_texture_id)) = load_result else {
      return;
    };

    titlecard_state.titlecard_image = Some(image_handle);
    titlecard_state.titlecard_egui_tex_id = Some(egui_texture_id);
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
  mut commands: Commands
) {
  if ui.button("New...").clicked() {
    new_project_event_writer.send_default();
  }
  if ui.button("Open...").clicked() {
    commands.dialog().add_filter("YoteOke Lyric Editor Project", &["yoke"]).load_file::<crate::project::OpenProjectDialog>();
  }
  if ui.button("Save").clicked() {
    save_requested_event_writer.send_default();
  }
  if ui.button("Save As...").clicked() {
    if let Some(project_data) = editor_state.project_data.as_ref() {
      let serialized = serde_json::to_vec_pretty(project_data).unwrap();
      commands.dialog().add_filter("YoteOke Lyric Editor Project", &["yoke"]).save_file::<crate::project::SaveAsDialog>(serialized);
    }
  }
}

pub fn project_menu_ui(mut ui: InMut<egui::Ui>,
  mut project_settings_dialog: ResMut<ProjectSettingsDialog>,
  mut commands: Commands
) {
  if ui.button("Project Settings...").clicked() {
    project_settings_dialog.open();
  }
  if ui.button("Export...").clicked() {
    commands.dialog().add_filter("Video file", &["mp4"]).save_file::<crate::export::ExportFilePathDialog>(Vec::new());
  }
}

#[derive(Event, Default)]
pub struct SaveProjectRequestedEvent;

#[derive(Default, Resource)]
pub struct ProjectSettingsDialog {
  is_open: bool,
}

impl ProjectSettingsDialog {
  pub fn open(&mut self) {
    self.is_open = true;
  }
}

pub fn project_settings_dialog_ui(world: &mut World) {
  let ctx;
  let is_open;
  let mut properties;
  {
    let mut system_state: SystemState<(
      EguiContexts, Res<ProjectSettingsDialog>, NonSend<EditorState>)> 
      = SystemState::new(world);
    let (mut egui_contexts, project_settings_dialog, editor_state) = system_state.get_mut(world);
    ctx = egui_contexts.ctx_mut().clone();
    is_open = project_settings_dialog.is_open;
    if let Some(project_data) = &editor_state.project_data {
      properties = ProjectSettingsProperties::from_project_data(project_data)
    } else {
      return;
    }
  }
  
  if is_open {
    egui::Window::new("Project Settings").show(&ctx, |ui| {
      let changed = bevy_inspector_egui::bevy_inspector::ui_for_value(&mut properties, ui, world);
      if changed {
        let mut editor_state = world.get_non_send_resource_mut::<EditorState>().unwrap();
        properties.update_project_data(editor_state.project_data.as_mut().unwrap());
        editor_state.needs_save_before_exit = true;
      }
    });
  }
}

#[derive(Default, Event)]
struct SaveAsRequestedEvent;

#[derive(Reflect)]
struct ProjectSettingsProperties {
  pub background_color: Color,
  pub unsung_color: Color,
  pub sung_color: Color,
  pub titlecard_show_time: f32,
  pub song_delay_time: f32,
  pub titlecard_path: TitlecardPath,
  pub song_path: SongFilePath
}

impl ProjectSettingsProperties {
  fn from_project_data(project_data: &ProjectData) -> Self {
    Self {
      background_color: project_data.background_color.unwrap_or(Color::default()),
      unsung_color: project_data.unsung_color.unwrap_or(Color::default()),
      sung_color: project_data.sung_color.unwrap_or(Color::default()),
      titlecard_show_time: project_data.titlecard_show_time.unwrap_or_default(),
      song_delay_time: project_data.song_delay_time.unwrap_or_default(),
      titlecard_path: TitlecardPath(project_data.thumbnail_path.clone()),
      song_path: SongFilePath(project_data.song_file.clone())
    }
  }

  fn update_project_data(&self, project_data: &mut ProjectData) {
    project_data.background_color = Some(self.background_color);
    project_data.unsung_color = Some(self.unsung_color);
    project_data.sung_color = Some(self.sung_color);
    project_data.titlecard_show_time = Some(self.titlecard_show_time);
    project_data.song_delay_time = Some(self.song_delay_time);
    project_data.thumbnail_path = self.titlecard_path.0.clone();
  }
}

#[derive(Reflect, Clone)]
struct TitlecardPath(Option<PathBuf>);

impl InspectorPrimitive for TitlecardPath {
  fn ui(
    &mut self,
    ui: &mut egui::Ui,
    options: &dyn std::any::Any,
    id: egui::Id,
    mut env: bevy_inspector_egui::reflect_inspector::InspectorUi<'_, '_>,
  ) -> bool {
    self.ui_readonly(ui, options, id, env.reborrow());

    if ui.button("Set").clicked() {
      env.context.queue.as_mut().unwrap().push(|world: &mut World| {
        world.commands().dialog().add_filter("Image file", &["png", "jpeg", "jpg", "bmp", "tga", "tiff", "webp"]).pick_file_path::<TitlecardFilePathDialog>();
      });
    }
    false
  }

  fn ui_readonly(
    &self,
    ui: &mut egui::Ui,
    options: &dyn std::any::Any,
    id: egui::Id,
    env: bevy_inspector_egui::reflect_inspector::InspectorUi<'_, '_>,
  ) {
    let tex_id = env.context.world.as_mut().unwrap()
      .get_resource_mut::<crate::editor::TitlecardState>()
      .unwrap().titlecard_egui_tex_id;
    if let Some(tex_id) = tex_id {
      ui.image(egui::ImageSource::Texture(SizedTexture::new(tex_id, [128., 72.])));
    }
  }
}

#[derive(Reflect, Clone)]
struct SongFilePath(Option<PathBuf>);

impl InspectorPrimitive for SongFilePath {
  fn ui(&mut self, ui: &mut egui::Ui, options: &dyn std::any::Any, id: egui::Id,
    mut env: InspectorUi<'_, '_>) -> bool 
  {
    self.ui_readonly(ui, options, id, env.reborrow());

    if ui.button("Set").clicked() {
      env.context.queue.as_mut().unwrap().push(|world: &mut World| {
        world.commands().dialog().add_filter("Audio file", &["mp3", "wav", "ogg", "flac"]).pick_file_path::<SongFilePathDialog>();
      });
    }

    false
  }

  fn ui_readonly(&self, ui: &mut egui::Ui, options: &dyn std::any::Any,
    id: egui::Id, env: InspectorUi<'_, '_>) 
  {
    if self.0.is_some() {
      ui.label(self.0.as_ref().unwrap().as_os_str().to_string_lossy());
    } else {
      ui.label("None");
    }
  }
}

struct SongFilePathDialog;

fn handle_song_file_path_dialog(
  mut events: EventReader<DialogFilePicked<SongFilePathDialog>>,
  mut audio_state: NonSendMut<AudioState>,
  mut editor_state: NonSendMut<EditorState>
) {
  for ev in events.read() {
    if let Some(project_data) = &mut editor_state.project_data {
      project_data.song_file = Some(ev.path.clone());
      if let Some(music_handle) = &mut audio_state.music_handle {
        music_handle.pause(Tween::default());
      }
      audio_state.music_handle = None;
    }
  }
}