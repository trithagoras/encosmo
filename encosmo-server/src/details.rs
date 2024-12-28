use uuid::Uuid;


// global primitive details about the player - incl. connection / game id, in-game name, etc.
pub struct Details {
    pub id: Uuid,
    pub name: Option<String>
}
