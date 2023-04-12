use bevy::prelude::{Component, Commands, Entity, Name, Children, Query, Added, DerefMut, Deref};
use std::fmt::Debug;
use crate::{AnimationPlayer, EntityPath};


// A hacky solution for always having the entity path attached to an entity; 
// be cautious of this not staying up to date (if not used in the editor).

#[derive(Component, Clone, Deref, DerefMut)]
pub struct TrackedEntityPath(pub EntityPath);

impl Debug for TrackedEntityPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.0.parts.iter().map(|p| p.as_str())).finish()
    }
}

pub fn add_entity_paths(
    mut commands: Commands,
    player_query: Query<(Entity, &Name), Added<AnimationPlayer>>,
    entity_path_query: Query<(&Children, &TrackedEntityPath), Added<TrackedEntityPath>>,
    name_query: Query<&Name>,
) {
    for (entity, name) in player_query.iter() {
        let mut entity_path = EntityPath::default();
        entity_path.parts.push(name.clone());
        commands.entity(entity).insert(TrackedEntityPath(entity_path));
    }
    for (children, entity_path) in entity_path_query.iter() {
        for child in children.iter() {
            let Ok(name) = name_query.get(*child) else { continue; };
            let mut new_entity_path = entity_path.clone();
            new_entity_path.0.parts.push(name.clone());
            commands.entity(*child).insert(new_entity_path);
        }
    }
}