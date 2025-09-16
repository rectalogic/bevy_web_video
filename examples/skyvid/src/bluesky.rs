use atrium_api::{
    client::AtpServiceClient,
    types::{LimitedNonZeroU8, Union},
};
use atrium_xrpc_client::reqwest::ReqwestClient;
use bevy::{prelude::*, tasks::IoTaskPool};

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup);
}

struct Video {
    url: String,
    aspect_ratio: Option<(u64, u64)>,
}

#[derive(Resource)]
struct VideoChannel(async_channel::Receiver<Video>);

fn setup(mut commands: Commands) {
    let (tx, rx) = async_channel::bounded(5);
    commands.insert_resource(VideoChannel(rx));
    IoTaskPool::get().spawn(async move {
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
                            && tx.send( Video { url: video.playlist.clone(), aspect_ratio: video.aspect_ratio.as_ref().map(|a| (a.width.get(), a.height.get())) }).await.is_err() {
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
    }).detach();
}
