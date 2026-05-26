use std::cell::Cell;
use std::io::ErrorKind;
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use gtk::prelude::*;
use gtk::{glib, glib::clone};
use gtk::{Application, ApplicationWindow};
use gtk::{
    Box, Button, ColorButton, ComboBoxText, Label, LinkButton, Scale, Stack, StackSwitcher, Switch,
    ToolItem, Toolbar,
};

// sudo apt install libgdk-pixbuf2.0-dev libcairo-dev libatk1.0-dev
// sudo apt install libpango1.0-dev

#[path = "../comms.rs"]
mod comms;
mod error_handling;
mod util;
mod widgets;

use error_handling::*;
use service::SupportedDevice;
use util::*;
use widgets::*;

const COOLING_PAD_TAB_POSITION: i32 = 3;
type CoolingPadState = (bool, i32, String, Vec<u8>);

#[derive(Clone)]
struct CoolingPadUi {
    fan_auto_switch: Switch,
    fan_scale: Scale,
    effect_options: ComboBoxText,
    color_row: gtk::ListBoxRow,
    color_picker: ColorButton,
    write_button: Button,
}

impl CoolingPadUi {
    fn apply_state(&self, state: &CoolingPadState) {
        let (present, fan_rpm, _, _) = state;
        sync_cooling_pad_controls(&self.fan_auto_switch, &self.fan_scale, *present, *fan_rpm);
        self.effect_options.set_sensitive(*present);
        let effect = cooling_pad_effect_name(self.effect_options.active().unwrap_or(0));
        self.color_row
            .set_visible(cooling_pad_effect_uses_color(effect));
        self.color_picker.set_sensitive(*present);
        self.write_button.set_sensitive(*present);
    }
}

fn send_data(opt: comms::DaemonCommand) -> Option<comms::DaemonResponse> {
    match comms::try_bind() {
        Ok(socket) => comms::send_to_daemon(opt, socket),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            crash_with_msg("Can't connect to the daemon");
        }
        Err(error) => {
            println!("Error opening socket: {error}");
            None
        }
    }
}

fn try_send_data(opt: comms::DaemonCommand) -> Option<comms::DaemonResponse> {
    let socket = comms::try_bind().ok()?;
    comms::send_to_daemon(opt, socket)
}

fn get_device_name() -> Option<String> {
    let response = send_data(comms::DaemonCommand::GetDeviceName)?;

    use comms::DaemonResponse::*;
    match response {
        GetDeviceName { name } => Some(name),
        response => {
            // This should not happen
            println!("Instead of GetDeviceName got {response:?}");
            None
        }
    }
}

fn get_bho() -> Option<(bool, u8)> {
    let response = send_data(comms::DaemonCommand::GetBatteryHealthOptimizer())?;

    use comms::DaemonResponse::*;
    match response {
        GetBatteryHealthOptimizer { is_on, threshold } => Some((is_on, threshold)),
        response => {
            // This should not happen
            println!("Instead of GetBatteryHealthOptimizer got {response:?}");
            None
        }
    }
}

fn set_bho(is_on: bool, threshold: u8) -> Option<bool> {
    let response = send_data(comms::DaemonCommand::SetBatteryHealthOptimizer { is_on, threshold })?;

    use comms::DaemonResponse::*;
    match response {
        SetBatteryHealthOptimizer { result } => Some(result),
        response => {
            // This should not happen
            println!("Instead of SetBatteryHealthOptimizer got {response:?}");
            None
        }
    }
}

fn get_brightness(ac: bool) -> Option<u8> {
    let ac = if ac { 1 } else { 0 };
    let response = send_data(comms::DaemonCommand::GetBrightness { ac })?;

    use comms::DaemonResponse::*;
    match response {
        GetBrightness { result } => Some(result),
        response => {
            // This should not happen
            println!("Instead of GetBrightness got {response:?}");
            None
        }
    }
}

fn set_brightness(ac: bool, val: u8) -> Option<bool> {
    let ac = if ac { 1 } else { 0 };
    let response = send_data(comms::DaemonCommand::SetBrightness { ac, val })?;

    use comms::DaemonResponse::*;
    match response {
        SetBrightness { result } => Some(result),
        response => {
            // This should not happen
            println!("Instead of SetBrightness got {response:?}");
            None
        }
    }
}

