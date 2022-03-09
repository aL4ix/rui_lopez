pub mod sdl {
    use std::collections::HashMap;
    use std::fmt::{Debug, Formatter};
    use std::ptr;
    use std::time::Duration;

    use sdl2::{Sdl, sys, VideoSubsystem};
    use sdl2::event::Event;
    use sdl2::keyboard::Keycode;
    use sdl2::pixels::Color;
    use sdl2::render::{Texture, WindowCanvas};
    use sdl2::ttf::Sdl2TtfContext;

    use crate::elements::*;

//Structs and Traits *******************************************************************************

    #[derive(Clone)]
    pub struct SDLPolygon {
        pub vers: Vec<sys::SDL_Vertex>,
        pub inds: Vec<i32>,
    }

    impl Debug for SDLPolygon {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let vers = self.vers.iter()
                .map(|v| format!("SDL_Vertex {{ x: {}, y: {} }}", v.position.x, v.position.y))
                .collect::<Vec<String>>().join(", ");
            let inds = self.inds.iter()
                .map(|i| format!("{}", i))
                .collect::<Vec<String>>().join(" ,");
            write!(f, "SDLOtherPoly {{ vers: [{}], inds: [{}] }}", vers, inds)
        }
    }

    /// A representation of SDL's geometry as defined in SDL_RenderGeometry
    ///
    /// # Examples
    ///
    /// ```
    ///
    #[derive(Clone)]
    pub struct SDLTexturedPolygon {
        pub poly: SDLPolygon,
        pub tex: Option<sys::SDL_Texture>,
    }

    impl Debug for SDLTexturedPolygon {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let poly = format!("{:?}", self.poly);
            write!(f, "SDLPolygon {{ {} }}", poly)
        }
    }

    /// It's a group of separate polygons
    #[derive(Debug, Clone)]
    pub struct SDLBody {
        _name: String,
        polygons: Vec<SDLTexturedPolygon>,
    }

    /// This SDL engine will only use SDLBody as NativeDrawable, this is enforced by NativeDrawable
    /// having a private::Sealed trait and BuiltWindow.render() only accepting Vec<SDLBody>
    impl NativeDrawable for SDLBody {}

    /// This is the SDL version of the rather dynamic Component trait, SDL components use this
    /// version whenever is possible instead of Component.build_dyn(), which is only used by
    /// Container because it can contain any type of Component
    pub trait SDLComponent: Debug + Component {
        fn build(&self, parent: &dyn Component) -> SDLBody;
    }

    pub struct SDLFontsCache<'ttf_module> {
        pub ttf_context: &'ttf_module Sdl2TtfContext,
        cache: HashMap<String, sdl2::ttf::Font<'ttf_module, 'static>>,
    }

    unsafe impl<'ttf_module> Send for SDLFontsCache<'ttf_module> {}

    //unsafe impl Sync for SDLFont<'_> {}
    impl<'ttf_module> SDLFontsCache<'ttf_module> {
        fn new(ttf_context: &'ttf_module Sdl2TtfContext) -> Self {
            SDLFontsCache {
                ttf_context,
                cache: HashMap::new(),
            }
        }
    }

    // impl NativeFonts for SDLFontsCache<'static> {}
    //
    // trait DynNativeFontToSDLFontsCache<'ttf_module> {
    //     fn dyn_to_sdl_fonts_cache(self) -> SDLFontsCache<'ttf_module>;
    // }
    //
    // impl<'ttf_module> DynNativeFontToSDLFontsCache<'ttf_module> for Box<dyn NativeFonts> {
    //     fn dyn_to_sdl_fonts_cache(self) -> SDLFontsCache<'ttf_module> {
    //         *self.downcast::<SDLFontsCache>().expect("Only SDLBody should be NativeDrawable!")
    //     }
    // }

    /// A struct to contain all the necessary initialized SDL objects
    pub struct SDLContextAndSubsystems {
        pub context: Sdl,
        pub video: VideoSubsystem,
        pub ttf: Box<Sdl2TtfContext>,
    }

    /// This is a little fun here, since Component.build_dyn() returns the dynamic NativeDrawable
    /// trait they need to be casted back to SDLBody
    trait DynNativeDrawableToSDLBody {
        fn dyn_to_sdl_body(self) -> SDLBody;
    }

    /// Box has the ability to downcast to a concrete type only when the contained trait is Any, so
    /// NativeDrawable has an Any trait, which is mopa::Any
    ///
    /// # Panics
    /// When provided NativeDrawable is not a SDLBody
    impl DynNativeDrawableToSDLBody for Box<dyn NativeDrawable> {
        fn dyn_to_sdl_body(self) -> SDLBody {
            *self.downcast::<SDLBody>().expect("Only SDLBody should be NativeDrawable!")
        }
    }

    // trait DynComponentToSDLComponent {
    //     fn dyn_to_sdl_component(self) -> dyn SDLComponent;
    // }
    //
    // impl DynComponentToSDLComponent for Box<dyn Component> {
    //     fn dyn_to_sdl_component(self) -> dyn SDLComponent {
    //         *self.downcast::<dyn SDLComponent>().expect("Only SDLComponent should be Component!")
    //     }
    // }


    // Globals *************************************************************************************

    // none yeah!

    // Functions ***********************************************************************************

    /// Initializes the context and subsystems
    pub fn init() -> Result<SDLContextAndSubsystems, String> {
        let context = sdl2::init()?;
        let video = context.video()?;
        let ttf = Box::new(sdl2::ttf::init().map_err(|e| e.to_string())?);
        Ok(SDLContextAndSubsystems {
            context,
            video,
            ttf,
        })
    }

    /// A blocking main_loop() for the cases when non-blocking is not necessary. You could also use
    /// BuiltWindow.render() to tell the GUI when you want to render so you are in control of the
    /// loop, specially useful in multimedia applications
    pub fn main_loop(windows: Vec<Window>) -> Result<(), String> {
        let window = &windows[0];
        let sdl_ctx = init()?;
        let mut sdl_window = SDLWindow::new(window, &sdl_ctx)?;
        let drawables = sdl_window.build(window);
        println!("{:?}", drawables);

        let font_name = "Nouveau_IBM.ttf";
        let font = sdl_ctx.ttf.load_font(font_name, 20)?;

        let surface = font.render("Hello").solid(Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }).map_err(|e| e.to_string())?;
        let creator = sdl_window.canvas.texture_creator();
        let mut texture = surface.as_texture(&creator)
            .map_err(|e| e.to_string())?;


        let mut event_pump = sdl_ctx.context.event_pump()?;
        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => break 'running,
                    _ => {}
                }
            }

            sdl_window.render(&drawables, &mut texture)?;
            ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 30));
        }
        Ok(())
    }

    // Window **************************************************************************************

    impl Window {}

    // SDLWindow ***********************************************************************************

    /// BuiltWindow (not to be confused with Window) is the result of Window.init(), it contains
    /// the necessary data to keep track of the components, their evolutions and how to draw them
    pub struct SDLWindow<'ttf_module> {
        old_window: Window,
        canvas: sdl2::render::WindowCanvas,
        fonts: SDLFontsCache<'ttf_module>,
        components: Vec<SDLBody>,
    }

    impl<'ttf_module> SDLWindow<'ttf_module> {
        /// It does all the things that are only necessary to do once, like creating the SDL window
        /// and returning an already SDLWindow
        pub fn new<'a>(window: &Window, sdl_ctx: &'a SDLContextAndSubsystems)
                       -> Result<SDLWindow<'a>, String> {
            let sdl_window = sdl_ctx.video
                .window(window.title.as_str(), 800, 600)
                .build()
                .map_err(|e| e.to_string())?;
            let canvas = sdl_window.into_canvas().build().map_err(|e| e.to_string())?;

            let mut fonts = SDLFontsCache::new(&sdl_ctx.ttf);
            let font_name = "Nouveau_IBM.ttf";
            let font = sdl_ctx.ttf.load_font(font_name, 20)?;
            fonts.cache.insert(font_name.to_string(),
                               font);
            let font = fonts.cache.get(font_name).unwrap();

            Ok(SDLWindow {
                old_window: window.clone(),
                canvas,
                fonts,
                components: vec![],
            })
        }

        /// This is where the magic happens, the Window model and its children are taken and
        /// converted into SDLBody (trait NativeDrawable)
        pub fn build(&self, window: &Window) -> Vec<SDLBody> {
            let pseudo = RUIIcon {};
            let icon = RUIIcon {}.build(&pseudo);
            let mut res = vec![icon];
            if let Some(menu) = &window.menu {
                res.push(menu.build(&pseudo));
            }
            // if let Some(container) = &window.container {
            //     res.push(container.build(&pseudo));
            // }
            res
        }

        /// It takes many SDLBody (trait NativeDrawable) and renders them by using SDL
        pub fn render(&mut self, drawables: &Vec<SDLBody>, texture: &Texture) -> Result<(), String> {
            let canvas = &mut self.canvas;
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas.clear();
            for body in drawables.iter() {
                for tex_poly in body.polygons.iter() {
                    SDLWindow::render_geometry(canvas, tex_poly.tex,
                                               &tex_poly.poly.vers, &tex_poly.poly.inds)?;
                }
            }
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();
            Ok(())
        }

        /// This function shouldn't be here, SDL_RenderGeometry was introduced in SDL 2.0.18 but
        /// rust-sdl only support earlier versions so the binding for that function was to be done
        /// here since SDL_RenderGeometry is the basis of this engine
        fn render_geometry(canvas: &mut WindowCanvas, texture: Option<sys::SDL_Texture>,
                           vertices: &Vec<sys::SDL_Vertex>, indices: &Vec<i32>) -> Result<(), String> {
            if !vertices.is_empty() {
                let sdl_renderer = canvas.raw();
                let vers_num = vertices.len() as i32;
                let vers_ptr = (&vertices[0]) as *const sys::SDL_Vertex;
                let tex_ptr: *mut sys::SDL_Texture = match texture {
                    None => ptr::null_mut(),
                    Some(mut t) => &mut t as *mut sys::SDL_Texture
                };
                let ind_num = indices.len() as i32;
                let inds_ptr = match ind_num {
                    0 => ptr::null(),
                    _ => (&indices[0])
                };
                let ret = unsafe {
                    sys::SDL_RenderGeometry(sdl_renderer, tex_ptr, vers_ptr, vers_num, inds_ptr, ind_num)
                };
                if ret == -1 {
                    return Err(format!("Failed at SDL_RenderGeometry {}", sdl2::get_error()));
                }
            }

            Ok(())
        }
    }

    // MainMenu ************************************************************************************

    impl SDLComponent for MainMenu {
        fn build(&self, parent: &dyn Component) -> SDLBody {
            SDLBody {
                _name: "MainMenu".to_string(),
                polygons: vec![],
            }
        }
    }

    impl Component for MainMenu {
        fn get_height(&self) -> &Dimension {
            todo!()
        }

        fn get_width(&self) -> &Dimension {
            todo!()
        }

        fn build_dyn(&self, parent: &dyn Component) -> Box<dyn NativeDrawable> {
            Box::new(self.build(parent))
        }

        fn clone_dyn(&self) -> Box<dyn Component> {
            Box::new(self.clone())
        }
    }

    // Container ***********************************************************************************

    impl SDLComponent for Container {
        fn build(&self, parent: &dyn Component) -> SDLBody {
            // for child in self.children.iter() {
            //     child.dyn_to_sdl_body().
            // }
            todo!()
        }
    }

    impl Component for Container {
        fn get_height(&self) -> &Dimension {
            &self.height
        }

        fn get_width(&self) -> &Dimension {
            &self.width
        }

        fn build_dyn(&self, parent: &dyn Component) -> Box<dyn NativeDrawable> {
            Box::new(self.build(parent))
        }

        fn clone_dyn(&self) -> Box<dyn Component> {
            Box::new(self.clone())
        }
    }

    // RUIIcon *************************************************************************************

    #[derive(Debug, Clone)]
    struct RUIIcon;

    impl SDLComponent for RUIIcon {
        fn build(&self, parent: &dyn Component) -> SDLBody {
            let v0 = sys::SDL_Vertex {
                position: sys::SDL_FPoint {
                    x: 400.,
                    y: 150.,
                },
                color: sys::SDL_Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                tex_coord: sys::SDL_FPoint {
                    x: 0.,
                    y: 0.,
                },
            };
            let v1 = sys::SDL_Vertex {
                position: sys::SDL_FPoint {
                    x: 200.,
                    y: 450.,
                },
                color: sys::SDL_Color {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 255,
                },
                tex_coord: sys::SDL_FPoint {
                    x: 0.,
                    y: 0.,
                },
            };
            let v2 = sys::SDL_Vertex {
                position: sys::SDL_FPoint {
                    x: 600.,
                    y: 450.,
                },
                color: sys::SDL_Color {
                    r: 0,
                    g: 255,
                    b: 0,
                    a: 255,
                },
                tex_coord: sys::SDL_FPoint {
                    x: 0.,
                    y: 0.,
                },
            };

            SDLBody {
                _name: "RUIIcon".to_string(),
                polygons: vec![SDLTexturedPolygon {
                    poly: SDLPolygon {
                        vers: vec![v0, v1, v2],
                        inds: vec![],
                    },
                    tex: None,
                }],
            }
        }
    }

    impl Component for RUIIcon {
        fn get_height(&self) -> &Dimension {
            todo!()
        }

        fn get_width(&self) -> &Dimension {
            todo!()
        }

        fn build_dyn(&self, parent: &dyn Component) -> Box<dyn NativeDrawable> {
            Box::new(self.build(parent))
        }

        fn clone_dyn(&self) -> Box<dyn Component> {
            Box::new(self.clone())
        }
    }

    // SDLText *************************************************************************************

    #[derive(Debug, Clone)]
    struct SDLText {
        text: String,
        native_size: u32,
        poly: SDLTexturedPolygon,
    }

    impl SDLComponent for SDLText {
        fn build(&self, parent: &dyn Component) -> SDLBody {
            // TODO: Insert text code

            SDLBody {
                _name: "SDLText".to_string(),
                polygons: vec![],
            }
        }
    }

    impl Component for SDLText {
        fn get_height(&self) -> &Dimension {
            todo!()
        }

        fn get_width(&self) -> &Dimension {
            todo!()
        }

        fn build_dyn(&self, parent: &dyn Component) -> Box<dyn NativeDrawable> {
            Box::new(self.build(parent))
        }

        fn clone_dyn(&self) -> Box<dyn Component> {
            Box::new(self.clone())
        }
    }

    // Button **************************************************************************************

    impl SDLComponent for Button {
        fn build(&self, parent: &dyn Component) -> SDLBody {
            todo!()
        }
    }

    impl Component for Button {
        fn get_height(&self) -> &Dimension {
            todo!()
        }

        fn get_width(&self) -> &Dimension {
            todo!()
        }

        fn build_dyn(&self, parent: &dyn Component) -> Box<dyn NativeDrawable> {
            Box::new(self.build(parent))
        }

        fn clone_dyn(&self) -> Box<dyn Component> {
            Box::new(self.clone())
        }
    }

    // TextField ***********************************************************************************

    impl SDLComponent for TextField {
        fn build(&self, parent: &dyn Component) -> SDLBody {
            todo!()
        }
    }

    impl Component for TextField {
        fn get_height(&self) -> &Dimension {
            todo!()
        }

        fn get_width(&self) -> &Dimension {
            todo!()
        }

        fn build_dyn(&self, parent: &dyn Component) -> Box<dyn NativeDrawable> {
            Box::new(self.build(parent))
        }

        fn clone_dyn(&self) -> Box<dyn Component> {
            Box::new(self.clone())
        }
    }

    // Text ****************************************************************************************

    fn update_texture(rect: glyph_brush::Rectangle<u32>, tex_data: &[u8], texture: &mut Texture, color: &Color) {
        let format_enum = texture.query().format;
        let bytes_per_pixel = format_enum.byte_size_per_pixel();
        let pitch = bytes_per_pixel * rect.width() as usize;
        let pixel_format = sdl2::pixels::PixelFormat::try_from(format_enum)
            .expect("Failed to get a PixelFormat from PixelFormatEnum at update_texture()");
        let mut sdl_color = sdl2::pixels::Color {
            r: color.r,
            g: color.g,
            b: color.b,
            a: 0,
        };
        let mut new_data: Vec<u8> = vec![];
        for alpha in tex_data {
            sdl_color.a = *alpha;
            let native = sdl_color.to_u32(&pixel_format).to_ne_bytes();
            new_data.extend_from_slice(&native);
        }
        let r = sdl2::rect::Rect::new(rect.min[0].try_into().unwrap(), rect.min[1].try_into().unwrap(),
                                      rect.width(), rect.height());
        texture.update(r, &new_data, pitch).expect(
            &format!("Failed to update_texture() {}", sdl2::get_error()));
    }

    fn into_vertex(vd: glyph_brush::GlyphVertex) -> SDLPolygon {
        let global_alpha = 128;
        let v1 = sys::SDL_Vertex {
            position: sys::SDL_FPoint {
                x: vd.pixel_coords.min.x,
                y: vd.pixel_coords.min.y,
            },
            color: sys::SDL_Color {
                r: 255,
                g: 255,
                b: 255,
                a: global_alpha,
            },
            tex_coord: sys::SDL_FPoint {
                x: vd.tex_coords.min.x,
                y: vd.tex_coords.min.y,
            },
        };
        let v2 = sys::SDL_Vertex {
            position: sys::SDL_FPoint {
                x: vd.pixel_coords.min.x,
                y: vd.pixel_coords.max.y,
            },
            color: sys::SDL_Color {
                r: 255,
                g: 255,
                b: 255,
                a: global_alpha,
            },
            tex_coord: sys::SDL_FPoint {
                x: vd.tex_coords.min.x,
                y: vd.tex_coords.max.y,
            },
        };
        let v3 = sys::SDL_Vertex {
            position: sys::SDL_FPoint {
                x: vd.pixel_coords.max.x,
                y: vd.pixel_coords.max.y,
            },
            color: sys::SDL_Color {
                r: 255,
                g: 255,
                b: 255,
                a: global_alpha,
            },
            tex_coord: sys::SDL_FPoint {
                x: vd.tex_coords.max.x,
                y: vd.tex_coords.max.y,
            },
        };
        let v4 = sys::SDL_Vertex {
            position: sys::SDL_FPoint {
                x: vd.pixel_coords.max.x,
                y: vd.pixel_coords.min.y,
            },
            color: sys::SDL_Color {
                r: 255,
                g: 255,
                b: 255,
                a: global_alpha,
            },
            tex_coord: sys::SDL_FPoint {
                x: vd.tex_coords.max.x,
                y: vd.tex_coords.min.y,
            },
        };
        SDLPolygon {
            vers: vec![v1, v2, v3, v4],
            inds: vec![0, 1, 2, 2, 3, 0]
        }
    }
} // END mod sdl