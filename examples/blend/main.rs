// Copyright 2015 The Gfx-rs Developers.
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

#[macro_use]
extern crate gfx;
extern crate gfx_app;
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



pub use gfx::format::{Rgba8, Srgba8, DepthStencil};

gfx_vertex_struct!( Vertex {
    pos: [f32; 2] = "a_Pos",
    uv: [f32; 2] = "a_Uv",
});

impl Vertex {
    fn new(p: [f32; 2], u: [f32; 2]) -> Vertex {
        Vertex {
            pos: p,
            uv: u,
        }
    }
}

gfx_constant_struct!( Locals {
    blend: i32 = "u_Blend",
});

gfx_pipeline!( pipe {
    vbuf: gfx::VertexBuffer<Vertex> = (),
    lena: gfx::TextureSampler<[f32; 4]> = "t_Lena",
    tint: gfx::TextureSampler<[f32; 4]> = "t_Tint",
    blend: gfx::Global<i32> = "i_Blend",
    locals: gfx::ConstantBuffer<Locals> = "Locals",
    out: gfx::RenderTarget<Srgba8> = "Target0",
});

fn load_texture<R, F>(factory: &mut F, data: &[u8])
                -> Result<gfx::handle::ShaderResourceView<R, [f32; 4]>, String> where
                R: gfx::Resources, F: gfx::Factory<R> {
    use std::io::Cursor;
    use gfx::tex as t;
    let img = image::load(Cursor::new(data), image::PNG).unwrap().to_rgba();
    let (width, height) = img.dimensions();
    let kind = t::Kind::D2(width as t::Size, height as t::Size, t::AaMode::Single);
    let (_, view) = factory.create_texture_const_u8::<Rgba8>(kind, &[&img]).unwrap();
    Ok(view)
}

const BLENDS: [&'static str; 9] = [
    "Screen",
    "Dodge",
    "Burn",
    "Overlay",
    "Multiply",
    "Add",
    "Divide",
    "Grain Extract",
    "Grain Merge",
];

struct App<R: gfx::Resources>{
    bundle: pipe::Bundle<R>,
    id: Rc<Cell<u8>>,
    view: Option<View>,
}

impl<R: gfx::Resources> gfx_app::Application<R> for App<R> {
    fn new<F: gfx::Factory<R>>(mut factory: F, init: gfx_app::Init<R>) -> Self {
        use gfx::traits::FactoryExt;

        let vs = gfx_app::shade::Source {
            glsl_120: include_bytes!("shader/blend_120.glslv"),
            glsl_150: include_bytes!("shader/blend_150.glslv"),
            hlsl_40:  include_bytes!("data/vertex.fx"),
            .. gfx_app::shade::Source::empty()
        };
        let ps = gfx_app::shade::Source {
            glsl_120: include_bytes!("shader/blend_120.glslf"),
            glsl_150: include_bytes!("shader/blend_150.glslf"),
            hlsl_40:  include_bytes!("data/pixel.fx"),
            .. gfx_app::shade::Source::empty()
        };

        // fullscreen quad
        let vertex_data = [
            Vertex::new([-1.0, -1.0], [0.0, 1.0]),
            Vertex::new([ 1.0, -1.0], [1.0, 1.0]),
            Vertex::new([ 1.0,  1.0], [1.0, 0.0]),

            Vertex::new([-1.0, -1.0], [0.0, 1.0]),
            Vertex::new([ 1.0,  1.0], [1.0, 0.0]),
            Vertex::new([-1.0,  1.0], [0.0, 0.0]),
        ];
        let (vbuf, slice) = factory.create_vertex_buffer(&vertex_data);

        let lena_texture = load_texture(&mut factory, &include_bytes!("image/lena.png")[..]).unwrap();
        let tint_texture = load_texture(&mut factory, &include_bytes!("image/tint.png")[..]).unwrap();
        let sampler = factory.create_sampler_linear();

        let pso = factory.create_pipeline_simple(
            vs.select(init.backend).unwrap(),
            ps.select(init.backend).unwrap(),
            gfx::state::CullFace::Nothing,
            pipe::new()
        ).unwrap();

        // we pass a integer to our shader to show what blending function we want
        // it to use. normally you'd have a shader program per technique, but for
        // the sake of simplicity we'll just branch on it inside the shader.

        // each index correspond to a conditional branch inside the shader
        println!("Using '{}' blend equation", BLENDS[0]);
        let cbuf = factory.create_constant_buffer(1);

        let data = pipe::Data {
            vbuf: vbuf,
            lena: (lena_texture, sampler.clone()),
            tint: (tint_texture, sampler),
            blend: 0,
            locals: cbuf,
            out: init.color,
        };

        App {
            bundle: pipe::bundle(slice, pso, data),
            id: Rc::new(Cell::new(0)),
            view: None,
        }
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

        let handler = Handler { host: Rc::downgrade(&host.clone()), blend: self.id.clone() };
        host.attach_handler(handler);

        let blends: sciter::Value = BLENDS.iter().cloned().collect();
        root.call_function("setupBlending", &[blends]).ok();

      } else {
        println!("oops: no root element!");
      }
    }

    fn render<C: gfx::CommandBuffer<R>>(&mut self, encoder: &mut gfx::Encoder<R, C>) {
      let locals = Locals { blend: self.id.get() as i32 };
      encoder.update_constant_buffer(&self.bundle.data.locals, &locals);
      encoder.clear(&self.bundle.data.out, [0.0; 4]);
      self.bundle.encode(encoder);
    }

    fn render_post<C: gfx::CommandBuffer<R>>(&mut self, _encoder: &mut gfx::Encoder<R, C>) -> bool {
      self.render_document();
      return true;
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
  blend: Rc<Cell<u8>>,
}

impl sciter::EventHandler for Handler {

  #[allow(unused_variables)]
  fn on_script_call(&mut self, root: sciter::HELEMENT, name: &str, args: &[sciter::Value]) -> Option<sciter::Value> {
    let ok = sciter::Value::from(true);
    match name {
      "setBlending" => {
        let id = args[0].to_int().unwrap();
        println!("Using '{}' blend equation", BLENDS[id as usize]);
        self.blend.set(id as u8);
        Some(ok)
      },
      _ => None,
    }
  }
}

pub fn main() {
    use gfx_app::Application;
    App::launch_default("Blending example");
}
