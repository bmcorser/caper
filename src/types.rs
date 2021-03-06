/// type definition for a Vector3
pub type Vector3 = (f32, f32, f32);

/// type definition for a Quaternion
pub type Quaternion = (f32, f32, f32, f32);

/// struct for defining a Vector for creating meshes
#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texture: [f32; 2],
}
implement_vertex!(Vertex, position, normal, texture);

/// struct for handling transform data
pub struct Transform {
    pub pos: Vector3,
    pub rot: Quaternion,
    pub scale: Vector3,
    pub update_fn: Vec<fn(&mut Transform)>,
}

/// struct for abstracting items to be sent to render
pub struct RenderItem {
    pub vertices: Vec<Vertex>,
    pub shader_name: &'static str,
    pub instance_transforms: Vec<Transform>,
}

/// struct for abstacting text items to be rendered
pub struct TextItem {
    pub text: String,
    pub color: (f32, f32, f32, f32),
    pub pos: Vector3,
    pub scale: Vector3,
    pub update_fn: Vec<fn(&mut TextItem)>,
}

/// trait for updateable entities
pub trait Entity {
    /// ran every frame
    fn update(&mut self) -> ();
}

/// implementation of Entity for Transform
impl Entity for Transform {
    fn update(&mut self) {
        for i in 0..self.update_fn.len() {
            self.update_fn[i](self);
        }
    }
}

/// implementation of Entity for TextItem
impl Entity for TextItem {
    fn update(&mut self) {
        for i in 0..self.update_fn.len() {
            self.update_fn[i](self);
        }
    }
}

/// struct for abstracting the camera state
#[derive(Copy, Clone)]
pub struct CamState {
    pub cam_pos:Vector3,
    pub cam_rot:Vector3
}

/// struct for shader attributes
#[derive(Copy, Clone)]
pub struct Attr {
    pub world_position: Vector3,
    pub world_rotation: Quaternion,
    pub world_scale: Vector3
}
implement_vertex!(Attr, world_position, world_rotation, world_scale);
