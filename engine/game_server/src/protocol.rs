use actix::Message;
use core_protocol::dto::InvitationDto;
use core_protocol::id::PlayerId;
use core_protocol::id::SessionId;

/// For main to authenticate SessionIds before opening a websocket.
#[derive(Message)]
#[rtype(result = "Option<(PlayerId, Option<InvitationDto>)>")]
pub struct Authenticate {
    pub session_id: SessionId,
}
