use crate::{
  apub::{activity_sender::SendUserActivity, community::do_announce, insert_activity},
  LemmyContext,
  LemmyError,
};
use activitystreams::base::AnyBase;
use lemmy_db::{community::Community, user::User_};
use lemmy_utils::{get_apub_protocol_string, settings::Settings};
use url::{ParseError, Url};
use uuid::Uuid;

pub async fn send_activity_to_community(
  creator: &User_,
  community: &Community,
  to: Vec<Url>,
  activity: AnyBase,
  context: &LemmyContext,
) -> Result<(), LemmyError> {
  insert_activity(creator.id, activity.clone(), true, context.pool()).await?;

  // if this is a local community, we need to do an announce from the community instead
  if community.local {
    do_announce(activity, &community, creator, context).await?;
  } else {
    let message = SendUserActivity {
      activity,
      actor: creator.to_owned(),
      to,
    };
    context.activity_sender().send(message).await??;
  }

  Ok(())
}

pub(in crate::apub) fn generate_activity_id<T>(kind: T) -> Result<Url, ParseError>
where
  T: ToString,
{
  let id = format!(
    "{}://{}/activities/{}/{}",
    get_apub_protocol_string(),
    Settings::get().hostname,
    kind.to_string().to_lowercase(),
    Uuid::new_v4()
  );
  Url::parse(&id)
}