fn get_logo(ac: bool) -> Option<u8> {
    let ac = if ac { 1 } else { 0 };
    let response = send_data(comms::DaemonCommand::GetLogoLedState { ac })?;

    use comms::DaemonResponse::*;
    match response {
        GetLogoLedState { logo_state } => Some(logo_state),
        response => {
            // This should not happen
            println!("Instead of GetLogoLedState got {response:?}");
            None
        }
    }
}

fn set_logo(ac: bool, logo_state: u8) -> Option<bool> {
    let ac = if ac { 1 } else { 0 };
    let response = send_data(comms::DaemonCommand::SetLogoLedState { ac, logo_state })?;

    use comms::DaemonResponse::*;
    match response {
        SetLogoLedState { result } => Some(result),
        response => {
            // This should not happen
            println!("Instead of SetLogoLedState got {response:?}");
            None
        }
    }
}

fn set_effect(name: &str, values: Vec<u8>) -> Option<bool> {
    let response = send_data(comms::DaemonCommand::SetEffect {
        name: name.into(),
        params: values,
    })?;

    use comms::DaemonResponse::*;
    match response {
        SetEffect { result } => Some(result),
        response => {
            // This should not happen
            println!("Instead of SetEffect got {response:?}");
            None
        }
    }
}

fn get_cooling_pad_state() -> Option<CoolingPadState> {
    let response = try_send_data(comms::DaemonCommand::GetCoolingPadState)?;

    use comms::DaemonResponse::*;
    match response {
        GetCoolingPadState {
            present,
            fan_rpm,
            effect,
            effect_params,
        } => Some((present, fan_rpm, effect, effect_params)),
        response => {
            println!("Instead of GetCoolingPadState got {response:?}");
            None
        }
    }
}

fn set_cooling_pad_fan_speed(value: i32) -> Option<bool> {
    let response = send_data(comms::DaemonCommand::SetCoolingPadFanSpeed { rpm: value })?;

    use comms::DaemonResponse::*;
    match response {
        SetCoolingPadFanSpeed { result } => Some(result),
        response => {
            println!("Instead of SetCoolingPadFanSpeed got {response:?}");
            None
        }
    }
}

fn set_cooling_pad_effect(name: &str, values: Vec<u8>) -> Option<bool> {
    let response = send_data(comms::DaemonCommand::SetCoolingPadEffect {
        name: name.into(),
        params: values,
    })?;

    use comms::DaemonResponse::*;
    match response {
        SetCoolingPadEffect { result } => Some(result),
        response => {
            println!("Instead of SetCoolingPadEffect got {response:?}");
            None
        }
    }
}

fn get_power(ac: bool) -> Option<(u8, u8, u8)> {
    let ac = if ac { 1 } else { 0 };
    let mut result = (0, 0, 0);

    let response = send_data(comms::DaemonCommand::GetPwrLevel { ac })?;
    use comms::DaemonResponse::*;
    match response {
        GetPwrLevel { pwr } => {
            result.0 = pwr;
        }
        response => {
            // This should not happen
            println!("Instead of GetPwrLevel got {response:?}");
            return None;
        }
    }

    let response = send_data(comms::DaemonCommand::GetCPUBoost { ac })?;
    match response {
        GetCPUBoost { cpu } => {
            result.1 = cpu;
        }
        response => {
            // This should not happen
            println!("Instead of GetCPUBoost got {response:?}");
            return None;
        }
    }

    let response = send_data(comms::DaemonCommand::GetGPUBoost { ac })?;
    match response {
        GetGPUBoost { gpu } => {
            result.2 = gpu;
        }
        response => {
            // This should not happen
            println!("Instead of GetGPUBoost got {response:?}");
            return None;
        }
    }

    Some(result)
}

fn set_power(ac: bool, power: (u8, u8, u8)) -> Option<bool> {
    let ac = if ac { 1 } else { 0 };
    let response = send_data(comms::DaemonCommand::SetPowerMode {
        ac,
        pwr: power.0,
        cpu: power.1,
        gpu: power.2,
    })?;

    use comms::DaemonResponse::*;
    match response {
        SetPowerMode { result } => Some(result),
        response => {
            // This should not happen
            println!("Instead of SetPowerMode got {response:?}");
            None
        }
    }
}

fn get_fan_speed(ac: bool) -> Option<i32> {
    let ac = if ac { 1 } else { 0 };
    let response = send_data(comms::DaemonCommand::GetFanSpeed { ac })?;

    use comms::DaemonResponse::*;
    match response {
        GetFanSpeed { rpm } => Some(rpm),
        response => {
            // This should not happen
            println!("Instead of GetFanSpeed got {response:?}");
            None
        }
    }
}

