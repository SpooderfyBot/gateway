use std::time::Duration;

const GIGABYTE: f64 = (1024 * 1024 * 1024) as f64;
const MEGABYTE: f64 = (1024 * 1024) as f64;
const KILOBYTE: f64 = 1024 as f64;


/// Formats N amount of bytes into their readable form.
pub fn format_data(data_size: f64) -> String {
    if data_size >= GIGABYTE {
        format!("{:.2} GB", data_size / GIGABYTE)
    } else if data_size > MEGABYTE {
        format!("{:.2} MB", data_size / MEGABYTE)
    } else if data_size > KILOBYTE {
        format!("{:.2} KB", data_size / KILOBYTE)
    } else {
        format!("{:.2} B", data_size)
    }
}


/// Turns a fairly un-readable float in seconds / Duration into a human
/// friendly string.
///
/// E.g.
/// 10,000 seconds -> '2 hours, 46 minutes, 40 seconds'
pub fn humanize(time: Duration) -> String {
    let seconds = time.as_secs();

    let (minutes, seconds) = div_mod(seconds, 60);
    let (hours, minutes) = div_mod(minutes, 60);
    let (days, hours) = div_mod(hours, 24);

    let mut human = String::new();

    if days != 0 {
        human = format!("{} days, ", days);
    };

    if hours != 0 {
        human = format!("{}{} hours, ", human, hours);
    };

    if minutes != 0 {
        human = format!("{}{} minutes, ", human, minutes);
    };

    if seconds != 0 {
        human = format!("{}{} seconds", human, seconds);
    };

    human
}

/// Dirt simple div mod function.
pub fn div_mod(main: u64, divider: u64) -> (u64, u64) {
    let whole = main / divider;
    let rem = main % divider;

    (whole, rem)
}

