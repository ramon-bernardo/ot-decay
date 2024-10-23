use bevy::prelude::*;
use ot_decay::prelude::*;
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, DecayPlugin))
        .add_systems(Startup, startup)
        .add_observer(handle_completed_decay)
        .run();
}

#[derive(Component)]
struct Item;

fn startup(mut commands: Commands) {
    for min in 0..15 {
        for max in 15..30 {
            let min = Duration::from_secs(min);
            let max = Duration::from_secs(max);
            commands.spawn((Item, Decay, DecayDuration::new(min, max)));
        }
    }
}

fn handle_completed_decay(trigger: Trigger<DecayCompleted>, mut commands: Commands) {
    for entity in trigger.iter() {
        commands.entity(*entity).despawn_recursive();
    }
}
