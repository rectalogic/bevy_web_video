use atrium_api::{
    client::AtpServiceClient,
    types::{LimitedNonZeroU8, Union},
};
use atrium_xrpc_client::reqwest::ReqwestClient;
use bevy::{asset::AssetEvents, ecs::entity_disabling::Disabled, prelude::*, tasks::IoTaskPool};
use bevy_web_video::{
    EventListenerAppExt, EventSender, ListenerEvent, VideoElement, VideoElementAssetsExt,
    VideoElementRegistry, WebVideo, WebVideoError, events, new_event_type,
};

const DISTANCE: f32 = 5.0;

pub fn plugin(app: &mut App) {
    app.add_listener_event::<TimeUpdate>()
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .add_systems(PostUpdate, handle_image_resize.after(AssetEvents));
}

new_event_type!(TimeUpdate, "timeupdate");

#[derive(Debug)]
struct Video {
    url: String,
    aspect_ratio: f32,
}

#[derive(Resource, Deref)]
struct VideoReceiver(async_channel::Receiver<Video>);

#[derive(Component)]
struct InitialPosition(Vec3);

#[derive(Component)]
struct VideoImage(Handle<Image>);

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
    mut video_elements: ResMut<Assets<VideoElement>>,
    ended_event_sender: Res<EventSender<events::Ended>>,
    timeupdate_event_sender: Res<EventSender<TimeUpdate>>,
    mut registry: NonSendMut<VideoElementRegistry>,
) {
    let (tx, rx) = async_channel::bounded(5);
    commands.insert_resource(VideoReceiver(rx));
    IoTaskPool::get()
        .spawn(async move {
            send_videos(tx).await;
        })
        .detach();

    commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, DISTANCE)));

    let plane = meshes.add(Plane3d::new(Vec3::Z, Vec2::splat(0.5)));
    for pos in [
        Vec3::new(-0.5, -0.5, 0.0),
        Vec3::new(-0.5, 0.5, 0.0),
        Vec3::new(0.5, 0.5, 0.0),
        Vec3::new(0.5, -0.5, 0.0),
    ] {
        let video_image = images.reserve_handle();
        let (video_element_handle, element) = video_elements.new_video(&video_image, &mut registry);
        let video_asset_id = video_element_handle.id();
        let video_entity = commands
            .spawn((
                Disabled,
                InitialPosition(pos),
                WebVideo::new(video_element_handle),
                VideoImage(video_image.clone()),
                Mesh3d(plane.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color_texture: Some(video_image),
                    unlit: true,
                    ..default()
                })),
                Transform::from_translation(pos),
            ))
            .observe(ended_observer)
            .observe(timeupdate_observer)
            .id();
        element.set_cross_origin(Some("anonymous"));
        element.set_muted(true);
        ended_event_sender.enable_element_event_observers(
            video_asset_id,
            &element,
            &mut registry,
            video_entity,
        );
        timeupdate_event_sender.enable_element_event_observers(
            video_asset_id,
            &element,
            &mut registry,
            video_entity,
        );
    }
}

async fn send_videos(tx: async_channel::Sender<Video>) {
    let client = AtpServiceClient::new(ReqwestClient::new("https://public.api.bsky.app"));
    let mut cursor = None;
    loop {
        let result = client
             .service
             .app
             .bsky
             .feed
             .get_feed(
                 // Official bsky videos feed requires auth at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.generator/thevids
                 atrium_api::app::bsky::feed::get_feed::ParametersData {
                     feed: "at://did:plc:6i6n57nrkq6xavqbdo6bvkqr/app.bsky.feed.generator/trending-vids"
                         .into(),
                     limit: Some(LimitedNonZeroU8::try_from(5u8).unwrap()),
                     cursor,
                 }
                 .into(),
             )
             .await;
        match result {
            Ok(feed) => {
                cursor = feed.cursor.clone();
                for f in feed.feed.iter() {
                    if let Some(Union::Refs(ref embed)) = f.post.embed
                         && let atrium_api::app::bsky::feed::defs::PostViewEmbedRefs::AppBskyEmbedVideoView(
                             video,
                         ) = embed
                         && let Some(ref aspect_ratio) = video.aspect_ratio
                         && tx.send( Video { url: video.playlist.clone(), aspect_ratio: aspect_ratio.width.get() as f32 / aspect_ratio.height.get() as f32 }).await.is_err() {
                             error!("Error sending video url");
                             return;
                     }
                }
            }
            Err(e) => {
                error!("Error fetching feed: {}", e);
                return;
            }
        }
    }
}

fn handle_image_resize(
    mut events: EventReader<AssetEvent<Image>>,
    videos: Query<(&MeshMaterial3d<StandardMaterial>, &VideoImage)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for event in events.read() {
        if let AssetEvent::Modified { id: asset_id } = *event {
            for (MeshMaterial3d(material_handle), VideoImage(image_handle)) in videos {
                if asset_id == image_handle.id() {
                    materials.get_mut(material_handle);
                }
            }
        }
    }
}

fn ended_observer(trigger: Trigger<ListenerEvent<events::Ended>>, mut commands: Commands) {
    commands.entity(trigger.target()).insert(Disabled);
}

fn timeupdate_observer(
    trigger: Trigger<ListenerEvent<TimeUpdate>>,
    mut web_videos: Query<&mut Transform, With<WebVideo>>,
    registry: NonSend<VideoElementRegistry>,
) {
    if let Some(element) = registry.element(trigger.asset_id())
        && let Ok(mut transform) = web_videos.get_mut(trigger.target())
    {
        transform.translation.z =
            ((element.current_time() / element.duration()) * (DISTANCE - 2.0) as f64) as f32;
    }
}

fn update(
    videos: Res<VideoReceiver>,
    mut web_videos: Query<(Entity, &WebVideo, &mut Transform, &InitialPosition), With<Disabled>>,
    registry: NonSend<VideoElementRegistry>,
    mut commands: Commands,
) -> Result<()> {
    for (entity, web_video, mut transform, initial_position) in web_videos.iter_mut() {
        if let Some(element) = registry.element(web_video.asset_id())
            && let Ok(video) = videos.try_recv()
        {
            transform.translation = initial_position.0;
            transform.scale = Vec3::new(
                video.aspect_ratio.min(1.0),
                (1.0 / video.aspect_ratio).min(1.0),
                1.0,
            );
            element.set_src(&video.url);
            let _ = element.play().map_err(WebVideoError::from)?;

            commands.entity(entity).remove::<Disabled>(); //XXX defer this to playing event?
            // Workaround broken Bevy Disabled handling https://github.com/bevyengine/bevy/issues/18981
            commands.queue(move |world: &mut World| -> Result<()> {
                let component_ids: Vec<_> = world
                    .inspect_entity(entity)?
                    .map(|component_info| component_info.id())
                    .collect();
                let mut entity_mut = world.entity_mut(entity);
                for component_id in component_ids {
                    if let Ok(mut component) = entity_mut.get_mut_by_id(component_id) {
                        component.set_changed();
                    }
                }
                Ok(())
            });
        }
    }
    Ok(())
}
