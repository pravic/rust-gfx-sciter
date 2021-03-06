// Copyright 2016 The Gfx-rs Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate time;

#[macro_use]
extern crate gfx;
extern crate gfx_app;
extern crate cgmath;

extern crate image;

extern crate sciter;

use std::any::Any;
use std::cell::Cell;
use std::rc::{Rc, Weak};

type SciterHost = Rc<sciter::Host>;

struct View {
    api: &'static sciter::ISciterAPI,
    hwnd: sciter::types::HWINDOW,
}



use std::io::Cursor;
pub use gfx::format::{Srgba8, Depth, Rgba8};

gfx_vertex_struct!( Vertex {
    pos: [f32; 2] = "a_Pos",
});

impl Vertex {
    fn new(p: [f32; 2]) -> Vertex {
        Vertex {
            pos: p,
        }
    }
}

gfx_constant_struct!( Locals {
    inv_proj: [[f32; 4]; 4] = "u_InvProj",
    view: [[f32; 4]; 4] = "u_WorldToCamera",
});

gfx_pipeline!( pipe {
    vbuf: gfx::VertexBuffer<Vertex> = (),
    cubemap: gfx::TextureSampler<[f32; 4]> = "t_Cubemap",
    locals: gfx::ConstantBuffer<Locals> = "Locals",
    out: gfx::RenderTarget<Srgba8> = "Target0",
});

struct CubemapData<'a> {
    up: &'a [u8],
    down: &'a [u8],
    front: &'a [u8],
    back: &'a [u8],
    right: &'a [u8],
    left: &'a [u8],
}

impl<'a> CubemapData<'a> {
    fn as_array(self) -> [&'a [u8]; 6] {
        [self.right, self.left, self.up, self.down, self.front, self.back]
    }
}

fn load_cubemap<R, F>(factory: &mut F, data: CubemapData) -> Result<gfx::handle::ShaderResourceView<R, [f32; 4]>, String>
        where R: gfx::Resources, F: gfx::Factory<R>
{
    let images = data.as_array().iter().map(|data| {
        image::load(Cursor::new(data), image::JPEG).unwrap().to_rgba()
    }).collect::<Vec<_>>();
    let data: [&[u8]; 6] = [&images[0], &images[1], &images[2], &images[3], &images[4], &images[5]];
    let kind = gfx::tex::Kind::Cube(images[0].dimensions().0 as u16);
    match factory.create_texture_const_u8::<Rgba8>(kind, &data) {
        Ok((_, view)) => Ok(view),
        Err(_) => Err("Unable to create an immutable cubemap texture".to_owned()),
    }
}

struct App<R: gfx::Resources>{
    bundle: pipe::Bundle<R>,
    projection: cgmath::Matrix4<f32>,
    speed: Rc<Cell<f32>>,
    view: Option<View>,
}

impl<R: gfx::Resources> gfx_app::Application<R> for App<R> {
    fn new<F: gfx::Factory<R>>(mut factory: F, init: gfx_app::Init<R>) -> Self {
        use gfx::traits::FactoryExt;

        let vs = gfx_app::shade::Source {
            glsl_150: include_bytes!("shader/cubemap_150.glslv"),
            hlsl_40:  include_bytes!("data/vertex.fx"),
            .. gfx_app::shade::Source::empty()
        };
        let ps = gfx_app::shade::Source {
            glsl_150: include_bytes!("shader/cubemap_150.glslf"),
            hlsl_40:  include_bytes!("data/pixel.fx"),
            .. gfx_app::shade::Source::empty()
        };

        let vertex_data = [
            Vertex::new([-1.0, -1.0]),
            Vertex::new([ 3.0, -1.0]),
            Vertex::new([-1.0,  3.0])
        ];
        let (vbuf, slice) = factory.create_vertex_buffer(&vertex_data);

        let cubemap = load_cubemap(&mut factory, CubemapData {
            up: &include_bytes!("image/posy.jpg")[..],
            down: &include_bytes!("image/negy.jpg")[..],
            front: &include_bytes!("image/posz.jpg")[..],
            back: &include_bytes!("image/negz.jpg")[..],
            right: &include_bytes!("image/posx.jpg")[..],
            left: &include_bytes!("image/negx.jpg")[..],
        }).unwrap();

        let sampler = factory.create_sampler_linear();

        let proj = cgmath::perspective(cgmath::deg(60.0f32), init.aspect_ratio, 0.01, 100.0);

        let pso = factory.create_pipeline_simple(
            vs.select(init.backend).unwrap(),
            ps.select(init.backend).unwrap(),
            gfx::state::CullFace::Nothing,
            pipe::new()
        ).unwrap();

        let data = pipe::Data {
            vbuf: vbuf,
            cubemap: (cubemap, sampler),
            locals: factory.create_constant_buffer(1),
            out: init.color,
        };

        App {
            bundle: pipe::bundle(slice, pso, data),
            projection: proj,
            view: None,
            speed: Rc::new(Cell::new(0.25)),
        }
    }

