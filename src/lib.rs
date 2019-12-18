use ammolite_math::Mat4;
use serde::{Serialize, Deserialize};

pub use proc_macro_mapp::mapp;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct IO {
    pub out: Vec<u8>,
    pub err: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Model(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Entity(pub usize);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub id: usize,
    pub kind: CommandKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    pub command_id: usize,
    pub kind: CommandResponseKind,
}

macro_rules! command_kinds {
    {$($name:ident $({ $($request_fields:tt)* })? $(-> { $($response_fields:tt)* })?),*$(,)?} => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum CommandKind {
            $(
                $name $({
                    $($request_fields)*
                })?
            ),*
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum CommandResponseKind {
            $(
                $name $({
                    $($response_fields)*
                })?
            ),*
        }
    }
}

command_kinds! {
    ModelCreate {
        data: Vec<u8>,
    } -> {
        model: Model,
    },
    EntityRootGet -> {
        root_entity: Entity,
    },
    EntityCreate -> {
        entity: Entity,
    },
    EntityParentSet {
        entity: Entity,
        parent_entity: Option<Entity>,
    } -> {
        previous_parent_entity: Option<Entity>,
    },
    EntityModelSet {
        entity: Entity,
        model: Option<Model>,
    } -> {
        previous_model: Option<Model>,
    },
    EntityTransformSet {
        entity: Entity,
        transform: Option<Mat4>,
    } -> {
        previous_transform: Option<Mat4>,
    },
}
