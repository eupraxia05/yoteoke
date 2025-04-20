//! A texture for showing the view of a camera. Analogous to Godot's SubViewport.

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDescriptor, 
    TextureDimension, TextureFormat, TextureUsages};
use bevy::render::view::RenderLayers;
use bevy::render::camera::RenderTarget;
use bevy_egui::EguiUserTextures;
use bevy_egui::egui;

pub fn build(app: &mut App) {
  app.add_systems(Update, SubViewport::setup_added);
  app.add_systems(Update, SubViewport::update_clear_color);
}

// The texture storing the camera's viewport, with an associated texture ID with egui.
#[derive(Component)]
pub struct SubViewport {
  render_layers: RenderLayers,
  image: Option<Handle<Image>>,
  egui_texture_id: Option<egui::TextureId>,
  camera_ent: Option<Entity>,
  pub clear_color: ClearColorConfig,
}

#[derive(Component)]
struct SubViewportCamera;

impl SubViewport {
  pub fn new(render_layers: RenderLayers) -> Self {
    Self {
      render_layers,
      image: None,
      egui_texture_id: None,
      camera_ent: None,
      clear_color: ClearColorConfig::Default
    }
  }
}

impl SubViewport {
  fn setup_added(mut commands: Commands, 
    mut cam_to_tex_query: Query<&mut SubViewport, Added<SubViewport>>,
    mut egui_user_textures: ResMut<EguiUserTextures>, 
    mut images: ResMut<Assets<Image>>) 
  {
    for mut cam_to_tex in cam_to_tex_query.iter_mut() {
      if cam_to_tex.image.is_none() {
        info!("initializing cam");
        let size = Extent3d {
          width: 1920,
          height: 1080,
          ..default()
        };
        
        let mut image = Image {
          texture_descriptor: TextureDescriptor {
              label: None,
              size,
              dimension: TextureDimension::D2,
              format: TextureFormat::Rgba8UnormSrgb,
              mip_level_count: 1,
              sample_count: 1,
              usage: TextureUsages::TEXTURE_BINDING
                  | TextureUsages::COPY_DST
                  | TextureUsages::COPY_SRC
                  | TextureUsages::RENDER_ATTACHMENT,
              view_formats: &[],
          },
          ..default()
        };
        
        image.resize(size);
        
        let image_handle = images.add(image);
        let egui_texture_id = egui_user_textures.add_image(image_handle.clone());

        cam_to_tex.image = Some(image_handle.clone());
        cam_to_tex.egui_texture_id = Some(egui_texture_id);

        cam_to_tex.camera_ent = Some(
          commands.spawn(
            (
              SubViewportCamera,
              Camera2d {
                ..default()
              }, 
              Camera {
                target: RenderTarget::Image(image_handle),
                clear_color: ClearColorConfig::Default,
                ..default()
              }, 
              cam_to_tex.render_layers.clone()
            )
          ).id()
        );
      }
    }
  }

  fn update_clear_color(camera_tex_query: Query<&SubViewport>,
    mut camera_tex_camera_query: Query<(&SubViewportCamera, &mut Camera)>
  ) {
    for camera_tex in camera_tex_query.iter() {
      if let Some(camera_ent) = camera_tex.camera_ent {
        if let Ok(mut camera) = camera_tex_camera_query.get_mut(camera_ent) {
          camera.1.clear_color = camera_tex.clear_color.clone();
        }
      }
    }
  }

  pub fn show(&self, ui: &mut egui::Ui) {
    if let Some(egui_texture_id) = &self.egui_texture_id {
      let available_size = ui.available_size();
      ui.image(egui::load::SizedTexture::new(
        *egui_texture_id,
        egui::vec2(
            available_size.x,
            available_size.x * 1080. / 1920.
        )
      ));
    }
  }

  pub fn image_handle(&self) -> Handle<Image> {
    self.image.as_ref().unwrap().clone_weak()
  }
}

