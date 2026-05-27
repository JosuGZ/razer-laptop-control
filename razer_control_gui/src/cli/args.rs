use clap::{command, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(version="0.5.0", about="razer laptop configuration for linux", name="razer-cli")]
pub struct Cli {
    #[command(subcommand)]
    pub args: Args,
}

#[derive(Subcommand)]
pub enum Args {
    /// Read the current configuration of the device for some attribute
    Read {
        #[command(subcommand)]
        attr: ReadAttr,
    },
    /// Write a new configuration to the device for some attribute
    Write {
        #[command(subcommand)]
        attr: WriteAttr,
    },
    /// Write a standard effect
    StandardEffect {
        #[command(subcommand)]
        effect: StandardEffect,
    },
    /// Write a custom effect
    Effect {
        #[command(subcommand)]
        effect: Effect,
    },
    /// Displays information about present Razer Devices
    DeviceInfo
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OnOff {
    On,
    Off,
}

impl OnOff {
    pub fn is_on(&self) -> bool {
        matches!(self, Self::On)
    }
}

#[derive(Subcommand)]
pub enum ReadAttr {
    /// Read the current fan speed
    Fan(AcStateParam),
    /// Read the current power mode
    Power(AcStateParam),
    /// Read the current brightness
    Brightness(AcStateParam),
    /// Read the current logo mode
    Logo(AcStateParam),
    /// Read the current sync mode
    Sync,
    /// Read the current bho mode
    Bho,
    /// Read if light control is enabled
    LightControl,
}

#[derive(Subcommand)]
pub enum WriteAttr {
    /// Set the fan speed
    Fan(FanParams),
    /// Set the power mode
    Power(PowerParams),
    /// Set the brightness of the keyboard
    Brightness(BrightnessParams),
    /// Set the logo mode
    Logo(LogoParams),
    /// Set sync
    Sync(SyncParams),
    /// Set battery health optimization
    Bho(BhoParams),
    /// Enable or disable light control
    LightControl(LightControlParams),
}

#[derive(Parser)]
pub struct PowerParams {
    /// battery/plugged in
    pub ac_state: AcState,
    /// power mode (0, 1, 2, 3 or 4)
    pub pwr: u8,
    /// cpu boost (0, 1, 2 or 3)
    pub cpu_mode: Option<u8>,
    /// gpu boost (0, 1 or 2)
    pub gpu_mode: Option<u8>,
}

#[derive(Parser)]
pub struct FanParams {
    /// battery/plugged in
    pub ac_state: AcState,
    /// fan speed in RPM
    pub speed: i32,
}

#[derive(Parser)]
pub struct BrightnessParams {
    /// battery/plugged in
    pub ac_state: AcState,
    /// brightness
    pub brightness: i32,
}

#[derive(Parser)]
pub struct LogoParams {
    /// battery/plugged in
    pub ac_state: AcState,
    /// logo mode (0, 1 or 2)
    pub logo_state: i32,
}

#[derive(Parser)]
pub struct SyncParams {
    pub sync_state: OnOff,
}

#[derive(Parser)]
pub struct BhoParams {
    pub state: OnOff,
    /// charging threshold
    pub threshold: Option<u8>,
}

#[derive(Parser)]
pub struct LightControlParams {
    pub enable: OnOff,
}

#[derive(ValueEnum, Clone)]
pub enum AcState {
    /// battery
    Bat,
    /// plugged in
    Ac,
}

#[derive(Parser, Clone)]
pub struct AcStateParam {
    /// battery/plugged in
    pub ac_state: AcState,
}

#[derive(Subcommand)]
pub enum StandardEffect {
    Off,
    Wave(WaveParams),
    Reactive(ReactiveParams),
    Breathing(BreathingParams),
    Spectrum,
    Static(StaticParams),
    Starlight(StarlightParams),
}

#[derive(Parser)]
pub struct WaveParams {
    /// direction (0 or 1)
    pub direction: u8,
}

#[derive(Parser)]
pub struct ReactiveParams {
    /// speed (0-255)
    pub speed: u8,
    /// red (0-255)
    pub red: u8,
    /// green (0-255)
    pub green: u8,
    /// blue (0-255)
    pub blue: u8,
}

#[derive(Parser)]
pub struct BreathingParams {
    /// kind (0-2)
    pub kind: u8,
    /// red1 (0-255)
    pub red1: u8,
    /// green1 (0-255)
    pub green1: u8,
    /// blue1 (0-255)
    pub blue1: u8,
    /// red2 (0-255)
    pub red2: u8,
    /// green2 (0-255)
    pub green2: u8,
    /// blue2 (0-255)
    pub blue2: u8,
}

#[derive(Parser)]
pub struct StarlightParams {
    /// kind (0-2)
    pub kind: u8,
    /// speed (0-255)
    pub speed: u8,
    /// red1 (0-255)
    pub red1: u8,
    /// green1 (0-255)
    pub green1: u8,
    /// blue1 (0-255)
    pub blue1: u8,
    /// red2 (0-255)
    pub red2: u8,
    /// green2 (0-255)
    pub green2: u8,
    /// blue2 (0-255)
    pub blue2: u8,
}

#[derive(Subcommand)]
pub enum Effect {
    Static(StaticParams),
    StaticGradient(StaticGradientParams),
    WaveGradient(WaveGradientParams),
    BreathingSingle(BreathingSingleParams),
}

#[derive(Parser)]
pub struct StaticParams {
    /// red (0-255)
    pub red: u8,
    /// green (0-255)
    pub green: u8,
    /// blue (0-255)
    pub blue: u8,
}

#[derive(Parser)]
pub struct StaticGradientParams {
    /// red1 (0-255)
    pub red1: u8,
    /// green1 (0-255)
    pub green1: u8,
    /// blue1 (0-255)
    pub blue1: u8,
    /// red2 (0-255)
    pub red2: u8,
    /// green2 (0-255)
    pub green2: u8,
    /// blue2 (0-255)
    pub blue2: u8,
}

#[derive(Parser)]
pub struct WaveGradientParams {
    /// red1 (0-255)
    pub red1: u8,
    /// green1 (0-255)
    pub green1: u8,
    /// blue1 (0-255)
    pub blue1: u8,
    /// red2 (0-255)
    pub red2: u8,
    /// green2 (0-255)
    pub green2: u8,
    /// blue2 (0-255)
    pub blue2: u8,
}

#[derive(Parser)]
pub struct BreathingSingleParams {
    /// red (0-255)
    pub red: u8,
    /// green (0-255)
    pub green: u8,
    /// blue (0-255)
    pub blue: u8,
    /// duration (0-255)
    pub duration: u8,
}
