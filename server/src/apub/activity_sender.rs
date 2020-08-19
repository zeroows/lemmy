use crate::{
  apub::{check_is_apub_id_valid, extensions::signatures::sign, ActorType},
  LemmyError,
};
use activitystreams::base::AnyBase;
use actix::prelude::*;
use awc::Client;
use lemmy_db::{community::Community, user::User_};
use lemmy_utils::settings::Settings;
use log::debug;
use url::Url;

// We cant use ActorType here, because it doesnt implement Sized
#[derive(Message)]
#[rtype(result = "()")]
pub struct SendUserActivity {
  pub activity: AnyBase,
  pub actor: User_,
  pub to: Vec<Url>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SendCommunityActivity {
  pub activity: AnyBase,
  pub actor: Community,
  pub to: Vec<Url>,
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
  type Context = Context<Self>;
}

impl Handler<SendUserActivity> for ActivitySender {
  type Result = ();

  fn handle(&mut self, msg: SendUserActivity, _ctx: &mut Context<Self>) -> Self::Result {
    send_activity(msg.activity, &msg.actor, msg.to, &self.client);
  }
}

impl Handler<SendCommunityActivity> for ActivitySender {
  type Result = ();

  fn handle(&mut self, msg: SendCommunityActivity, _ctx: &mut Context<Self>) -> Self::Result {
    send_activity(msg.activity, &msg.actor, msg.to, &self.client);
  }
}

fn send_activity(activity: AnyBase, actor: &dyn ActorType, to: Vec<Url>, client: &Client) {
  if !Settings::get().federation.enabled {
    return;
  }

  let serialised_activity = serde_json::to_string(&activity).unwrap();
  debug!(
    "Sending activitypub activity {} to {:?}",
    &serialised_activity, &to
  );

  for to_url in &to {
    check_is_apub_id_valid(&to_url).unwrap();

    let request = client
      .post(to_url.as_str())
      .header("Content-Type", "application/json");

    let serialised_activity = serialised_activity.clone();
    Box::pin(async move {
      // TODO: need to remove the unwrap, but ? causes compile errors
      // TODO: if the sending fails, it should retry with exponential backoff
      let signed = sign(request, actor, serialised_activity).await.unwrap();
      let res = signed.send().await;
      debug!("Result for activity send: {:?}", res);
      Ok::<(), LemmyError>(())
    });
  }
}
