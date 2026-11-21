use std::{env, io, process::Command};

use serde::Deserialize;

const COMPACT_SIZE: [i32; 2] = [620, 330];
const MARGIN: i32 = 16;

#[derive(Deserialize)]
pub struct Placement {
    address: String,
    pid: u32,
    at: [i32; 2],
    size: [i32; 2],
    monitor: i32,
    floating: bool,
    pinned: bool,
    fullscreen: i32,
}

#[derive(Deserialize)]
struct Monitor {
    id: i32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    scale: f64,
    transform: u8,
    reserved: [i32; 4],
}

pub fn available() -> bool {
    env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

fn hyprctl(args: &[&str]) -> io::Result<String> {
    let output = Command::new("hyprctl").args(args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success() || (args.first() == Some(&"eval") && stdout != "ok") {
        return Err(io::Error::other(format!("Hyprland: {stdout}")));
    }
    Ok(stdout)
}

impl Monitor {
    fn bottom_right(&self, size: [i32; 2]) -> [i32; 2] {
        let (width, height) = if self.transform.is_multiple_of(2) {
            (self.width, self.height)
        } else {
            (self.height, self.width)
        };
        let [left, top, right, bottom] = self.reserved;
        [
            self.x
                + (left + MARGIN)
                    .max((f64::from(width) / self.scale) as i32 - right - size[0] - MARGIN),
            self.y
                + (top + MARGIN)
                    .max((f64::from(height) / self.scale) as i32 - bottom - size[1] - MARGIN),
        ]
    }
}

impl Placement {
    pub fn pin() -> io::Result<Self> {
        let current: Self = serde_json::from_str(&hyprctl(&["-j", "activewindow"])?)?;
        if current.pid != std::process::id() || current.fullscreen != 0 {
            return Err(io::Error::other(
                "Pin requires the active, non-fullscreen window",
            ));
        }
        let monitors: Vec<Monitor> = serde_json::from_str(&hyprctl(&["-j", "monitors"])?)?;
        let monitor = monitors
            .iter()
            .find(|monitor| monitor.id == current.monitor)
            .ok_or_else(|| io::Error::other("Window monitor not found"))?;
        current.apply(true, true, COMPACT_SIZE, monitor.bottom_right(COMPACT_SIZE))?;
        Ok(current)
    }

    pub fn restore(&self) -> io::Result<()> {
        self.apply(self.floating, self.pinned, self.size, self.at)
    }

    fn apply(
        &self,
        floating: bool,
        pinned: bool,
        [w, h]: [i32; 2],
        [x, y]: [i32; 2],
    ) -> io::Result<()> {
        let address = self
            .address
            .strip_prefix("0x")
            .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or_else(|| io::Error::other("Invalid window address"))?;
        let pin_action = if pinned { "set" } else { "unset" };
        let actions = if floating {
            format!(
                "run(d.float({{window=w,action='set'}})); run(d.resize({{window=w,x={w},y={h}}})); run(d.move({{window=w,x={x},y={y}}})); run(d.pin({{window=w,action='{pin_action}'}}))"
            )
        } else {
            "run(d.pin({window=w,action='unset'})); run(d.float({window=w,action='unset'}))".into()
        };
        let script = format!(
            "local d=hl.dsp.window; local w='address:0x{address}'; local run=hl.dispatch; {actions}; run(hl.dsp.focus({{window=w}}))"
        );
        hyprctl(&["eval", &script]).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottom_right_respects_scale_rotation_and_reserved_space() {
        let monitor = Monitor {
            id: 0,
            x: -1080,
            y: 120,
            width: 2160,
            height: 3840,
            scale: 2.0,
            transform: 1,
            reserved: [0, 26, 40, 20],
        };
        assert_eq!(monitor.bottom_right(COMPACT_SIZE), [164, 834]);
    }
}