fn set_fan_speed(ac: bool, value: i32) -> Option<bool> {
    let ac = if ac { 1 } else { 0 };
    let response = send_data(comms::DaemonCommand::SetFanSpeed { ac, rpm: value })?;

    use comms::DaemonResponse::*;
    match response {
        SetFanSpeed { result } => Some(result),
        response => {
            // This should not happen
            println!("Instead of SetFanSpeed got {response:?}");
            None
        }
    }
}

fn main() {
    setup_panic_hook();
    gtk::init().or_crash("Failed to initialize GTK.");

    let device_file = service::get_device_data();
    let devices: Vec<SupportedDevice> =
        serde_json::from_str(&device_file).or_crash("Failed to parse the device file");

    let device_name = get_device_name().or_crash("Failed to get device name");

    let app = Application::builder()
        .application_id("com.example.hello") // TODO: Change this name
        .build();

    app.connect_activate(move |app| {
        // For now we get the device from the device name. One is duplicated but
        // its settings are the same.
        // TODO: Document this or make it more robust
        let device = devices
            .iter()
            .find(|d| d.name == device_name)
            .or_crash("Failed to get device info");

        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(640)
            .default_height(480)
            .title("Razer Settings")
            .window_position(gtk::WindowPosition::Center)
            .build();

        let ac_settings_page = make_page(true, device.clone());
        let battery_settings_page = make_page(false, device.clone());
        let general_page = make_general_page();
        let initial_cooling_pad_state =
            get_cooling_pad_state().unwrap_or_else(|| (false, 0, String::new(), vec![]));
        let (cooling_pad_page, cooling_pad_ui) =
            make_cooling_pad_page(initial_cooling_pad_state.clone());
        let cooling_pad_container = cooling_pad_page.master_container.clone();
        let cooling_pad_visible = Rc::new(Cell::new(initial_cooling_pad_state.0));
        let about_page = make_about_page(device.clone());

        let stack = Stack::new();
        stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);

        stack.add_titled(&ac_settings_page.master_container, "AC", "AC");
        stack.add_titled(
            &battery_settings_page.master_container,
            "Battery",
            "Battery",
        );
        stack.add_titled(&general_page.master_container, "General", "General");
        if cooling_pad_visible.get() {
            stack.add_titled(&cooling_pad_container, "CoolingPad", "Cooling Pad");
            stack.set_child_position(&cooling_pad_container, COOLING_PAD_TAB_POSITION);
        }
        stack.add_titled(&about_page.master_container, "About", "About");

        stack.connect_screen_changed(|_, _| {
            println!("Page changed");
        });

        let stack_switcher = StackSwitcher::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        stack_switcher.set_stack(Some(&stack));
        stack_switcher.set_halign(gtk::Align::Center);
        stack_switcher.connect_screen_changed(|_, _| {
            println!("Page changed");
        });

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let toolbar = Toolbar::new();
        toolbar.style_context().add_class("primary-toolbar");
        vbox.pack_start(&toolbar, false, false, 0);
        vbox.pack_start(&stack, true, true, 0);
        // header_bar.set_title(Some("Razer Settings"));
        // header_bar.set_child(Some(&stack_switcher));
        // window.set_titlebar(Some(&header_bar));
        let tool_item = ToolItem::new();
        gtk::prelude::ToolItemExt::set_expand(&tool_item, true);
        tool_item.style_context().add_class("raised");
        let stask_switcher_holder = Box::new(gtk::Orientation::Horizontal, 0);
        stask_switcher_holder.set_border_width(1);
        stask_switcher_holder.pack_start(&stack_switcher, true, true, 0);
        tool_item.add(&stask_switcher_holder);
        toolbar.insert(&tool_item, 0);

        window.set_child(Some(&vbox));

        window.show_all();

        let (cooling_pad_tx, cooling_pad_rx) =
            glib::MainContext::channel::<CoolingPadState>(glib::Priority::default());
        let cooling_pad_polling_active = Arc::new(AtomicBool::new(true));
        let cooling_pad_polling_shutdown = Arc::clone(&cooling_pad_polling_active);
        window.connect_destroy(move |_| {
            cooling_pad_polling_shutdown.store(false, Ordering::Relaxed);
        });
        let cooling_pad_polling_thread = Arc::clone(&cooling_pad_polling_active);
        thread::spawn(move || {
            while cooling_pad_polling_thread.load(Ordering::Relaxed) {
                if let Some(state) = get_cooling_pad_state() {
                    if cooling_pad_tx.send(state).is_err() {
                        break;
                    }
                }

                thread::sleep(Duration::from_secs(2));
            }
        });
        cooling_pad_rx.attach(
            None,
            clone!(
                @weak stack,
                @strong cooling_pad_container,
                @strong cooling_pad_visible,
                @strong cooling_pad_ui
                => @default-return glib::ControlFlow::Break, move |state| {
                    let present = state.0;
                    if present && !cooling_pad_visible.get() {
                        stack.add_titled(&cooling_pad_container, "CoolingPad", "Cooling Pad");
                        stack.set_child_position(&cooling_pad_container, COOLING_PAD_TAB_POSITION);
                        cooling_pad_container.show_all();
                        cooling_pad_visible.set(true);
                    } else if !present && cooling_pad_visible.get() {
                        if stack.visible_child_name().as_deref() == Some("CoolingPad") {
                            stack.set_visible_child_name("General");
                        }
                        stack.remove(&cooling_pad_container);
                        cooling_pad_visible.set(false);
                    }

                    cooling_pad_ui.apply_state(&state);
                    glib::ControlFlow::Continue
                }
            ),
        );

        // If we know we are not running on AC, we show the battery tab by
        // default
        match check_if_running_on_ac_power() {
            Some(false) => stack.set_visible_child_name("Battery"),
            _ => {}
        }
    });

    app.run();
}

