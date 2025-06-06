use bevy::prelude::*;
use std::time::Duration;

use crate::editor::EditorState;
use crate::export::ExportState;
use bevy::render::view::RenderLayers;
use crate::SubViewport;
use bevy_egui::egui;

pub struct StagePlugin;

impl Plugin for StagePlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Startup, startup)
      .add_systems(Update, (cleanup_preview, update_preview).chain())
      .add_event::<TitlecardUpdatedEvent>()
      .add_systems(Update, handle_titlecard_updated);
  }
}

#[derive(Component)]
struct PreviewText;

#[derive(Component)]
struct LineText;

fn startup(mut commands: Commands) {
  // create a subviewport for the video preview
  commands.spawn(SubViewport::new(RenderLayers::layer(1)));

  commands.spawn((Sprite::from_color(Color::NONE, [1920., 1080.].into()), 
    RenderLayers::layer(1), TitlecardStageSprite, Transform::from_translation([0., 0., 1.].into())));
}

fn cleanup_preview(world: &mut World) {
  let line_texts = world.query_filtered::<Entity, With<LineText>>().iter(&world).collect::<Vec<_>>();
  for entity in line_texts.iter() {
    world.despawn(*entity);
  }

  let preview_texts = world.query_filtered::<Entity, With<PreviewText>>().iter(&world).collect::<Vec<_>>();
  for entity in preview_texts.iter() {
    world.despawn(*entity);
  }
}

fn update_preview(editor_state: NonSend<EditorState>,
  export_state: Res<ExportState>,
  mut commands: Commands,
  mut titlecard_stage_sprite_query: Query<&mut Sprite, With<TitlecardStageSprite>>,
  mut camera_tex_query: Query<&mut SubViewport>,
  titlecard_state: Res<crate::editor::TitlecardState>,
  mut audio_state: NonSendMut<crate::editor::AudioState>
)
{
  let mut pre_delay_time = 0.;
  if let Some(project_data) = &editor_state.project_data {
    camera_tex_query.single_mut().clear_color = ClearColorConfig::Custom(project_data.background_color.unwrap_or_default());
    pre_delay_time = project_data.song_delay_time.unwrap();
  }

  let song_position = if export_state.is_exporting() {
    Duration::from_secs_f64((export_state.frame_idx() as f64 / 12. - pre_delay_time as f64).max(0.))
  } else {
    audio_state.playhead_position()
  };

  let mut text: String = "".into();
  let mut chars_sung: usize = 0;
  if let Some(lyrics) = editor_state.parsed_lyrics.as_ref() {
    if let Some(block) = lyrics.get_block_at_time(&song_position, &Duration::from_secs(3)) {
      text = block.lyrics.clone();

      if let Some((ts1, ts2)) = block.get_timestamps_surrounding(&song_position) {
        if ts1.position <= ts2.position && ts1.time < ts2.time {
          let elapsed_in_syl = song_position - ts1.time;
          let total_syl_time = ts2.time - ts1.time;
          let chars_in_syl = ts2.position - ts1.position;
          let amount_sung = elapsed_in_syl.as_secs_f64() / total_syl_time.as_secs_f64();
          chars_sung = (amount_sung * chars_in_syl as f64) as usize + ts1.position;
        } else {
          warn!("non-sequential timestamps: {:?}, {:?}", ts1, ts2);
          chars_sung = 0
        }
      } else {
        chars_sung = 0
      }
    }
  }

  if text.len() > 2 && text.len() > chars_sung {
    let preview_text_ent = commands.spawn(
      (
        Text2d::default(), 
        TextLayout::new_with_justify(JustifyText::Center),
        RenderLayers::layer(1), 
        PreviewText
      )
    ).id();

    commands
      .spawn(
        (
          TextSpan::new(&text[0..chars_sung]),
          TextFont {
            font_size: 64.0,
            ..Default::default()
          }, 
          TextColor(
            editor_state.project_data.as_ref().unwrap().sung_color.unwrap_or_default()
          ), 
          LineText
        )
      )
      .set_parent(preview_text_ent);

    commands
      .spawn(
        (
          TextSpan::new(String::from(&text[chars_sung..]) + "\n"),
          TextFont {
            font_size: 64.0,
            
            ..Default::default()
          }, 
          TextColor(
            editor_state.project_data.as_ref().unwrap().unsung_color.unwrap_or_default()
          ), 
          LineText
        )
      )
      .set_parent(preview_text_ent);
  }

  let mut titlecard_stage_sprite_alpha = 0.0;
  if titlecard_state.titlecard_image.is_some() {
    let curr_pre_delay_time = if export_state.is_exporting {
      (export_state.frame_idx() as f32 / 12.).clamp(0., pre_delay_time)
    } else {
      editor_state.curr_pre_delay_time as f32
    };
    titlecard_stage_sprite_alpha = (editor_state.project_data.as_ref().unwrap().titlecard_show_time.unwrap() 
      - song_position.as_secs_f32() 
      - curr_pre_delay_time).clamp(0.0, 1.0);
  }

  let titlecard_stage_sprite_color = Color::srgba(1.0, 1.0, 1.0, titlecard_stage_sprite_alpha);

  titlecard_stage_sprite_query.single_mut().color = titlecard_stage_sprite_color;
}

#[derive(Component)]
struct TitlecardStageSprite;

#[derive(Event, Default)]
pub struct TitlecardUpdatedEvent;

fn handle_titlecard_updated(mut events: EventReader<TitlecardUpdatedEvent>, 
  mut titlecard_stage_sprite_query: Query<&mut Sprite, With<TitlecardStageSprite>>,
  editor_state: NonSend<EditorState>,
  titlecard_state: Res<crate::editor::TitlecardState>
) {
  for _ in events.read() {
    let mut sprite = titlecard_stage_sprite_query.single_mut();
    sprite.image = 
      if titlecard_state.titlecard_image.is_some() {
        sprite.color = Color::WHITE;
        titlecard_state.titlecard_image.as_ref().unwrap().clone_weak()
      } else {
        sprite.color = Color::NONE;
        Handle::default()
      }
  }
}

pub fn preview_ui(mut ui: InMut<egui::Ui>, camera_tex_query: Query<&SubViewport>, export_state: Res<ExportState>) 
{
  egui::TopBottomPanel::top("preview_header").show_inside(&mut ui, |ui| {
    if export_state.is_exporting() {
      ui.label("Exporting...");
    }
  });
  egui::CentralPanel::default().show_inside(&mut ui, |ui| {
    camera_tex_query.single().show(ui);
  });
}