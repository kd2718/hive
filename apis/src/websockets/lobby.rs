use crate::{
    common::server_result::{
        ChallengeUpdate, MessageDestination, ServerMessage, ServerResult, UserStatus, UserUpdate,
    },
    responses::{challenge::ChallengeResponse, game::GameResponse, user::UserResponse},
    websockets::messages::{ClientActorMessage, Connect, Disconnect, WsMessage},
};
use actix::prelude::{Actor, Context, Handler, Recipient};
use actix::AsyncContext;
use actix::WrapFuture;
use db_lib::{models::challenge::Challenge, models::user::User, DbPool};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

type Socket = Recipient<WsMessage>;

#[derive(Debug)]
pub struct Lobby {
    #[allow(dead_code)]
    id: String,
    sessions: HashMap<Uuid, Socket>, // user_id to (socket_)id
    games_users: HashMap<String, HashSet<Uuid>>, // game_id to set of users
    users_games: HashMap<Uuid, HashSet<String>>, // user_id to set of games
    #[allow(dead_code)]
    pool: DbPool,
}

impl Lobby {
    pub fn new(pool: DbPool) -> Lobby {
        Lobby {
            id: String::from("lobby"),
            sessions: HashMap::new(),
            games_users: HashMap::new(),
            users_games: HashMap::new(),
            pool,
        }
    }
}

impl Lobby {
    fn send_message(&self, message: &str, id_to: &Uuid) {
        if let Some(socket_recipient) = self.sessions.get(id_to) {
            socket_recipient.do_send(WsMessage(message.to_owned()));
        } else {
            println!("attempting to send message but couldn't find user id.");
        }
    }
}

impl Actor for Lobby {
    type Context = Context<Self>;
}

impl Handler<Disconnect> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        if self.sessions.remove(&msg.user_id).is_some() {
            if let Some(games) = self.users_games.remove(&msg.user_id) {
                for game in games.iter() {
                    if let Some(game_users) = self.games_users.get_mut(game) {
                        if game_users.len() > 1 {
                            game_users.remove(&msg.user_id);
                        } else {
                            //only one in the game, remove it entirely
                            self.games_users.remove(game);
                        }
                    }
                }
            }
            let message = ServerResult::Ok(ServerMessage::UserStatus(UserUpdate {
                status: UserStatus::Offline,
                user: None,
                username: msg.username,
            }));
            let serialized =
                serde_json::to_string(&message).expect("Failed to serialize a server message");
            if let Some(lobby) = self.games_users.get(&self.id) {
                lobby
                    .iter()
                    .filter(|conn_id| *conn_id.to_owned() != msg.user_id)
                    .for_each(|user_id| self.send_message(&serialized, user_id));
            }
        }
    }
}

