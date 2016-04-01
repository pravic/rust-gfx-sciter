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
extern crate sciter;

use std::rc::Rc;
use std::any::Any;

type SciterHost = Rc<sciter::Host>;



gfx_vertex_struct!( Vertex {
    pos: [f32; 2] = "a_Pos",
    color: [f32; 3] = "a_Color",
});

gfx_pipeline!(pipe {
    vbuf: gfx::VertexBuffer<Vertex> = (),
    out: gfx::RenderTarget<gfx::format::Srgba8> = "Target0",
});


struct View {
    api: &'static sciter::ISciterAPI,
    hwnd: sciter::types::HWINDOW,
    background: sciter::Element,
    foreground: sciter::Element,
}

struct App<R: gfx::Resources> {
    pso: gfx::PipelineState<R, pipe::Meta>,
    data: pipe::Data<R>,
    slice: gfx::Slice<R>,
    view: Option<View>,
}

impl<R: gfx::Resources> gfx_app::Application<R> for App<R> {
    fn new<F: gfx::Factory<R>>(mut factory: F, init: gfx_app::Init<R>) -> Self {
        use gfx::traits::FactoryExt;

        let vs = gfx_app::shade::Source {
            glsl_120: include_bytes!("shader/triangle_120.glslv"),
            glsl_150: include_bytes!("shader/triangle_150.glslv"),
            hlsl_40:  include_bytes!("data/vertex.fx"),
            .. gfx_app::shade::Source::empty()
        };
        let fs = gfx_app::shade::Source {
            glsl_120: include_bytes!("shader/triangle_120.glslf"),
            glsl_150: include_bytes!("shader/triangle_150.glslf"),
            hlsl_40:  include_bytes!("data/pixel.fx"),
            .. gfx_app::shade::Source::empty()
        };

        let vertex_data = [
            Vertex { pos: [ -0.5, -0.5 ], color: [1.0, 0.0, 0.0] },
            Vertex { pos: [  0.5, -0.5 ], color: [0.0, 1.0, 0.0] },
            Vertex { pos: [  0.0,  0.5 ], color: [0.0, 0.0, 1.0] },
        ];
        let (vbuf, slice) = factory.create_vertex_buffer(&vertex_data);

        App {
            pso: factory.create_pipeline_simple(
                vs.select(init.backend).unwrap(),
                fs.select(init.backend).unwrap(),
                gfx::state::CullFace::Nothing,
                pipe::new()
                ).unwrap(),
            data: pipe::Data {
                vbuf: vbuf,
                out: init.color,
            },
            slice: slice,
            view: Default::default(),
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
        let bg = root.find_first("section#back-layer");
        let fg = root.find_first("section#fore-layer");

        if bg.is_ok() && fg.is_ok() {
          let api: &'static sciter::ISciterAPI = sciter::SciterAPI();
          self.view = Some(View { api: api, hwnd: host.get_hwnd(), background: bg.unwrap().unwrap(), foreground: fg.unwrap().unwrap() });
        }

        let handler = Handler { host: Rc::downgrade(&host.clone()) };
        host.attach_handler(handler);

      } else {
        println!("oops: no root element!");
      }
    }

    fn render_pre<C: gfx::CommandBuffer<R>>(&mut self, encoder: &mut gfx::Encoder<R, C>) -> bool {
      encoder.clear(&self.data.out, [0.1, 0.2, 0.3, 1.0]);
      return true;
    }

    fn render<C: gfx::CommandBuffer<R>>(&mut self, encoder: &mut gfx::Encoder<R, C>) {
      self.render_layer(false);
      encoder.draw(&self.slice, &self.pso, &self.data);
    }

    fn render_post<C: gfx::CommandBuffer<R>>(&mut self, _encoder: &mut gfx::Encoder<R, C>) -> bool {
      self.render_layer(true);
      return true;
    }

}

impl<R: gfx::Resources> App<R> {

  #[allow(dead_code)]
  fn render_document(&mut self) {
    if self.view.is_none() {
      println!("wat??");
      return;
    }
    use sciter::types::BOOL;
    let view = self.view.as_ref().unwrap();
    let el = 0 as sciter::HELEMENT;
    let ok = (view.api.SciterRenderOnDirectXWindow)(view.hwnd, el, false as BOOL);
    assert_eq!(ok, 1);
  }

  fn render_layer(&mut self, foreground: bool) {
    if self.view.is_none() {
      return;
    }
    use sciter::types::BOOL;
    let view = self.view.as_ref().unwrap();
    let ok = if foreground == false {
      (view.api.SciterRenderOnDirectXWindow)(view.hwnd, view.background.as_ptr(), false as BOOL)
    } else {
      (view.api.SciterRenderOnDirectXWindow)(view.hwnd, view.foreground.as_ptr(), true as BOOL)
    };
    assert_eq!(ok, 1);
  }

}

#[allow(dead_code)]
struct Handler {
  host: ::std::rc::Weak<sciter::Host>,
}

impl sciter::EventHandler for Handler {

  #[allow(unused_variables)]
  fn on_script_call(&mut self, root: sciter::HELEMENT, name: &str, args: &[sciter::Value]) -> Option<sciter::Value> {
    use sciter::Value;

    let args = args.iter().map(|ref x| format!("{}", &x)).collect::<Vec<String>>().join(", ");
    println!("script->native: {}({}), root {:?}", name, args, root);

    let ok = Value::from(true);
    match name {
      "setRotationSpeed" => Some(ok),
      "setColorSpeed" => Some(ok),
      _ => None,
    }
  }
}

pub fn main() {
  use gfx_app::Application;
  App::launch_default("Sciter DirectX sample");
}
