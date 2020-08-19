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

pub fn send_activity<T, Kind>(
  activity_sender: &Addr<ActivitySender>,
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

  let message = SendActivity {
    activity: serialised_activity,
    to,
    actor_id: actor.actor_id()?,
    private_key: actor.private_key().context(location_info!())?,
  };
  activity_sender.do_send(message);

  Ok(())
}

#[derive(Message)]
#[rtype(result = "()")]
struct SendActivity {
  activity: String,
  to: Vec<Url>,
  actor_id: Url,
  private_key: String,
}

pub struct ActivitySender {
  client: Client,
}

impl ActivitySender {
  pub fn startup(client: Client) -> ActivitySender {
    ActivitySender { client }
  }
}

impl Actor for ActivitySender {
  type Context = actix::Context<Self>;
}

impl Handler<SendActivity> for ActivitySender {
  type Result = ();

  fn handle(&mut self, msg: SendActivity, _ctx: &mut actix::Context<Self>) -> Self::Result {
    debug!(
      "Sending activitypub activity {} to {:?}",
      &msg.activity, &msg.to
    );

    Box::pin(async move {
      for to_url in &msg.to {
        let request = self
          .client
          .post(to_url.as_str())
          .header("Content-Type", "application/json");

        let signed = sign(
          request,
          msg.activity.clone(),
          &msg.actor_id,
          msg.private_key.to_owned(),
        )
        .await;

        let signed = match signed {
          Ok(s) => s,
          Err(e) => {
            warn!(
              "Failed to sign activity {} from {}: {}",
              &msg.activity, &msg.actor_id, e
            );
            return;
          }
        };

        // TODO: if the sending fails, it should retry with exponential backoff
        match signed.send().await {
          Ok(_) => {}
          Err(e) => {
            warn!(
              "Failed to send activity {} to {}: {}",
              &msg.activity, &to_url, e
            );
          }
        }
      }
    });
  }
}
