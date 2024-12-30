use encosmo_shared::server_components::*;
use specs::{Builder, Entity, World, WorldExt};
use uuid::Uuid;

pub fn create_player(world: &mut World, id: Uuid) -> Entity {
    world
        .create_entity()
        .with(Translate::default())
        .with(Position::default())
        .with(PlayerDetails(id))
        .with(GameObjectDetails {
            name: "RANDO GENERATED NAME".to_owned(),
            description: "PLACEHOLDER - see DF style rando gen descriptions".to_owned()
        })
        .build()
}
