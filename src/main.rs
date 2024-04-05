use std::fs;

use resolve_path::PathResolveExt;
use rmp_serde as rmps;
use serde::{Deserialize, Serialize};
use slint::Model;

slint::slint! {
    import {HorizontalBox, Button, TextEdit, VerticalBox, CheckBox} from "std-widgets.slint";

    export struct ListItem {
        completed:   bool,
        description: string,
    }

    export global AppConfig {
        out property    <length>     font-size: 14px;
        out property    <string>     data_path: ".todo.dat";
        in-out property <[ListItem]> list-items: [];
    }

    export global AppLogic {
        callback dump_list_items();
        callback load_list_items();
        callback pop_list_item(int) -> ListItem;
        callback put_list_item(int, ListItem);
    }

    export component App inherits Window {
        min-width: 480px;
        max-width: 600px;
        background: grey.darker(90%);

        VerticalBox {
            // List items live here.
            Rectangle {
                background: grey.darker(20%);
                min-height: 480px;
                min-width:  140px;

                border-top-left-radius: 5px;
                border-top-right-radius: 5px;

                VerticalLayout {
                    padding: 3px;

                    for list-item[i] in AppConfig.list-items : Rectangle {
                        padding: 5px;
                        Rectangle {
                            height: parent.height - (parent.height * 8%);
                            background: grey.darker(37%);

                            border-radius: 5px;
                            border-color: #75abe6;
                            border-width: 1px;
                            // Helps make selection of individual
                            // items more pronounced.
                            touch_area := TouchArea{}
                            drop-shadow-blur: touch-area.has-hover ? 10px : 3px;
                            drop-shadow-color: touch-area.has-hover ? grey.darker(87%) : grey.darker(50%);
                            drop-shadow-offset-y: touch-area.has-hover ? 5px : 1px;
                            animate drop-shadow-blur {
                                duration: 100ms;
                                easing: ease-out;
                            }

                            // Handles the interace for this list
                            // item.
                            HorizontalBox {
                                CheckBox {
                                    checked: list-item.completed;
                                    toggled => {
                                        list-item.completed = self.checked;
                                        AppLogic.dump_list_items();
                                    }
                                }
                                Text{
                                    text: list-item.description;
                                    font-size: AppConfig.font-size;
                                    color: lightgrey;
                                    horizontal-alignment: left;
                                    vertical-alignment: center;
                                    overflow: elide;
                                }
                                Button {
                                    text: "-";
                                    width: 30px;
                                    clicked => {
                                        AppLogic.pop_list_item(i);
                                        AppLogic.dump_list_items();
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Items created here.
            Rectangle {
                background: grey;
                height: AppConfig.font-size + (AppConfig.font-size * 300%);

                border-bottom-left-radius: 5px;
                border-bottom-right-radius: 5px;

                todo-form := HorizontalBox {
                    callback create_new_item();
                    add_button  := Button{
                        text: "+";
                        width: self.height;
                        // Only allow users to add a new item
                        // if the text input is not empty.
                        enabled: txt-desc.text != "";
                        clicked => { create_new_item(); }
                    }

                    txt_desc := Rectangle {
                        in-out property <string> text;
                        background: white;

                        border-radius: 5px;
                        border-color: #75abe6;
                        border-width: 2px;

                        TextInput {
                            color: grey;
                            font-size: AppConfig.font-size;
                            padding-right: 20px;
                            text <=> parent.text;
                            vertical-alignment: center;

                            x: parent.x - 50px;
                            width: parent.width - 12px;
                        }
                    }

                    create_new_item => {
                        if (txt-desc.text != "") {
                            AppLogic.put_list_item(0, {completed: false, description: txt-desc.text });
                            txt-desc.text = "";
                            AppLogic.dump_list_items();
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ListItemData {
    completed:   bool,
    description: String,
}

impl From<ListItemData> for ListItem {
    fn from(val: ListItemData) -> Self {
        ListItem{
            completed: val.completed,
            description: val.description.into()
        }
    }
}

impl From<ListItem> for ListItemData {
    fn from(value: ListItem) -> Self {
        Self{
            completed: value.completed,
            description: value.description.to_string()
        }
    }
}

fn callback_declare_dump_list_items(app: &App) {
    let logic = app.global::<AppLogic>();

    let weak_app = app.as_weak();
    logic.on_dump_list_items(move || {
        let app = weak_app.upgrade().unwrap();
        let cfg = app.global::<AppConfig>();

        let items: Vec<ListItemData> = cfg
            .get_list_items()
            .iter()
            .map(|li| li.into())
            .collect();
        let item_buf = rmps::to_vec(&items).unwrap();

        let data_path = cfg.get_data_path();
        fs::write(data_path.as_str().resolve(), item_buf)
            .map_err(|err| eprintln!("{err:?}"))
            .unwrap_or_default();
    });
}

fn callback_declare_load_list_items(app: &App) {
    let logic = app.global::<AppLogic>();

    let weak_app = app.as_weak();
    logic.on_load_list_items(move || {
        let app = weak_app.upgrade().unwrap();
        let cfg = app.global::<AppConfig>();

        let data_path = cfg.get_data_path();
        let data: Vec<u8> = fs::read(data_path.as_str().resolve())
            .map_err(|err| eprintln!("{err:?}"))
            .unwrap_or_default();
        // Bail since we found no data.
        if data.is_empty() { return; }

        let items: Vec<ListItem> =
            rmps::from_slice::<Vec<ListItemData>>(&data)
            .unwrap()
            .iter()
            .map(|li| li.to_owned().into())
            .collect();
        let items_model = std::rc::Rc::new(slint::VecModel::from(items));
        cfg.set_list_items(items_model.into());
    });

    logic.invoke_load_list_items();
}

fn callback_declare_pop_list_item(app: &App) {
    let logic = app.global::<AppLogic>();

    // Need to create weak references to our root
    // application in order to interact with it
    // from closures defined in our business
    // logic.
    let weak_app = app.as_weak();
    logic.on_pop_list_item(move |idx| {
        // Need to upgrade and unwrap the root
        // app, as well as acquire the global
        // config.
        let app = weak_app.upgrade().unwrap();
        let cfg = app.global::<AppConfig>();
        // Collect list items from global config.
        let mut items: Vec<ListItem> = cfg
            .get_list_items()
            .iter()
            .collect();
        // Properties, if changed at the business
        // logic level, need to be digested into
        // some Slint model and reset on the owning
        // object we procured it from.
        let item = items.remove(idx as usize);
        let items_model = std::rc::Rc::new(slint::VecModel::from(items));
        cfg.set_list_items(items_model.into());
        item
    });
}

fn callback_declare_put_list_item(app: &App) {
    let logic = app.global::<AppLogic>();

    let weak_app = app.as_weak();
    logic.on_put_list_item(move |idx, item| {
        let app = weak_app.upgrade().unwrap();
        let cfg = app.global::<AppConfig>();
        let mut items: Vec<ListItem> = cfg
            .get_list_items()
            .iter()
            .collect();
        items.insert(idx as usize, item);
        let items_model = std::rc::Rc::new(slint::VecModel::from(items));
        cfg.set_list_items(items_model.into());
    });
}

fn main() -> anyhow::Result<()> {
    let app = App::new()?;
    callback_declare_dump_list_items(&app);
    callback_declare_load_list_items(&app);
    callback_declare_pop_list_item(&app);
    callback_declare_put_list_item(&app);
    app.run()?;
    Ok(())
}