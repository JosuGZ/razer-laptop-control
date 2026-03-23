# Razer Laptop Control

This is an experimental application designed to control certain features
available on Razer laptops. It is not officially supported by Razer, nor does
it have dedicated maintainers for each device. Use it at your own risk.

Support for devices is community-driven. Each supported model has been at least
partially tested by the contributor who added it, but functionality may be
incomplete or vary between devices.

## Features:
- Light control (you can opt out if you are using OpenRazer)
- Battery charging limits
- Power profiles
- Fan control
- Logo control

## Unofficial Razer Linux Channel

You can find support on the following discord server, under the
'razer-laptop-control' channel: [Discord link](https://discord.gg/GdHKf45).

## Installation

[Install](razer_control_gui/README.md)

## Troubleshooting

When having problems with the application, please share the following
information:

- Journal: `journalctl --user-unit razercontrol > razercontrol.journal.log`
- Output from `razer-cli device-info`
- Output from `systemctl --user status razercontrol`