    fn render<C: gfx::CommandBuffer<R>>(&mut self, encoder: &mut gfx::Encoder<R, C>) {
        {
            use cgmath::{AffineMatrix3, SquareMatrix, Transform, Vector3, Point3};
            // Update camera position
            let time = time::precise_time_s() as f32 * self.speed.get();
            let x = time.sin();
            let z = time.cos();

            let view: AffineMatrix3<f32> = Transform::look_at(
                Point3::new(x, x / 2.0, z),
                Point3::new(0.0, 0.0, 0.0),
                Vector3::unit_y(),
            );

            let locals = Locals {
                inv_proj: self.projection.invert().unwrap().into(),
                view: view.mat.into(),
            };
            encoder.update_constant_buffer(&self.bundle.data.locals, &locals);
        }

        encoder.clear(&self.bundle.data.out, [0.3, 0.3, 0.3, 1.0]);
        self.bundle.encode(encoder);
    }

    fn render_post<C: gfx::CommandBuffer<R>>(&mut self, _encoder: &mut gfx::Encoder<R, C>) -> bool {
      self.render_document();
      return true;
    }

    fn setup<WindowHost: Any>(&mut self, host: &WindowHost) {

      let any = host as &Any;
      if !any.is::<SciterHost>() {
        return;
      }
      let host = any.downcast_ref::<SciterHost>().unwrap();

      // load UI from html
      let ui = include_bytes!("facade.htm");
      host.load_html(ui, None);

      // attach root handler
      if let Some(root) = host.get_root() {
        println!("document loaded: {}", root);

        let api: &'static sciter::ISciterAPI = sciter::SciterAPI();
        self.view = Some(View { api: api, hwnd: host.get_hwnd() });

        let handler = Handler { host: Rc::downgrade(&host.clone()), speed: self.speed.clone() };
        host.attach_handler(handler);

      } else {
        println!("oops: no root element!");
      }
    }
}



impl<R: gfx::Resources> App<R> {

  fn render_document(&mut self) {
    if self.view.is_none() {
      return;
    }
    use sciter::types::BOOL;
    let view = self.view.as_ref().unwrap();
    let el = 0 as sciter::HELEMENT;
    (view.api.SciterRenderOnDirectXWindow)(view.hwnd, el, false as BOOL);
    // assert_eq!(ok, 1);
  }
}

#[allow(dead_code)]
struct Handler {
  host: Weak<sciter::Host>,
  speed: Rc<Cell<f32>>,
}

impl sciter::EventHandler for Handler {

  #[allow(unused_variables)]
  fn on_script_call(&mut self, root: sciter::HELEMENT, name: &str, args: &[sciter::Value]) -> Option<sciter::Value> {
    let ok = sciter::Value::from(true);
    match name {
      "setSpeed" => {
        let id = args[0].to_float().unwrap();
        self.speed.set(id as f32);
        Some(ok)
      },
      _ => None,
    }
  }
}

pub fn main() {
    use gfx_app::Application;
    App::launch_default("Skybox example");
}
