use crate::{
  apub::{check_is_apub_id_valid, extensions::signatures::sign, ActorType},
  LemmyError,
};
use activitystreams::{
  base::{Extends, ExtendsExt},
  object::AsObject,
};
use actix::prelude::*;
use anyhow::Context;
use awc::Client;
use lemmy_utils::{location_info, settings::Settings};
use log::{debug, warn};
use serde::Serialize;
use url::Url;
use background_jobs::{Backoff, MaxRetries, WorkerConfig, QueueHandle, Job, create_server};
use background_jobs::memory_storage::Storage;
use serde::Deserialize;
use anyhow::Error;
use futures::future::{Ready, ok};
use std::pin::Pin;
use std::future::Future;

pub fn send_activity<T, Kind>(
  activity_sender: &QueueHandle,
  activity: T,
  actor: &dyn ActorType,
  to: Vec<Url>,
) -> Result<(), LemmyError>
where
  T: AsObject<Kind>,
  T: Extends<Kind>,
  Kind: Serialize,
  <T as Extends<Kind>>::Error: From<serde_json::Error> + Send + Sync + 'static,
{
  if !Settings::get().federation.enabled {
    return Ok(());
  }

  let activity = activity.into_any_base()?;
  let serialised_activity = serde_json::to_string(&activity)?;

  for to_url in &to {
    check_is_apub_id_valid(&to_url)?;
  }

  // TODO: it would make sense to create a separate task for each destination server
  let message = SendActivityTask {
    activity: serialised_activity,
    to,
    actor_id: actor.actor_id()?,
    private_key: actor.private_key().context(location_info!())?,
  };
  activity_sender.queue::<SendActivityTask>(message)?;

  Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SendActivityTask {
  activity: String,
  to: Vec<Url>,
  actor_id: Url,
  private_key: String,
}

impl Job for SendActivityTask {
  type State = ();
  type Future = Pin<Box<dyn Future<Output = Result<(), Error>>>>;
  const NAME: &'static str = "SendActivityTask";

  const MAX_RETRIES: MaxRetries = MaxRetries::Count(10);
  const BACKOFF: Backoff = Backoff::Exponential(2);

  fn run(self, _: Self::State) -> Self::Future {
    Box::pin(async move {
      for to_url in &self.to {

        // TODO: should pass this in somehow instead of creating a new client every time
        //       i suppose this can be done through a state
        let client = Client::default();

        let request = client
          .post(to_url.as_str())
          .header("Content-Type", "application/json");

        // TODO: i believe we have to do the signing in here because it is only valid for a few seconds
        let signed = sign(
          request,
          self.activity.clone(),
          &self.actor_id,
          self.private_key.to_owned(),
        )
          .await?;
        signed.send().await?;
      }

      Ok(())
    })
  }
}

pub fn create_activity_queue() -> QueueHandle {

  // Start the application server. This guards access to to the jobs store
  let queue_handle = create_server(Storage::new());

  // Configure and start our workers
  WorkerConfig::new(||{})
    .register::<SendActivityTask>()
    .start(queue_handle.clone());

  // Queue our jobs
  //queue_handle.queue::<MyProcessor>(MyJob::new(1, 2))?;
  //queue_handle.queue::<MyProcessor>(MyJob::new(3, 4))?;
  //queue_handle.queue::<MyProcessor>(MyJob::new(5, 6))?;

  // Block on Actix
  queue_handle
}
