use crate::{VideoElement, VideoElementRegistry};
use bevy::{ecs::system::IntoObserverSystem, prelude::*};
use crossbeam_channel::unbounded;
use gloo_events::EventListener;
use std::marker::PhantomData;

pub fn plugin(app: &mut App) {
    app.add_listener_event::<events::LoadedMetadata>()
        .add_listener_event::<events::Resize>()
        .add_listener_event::<events::Playing>()
        .add_listener_event::<events::Error>();
}

pub trait EventListenerAppExt {
    fn add_listener_event<E: EventType>(&mut self) -> &mut Self;
}

impl EventListenerAppExt for App {
    fn add_listener_event<E: EventType>(&mut self) -> &mut Self {
        // Check if already initialized
        if self.world().contains_resource::<Events<ListenerEvent<E>>>() {
            return self;
        }
        let (tx, rx) = unbounded();
        self.add_event::<ListenerEvent<E>>()
            .insert_resource(EventSender::<E>(tx))
            .insert_resource(EventReceiver::<E>(rx))
            .add_systems(Update, listen_for_events::<E>)
    }
}

#[derive(Resource)]
pub struct EventSender<E: EventType>(crossbeam_channel::Sender<ListenerEvent<E>>);

impl<E: EventType> EventSender<E> {
    pub fn add_video_event_listener<'o, B, M, O>(
        &self,
        asset_id: impl Into<AssetId<VideoElement>>,
        element: &web_sys::EventTarget,
        registry: &mut VideoElementRegistry,
        observable: &'o mut O,
        observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>,
    ) -> &'o mut O
    where
        B: Bundle,
        O: ObservableEntity<E, B, M>,
    {
        self.add_listener(asset_id, element, registry, Some(observable.id()));
        observable.observe(observer)
    }

    pub(crate) fn add_video_event_listener_internal(
        &self,
        asset_id: impl Into<AssetId<VideoElement>>,
        element: &web_sys::EventTarget,
        registry: &mut VideoElementRegistry,
    ) {
        self.add_listener(asset_id, element, registry, None);
    }

    fn add_listener(
        &self,
        asset_id: impl Into<AssetId<VideoElement>>,
        element: &web_sys::EventTarget,
        registry: &mut VideoElementRegistry,
        target: Option<Entity>,
    ) {
        let tx = self.0.clone();
        let asset_id = asset_id.into();
        let listener =
            EventListener::new(element, E::EVENT_NAME, move |_event: &web_sys::Event| {
                if let Err(err) = tx.send(ListenerEvent::<E>::new(asset_id, target)) {
                    warn!("Failed to fire video event {}: {err:?}", E::EVENT_NAME);
                };
            });
        registry.add_event_listener(asset_id, listener);
    }
}

#[derive(Resource)]
struct EventReceiver<E: EventType>(crossbeam_channel::Receiver<ListenerEvent<E>>);

#[derive(Event, Copy, Clone)]
pub struct ListenerEvent<E: EventType> {
    asset_id: AssetId<VideoElement>,
    target: Option<Entity>,
    _phantom: PhantomData<E>,
}

impl<E: EventType> ListenerEvent<E> {
    pub(crate) fn new(asset_id: AssetId<VideoElement>, target: Option<Entity>) -> Self {
        Self {
            asset_id,
            target,
            _phantom: PhantomData,
        }
    }
}

pub trait EventWithVideoElementId {
    fn asset_id(&self) -> AssetId<VideoElement>;
}

impl<E: EventType> EventWithVideoElementId for ListenerEvent<E> {
    fn asset_id(&self) -> AssetId<VideoElement> {
        self.asset_id
    }
}

pub trait EventType: Copy + Clone + Send + Sync + 'static {
    const EVENT_NAME: &'static str;
}

pub mod events {
    use super::*;

    #[macro_export]
    macro_rules! new_event_type {
        ($name:ident, $event_name:literal) => {
            #[derive(Event, Copy, Clone, Debug)]
            pub struct $name;

            impl EventType for $name {
                const EVENT_NAME: &'static str = $event_name;
            }
        };
    }

    new_event_type!(LoadedMetadata, "loadedmetadata");
    new_event_type!(Resize, "resize");
    new_event_type!(Playing, "playing");
    new_event_type!(Error, "error");
}

fn listen_for_events<E: EventType>(receiver: Res<EventReceiver<E>>, mut commands: Commands) {
    while let Ok(event) = receiver.0.try_recv() {
        if let Some(target) = event.target {
            commands.trigger_targets(event, target);
        } else {
            commands.trigger(event);
        }
    }
}

pub trait ObservableEntity<E, B, M>
where
    E: EventType,
    B: Bundle,
{
    fn id(&self) -> Entity;
    fn observe(&mut self, observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>) -> &mut Self;
}

impl<E, B, M> ObservableEntity<E, B, M> for EntityWorldMut<'_>
where
    E: EventType,
    B: Bundle,
{
    fn id(&self) -> Entity {
        self.id()
    }

    fn observe(&mut self, observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>) -> &mut Self {
        self.observe(observer)
    }
}

impl<E, B, M> ObservableEntity<E, B, M> for EntityCommands<'_>
where
    E: EventType,
    B: Bundle,
{
    fn id(&self) -> Entity {
        self.id()
    }

    fn observe(&mut self, observer: impl IntoObserverSystem<ListenerEvent<E>, B, M>) -> &mut Self {
        self.observe(observer)
    }
}
