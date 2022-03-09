use std::fmt::Debug;

use crate::elements::Dimension::Relative;

pub trait NativeDrawable: mopa::Any + Debug + private::Sealed {}
mopafy!(NativeDrawable);

mod private {
    use crate::engines::sdl::SDLBody;

    pub trait Sealed {}

    impl Sealed for SDLBody {}
}

// pub trait NativeFonts: mopa::Any {}
// mopafy!(NativeFonts);

pub trait Component: Debug + mopa::Any {
    fn get_height(&self) -> &Dimension;
    fn get_width(&self) -> &Dimension;
    fn build_dyn(&self, parent: &dyn Component) -> Box<dyn NativeDrawable>;
    fn clone_dyn(&self) -> Box<dyn Component>;
}
mopafy!(Component);

impl Clone for Box<dyn Component> {
    fn clone(&self) -> Self {
        self.clone_dyn()
    }
}

#[derive(Debug, Clone)]
pub enum Submenu {
    Menu(Menu),
    MenuItem(MenuItem),
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub title: String,
    pub children: Vec<Submenu>,
}


#[derive(Debug, Clone)]
pub struct MainMenu {
    pub menu: Menu,
}

// pub struct PopupMenu {
//     menu: Menu
// }

#[derive(Debug, Clone)]
pub enum Dimension {
    Relative(i32),
    Percentage(i32),
    Pixels(i32),
}

#[derive(Debug, Clone)]
pub struct Container {
    pub width: Dimension,
    pub height: Dimension,
    pub children: Vec<Box<dyn Component>>,
}

impl Default for Container {
    fn default() -> Self {
        Container {
            width: Relative(-1),
            height: Relative(-1),
            children: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatusBar {}

#[derive(Debug, Clone)]
pub struct Window {
    pub title: String,
    pub menu: Option<MainMenu>,
    pub container: Option<Container>,
    pub status_bar: Option<StatusBar>,
    pub height: Dimension,
    pub width: Dimension,
}

impl Default for Window {
    fn default() -> Self {
        Window {
            title: "RUI Lopez".to_string(),
            menu: None,
            container: None,
            status_bar: None,
            height: Relative(-1),
            width: Relative(-1),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Event {}

#[derive(Debug, Clone)]
pub struct Button {
    pub title: String,
    pub on_action: fn(Event) -> bool,
}

impl Default for Button {
    fn default() -> Self {
        Button {
            title: "Button".to_string(),
            on_action: |_event| true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextField {
    pub text: String,
    pub editable: bool,
}

impl Default for TextField {
    fn default() -> Self {
        TextField {
            text: "TextField".to_string(),
            editable: false,
        }
    }
}

struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8
}