impl Handler<Connect> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) -> Self::Result {
        let user_id = msg.user_id;
        self.games_users
            .entry(msg.game_id.clone())
            .or_default()
            .insert(msg.user_id);
        self.users_games
            .entry(msg.user_id)
            .or_default()
            .insert(msg.game_id.clone());
        self.sessions.insert(msg.user_id, msg.addr.clone());
        let pool = self.pool.clone();
        let address = ctx.address().clone();
        let games_users = self.games_users.clone();
        let sessions = self.sessions.clone();
        let future = async move {
            //Get currently online users
            for uuid in sessions.keys() {
                if let Ok(user_response) = UserResponse::from_uuid(uuid, &pool).await {
                    let message = ServerResult::Ok(ServerMessage::UserStatus(UserUpdate {
                        status: UserStatus::Online,
                        user: Some(user_response.clone()),
                        username: user_response.username,
                    }));
                    let serialized = serde_json::to_string(&message)
                        .expect("Failed to serialize a server message");
                    let cam = ClientActorMessage {
                        destination: MessageDestination::Direct(user_id),
                        serialized,
                        from: user_id,
                    };
                    address.do_send(cam);
                }
            }
            let serialized = if let Ok(user) = User::find_by_uuid(&user_id, &pool).await {
                if let Ok(user_response) = UserResponse::from_user(&user, &pool).await {
                    let message = ServerResult::Ok(ServerMessage::UserStatus(UserUpdate {
                        status: UserStatus::Online,
                        user: Some(user_response),
                        username: msg.username,
                    }));
                    let serialized = serde_json::to_string(&message)
                        .expect("Failed to serialize a server message");
                    // TODO: one needs to be a game::join to everyone in the game, the other one just to the
                    // lobby that the user came online
                    games_users
                        .get(&msg.game_id)
                        .expect("Uuid exists")
                        .iter()
                        .for_each(|conn_id| {
                            if let Some(socket_recipient) = sessions.get(conn_id) {
                                socket_recipient.do_send(WsMessage(serialized.clone()));
                            } else {
                                println!("attempting to send message but couldn't find user id.");
                            }
                        });
                }

                // Send games which require input from the user
                let game_ids = user
                    .get_urgent_nanoids(&pool)
                    .await
                    .expect("to get urgent game_ids");
                if !game_ids.is_empty() {
                    let mut games = Vec::new();
                    for game_id in game_ids {
                        if let Ok(game) = GameResponse::new_from_nanoid(&game_id, &pool).await {
                            games.push(game)
                        }
                    }
                    let message = ServerResult::Ok(ServerMessage::GameActionNotification(games));
                    let serialized = serde_json::to_string(&message)
                        .expect("Failed to serialize a server message");
                    let cam = ClientActorMessage {
                        destination: MessageDestination::Direct(user_id),
                        serialized,
                        from: user_id,
                    };
                    address.do_send(cam);
                }
                // Send challenges on join
                let mut responses = Vec::new();
                if let Ok(challenges) = Challenge::get_public_exclude_user(user.id, &pool).await {
                    for challenge in challenges {
                        if let Ok(response) = ChallengeResponse::from_model(&challenge, &pool).await
                        {
                            responses.push(response);
                        }
                    }
                }
                if let Ok(challenges) = Challenge::get_own(user.id, &pool).await {
                    for challenge in challenges {
                        if let Ok(response) = ChallengeResponse::from_model(&challenge, &pool).await
                        {
                            responses.push(response);
                        }
                    }
                }
                let message = ServerResult::Ok(ServerMessage::Challenge(
                    ChallengeUpdate::Challenges(responses),
                ));
                serde_json::to_string(&message).expect("Failed to serialize a server message")
            } else {
                let mut responses = Vec::new();
                if let Ok(challenges) = Challenge::get_public(&pool).await {
                    for challenge in challenges {
                        if let Ok(response) = ChallengeResponse::from_model(&challenge, &pool).await
                        {
                            responses.push(response);
                        }
                    }
                }
                let message = ServerResult::Ok(ServerMessage::Challenge(
                    ChallengeUpdate::Challenges(responses),
                ));
                serde_json::to_string(&message).expect("Failed to serialize a server message")
            };
            let cam = ClientActorMessage {
                destination: MessageDestination::Direct(user_id),
                serialized,
                from: user_id,
            };
            address.do_send(cam);
        };

        let actor_future = future.into_actor(self);
        ctx.wait(actor_future);

        // if msg.game_id == self.id {
        //     self.send_message(&format!("You joined {}", msg.game_id), &msg.user_id);
        //     return ();
        // }
    }
}

impl Handler<ClientActorMessage> for Lobby {
    type Result = ();

    fn handle(&mut self, cam: ClientActorMessage, _ctx: &mut Context<Self>) -> Self::Result {
        match cam.destination {
            MessageDestination::Global => {
                // Make sure the user is in the game:
                self.games_users
                    .entry(self.id.clone())
                    .or_default()
                    .insert(cam.from);
                // Send the message to everyone
                self.games_users
                    .get(&self.id)
                    .expect("Game to exists")
                    .iter()
                    .for_each(|client| self.send_message(&cam.serialized, client));
            }
            MessageDestination::Game(game_id) => {
                // Make sure the user is in the game:
                self.games_users
                    .entry(game_id.clone())
                    .or_default()
                    .insert(cam.from);
                // Send the message to everyone
                self.games_users
                    .get(&game_id)
                    .expect("Game to exists")
                    .iter()
                    .for_each(|client| self.send_message(&cam.serialized, client));
            }
            MessageDestination::Direct(user_id) => {
                self.send_message(&cam.serialized, &user_id);
            }
        }
    }
}
