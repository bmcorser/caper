use glium::{ Display, DrawParameters, DisplayBuild, Surface, Depth };
use glium::index::{ NoIndices, PrimitiveType };
use glium::DepthTest::IfLess;
use glium::vertex::VertexBuffer;
use glium::glutin::{ WindowBuilder, get_primary_monitor };
use glium::glutin::CursorState::Hide;//{ Grab, Hide };
use glium::draw_parameters::BackfaceCullingMode::CullClockwise;

use glium_text;
use glium_text::{ TextSystem, FontTexture, TextDisplay };

use time;
use shader::Shaders;
use std::default::Default;
use std::f32::consts::PI;

use imgui::{ ImGui, Ui };
use imgui::glium_renderer::Renderer as ImGuiRenderer;

use utils::*;
use posteffect::*;
use types::*;

pub const FIXED_TIME_STAMP: u64 = 16666667;

/// struct for abstracting the render state
pub struct Renderer {
    pub display: Display,
    pub text_system: TextSystem,
    default_font: FontTexture,
    imgui: ImGui,
    imgui_rend: ImGuiRenderer,
    post_effect: PostEffect,
    pub start_time: f64,
    pub shaders: Shaders,
}

impl Renderer {
    /// Creates new Renderer instance
    pub fn new(title:String) -> Renderer {
        // create a diplay instance
        let display = WindowBuilder::new()
            .with_depth_buffer(24)
            //.with_multisampling(16) // multisampling doesn't work on chromebook
            .with_title(title)
            .with_vsync()
            .with_fullscreen(get_primary_monitor())
            .build_glium()
            .unwrap();

        // create a text system instance and font
        let text_system = TextSystem::new(&display);
        let font = FontTexture::new(&display, &include_bytes!("./resources/font.otf")[..], 100).unwrap();

        let mut imgui = ImGui::init();
        let imgui_rend = ImGuiRenderer::init(&mut imgui, &display).unwrap();

        let shaders = Shaders::new(&display);
        let post_fx = PostEffect::new(&display);

        let renderer = Renderer {
            display: display,
            text_system: text_system,
            default_font: font,
            imgui: imgui,
            imgui_rend: imgui_rend,
            post_effect: post_fx,
            start_time: time::precise_time_s(),
            shaders: shaders,
        };

        renderer.setup();

        renderer
    }

    /// Sets up the render window
    pub fn setup(&self) {
        // get the window for various values
        let window = self.display.get_window().unwrap();
        //window.set_cursor_state(Grab).ok();
        window.set_cursor_state(Hide).ok();
    }

    pub fn update_imgui_input(&mut self, pos: (i32, i32), btns: (bool, bool, bool)) {
        self.imgui.set_mouse_pos(pos.0 as f32, pos.1 as f32);
        self.imgui.set_mouse_down(&[btns.0, btns.1, btns.2, false, false]);
        //self.imgui.set_mouse_wheel(self.mouse_wheel);
    }

