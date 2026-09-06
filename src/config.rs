use std::{collections::HashMap, fs, io};

pub const CONFIG_FILE: &str = "config.txt";
pub const ANIMATION_FILE: &str = "anim.png";

pub struct Config {
    pub frame_count: u32,
    pub scale: f64,
    pub frame_delays: Vec<u16>,
    pub bounce_speed: f64,
    pub monitor: MonitorSelection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonitorSelection {
    All,
    Number(u32),
}

pub fn read() -> io::Result<Config> {
    parse(&fs::read_to_string(CONFIG_FILE)?)
}

fn parse(text: &str) -> io::Result<Config> {
    let mut values = HashMap::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let (key, value) = line.split_once('=').ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "config.txtの形式が不正です")
        })?;
        let key = key.trim().to_owned();
        if values.insert(key, value.trim().to_owned()).is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "config.txtに重複した設定項目があります",
            ));
        }
    }
    let get = |key: &str| {
        values.get(key).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("config.txtに{key}がありません"),
            )
        })
    };
    let frame_count: u32 = get("frame_count")?.parse().map_err(io::Error::other)?;
    if frame_count == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame_countには1以上を指定してください",
        ));
    }
    let frame_delays = if let Some(value) = values.get("frame_delays_ms") {
        let delays: Vec<u16> = value
            .split(',')
            .map(str::trim)
            .map(|value| {
                value
                    .parse::<u32>()
                    .ok()
                    .filter(|delay| *delay > 0 && *delay <= u16::MAX as u32)
                    .map(|delay| delay as u16)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "frame_delays_msには1以上65535以下のミリ秒を指定してください",
                        )
                    })
            })
            .collect::<io::Result<_>>()?;
        if delays.len() != frame_count as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "frame_delays_msの個数({})がframe_count({})と一致しません",
                    delays.len(),
                    frame_count
                ),
            ));
        }
        delays
    } else {
        let fps: f64 = get("anim_fps")?.parse().map_err(io::Error::other)?;
        if !fps.is_finite() || fps <= 0.0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "anim_fpsには0より大きい数値を指定してください",
            ));
        }
        vec![(1000.0 / fps).max(1.0) as u16; frame_count as usize]
    };
    let scale: f64 = get("scale")?.parse().map_err(io::Error::other)?;
    if !scale.is_finite() || scale <= 0.0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "scaleには0より大きい有限の数値を指定してください",
        ));
    }
    let bounce_speed: f64 = get("speed")?.parse().map_err(io::Error::other)?;
    if !bounce_speed.is_finite() || bounce_speed < 0.0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "speedには0以上の有限の数値を指定してください",
        ));
    }
    let monitor = match values.get("monitor").map(String::as_str).unwrap_or("all") {
        "all" => MonitorSelection::All,
        value => value.parse::<u32>().ok().filter(|number| *number > 0).map_or_else(
            || {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "monitorにはallまたは1以上のモニタ番号を指定してください",
                ))
            },
            |number| Ok(MonitorSelection::Number(number)),
        )?,
    };
    Ok(Config {
        frame_count,
        scale,
        frame_delays,
        bounce_speed,
        monitor,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse, MonitorSelection};

    const VALID_CONFIG: &str = "\
frame_count=2
scale=1.0
anim_fps=5.0
speed=1.0
";

    #[test]
    fn parses_valid_config() {
        let config = parse(VALID_CONFIG).unwrap();
        assert_eq!(config.frame_count, 2);
        assert_eq!(config.frame_delays, vec![200, 200]);
        assert_eq!(config.monitor, MonitorSelection::All);
    }

    #[test]
    fn parses_monitor_number() {
        let config = parse("frame_count=1\nscale=1.0\nanim_fps=5.0\nspeed=1.0\nmonitor=2\n")
            .unwrap();
        assert_eq!(config.monitor, MonitorSelection::Number(2));
    }

    #[test]
    fn parses_per_frame_delays() {
        let config =
            parse("frame_count=2\nscale=1.0\nanim_fps=5.0\nframe_delays_ms=200,100\nspeed=1.0\n")
                .unwrap();
        assert_eq!(config.frame_delays, vec![200, 100]);
    }

    #[test]
    fn rejects_zero_frame_count() {
        let result = parse(&VALID_CONFIG.replace("frame_count=2", "frame_count=0"));
        assert!(result.is_err());
    }

    #[test]
    fn rejects_non_finite_scale_and_speed() {
        for (key, value) in [("scale", "NaN"), ("speed", "inf")] {
            let config = VALID_CONFIG.replace(&format!("{key}=1.0"), &format!("{key}={value}"));
            assert!(parse(&config).is_err(), "{key}={value} should be rejected");
        }
    }

    #[test]
    fn rejects_duplicate_keys() {
        let config = format!("{VALID_CONFIG}speed=2.0\n");
        assert!(parse(&config).is_err());
    }
}