fn make_page(ac: bool, device: SupportedDevice) -> SettingsPage {
    let fan_speed = get_fan_speed(ac).or_crash("Error reading fan speed");
    let brightness = get_brightness(ac).or_crash("Error reading brightness");
    let power = get_power(ac);

    let min_fan_speed = *device.fan.get(0).or_crash("Invalid fan values") as f64;
    let max_fan_speed = *device.fan.get(1).or_crash("Invalid fan values") as f64;

    let settings_page = SettingsPage::new();

    // Logo section
    if device.has_logo() {
        let logo = get_logo(ac).or_crash("Error reading logo");
        let settings_section = settings_page.add_section(Some("Logo"));
        let label = Label::new(Some("Turn on logo"));
        let logo_options = ComboBoxText::new();
        logo_options.append_text("Off");
        logo_options.append_text("On");
        logo_options.append_text("Breathing");
        logo_options.set_active(Some(logo as u32));
        logo_options.connect_changed(move |options| {
            let logo = options.active().or_crash("Illegal state") as u8;
            set_logo(ac, logo);
            let logo = get_logo(ac).or_crash("Error reading logo").clamp(0, 2);
            options.set_active(Some(logo as u32));
        });
        let row = SettingsRow::new(&label, &logo_options);
        settings_section.add_row(&row.master_container);
    }

    // Power section
    if let Some(power) = power {
        let settings_section = settings_page.add_section(Some("Power"));
        let label = Label::new(Some("Power Profile"));
        let power_profile = ComboBoxText::new();
        power_profile.append_text("Balanced");
        power_profile.append_text("Gaming");
        power_profile.append_text("Creator");
        power_profile.append_text("Silent");
        power_profile.append_text("Custom");
        power_profile.set_active(Some(power.0 as u32));
        power_profile.set_width_request(100);
        let row = SettingsRow::new(&label, &power_profile);
        settings_section.add_row(&row.master_container);
        let label = Label::new(Some("CPU Boost"));
        let cpu_boost = ComboBoxText::new();
        cpu_boost.append_text("Low");
        cpu_boost.append_text("Medium");
        cpu_boost.append_text("High");
        if device.can_boost() {
            cpu_boost.append_text("Boost")
        };
        cpu_boost.set_active(Some(power.1 as u32));
        cpu_boost.set_width_request(100);
        let row = SettingsRow::new(&label, &cpu_boost);
        let cpu_boost_row = &row.master_container;
        settings_section.add_row(cpu_boost_row);
        let label = Label::new(Some("GPU Boost"));
        let gpu_boost = ComboBoxText::new();
        gpu_boost.append_text("Low");
        gpu_boost.append_text("Medium");
        gpu_boost.append_text("High");
        gpu_boost.set_active(Some(power.2 as u32));
        gpu_boost.set_width_request(100);
        let row = SettingsRow::new(&label, &gpu_boost);
        let gpu_boost_row = &row.master_container;
        settings_section.add_row(gpu_boost_row);

        cpu_boost_row.show_all();
        cpu_boost_row.set_no_show_all(true);
        gpu_boost_row.show_all();
        gpu_boost_row.set_no_show_all(true);
        if power.0 == 4 {
            cpu_boost_row.set_visible(true);
            gpu_boost_row.set_visible(true);
        } else {
            cpu_boost_row.set_visible(false);
            gpu_boost_row.set_visible(false);
        }

        power_profile.connect_changed(clone!(
            @weak cpu_boost, @weak gpu_boost,
            @weak cpu_boost_row, @weak gpu_boost_row
            =>
            move |power_profile| {
                let profile = power_profile.active().or_crash("Illegal state") as u8;
                let cpu     = cpu_boost.active().or_crash("Illegal state") as u8;
                let gpu     = gpu_boost.active().or_crash("Illegal state") as u8;
                set_power(ac, (profile, cpu, gpu)).or_crash("Error setting power");

                let power = get_power(ac).or_crash("Error reading power");
                power_profile.set_active(Some(power.0 as u32));
                cpu_boost.set_active(Some(power.1 as u32));
                gpu_boost.set_active(Some(power.2 as u32));

                if power.0 == 4 {
                    cpu_boost_row.set_visible(true);
                    gpu_boost_row.set_visible(true);
                } else {
                    cpu_boost_row.set_visible(false);
                    gpu_boost_row.set_visible(false);
                }
            }
        ));
        cpu_boost.connect_changed(clone!(
            @weak power_profile, @weak gpu_boost
            =>
            move |cpu_boost| {
                let profile = power_profile.active().or_crash("Illegal state") as u8;
                let cpu     = cpu_boost.active().or_crash("Illegal state") as u8;
                let gpu     = gpu_boost.active().or_crash("Illegal state") as u8;
                set_power(ac, (profile, cpu, gpu)).or_crash("Error setting power");

                let power = get_power(ac).or_crash("Error reading power");
                power_profile.set_active(Some(power.0 as u32));
                cpu_boost.set_active(Some(power.1 as u32));
                gpu_boost.set_active(Some(power.2 as u32));
            }
        ));
        gpu_boost.connect_changed(clone!(
            @weak power_profile, @weak cpu_boost
            =>
            move |gpu_boost| {
                let profile = power_profile.active().or_crash("Illegal state") as u8;
                let cpu     = cpu_boost.active().or_crash("Illegal state") as u8;
                let gpu     = gpu_boost.active().or_crash("Illegal state") as u8;
                set_power(ac, (profile, cpu, gpu)).or_crash("Error setting power");

                let power = get_power(ac).or_crash("Error reading power");
                power_profile.set_active(Some(power.0 as u32));
                cpu_boost.set_active(Some(power.1 as u32));
                gpu_boost.set_active(Some(power.2 as u32));
            }
        ));
    }

    // Fan Speed Section
    let settings_section = settings_page.add_section(Some("Fan Speed"));
    let label = Label::new(Some("Auto"));
    let switch = Switch::new();
    let auto = fan_speed == 0;
    switch.set_state(auto);
    let row = SettingsRow::new(&label, &switch);
    settings_section.add_row(&row.master_container);
    let label = Label::new(Some("Fan Speed"));
    let scale = Scale::with_range(
        gtk::Orientation::Horizontal,
        min_fan_speed,
        max_fan_speed,
        1f64,
    );
    scale.set_value(fan_speed as f64);
    scale.set_sensitive(fan_speed != 0);
    scale.set_width_request(100);
    scale.connect_change_value(clone!(@weak switch => @default-return gtk::glib::Propagation::Stop, move |scale, _, value| {
            let value = value.clamp(min_fan_speed, max_fan_speed);
            set_fan_speed(ac, value as i32).or_crash("Error setting fan speed");
            let fan_speed = get_fan_speed(ac).or_crash("Error reading fan speed");
            let auto = fan_speed == 0;
            scale.set_value(fan_speed as f64);
            scale.set_sensitive(!auto);
            switch.set_state(auto);
            return gtk::glib::Propagation::Stop;
        }));
    switch.connect_changed_active(clone!(@weak scale => move |switch| {
            set_fan_speed(ac, if switch.is_active() { 0 } else { min_fan_speed as i32 }).or_crash("Error setting fan speed");
            let fan_speed = get_fan_speed(ac).or_crash("Error reading fan speed");
            let auto = fan_speed == 0;
            scale.set_value(fan_speed as f64);
            scale.set_sensitive(!auto);
            switch.set_state(auto);
        }));
    let row = SettingsRow::new(&label, &scale);
    settings_section.add_row(&row.master_container);

    // Keyboard Section
    let settings_section = settings_page.add_section(Some("Keyboard"));
    let label = Label::new(Some("Brightness"));
    let scale = Scale::with_range(gtk::Orientation::Horizontal, 0f64, 100f64, 1f64);
    scale.set_value(brightness as f64);
    scale.set_width_request(100);
    scale.connect_change_value(move |scale, _, value| {
        let value = value.clamp(0f64, 100f64);
        set_brightness(ac, value as u8).or_crash("Error setting brightness");
        let brightness = get_brightness(ac).or_crash("Error reading brightness");
        scale.set_value(brightness as f64);
        gtk::glib::Propagation::Stop
    });
    let row = SettingsRow::new(&label, &scale);
    settings_section.add_row(&row.master_container);

    settings_page
}

