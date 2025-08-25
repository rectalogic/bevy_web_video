use std::marker::PhantomData;

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use crossbeam_channel::unbounded;

use crate::registry::{Registry, RegistryId, VideoElement};

pub trait EventListenerApp {
    fn add_listener_event<E: EventType>(&mut self) -> &mut Self;
}

impl EventListenerApp for App {
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
    pub fn tx(&self) -> crossbeam_channel::Sender<ListenerEvent<E>> {
        self.0.clone()
    }
}

#[derive(Resource)]
pub struct EventReceiver<E: EventType>(crossbeam_channel::Receiver<ListenerEvent<E>>);

impl<E: EventType> EventReceiver<E> {
    pub fn rx(&self) -> crossbeam_channel::Receiver<ListenerEvent<E>> {
        self.0.clone()
    }
}

#[derive(Event, Copy, Clone)]
pub struct ListenerEvent<E: EventType> {
    registry_id: RegistryId,
    target: Option<Entity>,
    _phantom: PhantomData<E>,
}

impl<E: EventType> ListenerEvent<E> {
    pub(crate) fn new(registry_id: RegistryId, target: Option<Entity>) -> Self {
        Self {
            registry_id,
            target,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn registry_id(&self) -> RegistryId {
        self.registry_id
    }

    pub fn video_element(&self) -> Option<web_sys::HtmlVideoElement> {
        Registry::with_borrow(|registry| registry.get(&self.registry_id).map(|e| e.element()))
    }
}

pub trait EventType: Copy + Clone + Send + Sync + 'static {
    const EVENT_NAME: &'static str;
}

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

fn listen_for_events<E: EventType>(receiver: Res<EventReceiver<E>>, mut commands: Commands) {
    while let Ok(event) = receiver.0.try_recv() {
        if let Some(target) = event.target {
            commands.trigger_targets(event, target);
        } else {
            commands.trigger(event);
        }
    }
}

fn resize_image(video_element: &VideoElement, images: &mut Assets<Image>) {
    let mut image = Image::new_uninit(
        Extent3d {
            width: video_element.element().video_width(),
            height: video_element.element().video_height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    images.insert(video_element.target_texture_id(), image);
}

fn on_loaded_metadata(
    trigger: Trigger<ListenerEvent<LoadedMetadata>>,
    mut images: ResMut<Assets<Image>>,
) {
    Registry::with_borrow(|registry| {
        if let Some(video_element) = registry.get(&trigger.registry_id()) {
            resize_image(video_element, &mut images);
        }
    });
}

fn on_resize(trigger: Trigger<ListenerEvent<Resize>>, mut images: ResMut<Assets<Image>>) {
    Registry::with_borrow(|registry| {
        if let Some(video_element) = registry.get(&trigger.registry_id()) {
            resize_image(video_element, &mut images);
        }
    });
}

fn on_error(trigger: Trigger<ListenerEvent<Error>>) {
    let video = trigger.video_element();
    warn!("Video {video:?} failed with error");
}

fn on_playing(trigger: Trigger<ListenerEvent<Playing>>) {
    Registry::with_borrow_mut(|registry| {
        if let Some(video_element) = registry.get_mut(&trigger.registry_id()) {
            video_element.set_loaded();
        }
    });
}
