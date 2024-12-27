use bevy::prelude::*;

pub fn build(app: &mut App) {
  app.add_systems(Startup, startup);
  app.insert_resource(ExportState::default());
  app.add_event::<ExportInitiatedEvent>();
  app.add_systems(Update, handle_export_initiated);
  app.add_systems(Update, update_export);
}

fn startup() {

}

#[derive(Default, Resource, Debug)]
pub struct ExportState {
  is_exporting: bool,
  frame_idx: usize
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

fn handle_export_initiated(
  mut event_reader: EventReader<ExportInitiatedEvent>,
  mut export_state: ResMut<ExportState>) 
{
  for ev in event_reader.read() {
    if !export_state.is_exporting {
      export_state.is_exporting = true;
    }
  }
}

fn update_export(mut export_state: ResMut<ExportState>) {
  export_state.frame_idx += 1;
}