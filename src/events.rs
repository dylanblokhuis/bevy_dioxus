use bevy::{ecs::{
    entity::Entity,
    event::{Events, ManualEventReader},
    system::Resource,
}, math::Vec2};
use bevy_mod_picking::events::{Click, Pointer};
use std::{any::Any, rc::Rc};

// TODO: Other events

// TODO: Naming is still undecided
#[derive(Default)]
pub struct PointerInput  {
    /// Where is the pointer?
    pub position: Vec2,
}

#[derive(Resource, Default)]
pub struct EventReaders {
    clicks: ManualEventReader<Pointer<Click>>,
}

impl EventReaders {
    pub fn get_dioxus_events(
        &mut self,
        clicks: &Events<Pointer<Click>>,
    ) -> Vec<(Entity, &'static str, Rc<dyn Any>)> {
        let mut events: Vec<(Entity, &'static str, Rc<dyn Any>)> = Vec::new();

        for event in self.clicks.read(clicks) {
            events.push((event.target, "click", Rc::new(PointerInput::default())));
        }

        events
    }
}

pub fn is_supported_event(event: &str) -> bool {
    event == "click"
}
