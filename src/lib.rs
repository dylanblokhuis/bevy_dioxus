#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

mod apply_mutations;
pub mod colors;
mod deferred_system;
mod events;
pub mod hooks;
mod tick;

use self::{
    apply_mutations::BevyTemplate, deferred_system::DeferredSystemRegistry, events::EventReaders,
    hooks::EcsSubscriptions, tick::tick_dioxus_ui,
};
use bevy::{
    app::{App, Plugin, Update},
    ecs::{bundle::Bundle, component::Component, entity::Entity},
    prelude::Deref,
    ui::node_bundles::NodeBundle,
    utils::{EntityHashMap, HashMap},
};
use dioxus::core::{Element, ElementId, Scope, VirtualDom};

pub use bevy_mod_picking;
pub use dioxus;

pub struct DioxusUiPlugin;

impl Plugin for DioxusUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<UiContext>()
            .init_resource::<DeferredSystemRegistry>()
            .init_resource::<EventReaders>()
            .add_systems(Update, tick_dioxus_ui);
    }
}

#[derive(Bundle)]
pub struct DioxusUiBundle {
    pub dioxus_ui_root: DioxusUiRoot,
    pub node_bundle: NodeBundle,
}

#[derive(Component, Deref, Hash, PartialEq, Eq, Clone, Copy)]
pub struct DioxusUiRoot(pub fn(Scope) -> Element);

#[derive(Default)]
struct UiContext {
    roots: HashMap<(Entity, DioxusUiRoot), UiRoot>,
    subscriptions: EcsSubscriptions,
}

struct UiRoot {
    virtual_dom: VirtualDom,
    element_id_to_bevy_ui_entity: HashMap<ElementId, Entity>,
    bevy_ui_entity_to_element_id: EntityHashMap<Entity, ElementId>,
    templates: HashMap<String, BevyTemplate>,
    needs_rebuild: bool,
}

impl UiRoot {
    fn new(root_component: DioxusUiRoot) -> Self {
        Self {
            virtual_dom: VirtualDom::new(root_component.0),
            element_id_to_bevy_ui_entity: HashMap::new(),
            bevy_ui_entity_to_element_id: EntityHashMap::default(),
            templates: HashMap::new(),
            needs_rebuild: true,
        }
    }
}


#[doc(hidden)]
pub trait EventReturn<P>: Sized {
    fn spawn(self, _cx: &dioxus::core::ScopeState) {}
}

impl EventReturn<()> for () {}
#[doc(hidden)]
pub struct AsyncMarker;

impl<T> EventReturn<AsyncMarker> for T
where
    T: std::future::Future<Output = ()> + 'static,
{
    #[inline]
    fn spawn(self, cx: &dioxus::core::ScopeState) {
        cx.spawn(self);
    }
}

macro_rules! impl_event {
    (
        $data:ty;
        $(
            $( #[$attr:meta] )*
            $name:ident
        )*
    ) => {
        $(
            $( #[$attr] )*
            #[inline]
            pub fn $name<'a, E: crate::EventReturn<T>, T>(_cx: &'a dioxus::core::ScopeState, mut _f: impl FnMut(dioxus::core::Event<$data>) -> E + 'a) -> dioxus::core::Attribute<'a> {
                dioxus::core::Attribute::new(
                    stringify!($name),
                    _cx.listener(move |e: dioxus::core::Event<$data>| {
                        _f(e).spawn(_cx);
                    }),
                    None,
                    false,
                )
            }
        )*
    };
}

pub mod prelude {
    use dioxus::prelude::IntoAttributeValue;


pub trait ColorExt: Sized {}
    impl ColorExt for bevy::render::color::Color {
        // TODO
    }

    impl<'a, T: ColorExt> IntoAttributeValue<'a> for &'a  {
        fn into_value(self, bump: &'a dioxus::core::exports::bumpalo::Bump) -> dioxus::core::AttributeValue<'a> {
            
        }
    }

    pub mod dioxus_elements {
        pub type AttributeDescription = (&'static str, Option<&'static str>, bool);
    
        pub struct node;
        impl node {
            pub const TAG_NAME: &'static str = "node";
            pub const NAME_SPACE: Option<&'static str> = None;
    
            pub const width: AttributeDescription = ("width", None, false);
            pub const height: AttributeDescription = ("height", None, false);
            pub const justify_content: AttributeDescription = ("justify-content", None, false);
            pub const flex_direction: AttributeDescription = ("flex-direction", None, false);
            pub const padding: AttributeDescription = ("padding", None, false);
            pub const background_color: AttributeDescription = ("background-color", None, false);

            // TODO: Many more attributes
        }
    
        pub mod events {
            impl_event! [
                crate::events::PointerInput;
                onclick
            ];
        }
    }
}
