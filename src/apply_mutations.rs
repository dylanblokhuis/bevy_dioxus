use crate::events::is_supported_event;
use bevy::{
    ecs::{entity::Entity, system::Command, world::World},
    hierarchy::{BuildWorldChildren, Children, DespawnRecursive, Parent},
    prelude::default,
    render::color::Color,
    text::{Text, TextLayoutInfo, TextStyle},
    ui::{
        node_bundles::{NodeBundle, TextBundle},
        widget::TextFlags,
        *,
    },
    utils::{EntityHashMap, HashMap},
};
use dioxus::core::{
    BorrowedAttributeValue, ElementId, Mutation, Mutations, Template, TemplateAttribute,
    TemplateNode,
};

pub fn apply_mutations(
    mutations: Mutations,
    element_id_to_bevy_ui_entity: &mut HashMap<ElementId, Entity>,
    bevy_ui_entity_to_element_id: &mut EntityHashMap<Entity, ElementId>,
    templates: &mut HashMap<String, BevyTemplate>,
    root_entity: Entity,
    world: &mut World,
) {
    for new_template in mutations.templates {
        templates.insert(
            new_template.name.to_owned(),
            BevyTemplate::from_dioxus(&new_template),
        );
    }

    element_id_to_bevy_ui_entity.insert(ElementId(0), root_entity);
    bevy_ui_entity_to_element_id.insert(root_entity, ElementId(0));
    let mut stack = vec![root_entity];

    for edit in mutations.edits {
        match edit {
            Mutation::AppendChildren { id, m } => {
                let mut parent = world.entity_mut(element_id_to_bevy_ui_entity[&id]);
                for child in stack.drain((stack.len() - m)..) {
                    parent.add_child(child);
                }
            }
            Mutation::AssignId { path, id } => {
                let mut entity = *stack.last().unwrap();
                for index in path {
                    entity = world.entity(entity).get::<Children>().unwrap()[*index as usize];
                }
                element_id_to_bevy_ui_entity.insert(id, entity);
                bevy_ui_entity_to_element_id.insert(entity, id);
            }
            Mutation::CreatePlaceholder { id } => {
                let entity = world.spawn(NodeBundle::default()).id();
                element_id_to_bevy_ui_entity.insert(id, entity);
                bevy_ui_entity_to_element_id.insert(entity, id);
                stack.push(entity);
            }
            Mutation::CreateTextNode { value, id } => {
                let entity =
                    BevyTemplateNode::from_dioxus(&TemplateNode::Text { text: value }).spawn(world);
                element_id_to_bevy_ui_entity.insert(id, entity);
                bevy_ui_entity_to_element_id.insert(entity, id);
                stack.push(entity);
            }
            Mutation::HydrateText { path, value, id } => {
                let mut entity = *stack.last().unwrap();
                for index in path {
                    entity = world.entity(entity).get::<Children>().unwrap()[*index as usize];
                }
                world.entity_mut(entity).insert((
                    Text::from_section(value, TextStyle::default()),
                    TextLayoutInfo::default(),
                    TextFlags::default(),
                    ContentSize::default(),
                ));
                element_id_to_bevy_ui_entity.insert(id, entity);
                bevy_ui_entity_to_element_id.insert(entity, id);
            }
            Mutation::LoadTemplate { name, index, id } => {
                let entity = templates[name].roots[index].spawn(world);
                element_id_to_bevy_ui_entity.insert(id, entity);
                bevy_ui_entity_to_element_id.insert(entity, id);
                stack.push(entity);
            }
            Mutation::ReplaceWith { id, m } => {
                let new_nodes = stack.split_off(stack.len() - m);
                let existing = element_id_to_bevy_ui_entity[&id];

                // here we insert before the old entity that's going to be removed after
                let parent = world.entity(existing).get::<Parent>().unwrap().get();
                let mut parent = world.entity_mut(parent);
                let index = parent
                    .get::<Children>()
                    .unwrap()
                    .iter()
                    .position(|child| *child == existing)
                    .unwrap();
                parent.insert_children(index, &new_nodes);

                DespawnRecursive { entity: existing }.apply(world);
                // TODO: We're not removing child entities from the element maps
                if let Some(existing_element_id) = bevy_ui_entity_to_element_id.remove(&existing) {
                    element_id_to_bevy_ui_entity.remove(&existing_element_id);
                } 
            },
            Mutation::ReplacePlaceholder { path, m } => {
                let mut existing = stack[stack.len() - m - 1];
                for index in path {
                    existing = world.entity(existing).get::<Children>().unwrap()[*index as usize];
                }
                let existing_parent = world.entity(existing).get::<Parent>().unwrap().get();
                let mut existing_parent = world.entity_mut(existing_parent);

                let existing_index = existing_parent
                    .get::<Children>()
                    .unwrap()
                    .iter()
                    .position(|child| *child == existing)
                    .unwrap();
                let new = stack.drain((stack.len() - m)..).collect::<Vec<Entity>>();
                existing_parent.insert_children(existing_index, &new);

                DespawnRecursive { entity: existing }.apply(world);
                // TODO: We're not removing child entities from the element maps
                if let Some(existing_element_id) = bevy_ui_entity_to_element_id.remove(&existing) {
                    element_id_to_bevy_ui_entity.remove(&existing_element_id);
                } 
            }
            Mutation::InsertAfter { id, m } => {
                let entity = element_id_to_bevy_ui_entity[&id];
                let parent = world.entity(entity).get::<Parent>().unwrap().get();
                let mut parent = world.entity_mut(parent);

                let index = parent
                    .get::<Children>()
                    .unwrap()
                    .iter()
                    .position(|child| *child == entity)
                    .unwrap();
                let new = stack.drain((stack.len() - m)..).collect::<Vec<Entity>>();
                parent.insert_children(index + 1, &new);
            }
            Mutation::InsertBefore { id, m } => {
                let new_nodes = stack.split_off(stack.len() - m);
                let existing = element_id_to_bevy_ui_entity[&id];

                let parent = world.entity(existing).get::<Parent>().unwrap().get();
                let mut parent = world.entity_mut(parent);
                let index = parent
                    .get::<Children>()
                    .unwrap()
                    .iter()
                    .position(|child| *child == existing)
                    .unwrap();
                parent.insert_children(index, &new_nodes);
            }
            Mutation::SetAttribute {
                name,
                value,
                id,
                ns: _,
            } => {
                let value = match value {
                    BorrowedAttributeValue::Text(value) => value,
                    value => {
                        panic!("Encountered unsupported bevy_dioxus attribute `{name}: {value:?}`.")
                    }
                };

                let (mut style, mut background_color) = world
                    .query::<(&mut Style, &mut BackgroundColor)>()
                    .get_mut(world, element_id_to_bevy_ui_entity[&id])
                    .unwrap();
                set_style_attribute(name, value, &mut style, &mut background_color);
            }
            Mutation::SetText { value, id } => {
                world
                    .entity_mut(element_id_to_bevy_ui_entity[&id])
                    .insert(Text::from_section(value, TextStyle::default()));
            }
            Mutation::NewEventListener { name, id: _ } => {
                if !is_supported_event(name) {
                    panic!("Encountered unsupported bevy_dioxus event `{name}`.");
                }
            }
            Mutation::RemoveEventListener { .. } => {}
            Mutation::Remove { id } => {
                let existing = element_id_to_bevy_ui_entity[&id];
                DespawnRecursive {
                    entity: existing,
                }
                .apply(world);
                // TODO: We're not removing child entities from the element maps
                if let Some(existing_element_id) = bevy_ui_entity_to_element_id.remove(&existing) {
                    element_id_to_bevy_ui_entity.remove(&existing_element_id);
                }
            }
            Mutation::PushRoot { id } => stack.push(element_id_to_bevy_ui_entity[&id]),
        }
    }
}