fn make_general_page() -> SettingsPage {
    let bho = get_bho();

    let page = SettingsPage::new();

    // Keyboard Section
    let settings_section = page.add_section(Some("Keyboard"));
    let label = Label::new(Some("Effect"));
    let effect_options = ComboBoxText::new();
    effect_options.append_text("Static");
    effect_options.append_text("Static Gradient");
    effect_options.append_text("Wave Gradient");
    effect_options.append_text("Breathing");
    effect_options.set_active(Some(0));
    let row = SettingsRow::new(&label, &effect_options);
    settings_section.add_row(&row.master_container);
    let label = Label::new(Some("Color 1"));
    let color_picker = ColorButton::new();
    let row = SettingsRow::new(&label, &color_picker);
    settings_section.add_row(&row.master_container);
    let label = Label::new(Some("Color 2"));
    let color_picker_2 = ColorButton::new();
    let row = SettingsRow::new(&label, &color_picker_2);
    settings_section.add_row(&row.master_container);
    let label = Label::new(Some("Write effect"));
    let button = Button::with_label("Write");
    button.connect_clicked(
        clone!(@weak effect_options, @weak color_picker, @weak color_picker_2 =>
            move |_| {
                let color = color_picker.color();
                let red   = (color.red   / 256) as u8;
                let green = (color.green / 256) as u8;
                let blue  = (color.blue  / 256) as u8;

                let color = color_picker_2.color();
                let red2   = (color.red   / 256) as u8;
                let green2 = (color.green / 256) as u8;
                let blue2  = (color.blue  / 256) as u8;

                let effect = effect_options.active().or_crash("Illegal state");
                match effect {
                    0 => {
                        set_effect("static", vec![red, green, blue])
                            .or_crash("Failed to set effect");
                    },
                    1 => {
                        set_effect(
                            "static_gradient",
                            vec![red, green, blue, red2, green2, blue2]
                        ).or_crash("Failed to set effect");
                    },
                    2 => {
                        set_effect("wave_gradient",
                            vec![red, green, blue, red2, green2, blue2]
                        ).or_crash("Failed to set effect");
                    }
                    3 => {
                        set_effect(
                            "breathing_single",
                            vec![red, green, blue, 10]
                        ).or_crash("Failed to set effect");
                    }
                    _ => {}
                }
            }
        ),
    );
    let row = SettingsRow::new(&label, &button);
    settings_section.add_row(&row.master_container);

    effect_options.connect_changed(clone!(@weak color_picker, @weak color_picker_2 =>
        move |options| {
            let logo = options.active().or_crash("Illegal state"); // Unwrap: There is always one active

            match logo {
                0 => {
                    // TODO: Color 1 visible
                },
                1 => {
                    // TODO: Color 1 and 2 visible
                },
                2 => {
                    // TODO: Color 1 and 2 visible
                }
                3 => {
                    // TODO: Color 1, 2, and duration visible
                }
                _ => {}
            }
        }
    ));

    // Battery Health Optimizer section
    if let Some(bho) = bho {
        let settings_section = page.add_section(Some("Battery Health Optimizer"));
        let label = Label::new(Some("Enable Battery Health Optimizer"));
        let switch = Switch::new();
        switch.set_state(bho.0);
        let row = SettingsRow::new(&label, &switch);
        settings_section.add_row(&row.master_container);
        let label = Label::new(Some("Threshold"));
        let scale = Scale::with_range(gtk::Orientation::Horizontal, 65f64, 80f64, 1f64);
        scale.set_value(bho.1 as f64);
        scale.set_width_request(100);
        scale.connect_change_value(clone!(@weak switch => @default-return gtk::glib::Propagation::Stop, move |scale, _, value| {
                let is_on = switch.is_active();
                let threshold = value.clamp(50f64, 80f64) as u8;

                set_bho(is_on, threshold).or_crash("Error setting bho");

                let (is_on, threshold) = get_bho().or_crash("Error reading bho");

                scale.set_value(threshold as f64);
                scale.set_visible(is_on);
                scale.set_sensitive(is_on);

                return gtk::glib::Propagation::Stop;
            }));
        scale.set_sensitive(bho.0);
        switch.connect_changed_active(clone!(@weak scale => move |switch| {
            let is_on = switch.is_active();
            let threshold = scale.value().clamp(50f64, 80f64) as u8;

            set_bho(is_on, threshold); // Ignoramos errores ya que leemos
                                       // el resultado de vuelta

            let (is_on, threshold) = get_bho().or_crash("Error reading bho");

            scale.set_value(threshold as f64);
            scale.set_visible(is_on);
            scale.set_sensitive(is_on);
        }));
        let row = SettingsRow::new(&label, &scale);
        settings_section.add_row(&row.master_container);
    }

    page
}

