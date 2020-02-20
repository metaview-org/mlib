use ammolite_math::{Mat4, Vec3};
use serde::{Serialize, Deserialize};

pub mod event;

pub use event::*;
pub use proc_macro_mapp::mapp;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct IO {
    pub out: Vec<u8>,
    pub err: Vec<u8>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Base64ByteSlice(String);

impl Base64ByteSlice {
    pub fn into_bytes(self) -> Vec<u8> {
        base64::decode(&self.0)
            .expect("Invalid base64 byte slice. Was there an error during encoding?")
    }
}

impl<T: AsRef<[u8]>> From<T> for Base64ByteSlice {
    fn from(other: T) -> Self {
        let bytes = other.as_ref();
        let string = base64::encode(bytes);
        Self(string)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Model(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Entity(pub usize);

#[derive(PartialEq, Clone, Default, Debug, Serialize, Deserialize)]
pub struct ViewFov {
    pub angle_left: f32,
    pub angle_right: f32,
    pub angle_up: f32,
    pub angle_down: f32,
}

#[derive(PartialEq, Clone, Default, Debug, Serialize, Deserialize)]
pub struct View {
    pub pose: Mat4,
    pub fov: ViewFov,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Intersection {
    pub position: Vec3,
    pub distance_from_origin: f32,
    pub entity: Entity,
}

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
        // Due to the fact that we're currently using JSON for command
        // serialization, it is actually faster to encode byte slices into
        // a utf-8 base64 string.
        data: Base64ByteSlice,
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
    // Consider changing the name
    GetViewOrientation {} -> {
        views_per_medium: Vec<Option<Vec<View>>>,
    },
    // TODO:
    // * Consider adding the ability to select which entities to ray trace via a mask.
    RayTrace {
        origin: Vec3,
        direction: Vec3,
    } -> {
        closest_intersection: Option<Intersection>,
    },
    Exit,
}
