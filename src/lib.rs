use bevy::prelude::*;
use rand::Rng;
use std::time::Duration;

pub mod prelude {
    pub use super::{
        Decay, DecayCompleted, DecayDuration, DecayPaused, DecayPlugin, DecayStarted, DecayingSet,
    };
}

/// Predefined set for systems handling decaying entities.
///
/// This system set groups together systems that operate on entities with the `Decay` component.
/// It is typically used to control the execution order of decay-related systems, ensuring
/// that decay processes are updated consistently each frame.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DecayingSet;

/// Plugin that manages the decay system, allowing entities to decay over time.
///
/// The `DecayPlugin` is responsible for setting up the necessary systems and events
/// for managing the decay lifecycle of entities. It adds systems to handle starting,
/// pausing, and completing the decay process, as well as managing the timers associated
/// with decaying entities.
pub struct DecayPlugin;

impl Plugin for DecayPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(PreUpdate, DecayingSet);

        app.add_event::<DecayStarted>()
            .add_event::<DecayPaused>()
            .add_event::<DecayCompleted>();

        app.add_systems(PreUpdate, decaying.in_set(DecayingSet));

        app.add_observer(handle_decay_start)
            .add_observer(handle_decay_pause);
    }
}

/// Marker component for entities that should decay over time.
///
/// This component indicates that an entity is subject to a decay process.
/// It is used in conjunction with the `DecayDuration` component to manage
/// the lifecycle of decaying entities within the world.
#[derive(Component, Debug)]
#[require(DecayDuration)]
pub struct Decay;

/// Stores the duration for decay, with a minimum and maximum time range.
///
/// This component is utilized by the `Decay` component to define the range within which the
/// entity will decay. The actual decay duration is randomized between the specified `min`
/// and `max` values to introduce variability in decay times.
#[derive(Component, Default, Debug)]
pub struct DecayDuration {
    /// The minimum duration for decay.
    min: Duration,
    /// The maximum duration for decay.
    max: Duration,
}

impl DecayDuration {
    /// Creates a new `DecayDuration` with the specified minimum and maximum durations.
    ///
    /// If the provided `min` duration is greater than the `max` duration,
    /// the values are swapped to ensure valid range.
    pub fn new(min: Duration, max: Duration) -> Self {
        if min > max {
            Self { min: max, max: min }
        } else {
            Self { min, max }
        }
    }

    /// Checks if the decay duration is effectively zero.
    ///
    /// Returns `true` if both `min` and `max` durations are zero; otherwise, `false`.
    pub fn is_zero(&self) -> bool {
        self.min == Duration::ZERO && self.max == Duration::ZERO
    }

    /// Generates a random duration within the `min` to `max` range.
    ///
    /// This method utilizes a random number generator to select a duration between the
    /// minimum and maximum values, inclusive. The resulting duration is suitable for
    /// initiating the decay process for an entity.
    pub fn random_duration(&self) -> Duration {
        let mut rng = rand::thread_rng();
        let random_millis = rng.gen_range(self.min.as_millis()..=self.max.as_millis());
        return Duration::from_millis(random_millis as u64);
    }
}

/// A timer component used for counting down the decay time.
///
/// When attached to an entity, this timer counts down and signals when the decay is complete.
#[derive(Component, Default, Deref, DerefMut, Debug)]
struct DecayTimer(Timer);

impl DecayTimer {
    /// Creates a new `DecayTimer` with the given duration. The timer is set to run once.
    pub fn new(duration: Duration) -> Self {
        Self(Timer::new(duration, TimerMode::Once))
    }
}

/// Event triggered when the decay process starts for an entity.
///
/// This event is dispatched when an entity with a `Decay` component begins the decay process.
/// It provides the entity that is decaying and the duration for which the decay will last.
#[derive(Event)]
pub struct DecayStarted {
    /// The entity that has started decaying.
    pub entity: Entity,
    /// The duration for which the entity will decay.
    pub duration: Duration,
}

/// Event triggered when the decay process is paused for an entity.
///
/// This event is fired when an entity with an active decay timer has its decay paused.
/// It includes the entity and the remaining duration of the decay at the time of pausing.
#[derive(Event)]
pub struct DecayPaused {
    /// The entity that has paused its decay process.
    pub entity: Entity,
    /// The remaining duration of decay when the process was paused.
    pub remaining_duration: Duration,
}

/// Event triggered when the decay process is completed for an entity.
///
/// This event is sent when an entity's decay timer has finished and the decay process is complete.
/// The event contains the entities that have completed their decay.
#[derive(Event, Deref, DerefMut)]
pub struct DecayCompleted(pub Vec<Entity>);

/// System that handles the initiation of decay for entities when the `Decay` component is added.
fn handle_decay_start(
    trigger: Trigger<OnAdd, Decay>,
    mut commands: Commands,
    mut query: Query<(Entity, &DecayDuration, Option<&mut DecayTimer>)>,
) {
    let Ok((entity, duration, timer)) = query.get_mut(trigger.entity()) else {
        return;
    };

    // If the decay duration is zero, remove the `Decay` and `DecayTimer` components immediately.
    if duration.is_zero() {
        commands
            .entity(entity)
            .remove::<Decay>()
            .remove::<DecayTimer>();
    }
    // If a timer already exists, unpause it.
    else if let Some(mut timer) = timer {
        timer.unpause();

        // Trigger the `DecayStarted` event with the remaining duration.
        commands.trigger(DecayStarted {
            entity,
            duration: timer.remaining(),
        });
    }
    // If no timer exists, create a new timer with a random duration and start the decay process.
    else {
        let duration = duration.random_duration();
        commands.entity(entity).insert(DecayTimer::new(duration));

        // Trigger the `DecayStarted` event with the duration.
        commands.trigger(DecayStarted { entity, duration });
    }
}

/// System that handles pausing decay for entities when the `Decay` component is removed.
fn handle_decay_pause(
    trigger: Trigger<OnRemove, Decay>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut DecayTimer)>,
) {
    if let Ok((entity, mut timer)) = query.get_mut(trigger.entity()) {
        // Pause the decay timer for the entity.
        timer.pause();

        // Send a `DecayPaused` event, including the remaining duration.
        commands.trigger(DecayPaused {
            entity,
            remaining_duration: timer.remaining(),
        });
    }
}

/// System that processes decaying entities by ticking their timers.
fn decaying(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut DecayTimer), With<Decay>>,
) {
    let mut decayed_entities = vec![];

    for (entity, mut timer) in query.iter_mut() {
        // Progress the decay timer based on the time elapsed since the last frame.
        timer.tick(time.delta());

        // If the timer has completed its countdown...
        if timer.finished() {
            // Remove the `Decay` and `DecayTimer` components from the entity.
            commands
                .entity(entity)
                .remove::<Decay>()
                .remove::<DecayTimer>();

            // Collect the entity for triggering...
            decayed_entities.push(entity);
        }
    }

    // If any entities have completed decaying, trigger the DecayCompleted event.
    if !decayed_entities.is_empty() {
        commands.trigger(DecayCompleted(decayed_entities));
    }
}