fn sync_cooling_pad_controls(switch: &Switch, scale: &Scale, present: bool, fan_rpm: i32) {
    let auto = fan_rpm == 0;
    switch.set_sensitive(present);
    if switch.is_active() != auto {
        switch.set_state(auto);
    }

    scale.set_sensitive(present && !auto);
    let target_value = if fan_rpm > 0 { fan_rpm as f64 } else { 500f64 };
    if (scale.value() - target_value).abs() > f64::EPSILON {
        scale.set_value(target_value);
    }
}

fn cooling_pad_effect_index(name: &str) -> u32 {
    match name {
        "static" => 1,
        "breathing" => 2,
        "wave" => 3,
        _ => 0,
    }
}

fn cooling_pad_effect_name(index: u32) -> &'static str {
    match index {
        1 => "static",
        2 => "breathing",
        3 => "wave",
        _ => "off",
    }
}

fn cooling_pad_effect_uses_color(effect: &str) -> bool {
    matches!(effect, "static" | "breathing")
}

fn sync_cooling_pad_color_picker(color_picker: &ColorButton, effect_params: &[u8]) {
    let red = f64::from(*effect_params.first().unwrap_or(&255)) / 255f64;
    let green = f64::from(*effect_params.get(1).unwrap_or(&0)) / 255f64;
    let blue = f64::from(*effect_params.get(2).unwrap_or(&0)) / 255f64;
    let rgba = gtk::gdk::RGBA::new(red, green, blue, 1f64);
    color_picker.set_rgba(&rgba);
}