    /// Draws a frame
    pub fn draw<'ui, 'a: 'ui, F: FnMut(&Ui<'ui>)>(&'a mut self,
                                                  cam_state: &CamState,
                                                  render_items: &Vec<RenderItem>,
                                                  text_items: &Vec<TextItem>,
                                                  mut f: F) {
        // get display dimensions
        let (width, height) = self.display.get_framebuffer_dimensions();

        // draw parameters
        let params = DrawParameters {
            depth: Depth {
                test: IfLess,
                write: true,
                .. Default::default()
            },
            backface_culling: CullClockwise,
            .. Default::default()
        };

        let uniforms = uniform! {
            projection_matrix: Renderer::build_persp_proj_mat(60f32, width as f32/height as f32, 0.01f32, 1000f32),
            modelview_matrix: Renderer::build_fp_view_matrix(cam_state),
            cam_pos: cam_state.cam_pos,
            time: (time::precise_time_s() - self.start_time) as f32,
        };

        // drawing a frame
        let mut target = self.display.draw();

        render_post(&self.post_effect,
                    &self.shaders.post_shaders.get(self.post_effect.current_shader).unwrap(),
                    &mut target,
                    |target| {

            target.clear_color_and_depth((1.0, 1.0, 1.0, 1.0), 1.0);

            // drawing the render items TODO batching
            for item in render_items.iter() {
                // building the vertex and index buffers TODO possibly not create every frame
                let vertex_buffer = VertexBuffer::new(&self.display, &item.vertices).unwrap();

                // add positions for instances TODO possibly not create every frame
                let per_instance = {
                    let data = item.instance_transforms.iter().map(|t| {
                        Attr {
                            world_position: t.pos,
                            world_rotation: t.rot,
                            world_scale: t.scale
                        }
                    }).collect::<Vec<_>>();

                    VertexBuffer::dynamic(&self.display, &data).unwrap()
                };

                target.draw(
                    (&vertex_buffer, per_instance.per_instance().unwrap()),
                    &NoIndices(PrimitiveType::Patches { vertices_per_patch: 3 }),
                    &self.shaders.shaders.get(item.shader_name).unwrap(),
                    &uniforms,
                    &params).unwrap();
            }
        });

        // drawing the text items
        for text_item in text_items.iter() {
            // create the matrix for the text
            let matrix = [
                [0.02 * text_item.scale.0, 0.0, 0.0, 0.0],
                [0.0, 0.02 * text_item.scale.1 * (width as f32) / (height as f32), 0.0, 0.0],
                [0.0, 0.0, 0.02 * text_item.scale.2, 0.0],
                    [text_item.pos.0, text_item.pos.1, text_item.pos.2, 1.0f32]
            ];

            // create TextDisplay for item, TODO change this to not be done every frame
            let text = TextDisplay::new(&self.text_system, &self.default_font,
                                        text_item.text.as_str());

            glium_text::draw(&text,
                             &self.text_system,
                             &mut target,
                             matrix,
                             text_item.color);
        }

        // imgui elements
        let ui = self.imgui.frame(width, height, 0.1);
        f(&ui);
        self.imgui_rend.render(&mut target, ui).unwrap();


        match target.finish() {
            Ok(_) => {},
            Err(e) => println!("{:?}", e),
        };
    }

    /// Returns perspective projection matrix given fov, aspect ratio, z near and far
    pub fn build_persp_proj_mat(fov:f32,aspect:f32,znear:f32,zfar:f32) -> [[f32; 4]; 4] {
        let ymax = znear * (fov * (PI/360.0)).tan();
        let ymin = -ymax;
        let xmax = ymax * aspect;
        let xmin = ymin * aspect;

        let width = xmax - xmin;
        let height = ymax - ymin;

        let depth = zfar - znear;
        let q = -(zfar + znear) / depth;
        let qn = -2.0 * (zfar * znear) / depth;

        let w = 2.0 * znear / width;
        let h = 2.0 * znear / height;

        [
            [w, 0.0f32, 0.0f32, 0.0f32],
            [0.0f32, h, 0.0f32, 0.0f32],
            [0.0f32, 0.0f32, q, -1.0f32],
                [0.0f32, 0.0f32, qn, 0.0f32]
        ]
    }

    /// Returns the model view matrix for a first person view given cam position and rotation
    pub fn build_fp_view_matrix(cam_state: &CamState) -> [[f32; 4]; 4] {

        let (sin_yaw, cos_yaw, sin_pitch, cos_pitch) = (
            cam_state.cam_rot.1.sin(),
            cam_state.cam_rot.1.cos(),
            cam_state.cam_rot.0.sin(),
            cam_state.cam_rot.0.cos());
        let xaxis = [cos_yaw, 0.0, -sin_yaw];
        let yaxis = [sin_yaw * sin_pitch, cos_pitch, cos_yaw * sin_pitch];
        let zaxis = [sin_yaw * cos_pitch, -sin_pitch, cos_pitch * cos_yaw];

        let cam_arr = [cam_state.cam_pos.0, cam_state.cam_pos.1, cam_state.cam_pos.2];

        [
            [ xaxis[0], yaxis[0], zaxis[0], 0.0],
            [ xaxis[1], yaxis[1], zaxis[1], 0.0],
            [ xaxis[2], yaxis[2], zaxis[2], 0.0],
                [ dotp(&xaxis, &cam_arr), dotp(&yaxis, &cam_arr), dotp(&zaxis, &cam_arr), 1.0f32]
        ]
    }
}