pub struct BevyTemplate {
    roots: Box<[BevyTemplateNode]>,
}

enum BevyTemplateNode {
    Node {
        style: (Style, BackgroundColor),
        children: Box<[Self]>,
    },
    TextNode(Text),
}

impl BevyTemplate {
    fn from_dioxus(template: &Template) -> Self {
        Self {
            roots: template
                .roots
                .iter()
                .map(BevyTemplateNode::from_dioxus)
                .collect(),
        }
    }
}

impl BevyTemplateNode {
    fn from_dioxus(node: &TemplateNode) -> Self {
        match node {
            TemplateNode::Element {
                tag,
                namespace: _,
                attrs,
                children,
            } => {
                if *tag != "node" {
                    panic!(
                        "Encountered unsupported bevy_dioxus tag `{tag}`. Only `node` is supported."
                    );
                }
                Self::Node {
                    style: parse_style_attributes(attrs),
                    children: children.iter().map(Self::from_dioxus).collect(),
                }
            }
            TemplateNode::Text { text } => {
                Self::TextNode(Text::from_section(*text, TextStyle::default()))
            }
            TemplateNode::Dynamic { id: _ } => Self::Node {
                style: (Style::default(), Color::NONE.into()),
                children: Box::new([]),
            },
            TemplateNode::DynamicText { id: _ } => {
                Self::TextNode(Text::from_section("", TextStyle::default()))
            }
        }
    }