fn make_cooling_pad_page(state: CoolingPadState) -> (SettingsPage, Rc<CoolingPadUi>) {
    let page = SettingsPage::new();

    let fan_section = page.add_section(Some("Fan"));
    let label = Label::new(Some("Auto"));
    let fan_auto_switch = Switch::new();
    let row = SettingsRow::new(&label, &fan_auto_switch);
    fan_section.add_row(&row.master_container);

    let label = Label::new(Some("Fan Speed"));
    let fan_scale = Scale::with_range(gtk::Orientation::Horizontal, 500f64, 3200f64, 50f64);
    fan_scale.set_width_request(100);
    let row = SettingsRow::new(&label, &fan_scale);
    fan_section.add_row(&row.master_container);

    sync_cooling_pad_controls(&fan_auto_switch, &fan_scale, state.0, state.1);

    fan_scale.connect_change_value(clone!(
        @weak fan_auto_switch
        => @default-return gtk::glib::Propagation::Stop, move |scale, _, value| {
            let value = value.clamp(500f64, 3200f64) as i32;
            let _ = set_cooling_pad_fan_speed(value);
            sync_cooling_pad_controls(&fan_auto_switch, scale, true, value);
            gtk::glib::Propagation::Stop
        }
    ));

    fan_auto_switch.connect_changed_active(clone!(
        @weak fan_scale
        => move |switch| {
            let target = if switch.is_active() { 0 } else { fan_scale.value().clamp(500f64, 3200f64) as i32 };
            let _ = set_cooling_pad_fan_speed(target);
            sync_cooling_pad_controls(switch, &fan_scale, true, target);
        }
    ));

    let rgb_section = page.add_section(Some("RGB"));
    let label = Label::new(Some("Effect"));
    let effect_options = ComboBoxText::new();
    effect_options.append_text("Off");
    effect_options.append_text("Static");
    effect_options.append_text("Breathing");
    effect_options.append_text("Wave");
    effect_options.set_active(Some(cooling_pad_effect_index(&state.2)));
    let row = SettingsRow::new(&label, &effect_options);
    rgb_section.add_row(&row.master_container);

    let label = Label::new(Some("Color"));
    let color_picker = ColorButton::new();
    sync_cooling_pad_color_picker(&color_picker, &state.3);
    let row = SettingsRow::new(&label, &color_picker);
    let color_row = row.master_container;
    rgb_section.add_row(&color_row);
    color_row.set_visible(cooling_pad_effect_uses_color(&state.2));

    effect_options.connect_changed(clone!(
        @weak color_row
        => move |options| {
            let effect = cooling_pad_effect_name(options.active().unwrap_or(0));
            color_row.set_visible(cooling_pad_effect_uses_color(effect));
        }
    ));

    let label = Label::new(Some("Write effect"));
    let write_button = Button::with_label("Write");
    write_button.set_sensitive(state.0);
    effect_options.set_sensitive(state.0);
    color_picker.set_sensitive(state.0);
    write_button.connect_clicked(clone!(
        @weak effect_options, @weak color_picker
        => move |_| {
            let effect = cooling_pad_effect_name(effect_options.active().unwrap_or(0));
            let color = color_picker.rgba();
            let red = (color.red() * 255f64).round().clamp(0f64, 255f64) as u8;
            let green = (color.green() * 255f64).round().clamp(0f64, 255f64) as u8;
            let blue = (color.blue() * 255f64).round().clamp(0f64, 255f64) as u8;

            let params = match effect {
                "static" | "breathing" => vec![red, green, blue],
                _ => vec![],
            };

            let _ = set_cooling_pad_effect(effect, params);
        }
    ));
    let row = SettingsRow::new(&label, &write_button);
    rgb_section.add_row(&row.master_container);

    let cooling_pad_ui = Rc::new(CoolingPadUi {
        fan_auto_switch,
        fan_scale,
        effect_options,
        color_row,
        color_picker,
        write_button,
    });
    cooling_pad_ui.apply_state(&state);

    (page, cooling_pad_ui)
}

