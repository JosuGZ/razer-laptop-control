# Razer laptop control project
An application designed for Razer notebooks

## Join the unofficial Razer linux Channel
I can be contacted on this discord server under the 'laptop-control' channel
[Discord link](https://discord.gg/GdHKf45)

## Install
[Install](razer_control_gui/README.md)

## What does this control
On all razer notebooks, the following is supported:
* RGB keyboard control (Experimental)
* Fan control
* Power control
* Battery optimizer

## Supported Models

The following Razer laptop models are currently supported:

| Model | VID | PID | Features | Fan Range (RPM) |
|-------|-----|-----|----------|-----------------|
| Blade 14 2021 | 1532 | 0270 | logo | 3500-5000 |
| Blade 15 2016 | 1532 | 0224 | logo | 3500-5000 |
| Blade 2017 pro | 1532 | 0225 | logo | 3500-5000 |
| Blade 2017 pro | 1532 | 0210 | logo | 3500-5000 |
| Blade 2017 stealth | 1532 | 022D | logo | 3500-5000 |
| Blade 2018 15 Mercury edition | 1532 | 0240 | logo | 3500-5000 |
| Blade 2018 15 advanced | 1532 | 0233 | logo | 3500-5000 |
| Blade 2018 15 base | 1532 | 023B | logo | 3500-5000 |
| Blade 2018 pro FHD | 1532 | 022F | logo | 3500-5000 |
| Blade 2019 15 Mercury edition | 1532 | 0245 | logo, creator_mode | 3500-5300 |
| Blade 2019 15 advanced | 1532 | 023A | logo, creator_mode | 3500-5300 |
| Blade 2019 15 base | 1532 | 0246 | logo | 3500-5000 |
| Blade 2019 pro | 1532 | 0234 | logo | 3500-5300 |
| Blade 2019 stealth | 1532 | 0239 | logo | 3500-5300 |
| Blade 2019 stealth (With GTX) | 1532 | 024A | logo | 3500-5000 |
| Blade 2020 15 advanced | 1532 | 0253 | logo, creator_mode, boost | 3500-5300 |
| Blade 2020 15 base | 1532 | 0255 | logo | 3500-5000 |
| Blade 2020 pro | 1532 | 0256 | logo | 3500-5300 |
| Blade 2020 stealth | 1532 | 0252 | logo | 3500-5000 |
| Blade 2021 15 advanced | 1532 | 0276 | logo, boost | 3500-5000 |
| Blade 2021 15 base | 1532 | 026F | logo | 3500-5000 |
| Blade 2022 14 | 1532 | 028C | logo, boost, bho | 3500-5000 |
| Blade 2022 17 | 1532 | 028B | logo, boost | 3500-5000 |
| Blade 2023 14 | 1532 | 029D | logo, boost, bho | 2200-5000 |
| Blade 2024 14 | 1532 | 02b6 | logo, boost, bho | 2200-5000 |
| Blade Early 2021 17 pro | 1532 | 026E | logo, boost | 2300-4300 |
| Blade Mid 2021 17 pro | 1532 | 0279 | logo, boost | 2300-4300 |
| Blade QHD | 1532 | 020F | logo | 3500-5000 |
| Blade early 2022 15 advanced | 1532 | 028A | logo, boost, bho | 3500-5000 |
| Blade late 16 | 1532 | 029F | logo, boost, bho | 2200-5000 |
| Blade late 2020 stealth | 1532 | 0259 | logo | 3500-5000 |
| Blade late 2021 15 advanced | 1532 | 026D | logo, boost | 3500-5000 |
| Blade late 2021 15 base | 1532 | 027A | logo | 3500-5000 |
| Book 2020 13 | 1532 | 026A | None | 3500-5000 |
| Razer Blade 16 2024 | 1532 | 02b7 | logo, boost, bho | 2200-4500 |
| Razer Blade 18 2023 | 1532 | 02a0 | logo, boost, bho | 2200-5000 |
| Razer Blade 18 2024 | 1532 | 02b8 | logo, boost, bho | 2200-4500 |
| late Blade 2017 stealth | 1532 | 0232 | logo | 3500-5000 |

**Features:**
- `logo`: RGB logo control
- `creator_mode`: Creator mode power profiles
- `boost`: CPU boost control
- `bho`: Battery Health Optimizer

## Requesting Support for a New Model

If your Razer laptop model is not listed above, you can request support by following these steps:

### 1. Find your laptop's USB VID and PID

You can find your laptop's USB Vendor ID (VID) and Product ID (PID) using one of these methods:

#### Method A: Using this application's CLI tool
```bash
# After installing razer-laptop-control
razer-cli device-info
```

#### Method B: Using lsusb (Linux)
```bash
lsusb | grep -i razer
```
Look for a line like: `Bus 001 Device 003: ID 1532:029d Razer USA, Ltd`
- VID = `1532` (always the same for Razer devices)
- PID = `029d` (specific to your model)

#### Method C: Using System Information (Windows with dual boot)
1. Open Device Manager
2. Find your Razer device under "Human Interface Devices" or "Keyboards"
3. Right-click → Properties → Details → Hardware Ids
4. Look for `VID_1532&PID_XXXX` where XXXX is your product ID

### 2. Gather Additional Information

When requesting support for your model, please provide:

1. **Device Information:**
   - Exact model name (e.g., "Razer Blade 15 2023")
   - USB VID and PID (found above)
   - Year and specific variant if applicable

2. **Screenshots from Razer Synapse (if you have Windows):**
   - Performance tab showing all power modes and options
   - Fan control settings (auto/manual ranges)
   - Any special features like Creator Mode
   - Battery Health Optimizer settings (if available)

3. **Feature Testing:**
   - Does the logo RGB work?
   - Are there multiple power modes?
   - Can you control fan speeds manually?
   - Is there a Battery Health Optimizer?
   - Any special gaming/creator modes?

### 3. Submit Your Request

Create a new issue in this repository with the title format: `[Feature] Support for [Your Model Name]`

Include in your issue:
```json
{
   "name": "Your Model Name Here",
   "vid": "1532",
   "pid": "YOUR_PID_HERE",
   "features": ["logo", "boost", "bho"],
   "fan": [2200, 5000]
}
```

**Example request format:**
```
Device Information:
- Model: Razer Blade 15 2024
- VID: 1532
- PID: 02c1

Synapse Screenshots:
[Attach screenshots of performance settings, fan controls, etc.]

Tested Features:
- Logo RGB: Yes
- Fan Control: Manual range 2200-5000 RPM
- Power Modes: Balanced, Gaming, Creator, Custom
- Battery Health Optimizer: 50-80% range
- CPU Boost: Available
```

### 4. What Happens Next

1. A maintainer will review your request
2. The device definition will be added to the supported devices list
3. You'll be asked to test the implementation
4. Once confirmed working, support will be officially added

### Tips for Better Support

- **Be specific**: "Blade 15 2023 Advanced" is better than just "Blade 15"
- **Include all variants**: If there are multiple configs (base/advanced), mention which one
- **Test thoroughly**: Help verify that all features work as expected
- **Screenshots help**: Visual confirmation from Synapse is very valuable

For questions or help with the request process, join our [Discord server](https://discord.gg/GdHKf45) in the #laptop-control channel.
