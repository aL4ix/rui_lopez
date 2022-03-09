extern crate sdl2;

use std::default::Default;

use rui_lopez::elements::*;

pub fn main() -> Result<(), String> {
    let window = Window {
        title: "Hello World".to_string(),
        menu: Some(MainMenu {
            menu: Menu {
                title: "File".to_string(),
                children: vec![Submenu::MenuItem(MenuItem {
                    title: "Open".to_string()
                }), Submenu::MenuItem(MenuItem {
                    title: "Exit".to_string()
                })],
            }
        }),
        container: Some(Container {
            children: vec![Box::new(TextField {
                ..Default::default()
            }), Box::new(Button {
                on_action: |event| {
                    println!("Clicked! {:?}", &event);
                    true
                },
                ..Default::default()
            })],
            ..Default::default()
        }),
        ..Default::default()
    };
    println!("{:?}", window);
    rui_lopez::engines::sdl::main_loop(vec![window])
}