    fn spawn(&self, world: &mut World) -> Entity {
        match self {
            BevyTemplateNode::Node {
                style: (style, background_color),
                children,
            } => {
                let children = children
                    .iter()
                    .map(|child| child.spawn(world))
                    .collect::<Box<[_]>>();
                world
                    .spawn(NodeBundle {
                        style: style.clone(),
                        background_color: background_color.clone(),
                        ..default()
                    })
                    .push_children(&children)
                    .id()
            }
            Self::TextNode(text) => world
                .spawn(TextBundle {
                    text: text.clone(),
                    ..default()
                })
                .id(),
        }
    }
}

fn parse_style_attributes(attributes: &[TemplateAttribute]) -> (Style, BackgroundColor) {
    let mut style = Style::default();
    let mut background_color = Color::NONE.into();
    for attribute in attributes {
        if let TemplateAttribute::Static {
            name,
            value,
            namespace: _,
        } = attribute
        {
            set_style_attribute(name, value, &mut style, &mut background_color);
        }
    }
    (style, background_color)
}

fn set_style_attribute(
    name: &str,
    value: &str,
    style: &mut Style,
    background_color: &mut BackgroundColor,
) {
    // TODO: The rest of Style
    match (name, value) {
        ("display", "flex") => style.display = Display::Flex,
        ("display", "grid") => style.display = Display::Grid,
        ("display", "none") => style.display = Display::None,
        ("position", "relative") => style.position_type = PositionType::Relative,
        ("position", "absolute") => style.position_type = PositionType::Absolute,
        ("flex-direction", "column") => style.flex_direction = FlexDirection::Column,
        ("background-color", hex) => {
            background_color.0 = Color::hex(hex).expect(&format!(
                "Encountered unsupported bevy_dioxus hex Color `{hex}`."
            ))
        }
        ("padding", val) => style.padding = UiRect::all(parse_val(val)),
        ("width", val) => style.width = parse_val(val),
        ("height", val) => style.height = parse_val(val),
        ("justify-content", "space-between") => {
            style.justify_content = JustifyContent::SpaceBetween;
        }
        ("align-content", "space-between") => style.align_content = AlignContent::SpaceBetween,
        _ => panic!("Encountered unsupported bevy_dioxus attribute `{name}: {value}`."),
    }
}

fn parse_val(val: &str) -> Val {
    if let Ok(val) = val.parse::<f32>() {
        return Val::Px(val);
    }
    if let Some((val, "")) = val.split_once("px") {
        if let Ok(val) = val.parse::<f32>() {
            return Val::Px(val);
        }
    }
    if let Some((val, "")) = val.split_once("vw") {
        if let Ok(val) = val.parse::<f32>() {
            return Val::Vw(val);
        }
    }
    if let Some((val, "")) = val.split_once("vh") {
        if let Ok(val) = val.parse::<f32>() {
            return Val::Vh(val);
        }
    }
    panic!("Encountered unsupported bevy_dioxus Val `{val}`.");
}