fn make_about_page(device: SupportedDevice) -> SettingsPage {
    let page = SettingsPage::new();

    // About page
    let settings_section = page.add_section(Some("Razer Laptop Control"));
    let label = Label::new(Some("Project"));
    let url = LinkButton::with_label(
        "https://github.com/JosuGZ/razer-laptop-control",
        "Project repository",
    );
    let row = SettingsRow::new(&label, &url);
    settings_section.add_row(&row.master_container);
    let report_bug_url = "https://github.com/JosuGZ/razer-laptop-control/issues/new?labels=bug&template=bug_report.md&title=%5BBUG%5D";
    let label = Label::new(Some("Bug reports"));
    let url = LinkButton::with_label(report_bug_url, "Report bug");
    let row = SettingsRow::new(&label, &url);
    settings_section.add_row(&row.master_container);
    let label = Label::new(Some("Discord"));
    let url = LinkButton::with_label("https://discord.gg/GdHKf45", "Razer Linux");
    let row = SettingsRow::new(&label, &url);
    settings_section.add_row(&row.master_container);

    // Model section
    let settings_section = page.add_section(Some("Laptop Information"));
    let label = Label::new(Some("Model"));
    let model_label = Label::new(Some(&device.name));
    let row = SettingsRow::new(&label, &model_label);
    settings_section.add_row(&row.master_container);
    let label = Label::new(Some("Features"));
    let features = device.features.join(", ");
    let features_label = Label::new(Some(&features));
    let row = SettingsRow::new(&label, &features_label);
    settings_section.add_row(&row.master_container);

    page
}
